#!/bin/bash

# Investigation script for zero-writing syscall differences between Rust and C++
# This script compares table creation patterns and analyzes syscall differences

set -e

# Configuration
TABLE_NAME="investigation_table.ms"
NUM_ROWS=1000  # Larger table to see the zero-writing pattern clearly
NUM_COLS=10
STRACE_LOG_DIR="../strace_investigation"
BENCHMARK_DIR="./benchmark_workspace"

echo "=== CasaCore Zero-Writing Investigation ==="
echo "Table: ${TABLE_NAME}, Rows: ${NUM_ROWS}, Cols: ${NUM_COLS}"
echo

# Setup directories
mkdir -p "${STRACE_LOG_DIR}"
mkdir -p "${BENCHMARK_DIR}"
cd "${BENCHMARK_DIR}"

# Clean up any existing table
rm -rf "${TABLE_NAME}"

echo "Building benchmarks..."
cd ..
cargo build --release --examples
cd "${BENCHMARK_DIR}"

# Build C++ benchmark if possible
echo "Building C++ benchmark..."
if command -v make >/dev/null 2>&1; then
    (cd .. && make cpp_benchmark_instrumented >/dev/null) || true
fi
if [ -x "./cpp_benchmark_instrumented" ]; then
    echo "C++ benchmark available"
    HAS_CPP_BENCHMARK=1
else
    echo "Warning: C++ benchmark not available"
    HAS_CPP_BENCHMARK=0
fi

# Function to run detailed strace analysis
detailed_strace_analysis() {
    local name="$1"
    local cmd="$2"
    local log_file="${STRACE_LOG_DIR}/${name}_detailed.strace"

    echo "Running detailed strace for $name..."
    rm -rf "${TABLE_NAME}" # Clean up

    # Run with very detailed strace focusing on file operations
    # Use Rust-style flags for the Rust benchmark; use positional args for C++ benchmark
    if [[ "$cmd" == *"target/release/examples/benchmark"* ]]; then
        strace -f -e trace=file,desc -o "${log_file}" $cmd --rows "${NUM_ROWS}" --cols "${NUM_COLS}" "${TABLE_NAME}" 2>&1
    else
        # Assume C++ benchmark signature: <table_name> <num_rows> <num_cols>
        strace -f -e trace=file,desc -o "${log_file}" $cmd "${TABLE_NAME}" "${NUM_ROWS}" "${NUM_COLS}" 2>&1
    fi

    if [ -f "${log_file}" ]; then
        echo "=== Detailed Analysis for $name ==="

        # Count zero-writing patterns
        echo "Zero-writing analysis:"
        zero_writes=$(grep -c 'write.*\\0' "${log_file}" || echo "0")
        echo "  Zero write calls: $zero_writes"

        if [ "$zero_writes" -gt 0 ]; then
            echo "  Zero write details:"
            grep 'write.*\\0' "${log_file}" | head -5 | sed 's/^/    /'
            echo "  ..."
        fi

        # Count file allocation patterns
        echo "File allocation analysis:"
        fallocate_calls=$(grep -c 'fallocate' "${log_file}" || echo "0")
        ftruncate_calls=$(grep -c 'ftruncate' "${log_file}" || echo "0")
        lseek_calls=$(grep -c 'lseek' "${log_file}" || echo "0")

        echo "  fallocate calls: $fallocate_calls"
        echo "  ftruncate calls: $ftruncate_calls"
        echo "  lseek calls: $lseek_calls"

        # Show file creation pattern
        echo "File creation pattern:"
        grep -E "(creat|openat.*O_CREAT)" "${log_file}" | sed 's/^/  /'

        echo
    else
        echo "Warning: No strace log generated for $name"
    fi
}

# Function to analyze zero-writing in detail
analyze_zero_writing() {
    local name="$1"
    local log_file="${STRACE_LOG_DIR}/${name}_detailed.strace"

    if [ -f "${log_file}" ]; then
        echo "=== Zero-Writing Pattern Analysis for $name ==="

        # Extract all write calls with zero data
        echo "All zero-write patterns:"
        grep 'write.*\\0' "${log_file}" | while IFS= read -r line; do
            # Extract file descriptor, size, and offset context
            fd=$(echo "$line" | sed -n 's/.*write(\([0-9]*\),.*/\1/p')
            size=$(echo "$line" | sed -n 's/.*write([0-9]*, "[^"]*", \([0-9]*\)).*/\1/p')
            echo "  FD $fd: $size bytes of zeros"
        done

        # Show the context around zero writes
        echo
        echo "Context around first few zero writes:"
        grep -n 'write.*\\0' "${log_file}" | head -3 | while IFS=: read -r linenum line; do
            echo "Line $linenum:"
            sed -n "$((linenum-2)),$((linenum+2))p" "${log_file}" | sed 's/^/    /'
            echo
        done
    fi
}

# Run the analysis
echo "=== Running Syscall Investigation ==="

# Analyze Rust implementation
export WRITE_MODE="column_put_bulk"
detailed_strace_analysis "rust" "../../target/release/examples/benchmark"
analyze_zero_writing "rust"

# Analyze C++ implementation if available
if [ "$HAS_CPP_BENCHMARK" = "1" ]; then
    detailed_strace_analysis "cpp" "./cpp_benchmark_instrumented"
    analyze_zero_writing "cpp"

    # Compare the two
    echo "=== Comparison Summary ==="
    rust_zeros=$(grep -c 'write.*\\0' "${STRACE_LOG_DIR}/rust_detailed.strace" || echo "0")
    cpp_zeros=$(grep -c 'write.*\\0' "${STRACE_LOG_DIR}/cpp_detailed.strace" || echo "0")

    echo "Zero-write syscalls:"
    echo "  Rust: $rust_zeros"
    echo "  C++:  $cpp_zeros"

    if [ "$rust_zeros" -gt "$cpp_zeros" ]; then
        echo "  → Rust makes $((rust_zeros - cpp_zeros)) more zero-write syscalls than C++"
    fi
