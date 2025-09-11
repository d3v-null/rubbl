#!/bin/bash

# Quick Rust vs C++ comparison script
# Usage: ./quick_compare.sh [write_mode] [initialize]
# Example: ./quick_compare.sh table_put_cell false

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Configuration
WRITE_MODE="${1:-table_put_cell}"
INITIALIZE="${2:-false}"
ROWS="${ROWS:-100}"
TEMP_DIR="/tmp/rubbl_quick_compare"

echo "🚀 Quick Rust vs C++ Comparison"
echo "==============================="
echo "Mode: $WRITE_MODE, Initialize: $INITIALIZE, Rows: $ROWS"
echo

# Setup
rm -rf "$TEMP_DIR"
mkdir -p "$TEMP_DIR"

# Build both examples
echo "🔨 Building examples..."
cargo build --example write_ms >/dev/null 2>&1
(cd examples && make write_ms >/dev/null 2>&1)

# Run Rust
echo "⚡ Running Rust implementation..."
RUST_TABLE="$TEMP_DIR/rust.ms"
RUST_STRACE="$TEMP_DIR/rust.strace"
rm -rf "$RUST_TABLE"
strace -f -s 256 -k -o "$RUST_STRACE" ../target/debug/examples/write_ms "$RUST_TABLE" \
    -r "$ROWS" -w "$WRITE_MODE" -i "$INITIALIZE" >/dev/null 2>&1

# Run C++
echo "⚡ Running C++ implementation..."
CPP_TABLE="$TEMP_DIR/cpp.ms"
CPP_STRACE="$TEMP_DIR/cpp.strace"
rm -rf "$CPP_TABLE"
strace -f -s 256 -k -o "$CPP_STRACE" examples/write_ms \
    -path "$CPP_TABLE" -rows "$ROWS" -write_mode "$WRITE_MODE" -initialize "$INITIALIZE" >/dev/null 2>&1

# Analyze results
echo "📊 Results:"
echo "==========="

analyze_strace() {
    local file="$1"
    local lang="$2"
    
    local total=$(wc -l < "$file")
    local writes=$(grep -c " write(" "$file" || echo "0")
    local lseeks=$(grep -c " lseek(" "$file" || echo "0")
    local opens=$(grep -c " open\|openat(" "$file" || echo "0")
    
    # Count zero writes
    local zeros
    if command -v python3 >/dev/null 2>&1; then
        zeros=$(python3 -c "
import re
zero_count = 0
with open('$file', 'r') as f:
    for line in f:
        if 'write(' in line and 'strace:' not in line:
            match = re.search(r'\"([^\"]*)', line)
            if match:
                data = match.group(1)
                if re.match(r'^(\\\\0)*$', data.replace('\\\\x00', '\\\\0')):
                    zero_count += 1
print(zero_count)
" 2>/dev/null || echo "0")
    else
        zeros="N/A"
    fi
    
    printf "%-6s %5d syscalls, %3d writes (%2d zeros), %3d lseeks, %2d opens\n" \
        "$lang:" "$total" "$writes" "$zeros" "$lseeks" "$opens"
}

analyze_strace "$RUST_STRACE" "Rust"
analyze_strace "$CPP_STRACE" "C++"

# File size comparison
if [ -d "$RUST_TABLE" ] && [ -d "$CPP_TABLE" ]; then
    echo
    echo "📁 File sizes:"
    RUST_SIZE=$(du -sb "$RUST_TABLE" | cut -f1)
    CPP_SIZE=$(du -sb "$CPP_TABLE" | cut -f1)
    DIFF=$((RUST_SIZE - CPP_SIZE))
    
    printf "Rust: %d bytes, C++: %d bytes, Difference: %d bytes\n" \
        "$RUST_SIZE" "$CPP_SIZE" "$DIFF"
fi

echo
echo "🔍 Detailed strace logs saved:"
echo "   Rust: $RUST_STRACE"
echo "   C++:  $CPP_STRACE"
echo
echo "💡 Usage examples:"
echo "   ./quick_compare.sh create_only false"
echo "   ./quick_compare.sh table_put_cell true"
echo "   ROWS=50 ./quick_compare.sh table_put_cell false"