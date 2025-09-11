#!/bin/bash

# Simple script to format existing strace logs for easier diffing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "🔬 Formatting Existing Strace Logs for Diffing"
echo "=============================================="

# Check if we have strace logs
if [ ! -d "strace_logs" ] || [ -z "$(ls strace_logs/*.strace 2>/dev/null)" ]; then
    echo "❌ No strace logs found in strace_logs/ directory"
    echo "💡 Run ./enhanced_strace_analysis.sh first to generate fresh logs"
    exit 1
fi

# Create output directory
mkdir -p strace_diffs

echo "📝 Processing available strace log pairs..."

# Process each pair of rust/cpp logs
for rust_log in strace_logs/rust_*.strace; do
    if [ -f "$rust_log" ]; then
        basename=$(basename "$rust_log" .strace)
        rust_type=${basename#rust_}
        cpp_log="strace_logs/cpp_${rust_type}.strace"
        
        if [ -f "$cpp_log" ]; then
            echo "🔄 Processing: $rust_type"
            
            # Generate formatted versions and comparison
            python3 format_strace_for_diff.py "$rust_log" \
                -c "$cpp_log" --output-dir "strace_diffs"
            
            # Create a focused diff on just the syscalls we care about
            RUST_FORMATTED="strace_diffs/$(basename "$rust_log" .strace)_formatted.txt"
            CPP_FORMATTED="strace_diffs/$(basename "$cpp_log" .strace)_formatted.txt"
            DIFF_FILE="strace_diffs/${rust_type}_comparison.diff"
            
            echo "📊 Creating diff: $DIFF_FILE"
            diff -u "$CPP_FORMATTED" "$RUST_FORMATTED" > "$DIFF_FILE" || true
            
            # Extract just the interesting syscalls for easier reading
            FOCUSED_DIFF="strace_diffs/${rust_type}_focused.txt"
            echo "# Focused comparison for $rust_type" > "$FOCUSED_DIFF"
            echo "# Key syscall differences between C++ and Rust" >> "$FOCUSED_DIFF"
            echo "" >> "$FOCUSED_DIFF"
            
            # Show write operations side by side
            echo "## WRITE OPERATIONS" >> "$FOCUSED_DIFF"
            echo "### C++ writes:" >> "$FOCUSED_DIFF"
            grep -A 20 "WRITE_OPS" "$CPP_FORMATTED" | head -25 >> "$FOCUSED_DIFF" 2>/dev/null || echo "No write ops found" >> "$FOCUSED_DIFF"
            echo "" >> "$FOCUSED_DIFF"
            echo "### Rust writes:" >> "$FOCUSED_DIFF"
            grep -A 20 "WRITE_OPS" "$RUST_FORMATTED" | head -25 >> "$FOCUSED_DIFF" 2>/dev/null || echo "No write ops found" >> "$FOCUSED_DIFF"
            
            echo "✅ Generated focused analysis: $FOCUSED_DIFF"
        else
            echo "⚠️  No matching C++ log for $rust_log (expected: $cpp_log)"
        fi
    fi
done

echo ""
echo "📁 Generated Files:"
ls -la strace_diffs/

echo ""
echo "🔍 Key files to examine:"
echo "   📊 Summary comparisons: strace_diffs/comparison_summary.txt"
echo "   🎯 Focused diffs: strace_diffs/*_focused.txt"
echo "   📋 Full diffs: strace_diffs/*_comparison.diff"
echo "   📝 Formatted logs: strace_diffs/*_formatted.txt"

echo ""
echo "💡 Usage examples:"
echo "   # View focused comparison"
echo "   less strace_diffs/column_put_bulk_detailed_focused.txt"
echo ""
echo "   # View side-by-side diff with color"
echo "   colordiff -u strace_diffs/cpp_column_put_bulk_detailed_formatted.txt strace_diffs/rust_column_put_bulk_detailed_formatted.txt | less -R"
echo ""
echo "   # Compare specific syscall types"
echo "   grep -A5 'WRITE_OPS\\|SEEK_OPS' strace_diffs/*_formatted.txt"