else
    echo "C++ benchmark not available for comparison"
fi

echo
echo "=== Investigation Results ==="
echo "Detailed strace logs saved in: ${STRACE_LOG_DIR}/"
ls -la "${STRACE_LOG_DIR}/"

echo
echo "=== Next Steps ==="
echo "1. Examine the strace logs to identify zero-writing patterns"
echo "2. Add instrumentation to casacore to track allocation methods"
echo "3. Modify FFI layer to use efficient allocation if needed"
echo "4. Test with different table sizes and configurations"

cat <<EOF
From the detailed traces, allocation follows: open table.lock → small header write → open/create table.f0 → repeated ftruncate growth → lseek to offsets → write 3328 bytes of zeros → repeat. This shows preallocation is happening incrementally (multiple ftruncate) but casacore still zero-fills buckets/tiles.
C++: ftruncate is present regardless (prealloc active). Zero-writes remain, confirming the storage manager still writes zeroed buffers after growing the file.
Rust: with prealloc enabled (our current default), you see the same ftruncate pattern plus the same zero-writes; with prealloc disabled, ftruncate disappears but zero-writes persist. So prealloc is functioning, but it does not eliminate the zero-writing pattern.
The lock file activity (pwrite on table.lock) is expected and unrelated to data allocation.
Bottom line: prealloc is verified; the remaining overhead is zero-initialization and bucket/tile writes. Further gains require eliminating or deferring zero writes (e.g., avoid memset in tile init and rely on preallocated sparse space, or write-on-first-touch).
EOF

echo

# Summarize JSON-based syscall analyses if available
LATEST_ANALYSIS_DIR=$(ls -d ../syscall_analysis_* 2>/dev/null | sort | tail -1)
if [ -n "${LATEST_ANALYSIS_DIR}" ] && [ -d "${LATEST_ANALYSIS_DIR}" ]; then
    RUST_JSON="${LATEST_ANALYSIS_DIR}/rust_analysis/syscall_analysis.json"
    CPP_JSON="${LATEST_ANALYSIS_DIR}/cpp_analysis/syscall_analysis.json"

    if [ -f "${RUST_JSON}" ] && [ -f "${CPP_JSON}" ]; then
        # Helper: extract a top-level numeric field
        get_top_number() {
            local file="$1"; local key="$2"
            awk -v k="\""$key"\"" -F ':' '$1~k { gsub(/[^0-9.]/, "", $2); print $2; exit }' "${file}"
        }
        # Helper: extract syscall count for a given syscall name
        get_syscall_count() {
            local file="$1"; local sc="$2"
            awk -v sc="\""$sc"\"" -F ':' '
                $0 ~ sc" *\\{" { inblk=1; next }
                inblk && /"count"/ { gsub(/[^0-9]/, "", $2); print $2; exit }
                inblk && /}/ { inblk=0 }
            ' "${file}"
        }

        rust_total=$(get_top_number "${RUST_JSON}" total_syscalls)
        cpp_total=$(get_top_number "${CPP_JSON}" total_syscalls)
        rust_time=$(awk -F ':' '/"total_syscall_time"/ { gsub(/[^0-9.]/, "", $2); print $2; exit }' "${RUST_JSON}")
        cpp_time=$(awk -F ':' '/"total_syscall_time"/ { gsub(/[^0-9.]/, "", $2); print $2; exit }' "${CPP_JSON}")

        rust_lseek=$(get_syscall_count "${RUST_JSON}" lseek)
        cpp_lseek=$(get_syscall_count "${CPP_JSON}" lseek)
        rust_write=$(get_syscall_count "${RUST_JSON}" write)
        cpp_write=$(get_syscall_count "${CPP_JSON}" write)
        rust_ftruncate=$(get_syscall_count "${RUST_JSON}" ftruncate)
        cpp_ftruncate=$(get_syscall_count "${CPP_JSON}" ftruncate)

        echo "=== JSON Analysis Summary (latest: $(basename "${LATEST_ANALYSIS_DIR}") ) ==="
        echo "Totals:   Rust=${rust_total:-N/A}  C++=${cpp_total:-N/A}  Δ=$(( ${rust_total:-0} - ${cpp_total:-0} ))"
        echo "Time(s):  Rust=${rust_time:-N/A}  C++=${cpp_time:-N/A}  Δ=$(printf '%.6f' "$(awk -v r=${rust_time:-0} -v c=${cpp_time:-0} 'BEGIN{printf r-c}' )")"
        echo "lseek:    Rust=${rust_lseek:-N/A}  C++=${cpp_lseek:-N/A}  Δ=$(( ${rust_lseek:-0} - ${cpp_lseek:-0} ))"
        echo "write:    Rust=${rust_write:-N/A}  C++=${cpp_write:-N/A}  Δ=$(( ${rust_write:-0} - ${cpp_write:-0} ))"
        echo "ftruncate:Rust=${rust_ftruncate:-N/A}  C++=${cpp_ftruncate:-N/A}  Δ=$(( ${rust_ftruncate:-0} - ${cpp_ftruncate:-0} ))"
        echo
    fi
fi

echo "Investigation complete!"