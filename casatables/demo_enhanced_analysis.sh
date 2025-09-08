#!/bin/bash

# Enhanced syscall analysis demonstration
# Shows advanced I/O profiling techniques inspired by I/O profiling tools

set -e

echo "🔬 Enhanced Syscall Analysis Demo"
echo "=================================="
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Create output directory
OUTPUT_DIR="enhanced_analysis_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$OUTPUT_DIR"

echo -e "${BLUE}Output directory: $OUTPUT_DIR${NC}"
echo

# Function to run enhanced analysis
run_enhanced_analysis() {
    local title=$1
    local strace_file=$2
    local output_dir=$3

    echo -e "${YELLOW}Running enhanced analysis for $title...${NC}"

    # Generate analysis
    python3 analyze_syscalls.py "$strace_file" \
        --format html \
        --output "$output_dir" \
        --title "$title"

    echo -e "${GREEN}✓ Enhanced analysis completed${NC}"
    echo
}

# 1. Generate mock strace data
echo -e "${BLUE}1. Generating mock strace data${NC}"
python3 ./generate_mock_strace.py "$OUTPUT_DIR/mock_strace.txt" 5000
echo -e "${GREEN}✓ Generated 5000 mock strace lines${NC}"
echo

# 2. Run enhanced analysis on mock data
echo -e "${BLUE}2. Running enhanced syscall analysis${NC}"
run_enhanced_analysis "Enhanced Casatables Syscall Analysis" \
    "$OUTPUT_DIR/mock_strace.txt" \
    "$OUTPUT_DIR/enhanced_analysis"

# 3. Generate comparison data
echo -e "${BLUE}3. Generating comparison data${NC}"
python3 ./generate_mock_strace.py "$OUTPUT_DIR/mock_strace_python.txt" 3000
run_enhanced_analysis "Python Comparison" \
    "$OUTPUT_DIR/mock_strace_python.txt" \
    "$OUTPUT_DIR/python_analysis"

python3 ./generate_mock_strace.py "$OUTPUT_DIR/mock_strace_cpp.txt" 4000
run_enhanced_analysis "C++ Comparison" \
    "$OUTPUT_DIR/mock_strace_cpp.txt" \
    "$OUTPUT_DIR/cpp_analysis"

