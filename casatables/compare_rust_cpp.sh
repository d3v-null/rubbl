#!/bin/bash

# Script to compare Rust and C++ table operations
# This runs identical table operations in both languages and compares results

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Configuration
TEMP_DIR="${TEMP_DIR:-/tmp/rubbl_comparison}"
RUST_TABLE_PATH="$TEMP_DIR/test_rust.ms"
CPP_TABLE_PATH="$TEMP_DIR/test_cpp.ms"
STRACE_DIR="$TEMP_DIR/strace_logs"

# Test parameters (can be overridden by environment)
ROWS="${ROWS:-100}"
TSM_OPTION="${TSM_OPTION:-DEFAULT}"
INITIALIZE="${INITIALIZE:-false}"
WRITE_MODE="${WRITE_MODE:-table_put_cell}"
DATA_SHAPE="${DATA_SHAPE:-32,4}"

echo "🚀 Rust vs C++ Table Operations Comparison"
echo "=========================================="
echo "Configuration:"
echo "  Rows: $ROWS"
echo "  TSM Option: $TSM_OPTION"
echo "  Initialize: $INITIALIZE"
echo "  Write Mode: $WRITE_MODE"
echo "  Data Shape: $DATA_SHAPE"
echo

# Setup directories
rm -rf "$TEMP_DIR"
mkdir -p "$TEMP_DIR" "$STRACE_DIR"

# Build Rust example
echo "🔨 Building Rust example..."
cargo build --example write_ms
RUST_BINARY="../target/debug/examples/write_ms"

# Build C++ example
echo "🔨 Building C++ example..."
cd examples
make write_ms
CPP_BINARY="./write_ms"
cd ..

echo "✅ Build completed"
echo

