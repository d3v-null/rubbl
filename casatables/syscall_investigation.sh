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
if command -v g++ >/dev/null 2>&1; then
    # Try to find casacore libraries
    if pkg-config --exists casacore-tables 2>/dev/null; then
        echo "Using pkg-config for casacore"
        CASA_CFLAGS=$(pkg-config --cflags casacore-tables casacore-casa)
        CASA_LIBS=$(pkg-config --libs casacore-tables casacore-casa)
    else
        echo "Trying standard casacore locations"
        CASA_CFLAGS="-I/usr/include/casacore -I/usr/local/include/casacore"
        CASA_LIBS="-lcasa_tables -lcasa_casa"
    fi
    
    if g++ -std=c++11 ${CASA_CFLAGS} cpp_benchmark.cpp ${CASA_LIBS} -o cpp_benchmark 2>/dev/null; then
        echo "C++ benchmark compiled successfully"
        HAS_CPP_BENCHMARK=1
    else
        echo "Warning: Could not compile C++ benchmark"
        HAS_CPP_BENCHMARK=0
    fi
else
    echo "Warning: g++ not available"
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
    strace -f -e trace=file,desc -o "${log_file}" $cmd --rows "${NUM_ROWS}" --cols "${NUM_COLS}" "${TABLE_NAME}" 2>&1
    
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
    detailed_strace_analysis "cpp" "./cpp_benchmark"
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

echo
echo "Investigation complete!"