# 4. Create comparison dashboard
echo -e "${BLUE}4. Creating comparison dashboard${NC}"
cat > "$OUTPUT_DIR/comparison_dashboard.html" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Enhanced Syscall Analysis Comparison</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            margin: 0;
            padding: 20px;
            background: #f5f5f5;
        }
        .container {
            max-width: 1400px;
            margin: 0 auto;
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }
        .header {
            text-align: center;
            margin-bottom: 30px;
            padding-bottom: 20px;
            border-bottom: 2px solid #eee;
        }
        .comparison-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        .analysis-card {
            background: #f8f9fa;
            border-radius: 8px;
            overflow: hidden;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }
        .card-header {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 15px;
            text-align: center;
        }
        .card-content {
            padding: 20px;
        }
        .metric-grid {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 15px;
            margin-bottom: 20px;
        }
        .metric {
            background: white;
            padding: 15px;
            border-radius: 6px;
            text-align: center;
            box-shadow: 0 1px 2px rgba(0,0,0,0.05);
        }
        .metric-value {
            font-size: 1.5em;
            font-weight: bold;
            color: #333;
        }
        .metric-label {
            font-size: 0.9em;
            color: #666;
            margin-top: 5px;
        }
        .links {
            text-align: center;
            margin-top: 15px;
        }
        .links a {
            color: #667eea;
            text-decoration: none;
            margin: 0 10px;
            font-weight: 500;
        }
        .links a:hover {
            text-decoration: underline;
        }
        .insights {
            margin-top: 30px;
            padding: 20px;
            background: #e8f4fd;
            border-radius: 8px;
            border-left: 4px solid #667eea;
        }
        .insights h3 {
            margin-top: 0;
            color: #333;
        }
        .insights ul {
            margin: 15px 0;
            padding-left: 20px;
        }
        .insights li {
            margin: 8px 0;
            line-height: 1.5;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🔬 Enhanced Syscall Analysis Comparison</h1>
            <p>Advanced I/O profiling across different implementations</p>
        </div>

        <div class="comparison-grid" id="comparison-grid">
            <!-- Analysis cards will be inserted here -->
        </div>

        <div class="insights">
            <h3>📊 Key Insights from Enhanced Analysis</h3>
            <ul>
                <li><strong>I/O Pattern Analysis:</strong> Categorizes syscalls into file I/O, memory management, network operations, etc.</li>
                <li><strong>Performance Metrics:</strong> Tracks timing, error rates, and I/O efficiency</li>
                <li><strong>Stack Trace Integration:</strong> Connects syscalls to source code locations</li>
                <li><strong>Cross-Language Comparison:</strong> Enables performance comparison between Rust, Python, C++</li>
                <li><strong>Error Pattern Detection:</strong> Identifies problematic syscall patterns</li>
                <li><strong>Resource Usage Tracking:</strong> Monitors file descriptors, memory allocation patterns</li>
            </ul>
        </div>
    </div>

    <script>
        // Load and display analysis data
        const analyses = [
            { name: 'Enhanced Analysis', dir: 'enhanced_analysis' },
            { name: 'Python Comparison', dir: 'python_analysis' },
            { name: 'C++ Comparison', dir: 'cpp_analysis' }
        ];

        const grid = document.getElementById('comparison-grid');

        analyses.forEach(analysis => {
            fetch(`${analysis.dir}/syscall_analysis.json`)
                .then(response => response.ok ? response.json() : null)
                .then(data => {
                    if (data) {
                        const card = document.createElement('div');
                        card.className = 'analysis-card';

                        const ioPatterns = data.io_patterns || {};
                        const metrics = data.performance_metrics || {};

                        card.innerHTML = `
                            <div class="card-header">
                                <h3>${analysis.name}</h3>
                            </div>
                            <div class="card-content">
                                <div class="metric-grid">
                                    <div class="metric">
                                        <div class="metric-value">${data.total_syscalls.toLocaleString()}</div>
                                        <div class="metric-label">Total Syscalls</div>
                                    </div>
                                    <div class="metric">
                                        <div class="metric-value">${Object.keys(ioPatterns).length}</div>
                                        <div class="metric-label">I/O Categories</div>
                                    </div>
                                    <div class="metric">
                                        <div class="metric-value">${metrics.avg_io_size ? metrics.avg_io_size.toFixed(0) : 'N/A'}</div>
                                        <div class="metric-label">Avg I/O Size</div>
                                    </div>
                                    <div class="metric">
                                        <div class="metric-value">${metrics.error_rate ? metrics.error_rate.toFixed(2) : '0.00'}%</div>
                                        <div class="metric-label">Error Rate</div>
                                    </div>
                                </div>
                                <div class="links">
                                    <a href="${analysis.dir}/syscall_analysis.html" target="_blank">📈 View Analysis</a>
                                    <a href="mock_strace.txt" target="_blank">📄 Raw Data</a>
                                </div>
                            </div>
                        `;

                        grid.appendChild(card);
                    }
                })
                .catch(() => {
                    const card = document.createElement('div');
                    card.className = 'analysis-card';
                    card.innerHTML = `
                        <div class="card-header">
                            <h3>${analysis.name}</h3>
                        </div>
                        <div class="card-content">
                            <p style="text-align: center; color: #666;">Analysis not available</p>
                        </div>
                    `;
                    grid.appendChild(card);
                });
        });
    </script>
</body>
</html>
EOF

echo -e "${GREEN}✓ Comparison dashboard created${NC}"
echo

# 5. Print summary
echo -e "${GREEN}🎉 Enhanced Analysis Complete!${NC}"
echo
echo -e "${BLUE}Generated files in $OUTPUT_DIR/:${NC}"
echo "  ├── mock_strace.txt              # Mock strace data"
echo "  ├── mock_strace_python.txt       # Python comparison data"
echo "  ├── mock_strace_cpp.txt          # C++ comparison data"
echo "  ├── enhanced_analysis/           # Main enhanced analysis"
echo "  ├── python_analysis/             # Python comparison analysis"
echo "  ├── cpp_analysis/                # C++ comparison analysis"
echo "  └── comparison_dashboard.html    # Interactive comparison dashboard"
echo
echo -e "${YELLOW}Key Features Demonstrated:${NC}"
echo "  🔹 I/O Pattern Categorization (file I/O, memory, network, etc.)"
echo "  🔹 Performance Metrics (timing, error rates, I/O efficiency)"
echo "  🔹 Enhanced Parsing (I/O sizes, file descriptors, error detection)"
echo "  🔹 Cross-Language Comparison Capabilities"
echo "  🔹 Interactive Visualizations"
echo "  🔹 Stack Trace Integration"
echo
echo -e "${PURPLE}To explore the results:${NC}"
echo "  1. Open $OUTPUT_DIR/comparison_dashboard.html in your browser"
echo "  2. Click through individual analysis reports"
echo "  3. Compare I/O patterns across different implementations"
echo "  4. Analyze performance bottlenecks and optimization opportunities"
echo
echo -e "${BLUE}💡 Pro Tips:${NC}"
echo "  • Look at I/O pattern distributions to understand workload characteristics"
echo "  • Compare error rates between implementations"
echo "  • Analyze stack traces to identify syscall sources in code"
echo "  • Use timing data to identify performance bottlenecks"
echo "  • Monitor file descriptor usage for resource leaks"
