#!/usr/bin/env python3
"""
Analyze syscalls from strace output for casacore table operations.
"""
import sys
import json
import re
from collections import defaultdict, Counter

def parse_strace_output(filename):
    """Parse strace output and extract syscall information."""
    syscalls = []
    lseek_count = 0
    lseek_seek_set_count = 0
    
    with open(filename, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            
            # Skip empty lines and non-syscall lines
            if not line or not ('(' in line and ')' in line):
                continue
                
            # Extract syscall name
            try:
                if ' = ' in line:
                    syscall_part = line.split(' = ')[0]
                else:
                    syscall_part = line
                    
                syscall_match = re.match(r'^\d*\s*(\w+)\(', syscall_part)
                if syscall_match:
                    syscall_name = syscall_match.group(1)
                    syscalls.append({
                        'name': syscall_name,
                        'line': line_num,
                        'full_line': line
                    })
                    
                    # Count lseek calls
                    if syscall_name == 'lseek':
                        lseek_count += 1
                        # Check if it's SEEK_SET
                        if 'SEEK_SET' in line:
                            lseek_seek_set_count += 1
                            
            except Exception as e:
                # Skip malformed lines
                continue
    
    return {
        'total_syscalls': len(syscalls),
        'lseek_count': lseek_count,
        'lseek_seek_set_count': lseek_seek_set_count,
        'syscall_counts': dict(Counter(s['name'] for s in syscalls)),
        'syscalls': syscalls
    }

def extract_phase_boundaries(filename):
    """Extract putColumn phase boundaries from debug logs."""
    phases = []
    current_phase = None
    
    with open(filename, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            
            # Look for phase start markers
            if 'array_column_put_column:' in line:
                if current_phase:
                    # End previous phase
                    current_phase['end_line'] = line_num - 1
                    phases.append(current_phase)
                
                # Start new phase
                current_phase = {
                    'start_line': line_num,
                    'start_marker': line,
                    'type': 'COMPLEX' if 'COMPLEX' not in line or 'BOOL' not in line else 'UNKNOWN'
                }
                
                # Try to determine phase type from context
                if 'BOOL' in line:
                    current_phase['type'] = 'BOOL'
                elif 'COMPLEX' in line or 'complex' in line.lower():
                    current_phase['type'] = 'COMPLEX'
                    
            # Look for phase end markers
            elif 'putColumn done' in line and current_phase:
                current_phase['end_line'] = line_num
                current_phase['end_marker'] = line
                phases.append(current_phase)
                current_phase = None
    
    # Handle unclosed phase
    if current_phase:
        current_phase['end_line'] = line_num
        phases.append(current_phase)
    
    return phases

def count_seeks_in_phase(strace_data, phase_start, phase_end):
    """Count lseek calls within a specific phase."""
    lseek_count = 0
    lseek_seek_set_count = 0
    
    for syscall in strace_data['syscalls']:
        if phase_start <= syscall['line'] <= phase_end:
            if syscall['name'] == 'lseek':
                lseek_count += 1
                if 'SEEK_SET' in syscall['full_line']:
                    lseek_seek_set_count += 1
    
    return lseek_count, lseek_seek_set_count

def main():
    if len(sys.argv) != 2:
        print("Usage: python3 analyze_syscalls.py <strace_file>")
        sys.exit(1)
    
    filename = sys.argv[1]
    
    try:
        # Parse strace output
        strace_data = parse_strace_output(filename)
        
        # Extract phase boundaries (if debug logs are mixed in)
        phases = extract_phase_boundaries(filename)
        
        # Analyze phases
        phase_analysis = []
        for phase in phases:
            lseek_count, lseek_seek_set_count = count_seeks_in_phase(
                strace_data, phase['start_line'], phase['end_line']
            )
            phase_analysis.append({
                'type': phase['type'],
                'start_line': phase['start_line'],
                'end_line': phase['end_line'],
                'lseek_count': lseek_count,
                'lseek_seek_set_count': lseek_seek_set_count
            })
        
        # Create final analysis
        analysis = {
            'filename': filename,
            'summary': {
                'total_syscalls': strace_data['total_syscalls'],
                'total_lseek': strace_data['lseek_count'],
                'total_lseek_seek_set': strace_data['lseek_seek_set_count']
            },
            'syscall_counts': strace_data['syscall_counts'],
            'phases': phase_analysis
        }
        
        # Output as JSON
        print(json.dumps(analysis, indent=2))
        
    except FileNotFoundError:
        print(f"Error: File '{filename}' not found")
        sys.exit(1)
    except Exception as e:
        print(f"Error analyzing file: {e}")
        sys.exit(1)

if __name__ == '__main__':
    main()