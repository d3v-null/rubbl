#!/bin/bash

# Compare allocation methods between C++ and Rust implementations
# This will help identify the root cause of zero-writing differences

set -e

TABLE_NAME="allocation_test.ms"
NUM_ROWS=1000
NUM_COLS=10
STRACE_LOG_DIR="../strace_investigation"

echo "=== File Allocation Method Comparison ==="
echo "Table: ${TABLE_NAME}, Rows: ${NUM_ROWS}, Cols: ${NUM_COLS}"
echo

mkdir -p "${STRACE_LOG_DIR}"
cd benchmark_workspace

# Function to run allocation test
test_allocation_method() {
    local name="$1"
    local cmd="$2"
    local log_file="${STRACE_LOG_DIR}/${name}_allocation.strace"

    echo "Testing $name allocation..."
    rm -rf "${TABLE_NAME}"

    # Run with strace focusing on file operations
    # Check if cmd already contains arguments (for C++ benchmarks that are wrappers)
    if [[ $cmd == *" "* ]]; then
        # Command already has arguments, use as-is
        strace -f -e trace=file,desc -o "${log_file}" $cmd 2>&1 | tail -50
    else
        # Command needs arguments added
        strace -f -e trace=file,desc -o "${log_file}" $cmd "${TABLE_NAME}" 2>&1 | tail -50
    fi

    if [ -f "${log_file}" ]; then
        echo "=== Analysis for $name ==="

        # Count different syscall types
        zero_writes=$(grep -c 'write.*\\0' "${log_file}" 2>/dev/null || true); zero_writes=${zero_writes:-0}
        fallocate_calls=$(grep -c 'fallocate' "${log_file}" 2>/dev/null || true); fallocate_calls=${fallocate_calls:-0}
        ftruncate_calls=$(grep -c 'ftruncate' "${log_file}" 2>/dev/null || true); ftruncate_calls=${ftruncate_calls:-0}
        total_writes=$(grep -cE '\\b(write|pwrite64|pwrite|writev)\\(' "${log_file}" 2>/dev/null || true); total_writes=${total_writes:-0}
        total_seeks=$(grep -c 'lseek' "${log_file}" 2>/dev/null || true); total_seeks=${total_seeks:-0}

        echo "  Zero writes: $zero_writes"
        echo "  Total writes: $total_writes"
        echo "  fallocate calls: $fallocate_calls"
        echo "  ftruncate calls: $ftruncate_calls"
        echo "  lseek calls: $total_seeks"

        # Calculate efficiency metrics
        if [ "$total_writes" -gt 0 ]; then
            zero_ratio=$(echo "scale=2; $zero_writes * 100 / $total_writes" | bc -l 2>/dev/null || echo "N/A")
            echo "  Zero-write ratio: ${zero_ratio}%"
        fi

        # Show file size patterns
        echo "  File operations on data file:"
        grep 'table\.f0\|table\.dat' "${log_file}" | head -5 | sed 's/^/    /'

        echo
    fi
}

echo "Building benchmarks..."
cd ..
cargo build --release --examples
cd benchmark_workspace

echo
echo "=== Testing Rust Implementation ==="
test_allocation_method "rust" "../../target/release/examples/benchmark --rows ${NUM_ROWS} --cols ${NUM_COLS} ${TABLE_NAME}"

echo "=== Testing C++ Allocation Methods ==="

# Ensure C++ benchmark binary exists (rebuild if missing)
if [ ! -x ./cpp_benchmark_instrumented ]; then
    echo "Building cpp_benchmark_instrumented (wrapper that calls Rust directly)..."
    (cd .. && make cpp_benchmark_instrumented)
fi

# Test different C++ allocation strategies
for method in "zeros" "ftruncate" "fallocate"; do
    echo "--- C++ with ${method} allocation ---"
    export CPP_ALLOC_METHOD="$method"
    test_allocation_method "cpp_${method}" "../cpp_benchmark_instrumented ${TABLE_NAME} ${NUM_ROWS} ${NUM_COLS}"
    unset CPP_ALLOC_METHOD
done

echo "=== Summary Comparison ==="
echo "Method                | Zero Writes | Total Writes | Seeks | fallocate | ftruncate"
echo "--------------------|-----------|-----------|-----------|-----------|-----------"

for test in rust cpp_zeros cpp_ftruncate cpp_fallocate; do
    log_file="${STRACE_LOG_DIR}/${test}_allocation.strace"
    if [ -f "$log_file" ]; then
        zero_writes=$(grep -c 'write.*\\0' "$log_file" 2>/dev/null || true); zero_writes=${zero_writes:-0}
        total_writes=$(grep -cE '\\b(write|pwrite64|pwrite|writev)\\(' "$log_file" 2>/dev/null || true); total_writes=${total_writes:-0}
        total_seeks=$(grep -c 'lseek' "$log_file" 2>/dev/null || true); total_seeks=${total_seeks:-0}
        fallocate_calls=$(grep -c 'fallocate' "$log_file" 2>/dev/null || true); fallocate_calls=${fallocate_calls:-0}
        ftruncate_calls=$(grep -c 'ftruncate' "$log_file" 2>/dev/null || true); ftruncate_calls=${ftruncate_calls:-0}

        printf "%-20s| %9s | %9s | %9s | %9s | %9s\n" \
               "$test" "$zero_writes" "$total_writes" "$total_seeks" "$fallocate_calls" "$ftruncate_calls"
    fi
done

echo
echo "=== Key Findings ==="
echo "1. Efficient allocation methods (fallocate/ftruncate) should show:"
echo "   - Zero or minimal zero-write syscalls"
echo "   - Fewer total write operations"
echo "   - Reduced lseek operations"
echo "2. Inefficient allocation shows:"
echo "   - Many zero-write syscalls"
echo "   - High lseek count for positioning"
echo "   - Sequential writing of zero blocks"

echo
echo "=== Next Step: Casacore Instrumentation ==="
echo "Based on these results, we need to:"
echo "1. Identify why Rust FFI path uses inefficient allocation"
echo "2. Find the casacore functions responsible for file allocation"
echo "3. Modify FFI layer to use efficient allocation methods"
echo

echo "Allocation comparison complete!"