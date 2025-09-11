#!/usr/bin/env python3
"""
Format strace logs for easier diffing between Rust and C++ implementations.
This script processes raw strace logs and creates normalized, comparable output.
"""

import re
import sys
import argparse
from pathlib import Path

def normalize_syscall(line):
    """Normalize a syscall line for comparison."""
    # Remove PID and timestamp if present
    line = re.sub(r'^\d+\s+', '', line)
    
    # Remove addresses and file descriptors that might differ
    line = re.sub(r'0x[0-9a-fA-F]+', '0xADDR', line)
    line = re.sub(r'= \d+', '= FD', line)
    
    # Normalize file paths to relative paths
    line = re.sub(r'/tmp/[^/\s]+', '/tmp/TEMP', line)
    line = re.sub(r'/home/[^/\s]+', '/home/USER', line)
    
    # Normalize table names
    line = re.sub(r'(rust|cpp)\.ms', 'TABLE.ms', line)
    line = re.sub(r'benchmark_table\.ms', 'TABLE.ms', line)
    
    return line.strip()

def extract_syscalls(strace_file):
    """Extract and normalize syscalls from strace output."""
    syscalls = []
    
    with open(strace_file, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            
            # Skip summary lines and headers
            if (line.startswith('%') or line.startswith('-') or 
                'total' in line.lower() or 'time' in line and 'seconds' in line):
                continue
            
            # Skip empty lines
            if not line:
                continue
                
            # Look for syscall patterns
            if ('(' in line and ')' in line and '=' in line) or \
               ('write(' in line or 'read(' in line or 'lseek(' in line or 
                'open' in line or 'mkdir(' in line or 'fsync(' in line):
                normalized = normalize_syscall(line)
                syscalls.append((line_num, normalized))
    
    return syscalls

def categorize_syscalls(syscalls):
    """Categorize syscalls by type for better analysis."""
    categories = {
        'file_ops': [],
        'memory': [],
        'write_ops': [],
        'read_ops': [],
        'seek_ops': [],
        'other': []
    }
    
    for line_num, syscall in syscalls:
        if any(op in syscall for op in ['write(', 'pwrite']):
            categories['write_ops'].append((line_num, syscall))
        elif any(op in syscall for op in ['read(', 'pread']):
            categories['read_ops'].append((line_num, syscall))
        elif 'lseek(' in syscall:
            categories['seek_ops'].append((line_num, syscall))
        elif any(op in syscall for op in ['open', 'close', 'mkdir', 'fsync', 'fstat']):
            categories['file_ops'].append((line_num, syscall))
        elif any(op in syscall for op in ['mmap', 'mprotect', 'brk']):
            categories['memory'].append((line_num, syscall))
        else:
            categories['other'].append((line_num, syscall))
    
    return categories

def analyze_write_patterns(write_ops):
    """Analyze write operation patterns for zero writes and data patterns."""
    patterns = {
        'zero_writes': [],
        'data_writes': [],
        'small_writes': [],
        'large_writes': []
    }
    
    for line_num, syscall in write_ops:
        # Extract data from write calls
        data_match = re.search(r'"([^"]*)"', syscall)
        if data_match:
            data = data_match.group(1)
            
            # Check for zero writes
            if re.match(r'^(\\0|\\x00)*$', data.replace('\\0', '\\x00')):
                patterns['zero_writes'].append((line_num, syscall))
            else:
                patterns['data_writes'].append((line_num, syscall))
        
        # Categorize by size
        size_match = re.search(r'= (\d+)', syscall)
        if size_match:
            size = int(size_match.group(1))
            if size <= 64:
                patterns['small_writes'].append((line_num, syscall))
            else:
                patterns['large_writes'].append((line_num, syscall))
    
    return patterns

def generate_diff_friendly_output(strace_file, output_file):
    """Generate a diff-friendly formatted output."""
    syscalls = extract_syscalls(strace_file)
    categories = categorize_syscalls(syscalls)
    
    with open(output_file, 'w') as f:
        f.write(f"# Formatted strace output from {strace_file}\n")
        f.write(f"# Total syscalls: {len(syscalls)}\n\n")
        
        for category, calls in categories.items():
            if calls:
                f.write(f"## {category.upper()} ({len(calls)} calls)\n")
                
                if category == 'write_ops':
                    patterns = analyze_write_patterns(calls)
                    f.write(f"# Zero writes: {len(patterns['zero_writes'])}\n")
                    f.write(f"# Data writes: {len(patterns['data_writes'])}\n")
                    f.write(f"# Small writes: {len(patterns['small_writes'])}\n")
                    f.write(f"# Large writes: {len(patterns['large_writes'])}\n\n")
                
                for line_num, syscall in calls:
                    f.write(f"{line_num:4d}: {syscall}\n")
                f.write("\n")

def compare_implementations(rust_file, cpp_file, output_dir):
    """Compare Rust and C++ implementations side by side."""
    rust_syscalls = extract_syscalls(rust_file)
    cpp_syscalls = extract_syscalls(cpp_file)
    
    rust_categories = categorize_syscalls(rust_syscalls)
    cpp_categories = categorize_syscalls(cpp_syscalls)
    
    comparison_file = Path(output_dir) / "comparison_summary.txt"
    with open(comparison_file, 'w') as f:
        f.write("# Rust vs C++ Syscall Comparison\n\n")
        
        f.write("## Summary Statistics\n")
        f.write(f"Rust total syscalls: {len(rust_syscalls)}\n")
        f.write(f"C++ total syscalls:  {len(cpp_syscalls)}\n")
        f.write(f"Difference: {len(rust_syscalls) - len(cpp_syscalls)} ({((len(rust_syscalls) - len(cpp_syscalls))/len(cpp_syscalls)*100):+.1f}%)\n\n")
        
        f.write("## Category Breakdown\n")
        for category in rust_categories.keys():
            rust_count = len(rust_categories[category])
            cpp_count = len(cpp_categories[category])
            diff = rust_count - cpp_count
            if cpp_count > 0:
                pct = (diff / cpp_count) * 100
                f.write(f"{category:12}: Rust {rust_count:3d}, C++ {cpp_count:3d}, Diff {diff:+3d} ({pct:+5.1f}%)\n")
            else:
                f.write(f"{category:12}: Rust {rust_count:3d}, C++ {cpp_count:3d}, Diff {diff:+3d}\n")
        
        # Detailed write analysis
        if rust_categories['write_ops'] and cpp_categories['write_ops']:
            rust_write_patterns = analyze_write_patterns(rust_categories['write_ops'])
            cpp_write_patterns = analyze_write_patterns(cpp_categories['write_ops'])
            
            f.write("\n## Write Pattern Analysis\n")
            f.write(f"Rust zero writes: {len(rust_write_patterns['zero_writes'])}\n")
            f.write(f"C++ zero writes:  {len(cpp_write_patterns['zero_writes'])}\n")
            f.write(f"Rust data writes: {len(rust_write_patterns['data_writes'])}\n")
            f.write(f"C++ data writes:  {len(cpp_write_patterns['data_writes'])}\n")

def main():
    parser = argparse.ArgumentParser(description='Format strace logs for easier diffing')
    parser.add_argument('strace_file', help='Input strace file')
    parser.add_argument('-o', '--output', help='Output file (default: <input>_formatted.txt)')
    parser.add_argument('-c', '--compare', help='Compare with another strace file')
    parser.add_argument('--output-dir', default='.', help='Output directory for comparison')
    
    args = parser.parse_args()
    
    if not Path(args.strace_file).exists():
        print(f"Error: {args.strace_file} not found")
        sys.exit(1)
    
    if args.output:
        output_file = args.output
    else:
        output_file = str(Path(args.strace_file).with_suffix('')) + '_formatted.txt'
    
    print(f"Formatting {args.strace_file} -> {output_file}")
    generate_diff_friendly_output(args.strace_file, output_file)
    
    if args.compare:
        if not Path(args.compare).exists():
            print(f"Error: {args.compare} not found")
            sys.exit(1)
        
        print(f"Comparing {args.strace_file} vs {args.compare}")
        compare_implementations(args.strace_file, args.compare, args.output_dir)
        
        # Generate formatted versions of both files
        compare_output = str(Path(args.compare).with_suffix('')) + '_formatted.txt'
        generate_diff_friendly_output(args.compare, compare_output)
        
        print(f"Generated comparison files:")
        print(f"  - {output_file}")
        print(f"  - {compare_output}")
        print(f"  - {Path(args.output_dir) / 'comparison_summary.txt'}")

if __name__ == '__main__':
    main()