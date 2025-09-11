# Strace Analysis Tools for Rust vs C++ Performance Comparison

This directory contains tools for analyzing syscall differences between Rust FFI and C++ direct casacore implementations.

## 🔬 Analysis Tools

### 1. `format_strace_for_diff.py` - Core Formatting Tool
**Purpose**: Converts raw strace logs into normalized, diff-friendly format

**Features**:
- Normalizes addresses, file descriptors, and paths for consistent comparison
- Categorizes syscalls by type (file ops, memory, writes, reads, seeks)
- Analyzes write patterns (zero writes vs data writes)
- Generates summary statistics and comparisons

**Usage**:
```bash
# Format single strace log
python3 format_strace_for_diff.py input.strace -o output_formatted.txt

# Compare two strace logs
python3 format_strace_for_diff.py rust.strace -c cpp.strace --output-dir results/
```

### 2. `format_existing_straces.sh` - Batch Processing
**Purpose**: Processes all existing strace log pairs in `strace_logs/` directory

**Features**:
- Automatically finds matching Rust/C++ log pairs
- Generates formatted versions and comparison summaries
- Creates focused diffs highlighting key syscall differences

**Usage**:
```bash
./format_existing_straces.sh
```

### 3. `enhanced_strace_analysis.sh` - Complete Analysis Pipeline
**Purpose**: Generates fresh strace logs and performs comprehensive analysis

**Features**:
- Builds both Rust and C++ examples
- Runs identical operations with detailed strace capture
- Generates formatted output, patterns analysis, and side-by-side comparisons

**Usage**:
```bash
# Run with default parameters
./enhanced_strace_analysis.sh

# Custom parameters
./enhanced_strace_analysis.sh table_put_cell false
ROWS=50 ./enhanced_strace_analysis.sh column_put_bulk true
```

## 📊 Key Findings from Strace Analysis

### Current Performance Gap Summary
Based on analysis of existing logs:

```
Rust vs C++ Syscall Comparison (row_put_bulk_detailed):
========================================================
Rust total syscalls: 1716
C++ total syscalls:   104
Difference: +1612 (+1550.0%)

Category Breakdown:
file_ops    : Rust  58, C++  23, Diff +35 (+152.2%)
memory      : Rust  37, C++  47, Diff -10 (-21.3%)
write_ops   : Rust 396, C++   6, Diff +390 (+6500.0%)
read_ops    : Rust 397, C++  11, Diff +386 (+3509.1%)
seek_ops    : Rust 779, C++   0, Diff +779 (C++ has 0 seeks!)
other       : Rust  49, C++  17, Diff +32 (+188.2%)

Write Pattern Analysis:
Rust zero writes: 353 (89% of all writes are zeros!)
C++ zero writes:  0
Rust data writes: 43
C++ data writes:  6
```

### Key Observations

1. **Zero Write Pattern**: Rust implementation writes 353 zero-filled buffers while C++ writes 0
   - This represents the core performance issue beyond the "zero writes dead end"
   - Pattern shows massive file initialization overhead in Rust path

2. **Seek Operations**: Rust makes 779 lseek calls, C++ makes 0
   - Indicates different file access patterns between implementations
   - C++ may be using more efficient sequential writes or memory mapping

3. **File Operations**: Rust opens 35 more file handles than C++
   - Suggests different table structure initialization or locking strategies

## 🎯 Analysis Results in Action

### Generated Files Structure
```
strace_diffs/
├── comparison_summary.txt          # High-level statistics
├── *_focused.txt                   # Key differences by operation type
├── *_comparison.diff               # Full unified diffs
└── *_formatted.txt                 # Normalized strace logs
```

### Example Usage for Investigation

1. **View overall differences**:
   ```bash
   cat strace_diffs/comparison_summary.txt
   ```

2. **Compare write patterns side-by-side**:
   ```bash
   # With color highlighting
   colordiff -u strace_diffs/cpp_row_put_bulk_detailed_formatted.txt \
                strace_diffs/rust_row_put_bulk_detailed_formatted.txt | less -R
   
   # Focus on write operations only
   grep -A20 "WRITE_OPS" strace_diffs/*_formatted.txt
   ```

3. **Analyze specific syscall categories**:
   ```bash
   # Compare seek patterns
   grep -A10 "SEEK_OPS" strace_diffs/*_formatted.txt
   
   # Compare file operations
   grep -A15 "FILE_OPS" strace_diffs/*_formatted.txt
   ```

## 🔍 Investigation Insights

### Beyond Zero Writes
While the comment mentioned "zero writes thing seems to be a dead end", the strace analysis reveals the zero writes are actually a **major indicator** of the root cause:

1. **C++ Implementation**: 0 zero writes, 0 seeks
   - Uses efficient file allocation (likely fallocate/ftruncate)
   - Writes only actual data
   - Sequential access patterns

2. **Rust Implementation**: 353 zero writes, 779 seeks  
   - Initializes file space by writing zeros
   - Frequent seeking between file positions
   - Inefficient allocation strategy

### Root Cause Analysis
The massive syscall difference (1716 vs 104) indicates:
- **File Allocation Strategy**: Rust FFI path triggers inefficient casacore initialization
- **Storage Manager Differences**: Different code paths in casacore for Rust vs C++
- **Parameter Sensitivity**: Table creation parameters affect storage manager behavior

### Next Investigation Steps
Based on the formatted strace analysis:

1. **Focus on file allocation**: Why does Rust trigger zero-write initialization?
2. **Seek pattern analysis**: What causes 779 seeks in Rust vs 0 in C++?
3. **Storage manager behavior**: How do creation parameters affect casacore internals?
4. **Memory mapping**: Is C++ using mmap while Rust uses traditional I/O?

## 📋 Using the Tools

To reproduce the analysis:

```bash
# 1. Format existing logs for easy comparison
./format_existing_straces.sh

# 2. View the key differences
less strace_diffs/comparison_summary.txt

# 3. Create side-by-side diff
colordiff -u strace_diffs/cpp_*_detailed_formatted.txt \
             strace_diffs/rust_*_detailed_formatted.txt | less -R

# 4. Generate fresh analysis (optional)
./enhanced_strace_analysis.sh table_put_cell false
```

The formatted logs are now **easily diffable** and reveal the specific syscall patterns that cause the performance differences beyond just counting zero writes.