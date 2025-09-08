#!/usr/bin/env python3
"""
Syscall Analysis Tool for Cross-Language Comparison

This script analyzes strace output from different programming languages
to compare syscall patterns and performance characteristics.

Usage:
    python3 analyze_syscalls.py <strace_output_file> [options]

Options:
    --format FORMAT    Output format (json, html, text) [default: html]
    --compare FILE     Compare with another strace file
    --output DIR       Output directory [default: syscall_analysis]
    --title TITLE      Analysis title [default: Syscall Analysis]
"""

import argparse
import json
import re
import sys
from collections import defaultdict, Counter
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Tuple, Optional
import statistics

class SyscallAnalyzer:
    def __init__(self, title="Syscall Analysis"):
        self.title = title
        self.syscall_counts = Counter()
        self.syscall_stacks = defaultdict(list)
        self.syscall_times = defaultdict(list)
        self.syscall_sizes = defaultdict(list)  # For I/O size analysis
        self.syscall_fds = defaultdict(list)    # For file descriptor analysis
        self.syscall_errors = defaultdict(list) # For error analysis
        self.total_syscalls = 0
        self.duration = 0.0
        self.io_patterns = self._initialize_io_patterns()

    def _initialize_io_patterns(self) -> Dict[str, List[str]]:
        """Initialize syscall categorization patterns for I/O analysis"""
        return {
            'file_io': ['read', 'write', 'pread', 'pwrite', 'readv', 'writev'],
            'file_management': ['open', 'close', 'creat', 'unlink', 'rename', 'mkdir', 'rmdir'],
            'file_metadata': ['stat', 'fstat', 'lstat', 'access', 'chmod', 'chown'],
            'directory_ops': ['getdents', 'chdir', 'fchdir', 'getcwd'],
            'memory_management': ['mmap', 'munmap', 'mremap', 'brk', 'sbrk'],
            'process_management': ['fork', 'vfork', 'clone', 'execve', 'wait4', 'waitpid'],
            'network_io': ['socket', 'connect', 'bind', 'listen', 'accept', 'send', 'recv'],
            'signal_handling': ['signal', 'sigaction', 'kill', 'tkill'],
            'synchronization': ['semop', 'semget', 'shmget', 'shmat', 'shmdt'],
            'time_operations': ['gettimeofday', 'clock_gettime', 'nanosleep', 'alarm']
        }

    def categorize_syscall(self, syscall: str) -> str:
        """Categorize a syscall into I/O pattern groups"""
        for category, syscalls in self.io_patterns.items():
            if syscall in syscalls:
                return category
        return 'other'

    def parse_strace_line(self, line):
        """Parse a single strace output line with enhanced I/O analysis"""
        line = line.strip()
        if not line or line.startswith('---') or line.startswith('+++'):
            return None

        # Extract syscall name
        syscall_match = re.match(r'^(\w+)\(', line)
        if not syscall_match:
            return None

        syscall_name = syscall_match.group(1)

        # Extract parameters from syscall
        param_match = re.search(r'\((.*?)\)', line)
        params = param_match.group(1) if param_match else ""

        # Extract return value and potential error
        return_match = re.search(r'\)\s*=\s*([^<\s]+)', line)
        return_value = None
        is_error = False
        if return_match:
            return_value = return_match.group(1)
            # Check if it's an error (negative value or specific error indicators)
            if return_value.startswith('-') or 'EN' in return_value:
                is_error = True

        # Extract timing information
        time_match = re.search(r'<([0-9.]+)>$', line)
        timing = float(time_match.group(1)) if time_match else None

        # Extract I/O size information for relevant syscalls
        io_size = self._extract_io_size(syscall_name, params, return_value)

        # Extract file descriptor information
        fd = self._extract_file_descriptor(syscall_name, params)

        # Extract stack trace
        stack_trace = self._extract_stack_trace(line)

        return {
            'syscall': syscall_name,
            'params': params,
            'return_value': return_value,
            'is_error': is_error,
            'timing': timing,
            'io_size': io_size,
            'fd': fd,
            'stack_trace': stack_trace,
            'category': self.categorize_syscall(syscall_name)
        }

    def _extract_io_size(self, syscall: str, params: str, return_value: Optional[str]) -> Optional[int]:
        """Extract I/O size from syscall parameters or return value"""
        if syscall in ['read', 'write', 'pread', 'pwrite']:
            # For read/write, the third parameter is the size
            param_parts = params.split(',')
            if len(param_parts) >= 3:
                try:
                    return int(param_parts[2].strip())
                except ValueError:
                    pass
        elif syscall in ['readv', 'writev']:
            # For vector I/O, we could parse the iovec structure
            # This is more complex, so we'll return None for now
            pass
        elif return_value and syscall in ['read', 'write', 'pread', 'pwrite']:
            # If we couldn't get size from params, use return value
            try:
                size = int(return_value)
                return size if size > 0 else None
            except (ValueError, TypeError):
                pass
        return None

    def _extract_file_descriptor(self, syscall: str, params: str) -> Optional[int]:
        """Extract file descriptor from syscall parameters"""
        if syscall in ['read', 'write', 'close', 'fstat', 'lseek']:
            param_parts = params.split(',')
            if param_parts:
                try:
                    return int(param_parts[0].strip())
                except ValueError:
                    pass
        return None

    def _extract_stack_trace(self, line: str) -> List[str]:
        """Extract stack trace from strace output"""
        stack_match = re.search(r'>\s*(.+)$', line)
        if not stack_match:
            return []

        stack_part = stack_match.group(1)
        stack_trace = []
        for frame in stack_part.split('---'):
            frame = frame.strip()
            if frame and ('(' in frame or frame.startswith('0x') or '/' in frame):
                stack_trace.append(frame)
        return stack_trace

    def analyze_file(self, filepath):
        """Analyze strace output file with enhanced metrics"""
        print(f"Analyzing {filepath}...")

        with open(filepath, 'r') as f:
            for line_num, line in enumerate(f):
                parsed = self.parse_strace_line(line)
                if parsed:
                    syscall = parsed['syscall']
                    self.syscall_counts[syscall] += 1
                    self.total_syscalls += 1

                    # Collect enhanced metrics
                    if parsed['stack_trace']:
                        self.syscall_stacks[syscall].append(parsed['stack_trace'])

                    if parsed['timing'] is not None:
                        self.syscall_times[syscall].append(parsed['timing'])

                    if parsed['io_size'] is not None:
                        self.syscall_sizes[syscall].append(parsed['io_size'])

                    if parsed['fd'] is not None:
                        self.syscall_fds[syscall].append(parsed['fd'])

                    if parsed['is_error']:
                        self.syscall_errors[syscall].append(parsed['return_value'])

        print(f"Found {self.total_syscalls} syscalls from {len(self.syscall_counts)} unique syscalls")
        print(f"Enhanced metrics collected: I/O sizes, file descriptors, error patterns")

    def _calculate_category_statistics(self) -> Dict[str, Dict]:
        """Calculate statistics for each I/O pattern category"""
        category_stats = {}

        for category, syscalls in self.io_patterns.items():
            category_count = sum(self.syscall_counts.get(syscall, 0) for syscall in syscalls)
            if category_count > 0:
                # Calculate average timing for category
                category_times = []
                for syscall in syscalls:
                    category_times.extend(self.syscall_times[syscall])

                avg_time = statistics.mean(category_times) if category_times else 0
                total_time = sum(category_times) if category_times else 0

                category_stats[category] = {
                    'count': category_count,
                    'percentage': (category_count / self.total_syscalls) * 100,
                    'avg_time': avg_time,
                    'total_time': total_time,
                    'syscalls': [s for s in syscalls if self.syscall_counts.get(s, 0) > 0]
                }

        return category_stats

    def _calculate_performance_metrics(self) -> Dict[str, float]:
        """Calculate overall performance metrics"""
        metrics = {}

        # I/O efficiency metrics
        io_syscalls = self.io_patterns['file_io']
        total_io_operations = sum(self.syscall_counts.get(s, 0) for s in io_syscalls)

        if total_io_operations > 0:
            total_io_size = sum(
                sum(sizes) for syscall in io_syscalls
                for sizes in [self.syscall_sizes[syscall]]
            )
            avg_io_size = total_io_size / total_io_operations if total_io_size > 0 else 0
            metrics['avg_io_size'] = avg_io_size

        # Error rate
        total_errors = sum(len(errors) for errors in self.syscall_errors.values())
        metrics['error_rate'] = (total_errors / self.total_syscalls) * 100 if self.total_syscalls > 0 else 0

        # Timing statistics
        all_times = []
        for times in self.syscall_times.values():
            all_times.extend(times)

        if all_times:
            metrics['avg_syscall_time'] = statistics.mean(all_times)
            metrics['total_syscall_time'] = sum(all_times)
            metrics['syscall_time_stddev'] = statistics.stdev(all_times) if len(all_times) > 1 else 0

        return metrics

    def generate_report(self, output_format='html', output_dir='syscall_analysis'):
        """Generate analysis report"""
        output_path = Path(output_dir)
        output_path.mkdir(exist_ok=True)

        # Calculate I/O pattern statistics
        category_stats = self._calculate_category_statistics()

        data = {
            'title': self.title,
            'timestamp': datetime.now().isoformat(),
            'total_syscalls': self.total_syscalls,
            'unique_syscalls': len(self.syscall_counts),
            'io_patterns': category_stats,
            'performance_metrics': self._calculate_performance_metrics(),
            'syscalls': {}
        }

        # Process top syscalls
        top_syscalls = self.syscall_counts.most_common(50)
        for syscall, count in top_syscalls:
            data['syscalls'][syscall] = {
                'count': count,
                'percentage': (count / self.total_syscalls) * 100,
                'avg_time': sum(self.syscall_times[syscall]) / len(self.syscall_times[syscall]) if self.syscall_times[syscall] else 0,
                'stack_traces': len(self.syscall_stacks[syscall]),
                'sample_stacks': self.syscall_stacks[syscall][:5]  # First 5 stack traces
            }

        if output_format == 'json':
            self._generate_json_report(data, output_path)
        elif output_format == 'html':
            self._generate_html_report(data, output_path)
        elif output_format == 'text':
            self._generate_text_report(data, output_path)

        return data

    def _generate_json_report(self, data, output_path):
        """Generate JSON report"""
        json_file = output_path / 'syscall_analysis.json'
        with open(json_file, 'w') as f:
            json.dump(data, f, indent=2)
        print(f"JSON report saved to {json_file}")

    def _generate_html_report(self, data, output_path):
        """Generate HTML report with visualizations"""
        html_file = output_path / 'syscall_analysis.html'

        # Generate charts data
        syscall_names = list(data['syscalls'].keys())[:20]
        syscall_counts = [data['syscalls'][name]['count'] for name in syscall_names]
        syscall_percentages = [data['syscalls'][name]['percentage'] for name in syscall_names]

        html_content = f"""
<!DOCTYPE html>
<html>
<head>
    <title>{data['title']}</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            margin: 0;
            padding: 20px;
            background: #f5f5f5;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
            padding-bottom: 20px;
            border-bottom: 2px solid #eee;
        }}
        .summary {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        .summary-card {{
            background: #f8f9fa;
            padding: 20px;
            border-radius: 8px;
            text-align: center;
            border-left: 4px solid #007bff;
        }}
        .summary-card h3 {{
            margin: 0 0 10px 0;
            color: #333;
            font-size: 2em;
        }}
        .summary-card p {{
            margin: 0;
            color: #666;
            font-weight: 500;
        }}
        .chart-container {{
            width: 100%;
            height: 400px;
            margin: 20px 0;
            padding: 20px;
            background: white;
            border-radius: 8px;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }}
        .syscall-details {{
            margin-top: 30px;
        }}
        .syscall-item {{
            background: #f8f9fa;
            margin: 10px 0;
            padding: 15px;
            border-radius: 8px;
            border-left: 4px solid #28a745;
        }}
        .syscall-item h4 {{
            margin: 0 0 10px 0;
            color: #333;
        }}
        .syscall-stats {{
            display: flex;
            gap: 20px;
            margin-bottom: 10px;
        }}
        .stat {{
            font-size: 0.9em;
            color: #666;
        }}
        .stack-trace {{
            background: #2d3748;
            color: #e2e8f0;
            padding: 10px;
            border-radius: 4px;
            font-family: 'Monaco', 'Menlo', monospace;
            font-size: 12px;
            margin-top: 10px;
            overflow-x: auto;
        }}
        .stack-trace pre {{
            margin: 0;
            white-space: pre-wrap;
            word-break: break-all;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>{data['title']}</h1>
            <p>Generated on {data['timestamp']}</p>
        </div>

        <div class="summary">
            <div class="summary-card">
                <h3>{data['total_syscalls']:,}</h3>
                <p>Total Syscalls</p>
            </div>
            <div class="summary-card">
                <h3>{data['unique_syscalls']}</h3>
                <p>Unique Syscalls</p>
            </div>
            <div class="summary-card">
                <h3>{len(data.get('io_patterns', {}))}</h3>
                <p>I/O Categories</p>
            </div>
            <div class="summary-card">
                <h3>{data.get('performance_metrics', {}).get('error_rate', 0):.2f}%</h3>
                <p>Error Rate</p>
            </div>
        </div>

        <div class="chart-container">
            <canvas id="syscallChart"></canvas>
        </div>

        <div class="chart-container">
            <canvas id="percentageChart"></canvas>
        </div>

        <div class="chart-container">
            <canvas id="ioPatternChart"></canvas>
        </div>

        <div class="chart-container">
            <canvas id="performanceChart"></canvas>
        </div>

        <div class="syscall-details">
            <h2>I/O Pattern Analysis</h2>
            <div class="io-patterns">
                {"".join([f'''
                <div class="syscall-item">
                    <h4>{category.replace('_', ' ').title()}</h4>
                    <div class="syscall-stats">
                        <span class="stat">Count: {info['count']:,}</span>
                        <span class="stat">Percentage: {info['percentage']:.2f}%</span>
                        <span class="stat">Avg Time: {info['avg_time']:.6f}s</span>
                        <span class="stat">Total Time: {info['total_time']:.6f}s</span>
                    </div>
                    <p><strong>Syscalls:</strong> {', '.join(info['syscalls'])}</p>
                </div>
                ''' for category, info in data.get('io_patterns', {}).items()])}
            </div>

            <h2>Top Syscall Details</h2>
            {"".join([f'''
            <div class="syscall-item">
                <h4>{name}</h4>
                <div class="syscall-stats">
                    <span class="stat">Count: {info['count']:,}</span>
                    <span class="stat">Percentage: {info['percentage']:.2f}%</span>
                    <span class="stat">Stack Traces: {info['stack_traces']}</span>
                    {"".join([f'<span class="stat">Avg Time: {info["avg_time"]:.6f}s</span>' for _ in [None] if info['avg_time'] > 0])}
                </div>
                {"".join([f'''
                <div class="stack-trace">
                    <pre>{chr(10).join(stack)}</pre>
                </div>
                ''' for stack in info['sample_stacks'][:2]]) if info['sample_stacks'] else ""}
            </div>
            ''' for name, info in list(data['syscalls'].items())[:20]])}
        </div>
    </div>

    <script>
        // Chart data preparation
        const syscallData = {syscall_names};
        const syscallCounts = {syscall_counts};
        const syscallPercentages = {syscall_percentages};
        const ioPatterns = {json.dumps(data.get('io_patterns', {}))};
        const performanceMetrics = {json.dumps(data.get('performance_metrics', {}))};

        // Colors for charts
        const colors = [
            'rgba(255, 99, 132, 0.5)', 'rgba(54, 162, 235, 0.5)', 'rgba(255, 205, 86, 0.5)',
            'rgba(75, 192, 192, 0.5)', 'rgba(153, 102, 255, 0.5)', 'rgba(255, 159, 64, 0.5)',
            'rgba(201, 203, 207, 0.5)', 'rgba(255, 99, 132, 0.5)', 'rgba(54, 162, 235, 0.5)',
            'rgba(255, 205, 86, 0.5)', 'rgba(75, 192, 192, 0.5)', 'rgba(153, 102, 255, 0.5)',
            'rgba(255, 159, 64, 0.5)', 'rgba(201, 203, 207, 0.5)', 'rgba(255, 99, 132, 0.5)',
            'rgba(54, 162, 235, 0.5)', 'rgba(255, 205, 86, 0.5)', 'rgba(75, 192, 192, 0.5)',
            'rgba(153, 102, 255, 0.5)', 'rgba(255, 159, 64, 0.5)'
        ];

        // Syscall count chart
        const ctx1 = document.getElementById('syscallChart').getContext('2d');
        new Chart(ctx1, {{
            type: 'bar',
            data: {{
                labels: syscallData,
                datasets: [{{
                    label: 'Syscall Count',
                    data: syscallCounts,
                    backgroundColor: colors.slice(0, syscallData.length),
                    borderColor: colors.slice(0, syscallData.length).map(c => c.replace('0.5', '1')),
                    borderWidth: 1
                }}]
            }},
            options: {{
                responsive: true,
                scales: {{
                    y: {{
                        beginAtZero: true
                    }}
                }},
                plugins: {{
                    title: {{
                        display: true,
                        text: 'Top 20 Syscalls by Count'
                    }}
                }}
            }}
        }});

        // Percentage chart
        const ctx2 = document.getElementById('percentageChart').getContext('2d');
        new Chart(ctx2, {{
            type: 'doughnut',
            data: {{
                labels: syscallData,
                datasets: [{{
                    data: syscallPercentages,
                    backgroundColor: colors.slice(0, syscallData.length),
                    borderColor: colors.slice(0, syscallData.length).map(c => c.replace('0.5', '1')),
                    borderWidth: 1
                }}]
            }},
            options: {{
                responsive: true,
                plugins: {{
                    title: {{
                        display: true,
                        text: 'Syscall Distribution (Top 20)'
                    }}
                }}
            }}
        }});

        // I/O Pattern chart
        const ioPatternLabels = Object.keys(ioPatterns);
        const ioPatternData = ioPatternLabels.map(cat => ioPatterns[cat].count);

        const ctx3 = document.getElementById('ioPatternChart').getContext('2d');
        new Chart(ctx3, {{
            type: 'pie',
            data: {{
                labels: ioPatternLabels,
                datasets: [{{
                    data: ioPatternData,
                    backgroundColor: colors.slice(0, ioPatternLabels.length),
                    borderColor: colors.slice(0, ioPatternLabels.length).map(c => c.replace('0.5', '1')),
                    borderWidth: 1
                }}]
            }},
            options: {{
                responsive: true,
                plugins: {{
                    title: {{
                        display: true,
                        text: 'I/O Pattern Distribution'
                    }}
                }}
            }}
        }});

        // Performance metrics chart
        const performanceLabels = ['Avg Syscall Time', 'Total Syscall Time', 'Error Rate'];
        const performanceData = [
            performanceMetrics.avg_syscall_time || 0,
            performanceMetrics.total_syscall_time || 0,
            performanceMetrics.error_rate || 0
        ];

        const ctx4 = document.getElementById('performanceChart').getContext('2d');
        new Chart(ctx4, {{
            type: 'bar',
            data: {{
                labels: performanceLabels,
                datasets: [{{
                    label: 'Performance Metrics',
                    data: performanceData,
                    backgroundColor: 'rgba(75, 192, 192, 0.5)',
                    borderColor: 'rgba(75, 192, 192, 1)',
                    borderWidth: 1
                }}]
            }},
            options: {{
                responsive: true,
                scales: {{
                    y: {{
                        beginAtZero: true
                    }}
                }},
                plugins: {{
                    title: {{
                        display: true,
                        text: 'Performance Metrics'
                    }}
                }}
            }}
        }});
    </script>
</body>
</html>
"""

        with open(html_file, 'w') as f:
            f.write(html_content)
        print(f"HTML report saved to {html_file}")

    def _generate_text_report(self, data, output_path):
        """Generate text report"""
        text_file = output_path / 'syscall_analysis.txt'

        with open(text_file, 'w') as f:
            f.write(f"{data['title']}\n")
            f.write("=" * len(data['title']) + "\n\n")
            f.write(f"Generated on: {data['timestamp']}\n\n")
            f.write(f"Total syscalls: {data['total_syscalls']:,}\n")
            f.write(f"Unique syscalls: {data['unique_syscalls']}\n\n")

            f.write("Top 20 syscalls by count:\n")
            f.write("-" * 50 + "\n")

            for i, (syscall, info) in enumerate(list(data['syscalls'].items())[:20], 1):
                f.write("2d")
                f.write("12,d")
                f.write(".2f")
                f.write(f"{info['stack_traces']}\n")

                if info['sample_stacks']:
                    f.write("  Sample stack trace:\n")
                    for frame in info['sample_stacks'][0][:5]:
                        f.write(f"    {frame}\n")
                    f.write("\n")

        print(f"Text report saved to {text_file}")

