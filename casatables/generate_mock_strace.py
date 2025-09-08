#!/usr/bin/env python3
"""
Generate mock strace output for demonstration purposes.

This script creates realistic strace output that simulates what casatables
operations might look like when traced.
"""

import random
import sys
from pathlib import Path

def generate_mock_strace(output_file: str, n_lines: int = 1000):
    """Generate mock strace output"""

    # Common syscalls seen in data processing applications
    syscalls = {
        'read': {'freq': 0.25, 'params': lambda: f'(3, "", {random.randint(4096, 65536)})'},
        'write': {'freq': 0.15, 'params': lambda: f'(4, "", {random.randint(1024, 32768)})'},
        'open': {'freq': 0.08, 'params': lambda: f'("/tmp/casatables_data_{random.randint(1, 100)}.ms", O_RDONLY)'},
        'close': {'freq': 0.08, 'params': lambda: f'({random.randint(3, 20)})'},
        'mmap': {'freq': 0.06, 'params': lambda: f'(NULL, {random.randint(4096, 1048576)}, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0)'},
        'munmap': {'freq': 0.04, 'params': lambda: f'(0x{random.randint(0x100000000, 0x200000000):x}, {random.randint(4096, 1048576)})'},
        'fstat': {'freq': 0.05, 'params': lambda: f'({random.randint(3, 20)}, {{st_mode=S_IFREG|0644, st_size={random.randint(1024, 1048576)}, ...}})'},
        'lseek': {'freq': 0.03, 'params': lambda: f'({random.randint(3, 20)}, {random.randint(0, 1048576)}, SEEK_SET)'},
        'brk': {'freq': 0.02, 'params': lambda: f'(0x{random.randint(0x100000000, 0x200000000):x})'},
        'ioctl': {'freq': 0.01, 'params': lambda: f'({random.randint(3, 20)}, {random.choice(["TCGETS", "TIOCGWINSZ", "FIONREAD"])})'},
        'getdents': {'freq': 0.02, 'params': lambda: f'({random.randint(3, 20)}, {{...}})'},
        'stat': {'freq': 0.03, 'params': lambda: f'("/tmp/casatables_file_{random.randint(1, 50)}", {{st_mode=S_IFREG|0644, st_size={random.randint(1024, 1048576)}, ...}})'},
        'access': {'freq': 0.02, 'params': lambda: f'("/tmp/test_file_{random.randint(1, 20)}", F_OK)'},
        'gettimeofday': {'freq': 0.04, 'params': lambda: f'({{tv_sec={random.randint(1600000000, 1700000000)}, tv_usec={random.randint(0, 999999)}}}, NULL)'},
        'clock_gettime': {'freq': 0.03, 'params': lambda: f'(CLOCK_MONOTONIC, {{tv_sec={random.randint(1000, 10000)}, tv_nsec={random.randint(0, 999999999)}}})'},
    }

    # Calculate cumulative frequencies
    total_freq = sum(info['freq'] for info in syscalls.values())
    cumulative = []
    current = 0
    for name, info in syscalls.items():
        current += info['freq'] / total_freq
        cumulative.append((current, name))

    # Generate mock strace lines
    lines = []
    for i in range(n_lines):
        # Select syscall based on frequency
        r = random.random()
        selected_syscall = None
        for cum_freq, name in cumulative:
            if r <= cum_freq:
                selected_syscall = name
                break

        if selected_syscall:
            syscall_info = syscalls[selected_syscall]
            params = syscall_info['params']()

            # Generate return value (sometimes with errors)
            if random.random() < 0.05:  # 5% error rate
                return_val = f'-{random.choice(["ENOENT", "EACCES", "EINVAL", "EBADF"])}'
            else:
                if selected_syscall in ['read', 'write']:
                    return_val = str(random.randint(0, 65536))
                elif selected_syscall in ['open', 'close', 'fstat', 'lseek']:
                    return_val = str(random.randint(3, 20))
                elif selected_syscall in ['mmap']:
                    return_val = f'0x{random.randint(0x100000000, 0x200000000):x}'
                else:
                    return_val = '0'

            # Generate timing (microseconds)
            timing = random.uniform(0.001, 0.1)

            # Create strace line
            line = f'{selected_syscall}{params} = {return_val} <{timing:.6f}>'

            # Sometimes add stack trace
            if random.random() < 0.1:  # 10% have stack traces
                stack_frames = [
                    f'/usr/lib/x86_64-linux-gnu/libc.so.6({selected_syscall}+0x{random.randint(1000, 9999):x})',
                    f'/path/to/casatables/libcasatables.so(process_data+0x{random.randint(1000, 9999):x})',
                    f'/path/to/main(main+0x{random.randint(1000, 9999):x})'
                ]
                line += ' > ' + '---'.join(stack_frames)

            lines.append(line)

    # Write to file
    with open(output_file, 'w') as f:
        f.write('\n'.join(lines) + '\n')

    print(f"Generated {n_lines} mock strace lines in {output_file}")

def main():
    if len(sys.argv) != 3:
        print("Usage: python generate_mock_strace.py <output_file> <num_lines>")
        sys.exit(1)

    output_file = sys.argv[1]
    num_lines = int(sys.argv[2])

    generate_mock_strace(output_file, num_lines)

if __name__ == '__main__':
    main()
