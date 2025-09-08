#!/bin/bash

# Script to run syscall analysis on the casatables module
# This script uses strace to capture syscalls and stack traces

set -e

echo "Building syscall tracer..."
cargo build --bin syscall_tracer --release

echo "Running syscall analysis with strace..."

# Create output directory
mkdir -p syscall_output

# Run with strace capturing syscalls and stack traces
strace -f -s 256 -k -o syscall_output/strace_output.txt \
    ./target/release/syscall_tracer

echo "Parsing strace output..."

# Parse the strace output and generate analysis
python3 -c "
import json
import re
from collections import defaultdict, Counter
import sys

def parse_strace_line(line):
    # Extract syscall name
    syscall_match = re.match(r'^(\w+)\(', line)
    if not syscall_match:
        return None, None

    syscall_name = syscall_match.group(1)

    # Extract stack trace (after '>')
    stack_match = re.search(r'>\s*(.+)$', line)
    stack_trace = []
    if stack_match:
        stack_part = stack_match.group(1)
        # Parse stack frames
        for frame in stack_part.split('---'):
            frame = frame.strip()
            if frame and '(' in frame:
                stack_trace.append(frame)

    return syscall_name, stack_trace

def main():
    syscall_counts = Counter()
    syscall_stacks = defaultdict(list)

    with open('syscall_output/strace_output.txt', 'r') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('---') or line.startswith('+++'):
                continue

            syscall_name, stack_trace = parse_strace_line(line)
            if syscall_name:
                syscall_counts[syscall_name] += 1
                if stack_trace:
                    syscall_stacks[syscall_name].append(stack_trace)

    # Create profile data
    profile = {
        'total_syscalls': sum(syscall_counts.values()),
        'unique_syscalls': len(syscall_counts),
        'syscalls': {}
    }

    for syscall, count in syscall_counts.most_common():
        profile['syscalls'][syscall] = {
            'name': syscall,
            'count': count,
            'stack_traces': syscall_stacks[syscall][:10]  # Limit to first 10 stack traces
        }

    # Save to JSON
    with open('syscall_output/syscall_profile.json', 'w') as f:
        json.dump(profile, f, indent=2)

    # Print summary
    print(f'Total syscalls: {profile[\"total_syscalls\"]}')
    print(f'Unique syscalls: {profile[\"unique_syscalls\"]}')
    print()
    print('Top 10 syscalls by count:')
    for syscall, count in syscall_counts.most_common(10):
        print(f'  {syscall}: {count} calls')

if __name__ == '__main__':
    main()
"

echo "Generating visualization..."

# Create a simple HTML visualization
cat > syscall_output/visualization.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Syscall Analysis Visualization</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .chart-container { width: 800px; height: 400px; margin: 20px 0; }
        .summary { background: #f5f5f5; padding: 15px; border-radius: 5px; margin: 20px 0; }
        .stack-trace { background: #f9f9f9; padding: 10px; border-left: 3px solid #ccc; margin: 10px 0; font-family: monospace; font-size: 12px; }
    </style>
</head>
<body>
    <h1>Casatables Syscall Analysis</h1>

    <div class="summary">
        <h2>Summary</h2>
        <div id="summary-content">Loading...</div>
    </div>

    <div class="chart-container">
        <canvas id="syscallChart"></canvas>
    </div>

    <div class="chart-container">
        <canvas id="stackTraceChart"></canvas>
    </div>

    <h2>Detailed Syscall Information</h2>
    <div id="syscall-details"></div>

    <script>
        async function loadData() {
            const response = await fetch('syscall_profile.json');
            const data = await response.json();
            return data;
        }

        function createCharts(data) {
            // Top syscalls chart
            const topSyscalls = Object.entries(data.syscalls)
                .sort((a, b) => b[1].count - a[1].count)
                .slice(0, 15);

            const ctx1 = document.getElementById('syscallChart').getContext('2d');
            new Chart(ctx1, {
                type: 'bar',
                data: {
                    labels: topSyscalls.map(([name, _]) => name),
                    datasets: [{
                        label: 'Syscall Count',
                        data: topSyscalls.map(([_, info]) => info.count),
                        backgroundColor: 'rgba(54, 162, 235, 0.5)',
                        borderColor: 'rgba(54, 162, 235, 1)',
                        borderWidth: 1
                    }]
                },
                options: {
                    responsive: true,
                    scales: {
                        y: {
                            beginAtZero: true
                        }
                    },
                    plugins: {
                        title: {
                            display: true,
                            text: 'Top 15 Syscalls by Count'
                        }
                    }
                }
            });

            // Stack trace availability chart
            const withStacks = topSyscalls.filter(([_, info]) => info.stack_traces.length > 0).length;
            const withoutStacks = topSyscalls.length - withStacks;

            const ctx2 = document.getElementById('stackTraceChart').getContext('2d');
            new Chart(ctx2, {
                type: 'pie',
                data: {
                    labels: ['With Stack Traces', 'Without Stack Traces'],
                    datasets: [{
                        data: [withStacks, withoutStacks],
                        backgroundColor: [
                            'rgba(75, 192, 192, 0.5)',
                            'rgba(255, 99, 132, 0.5)'
                        ],
                        borderColor: [
                            'rgba(75, 192, 192, 1)',
                            'rgba(255, 99, 132, 1)'
                        ],
                        borderWidth: 1
                    }]
                },
                options: {
                    responsive: true,
                    plugins: {
                        title: {
                            display: true,
                            text: 'Stack Trace Availability (Top 15 Syscalls)'
                        }
                    }
                }
            });
        }

        function createDetails(data) {
            const summaryDiv = document.getElementById('summary-content');
            summaryDiv.innerHTML = `
                <p><strong>Total Syscalls:</strong> ${data.total_syscalls.toLocaleString()}</p>
                <p><strong>Unique Syscalls:</strong> ${data.unique_syscalls}</p>
            `;

            const detailsDiv = document.getElementById('syscall-details');
            const topSyscalls = Object.entries(data.syscalls)
                .sort((a, b) => b[1].count - a[1].count)
                .slice(0, 10);

            topSyscalls.forEach(([name, info]) => {
                const div = document.createElement('div');
                div.innerHTML = `
                    <h3>${name} (${info.count} calls)</h3>
                    ${info.stack_traces.length > 0 ?
                        `<div class="stack-trace">${info.stack_traces.slice(0, 3).map(stack =>
                            stack.join('<br>')
                        ).join('<br><br>')}</div>` :
                        '<p>No stack traces captured</p>'
                    }
                `;
                detailsDiv.appendChild(div);
            });
        }

        // Load and display data
        loadData().then(data => {
            createCharts(data);
            createDetails(data);
        });
    </script>
</body>
</html>
EOF

echo "Analysis complete!"
echo "Results saved in syscall_output/"
echo "Open syscall_output/visualization.html in a web browser to view the results"
echo ""
echo "Files generated:"
echo "  - syscall_output/strace_output.txt (raw strace output)"
echo "  - syscall_output/syscall_profile.json (parsed analysis data)"
echo "  - syscall_output/visualization.html (interactive visualization)"