def compare_analyses(analysis1, analysis2, title1="Analysis 1", title2="Analysis 2"):
    """Compare two syscall analyses"""
    print(f"\nComparing {title1} vs {title2}:")
    print("-" * 60)

    # Common syscalls
    common = set(analysis1.syscall_counts.keys()) & set(analysis2.syscall_counts.keys())
    only1 = set(analysis1.syscall_counts.keys()) - set(analysis2.syscall_counts.keys())
    only2 = set(analysis2.syscall_counts.keys()) - set(analysis1.syscall_counts.keys())

    print(f"Common syscalls: {len(common)}")
    print(f"Only in {title1}: {len(only1)}")
    print(f"Only in {title2}: {len(only2)}")
    print(f"Total in {title1}: {len(analysis1.syscall_counts)}")
    print(f"Total in {title2}: {len(analysis2.syscall_counts)}")

    # Compare top syscalls
    print("
Top syscall differences:")
    all_syscalls = set(analysis1.syscall_counts.keys()) | set(analysis2.syscall_counts.keys())

    diffs = []
    for syscall in all_syscalls:
        count1 = analysis1.syscall_counts[syscall]
        count2 = analysis2.syscall_counts[syscall]
        if count1 > 0 and count2 > 0:
            diff = abs(count1 - count2)
            if diff > 0:
                diffs.append((syscall, count1, count2, diff))

    diffs.sort(key=lambda x: x[3], reverse=True)
    for syscall, count1, count2, diff in diffs[:10]:
        print("12")

def main():
    parser = argparse.ArgumentParser(description='Analyze strace output for syscall patterns')
    parser.add_argument('input_file', help='Strace output file to analyze')
    parser.add_argument('--format', choices=['json', 'html', 'text'], default='html',
                       help='Output format (default: html)')
    parser.add_argument('--compare', help='Compare with another strace file')
    parser.add_argument('--output', default='syscall_analysis',
                       help='Output directory (default: syscall_analysis)')
    parser.add_argument('--title', default='Syscall Analysis',
                       help='Analysis title (default: Syscall Analysis)')

    args = parser.parse_args()

    # Analyze main file
    analyzer = SyscallAnalyzer(args.title)
    analyzer.analyze_file(args.input_file)

    # Generate report
    analyzer.generate_report(args.format, args.output)

    # Compare if requested
    if args.compare:
        analyzer2 = SyscallAnalyzer(f"{args.title} (Comparison)")
        analyzer2.analyze_file(args.compare)
        compare_analyses(analyzer, analyzer2, "Main", "Comparison")

    print("
Analysis completed!")

if __name__ == '__main__':
    main()