# Function to run with strace and collect metrics
run_with_strace() {
    local lang="$1"
    local binary="$2"
    local table_path="$3"
    local args="$4"
    local strace_file="$STRACE_DIR/${lang}_${WRITE_MODE}_${INITIALIZE}.strace"
    
    echo "📊 Running $lang implementation..."
    echo "   Command: $binary $args"
    echo "   Table: $table_path"
    echo "   Strace: $strace_file"
    
    # Clean up table path
    rm -rf "$table_path"
    
    # Run with strace
    strace -f -s 256 -k -o "$strace_file" $binary $args 2>/dev/null
    
    if [ $? -eq 0 ]; then
        echo "   ✅ $lang execution completed successfully"
    else
        echo "   ❌ $lang execution failed"
        return 1
    fi
    
    # Analyze strace output
    local total_syscalls=$(wc -l < "$strace_file")
    local write_calls=$(grep -c " write(" "$strace_file" || echo "0")
    local lseek_calls=$(grep -c " lseek(" "$strace_file" || echo "0")
    local open_calls=$(grep -c " open\|openat(" "$strace_file" || echo "0")
    
    # Count zero writes (writes with all zeros)
    local zero_writes
    if command -v python3 >/dev/null 2>&1; then
        zero_writes=$(python3 -c "
import re
import sys
zero_count = 0
with open('$strace_file', 'r') as f:
    for line in f:
        if 'write(' in line and 'strace:' not in line:
            # Extract the data part between quotes
            match = re.search(r'\"([^\"]*)', line)
            if match:
                data = match.group(1)
                # Check if it's all zeros (\\0 patterns)
                if re.match(r'^(\\\\0)*$', data.replace('\\\\x00', '\\\\0')):
                    zero_count += 1
print(zero_count)
" 2>/dev/null || echo "0")
    else
        zero_writes="N/A"
    fi
    
    echo "   📈 Syscalls: $total_syscalls total, $write_calls writes, $zero_writes zero-writes, $lseek_calls lseek, $open_calls open"
    
    # Store results for comparison
    eval "${lang}_total_syscalls=$total_syscalls"
    eval "${lang}_write_calls=$write_calls"
    eval "${lang}_zero_writes=$zero_writes"
    eval "${lang}_lseek_calls=$lseek_calls"
    eval "${lang}_open_calls=$open_calls"
}

# Function to compare file sizes
compare_file_sizes() {
    echo "📁 Comparing generated table sizes..."
    
    if [ -d "$RUST_TABLE_PATH" ] && [ -d "$CPP_TABLE_PATH" ]; then
        local rust_size=$(du -sb "$RUST_TABLE_PATH" | cut -f1)
        local cpp_size=$(du -sb "$CPP_TABLE_PATH" | cut -f1)
        
        echo "   Rust table size: $rust_size bytes"
        echo "   C++ table size:  $cpp_size bytes"
        
        if [ "$rust_size" -eq "$cpp_size" ]; then
            echo "   ✅ Table sizes match exactly"
        else
            local diff=$((rust_size - cpp_size))
            echo "   ⚠️  Size difference: $diff bytes"
        fi
    else
        echo "   ❌ One or both tables not found"
    fi
}

# Function to compare table contents (if possible)
compare_table_contents() {
    echo "🔍 Comparing table structures..."
    
    # Use tableinfo if available to compare table structures
    if command -v tableinfo >/dev/null 2>&1; then
        echo "   📊 Rust table info:"
        tableinfo "$RUST_TABLE_PATH" 2>/dev/null | head -10 | sed 's/^/      /'
        echo "   📊 C++ table info:"
        tableinfo "$CPP_TABLE_PATH" 2>/dev/null | head -10 | sed 's/^/      /'
    else
        echo "   ℹ️  tableinfo not available for structure comparison"
    fi
}

# Run tests for different scenarios
test_scenarios() {
    local scenarios=(
        "create_only false"
        "create_only true"
        "table_put_cell false"
    )
    
    echo "🧪 Running test scenarios..."
    echo
    
    for scenario in "${scenarios[@]}"; do
        local mode=$(echo "$scenario" | cut -d' ' -f1)
        local init=$(echo "$scenario" | cut -d' ' -f2)
        
        echo "🔬 Testing scenario: write_mode=$mode, initialize=$init"
        echo "================================================"
        
        # Set scenario-specific paths
        local rust_path="$TEMP_DIR/test_rust_${mode}_${init}.ms"
        local cpp_path="$TEMP_DIR/test_cpp_${mode}_${init}.ms"
        
        # Rust arguments
        local rust_args="$rust_path -r $ROWS -t $TSM_OPTION -i $init -w $mode -d $DATA_SHAPE"
        
        # C++ arguments
        local cpp_args="-path $cpp_path -rows $ROWS -tsm_option $TSM_OPTION -initialize $init -write_mode $mode -data_shape $DATA_SHAPE"
        
        # Run both implementations
        WRITE_MODE="$mode"
        INITIALIZE="$init"
        
        if run_with_strace "rust" "$RUST_BINARY" "$rust_path" "$rust_args" && \
           run_with_strace "cpp" "examples/$CPP_BINARY" "$cpp_path" "$cpp_args"; then
            
            # Store paths for comparison
            RUST_TABLE_PATH="$rust_path"
            CPP_TABLE_PATH="$cpp_path"
            
            # Compare results
            echo
            echo "📊 Comparison Results:"
            echo "====================="
            
            # Syscall comparison
            printf "%-15s %-10s %-10s %-12s %-10s %-10s\n" "Language" "Total" "Writes" "Zero-writes" "Lseek" "Open"
            printf "%-15s %-10s %-10s %-12s %-10s %-10s\n" "--------" "-----" "------" "-----------" "-----" "----"
            printf "%-15s %-10s %-10s %-12s %-10s %-10s\n" "Rust" "$rust_total_syscalls" "$rust_write_calls" "$rust_zero_writes" "$rust_lseek_calls" "$rust_open_calls"
            printf "%-15s %-10s %-10s %-12s %-10s %-10s\n" "C++" "$cpp_total_syscalls" "$cpp_write_calls" "$cpp_zero_writes" "$cpp_lseek_calls" "$cpp_open_calls"
            
            # Calculate differences
            if [ "$rust_zero_writes" != "N/A" ] && [ "$cpp_zero_writes" != "N/A" ]; then
                local zero_diff=$((rust_zero_writes - cpp_zero_writes))
                local write_diff=$((rust_write_calls - cpp_write_calls))
                
                echo
                echo "📈 Performance Analysis:"
                if [ "$zero_diff" -eq 0 ]; then
                    echo "   ✅ Zero-writes: IDENTICAL ($rust_zero_writes each)"
                else
                    echo "   ⚠️  Zero-writes: Rust has $zero_diff more than C++ ($rust_zero_writes vs $cpp_zero_writes)"
                fi
                
                if [ "$write_diff" -eq 0 ]; then
                    echo "   ✅ Total writes: IDENTICAL ($rust_write_calls each)"
                else
                    echo "   ⚠️  Total writes: Rust has $write_diff more than C++ ($rust_write_calls vs $cpp_write_calls)"
                fi
            fi
            
            echo
            compare_file_sizes
            echo
            compare_table_contents
            
        else
            echo "   ❌ One or both implementations failed for this scenario"
        fi
        
        echo
        echo "================================================================"
        echo
    done
}

# Main execution
main() {
    echo "🎯 Starting comprehensive Rust vs C++ comparison..."
    echo
    
    # Run test scenarios
    test_scenarios
    
    echo "📋 Summary"
    echo "=========="
    echo "Strace logs saved in: $STRACE_DIR"
    echo "Table files saved in: $TEMP_DIR"
    echo
    echo "🔍 To analyze specific strace logs:"
    echo "   grep 'write(' $STRACE_DIR/rust_*.strace | head -5"
    echo "   grep 'write(' $STRACE_DIR/cpp_*.strace | head -5"
    echo
    echo "📊 To get detailed syscall analysis:"
    echo "   ./analyze_syscalls.py $STRACE_DIR/rust_table_put_cell_false.strace"
    echo "   ./analyze_syscalls.py $STRACE_DIR/cpp_table_put_cell_false.strace"
    echo
    echo "✅ Comparison completed!"
}

# Run main function
main "$@"