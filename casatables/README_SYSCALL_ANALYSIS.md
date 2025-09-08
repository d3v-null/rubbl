# Casatables Syscall Analysis

This directory contains tools for analyzing system calls (syscalls) made by the casatables library across different programming languages. The goal is to understand the I/O patterns and performance characteristics of different implementations.

## Overview

System call analysis helps us understand:
- Which syscalls are most frequently used
- Memory allocation patterns
- I/O operations (file, network)
- Performance bottlenecks
- Cross-language comparison opportunities

## Files

- `examples/syscall_tracer.rs` - Rust example that performs casatables operations
- `analyze_syscalls.py` - Python script to analyze strace/dtruss output
- `syscall_tracer.py` - Python equivalent for cross-language comparison
- `syscall_tracer.cpp` - C++ equivalent for cross-language comparison
- `demo_syscall_analysis.sh` - Automated demonstration script
- `run_syscall_analysis.sh` - Original analysis script

## Usage

### 1. Rust Analysis

```bash
# Run the example with syscall tracing
strace -f -s 256 -k -o syscall_trace.txt cargo run --example syscall_tracer

# On macOS (requires sudo):
sudo dtruss -f -s -o syscall_trace.txt cargo run --example syscall_tracer

# Analyze the results
python3 analyze_syscalls.py syscall_trace.txt --format html
```

### 2. Python Analysis

```bash
# Run Python version with tracing
strace -f -s 256 -k -o python_trace.txt python3 syscall_tracer.py

# Analyze
python3 analyze_syscalls.py python_trace.txt --format html --title "Python Syscall Analysis"
```

### 3. C++ Analysis

```bash
# Compile and run C++ version
g++ -std=c++17 syscall_tracer.cpp -o syscall_tracer_cpp
strace -f -s 256 -k -o cpp_trace.txt ./syscall_tracer_cpp

# Analyze
python3 analyze_syscalls.py cpp_trace.txt --format html --title "C++ Syscall Analysis"
```

### 4. Automated Comparison

```bash
# Run the automated demo (requires strace)
./demo_syscall_analysis.sh
```

## Analysis Output

The analysis generates several types of output:

### JSON Format (`--format json`)
- Raw syscall counts and metadata
- Stack trace information
- Timing data (when available)

### HTML Format (`--format html`)
- Interactive visualizations
- Top syscall charts
- Stack trace browser
- Cross-reference capabilities

### Text Format (`--format text`)
- Simple tabular output
- Summary statistics

## Understanding Syscall Patterns

### Common Syscalls in Data Processing

- **`read`/`write`**: File I/O operations
- **`mmap`/`munmap`**: Memory mapping for large datasets
- **`open`/`close`**: File handle management
- **`fstat`**: File metadata queries
- **`lseek`**: File position manipulation
- **`brk`**: Heap memory allocation

### Casatables-Specific Patterns

- **Table I/O**: Reading/writing CASA table files
- **Memory Management**: Allocating buffers for array data
- **Metadata Operations**: Querying table structure
- **Indexing**: Managing row/column access patterns

## Cross-Language Comparison

When comparing implementations:

1. **Syscall Count**: Fewer syscalls often indicate better optimization
2. **Memory Patterns**: Look for efficient `mmap` usage vs `read`/`write`
3. **File Operations**: Compare file handle management efficiency
4. **Stack Traces**: Identify where syscalls originate in the code

### Expected Differences

- **Rust**: Should show efficient memory management and minimal syscalls
- **Python**: May show more syscalls due to interpreter overhead
- **C++**: Should be similar to Rust but may vary based on standard library usage

## Troubleshooting

### macOS Issues

macOS requires special privileges for syscall tracing:

```bash
# Enable developer mode
sudo DevToolsSecurity -enable

# Or use sudo with dtruss
sudo dtruss -f -s -o trace.txt command
```

### Missing Tools

```bash
# Install strace (Linux)
sudo apt-get install strace  # Ubuntu/Debian
sudo yum install strace      # CentOS/RHEL

# macOS alternatives
brew install dtrace          # If available
# Or use sudo dtruss directly
```

### Large Output Files

For large traces, use filtering:

```bash
# Trace only specific syscalls
strace -e trace=read,write,open,close cargo run --example syscall_tracer

# Limit string size
strace -s 128 -o trace.txt command
```

## Performance Considerations

- **File Size**: Syscall traces can be very large; use filtering when possible
- **Analysis Time**: Large traces take time to parse and analyze
- **Memory Usage**: HTML reports with many stack traces can be memory-intensive

## Extending the Analysis

### Custom Metrics

Modify `analyze_syscalls.py` to add custom metrics:

```python
def analyze_custom_metrics(syscalls):
    # Example: Analyze I/O patterns
    io_syscalls = ['read', 'write', 'pread', 'pwrite']
    io_count = sum(syscalls.get(syscall, 0) for syscall in io_syscalls)

    # Example: Memory allocation patterns
    mem_syscalls = ['mmap', 'munmap', 'brk', 'sbrk']
    mem_count = sum(syscalls.get(syscall, 0) for syscall in mem_syscalls)

    return {'io_operations': io_count, 'memory_operations': mem_count}
```

### Integration with Profiling Tools

Combine with other profilers:

```bash
# Memory profiling
valgrind --tool=massif cargo run --example syscall_tracer

# CPU profiling
perf record -g cargo run --example syscall_tracer
perf report
```

## Contributing

When adding new analysis features:

1. Update the example to include new operations
2. Test across all supported languages
3. Add documentation for new metrics
4. Consider performance impact of analysis

## Related Tools

- **[strace](https://strace.io/)**: Linux syscall tracer
- **[dtruss](https://developer.apple.com/library/archive/documentation/Darwin/Reference/ManPages/man1/dtruss.1m.html)**: macOS syscall tracer
- **[perf](https://perf.wiki.kernel.org/)**: Linux performance profiler
- **[DTrace](https://dtrace.org/)**: Comprehensive tracing framework
- **[Valgrind](https://valgrind.org/)**: Memory debugging and profiling
