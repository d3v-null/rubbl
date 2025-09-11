#!/bin/bash

# Enhanced strace analysis script for Rust vs C++ comparison
# This script generates fresh strace logs and creates diff-friendly output

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Configuration
WRITE_MODE="${1:-table_put_cell}"
INITIALIZE="${2:-false}"
ROWS="${ROWS:-100}"
TEMP_DIR="/tmp/rubbl_strace_analysis"
STRACE_DIR="$SCRIPT_DIR/strace_analysis"

echo "🔬 Enhanced Strace Analysis: Rust vs C++"
echo "========================================="
echo "Mode: $WRITE_MODE, Initialize: $INITIALIZE, Rows: $ROWS"
echo

# Setup
rm -rf "$TEMP_DIR" "$STRACE_DIR"
mkdir -p "$TEMP_DIR" "$STRACE_DIR"

# Build both examples
echo "🔨 Building examples..."
cargo build --example write_ms >/dev/null 2>&1
(cd examples && make write_ms >/dev/null 2>&1)

# Run Rust with detailed strace
echo "⚡ Running Rust implementation with detailed strace..."
RUST_TABLE="$TEMP_DIR/rust.ms"
RUST_STRACE="$STRACE_DIR/rust_detailed.strace"
rm -rf "$RUST_TABLE"
strace -f -s 1024 -e trace=file,read,write,lseek -o "$RUST_STRACE" \
    ../target/debug/examples/write_ms "$RUST_TABLE" \
    -r "$ROWS" -w "$WRITE_MODE" -i "$INITIALIZE" >/dev/null 2>&1

# Run C++ with detailed strace  
echo "⚡ Running C++ implementation with detailed strace..."
CPP_TABLE="$TEMP_DIR/cpp.ms"
CPP_STRACE="$STRACE_DIR/cpp_detailed.strace"
rm -rf "$CPP_TABLE"
strace -f -s 1024 -e trace=file,read,write,lseek -o "$CPP_STRACE" \
    examples/write_ms -path "$CPP_TABLE" -rows "$ROWS" \
    -write_mode "$WRITE_MODE" -initialize "$INITIALIZE" >/dev/null 2>&1

# Generate formatted output for easy diffing
echo "📝 Formatting strace logs for analysis..."
python3 format_strace_for_diff.py "$RUST_STRACE" -c "$CPP_STRACE" --output-dir "$STRACE_DIR"

# Generate side-by-side diff of formatted files
echo "🔍 Creating side-by-side diff..."
RUST_FORMATTED="$STRACE_DIR/rust_detailed_formatted.txt"
CPP_FORMATTED="$STRACE_DIR/cpp_detailed_formatted.txt"

if command -v diff >/dev/null 2>&1; then
    diff -u "$CPP_FORMATTED" "$RUST_FORMATTED" > "$STRACE_DIR/side_by_side.diff" || true
    
    # Create a more readable diff showing just the write operations
    echo "## WRITE OPERATIONS COMPARISON" > "$STRACE_DIR/write_ops_comparison.txt"
    echo "### C++ Write Operations:" >> "$STRACE_DIR/write_ops_comparison.txt"
    grep -A5 -B1 "WRITE_OPS" "$CPP_FORMATTED" | head -20 >> "$STRACE_DIR/write_ops_comparison.txt" || true
    echo "" >> "$STRACE_DIR/write_ops_comparison.txt"
    echo "### Rust Write Operations:" >> "$STRACE_DIR/write_ops_comparison.txt"
    grep -A5 -B1 "WRITE_OPS" "$RUST_FORMATTED" | head -20 >> "$STRACE_DIR/write_ops_comparison.txt" || true
fi

# Extract key patterns for analysis
echo "🎯 Extracting key patterns..."

# File access patterns
echo "## File Access Patterns" > "$STRACE_DIR/pattern_analysis.txt"
echo "### Rust file operations:" >> "$STRACE_DIR/pattern_analysis.txt"
grep -E "(openat|mkdir|creat)" "$RUST_STRACE" | head -10 >> "$STRACE_DIR/pattern_analysis.txt" || true
echo "" >> "$STRACE_DIR/pattern_analysis.txt"
echo "### C++ file operations:" >> "$STRACE_DIR/pattern_analysis.txt"
grep -E "(openat|mkdir|creat)" "$CPP_STRACE" | head -10 >> "$STRACE_DIR/pattern_analysis.txt" || true

# Write patterns
echo "" >> "$STRACE_DIR/pattern_analysis.txt"
echo "## Write Patterns" >> "$STRACE_DIR/pattern_analysis.txt"
echo "### Rust writes with zero data:" >> "$STRACE_DIR/pattern_analysis.txt"
grep 'write(' "$RUST_STRACE" | grep -E '\\0|\\x00' | head -5 >> "$STRACE_DIR/pattern_analysis.txt" || true
echo "" >> "$STRACE_DIR/pattern_analysis.txt"
echo "### C++ writes with zero data:" >> "$STRACE_DIR/pattern_analysis.txt"
grep 'write(' "$CPP_STRACE" | grep -E '\\0|\\x00' | head -5 >> "$STRACE_DIR/pattern_analysis.txt" || true

# Seek patterns  
echo "" >> "$STRACE_DIR/pattern_analysis.txt"
echo "## Seek Patterns" >> "$STRACE_DIR/pattern_analysis.txt"
echo "### Rust lseek operations:" >> "$STRACE_DIR/pattern_analysis.txt"
grep 'lseek(' "$RUST_STRACE" | head -10 >> "$STRACE_DIR/pattern_analysis.txt" || true
echo "" >> "$STRACE_DIR/pattern_analysis.txt"
echo "### C++ lseek operations:" >> "$STRACE_DIR/pattern_analysis.txt"
grep 'lseek(' "$CPP_STRACE" | head -10 >> "$STRACE_DIR/pattern_analysis.txt" || true

# Summary
echo ""
echo "📊 Analysis Results:"
echo "==================="
cat "$STRACE_DIR/comparison_summary.txt"

echo ""
echo "📁 Generated Analysis Files:"
echo "----------------------------"
ls -la "$STRACE_DIR"

echo ""
echo "🔍 Key Files for Examination:"
echo "   📋 Summary:       $STRACE_DIR/comparison_summary.txt"
echo "   📊 Patterns:      $STRACE_DIR/pattern_analysis.txt"
echo "   ✏️  Write Ops:     $STRACE_DIR/write_ops_comparison.txt"
echo "   🔄 Side-by-side:  $STRACE_DIR/side_by_side.diff"
echo "   📝 Raw Rust:      $RUST_STRACE"
echo "   📝 Raw C++:       $CPP_STRACE"

echo ""
echo "💡 To examine differences:"
echo "   diff -u $STRACE_DIR/cpp_detailed_formatted.txt $STRACE_DIR/rust_detailed_formatted.txt | less"
echo "   colordiff -u $STRACE_DIR/cpp_detailed_formatted.txt $STRACE_DIR/rust_detailed_formatted.txt | less"