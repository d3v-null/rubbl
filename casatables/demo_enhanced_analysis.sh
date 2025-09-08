#!/bin/bash

# Cross-language syscall analysis demonstration
# Compares syscall patterns across C++, Rust, and Python using casacore

set -e

echo "Cross-Language Syscall Analysis Demo"
echo "======================================="
echo

# Resolve script directory for robust relative paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Create output directory
OUTPUT_DIR="syscall_analysis_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$OUTPUT_DIR"

echo -e "${BLUE}Output directory: $OUTPUT_DIR${NC}"
echo

# Helpers: print file stats and select first non-empty trace
print_file_stats() {
    local f="$1"
    if [ -f "$f" ]; then
        local bytes lines
        bytes=$(wc -c < "$f" | tr -d ' ')
        lines=$(wc -l < "$f" | tr -d ' ')
        echo "Trace file: $f (bytes: $bytes, lines: $lines)"
    fi
}

select_trace() {
    # args: file1 file2 file3 ... returns first non-empty existing path
    for f in "$@"; do
        if [ -s "$f" ]; then
            echo "$f"
            return 0
        fi
    done
    echo ""
}

# Debugger-based syscall stack trace collection (GDB only)
run_with_gdb() {
    local out_file=$1
    shift
    local cmd=("$@")

    if ! command -v gdb >/dev/null 2>&1; then
        return 1
    fi

    local gdb_cmds
    gdb_cmds="$(mktemp -t gdb_cmds.XXXXXX)"
    cat > "$gdb_cmds" << 'GDBEOF'
set logging overwrite on
set logging file gdb_traces.txt
set logging on
set pagination 0
catch syscall read
commands
  printf "SYSCALL: read\n"
  bt
  continue
end
catch syscall write
commands
  printf "SYSCALL: write\n"
  bt
  continue
end
catch syscall open
commands
  printf "SYSCALL: open\n"
  bt
  continue
end
catch syscall openat
commands
  printf "SYSCALL: openat\n"
  bt
  continue
end
run
GDBEOF

    if gdb -q -batch --args "${cmd[@]}" -x "$gdb_cmds" >/dev/null 2>&1; then
        if [ -f gdb_traces.txt ]; then
            mv gdb_traces.txt "$out_file"
            rm -f "$gdb_cmds"
            return 0
        fi
    fi
    rm -f "$gdb_cmds" gdb_traces.txt 2>/dev/null || true
    return 1
}

    # LLDB disabled per user; GDB only

# OS and Python detection
OS_NAME="$(uname -s)"
PYTHON_BIN="python3"
RUST_BIN=""
if command -v brew >/dev/null 2>&1; then
    # Prefer Homebrew Python on macOS to ensure dev headers are present
    if [ "$OS_NAME" = "Darwin" ]; then
        if brew --prefix python@3.11 >/dev/null 2>&1; then
            PY_HOME="$(brew --prefix python@3.11)"
            if [ -x "$PY_HOME/bin/python3.11" ]; then
                PYTHON_BIN="$PY_HOME/bin/python3.11"
            fi
        elif brew --prefix python >/dev/null 2>&1; then
            PY_HOME="$(brew --prefix python)"
            if [ -x "$PY_HOME/bin/python3" ]; then
                PYTHON_BIN="$PY_HOME/bin/python3"
            fi
        fi
    fi
fi

# Function to run analysis
run_analysis() {
    local title=$1
    local strace_file=$2
    local output_dir=$3

    echo -e "${YELLOW}Running analysis for $title...${NC}"

    # Generate analysis
    "$PYTHON_BIN" "$SCRIPT_DIR/analyze_syscalls.py" "$strace_file" \
        --format json \
        --output "$output_dir" \
        --title "$title"

    echo -e "${GREEN}✓ Analysis completed for $title${NC}"
    echo
}

# Function to build C++ tracer
build_cpp_tracer() {
    echo -e "${BLUE}Building C++ tracer with casacore...${NC}"
    # Clean first (ignore errors if target doesn't exist)
    pushd "$SCRIPT_DIR" >/dev/null
    make clean 2>/dev/null || true
    if make; then
        echo -e "${GREEN}✓ C++ build successful${NC}"
        popd >/dev/null
        return 0
    else
        echo -e "${RED}✗ C++ build failed${NC}"
        popd >/dev/null
        return 1
    fi
}

# Function to build Rust example and set RUST_BIN
build_rust_example() {
    echo -e "${BLUE}Building Rust example (syscall_tracer)...${NC}"
    # Build from repo root
    pushd "$SCRIPT_DIR/.." >/dev/null
    if cargo build -p rubbl_casatables --example syscall_tracer >/dev/null 2>&1; then
        if [ -x "target/debug/examples/syscall_tracer" ]; then
            RUST_BIN="$(pwd)/target/debug/examples/syscall_tracer"
        elif [ -x "target/dev/examples/syscall_tracer" ]; then
            RUST_BIN="$(pwd)/target/dev/examples/syscall_tracer"
        else
            RUST_BIN=""
        fi
    fi
    popd >/dev/null
    if [ -n "$RUST_BIN" ]; then
        echo -e "${GREEN}✓ Rust example built at $RUST_BIN${NC}"
        return 0
    else
        echo -e "${RED}✗ Failed to build Rust example${NC}"
        return 1
    fi
}

# Function to check Python casacore
check_python_casacore() {
    echo -e "${BLUE}Checking Python casacore installation...${NC}"
    if "$PYTHON_BIN" -c "import casacore" 2>/dev/null; then
        echo -e "${GREEN}✓ Python casacore available${NC}"
        return 0
    else
        echo -e "${YELLOW}⚠ Python casacore not found, installing...${NC}"
        # Upgrade build toolchain and try install for the selected interpreter
        "$PYTHON_BIN" -m pip install --upgrade pip setuptools wheel scikit-build-core cmake ninja >/dev/null 2>&1 || true
        if "$PYTHON_BIN" -m pip install python-casacore; then
            echo -e "${GREEN}✓ Python casacore installed${NC}"
            return 0
        else
            echo -e "${RED}✗ Failed to install python-casacore${NC}"
            return 1
        fi
    fi
}

# 1. Setup dependencies
echo -e "${BLUE}1. Setting up dependencies (Linux GDB only)${NC}"

# Build C++ tracer
if ! build_cpp_tracer; then
    echo -e "${RED}Skipping C++ analysis due to build failure${NC}"
    SKIP_CPP=true
fi

# Build Rust example
if ! build_rust_example; then
    echo -e "${RED}Skipping Rust analysis due to build failure${NC}"
    SKIP_RUST=true
fi

# Check Python casacore
if ! check_python_casacore; then
    echo -e "${RED}Skipping Python analysis due to missing casacore${NC}"
    SKIP_PYTHON=true
fi

echo

# 2. Run analysis for each language
echo -e "${BLUE}2. Running syscall analysis for each language${NC}"

echo -e "${YELLOW}Rust Analysis:${NC}"
if [ "$SKIP_RUST" = "true" ]; then
    echo -e "${RED}Skipping Rust analysis due to build failure${NC}"
else
    if run_with_gdb "$OUTPUT_DIR/gdb_rust.txt" "$RUST_BIN"; then
        print_file_stats "$OUTPUT_DIR/gdb_rust.txt"
        run_analysis "Rust - Casatables" "$OUTPUT_DIR/gdb_rust.txt" "$OUTPUT_DIR/rust_analysis"
    else
        echo -e "${RED}✗ Failed to capture Rust trace (gdb unavailable). Skipping Rust analysis.${NC}"
    fi
fi

# Python analysis
if [ "$SKIP_PYTHON" != "true" ]; then
    echo -e "${YELLOW}Python Analysis:${NC}"
    if run_with_gdb "$OUTPUT_DIR/gdb_python.txt" "$PYTHON_BIN" "$SCRIPT_DIR/syscall_tracer.py"; then
        print_file_stats "$OUTPUT_DIR/gdb_python.txt"
        run_analysis "Python - Casacore" "$OUTPUT_DIR/gdb_python.txt" "$OUTPUT_DIR/python_analysis"
    else
        echo -e "${RED}✗ Failed to capture Python trace. Skipping Python analysis.${NC}"
    fi
fi

# C++ analysis
if [ "$SKIP_CPP" != "true" ]; then
    echo -e "${YELLOW}C++ Analysis:${NC}"
    if run_with_gdb "$OUTPUT_DIR/gdb_cpp.txt" "$SCRIPT_DIR/syscall_tracer_cpp"; then
        print_file_stats "$OUTPUT_DIR/gdb_cpp.txt"
        run_analysis "C++ - CasaCore" "$OUTPUT_DIR/gdb_cpp.txt" "$OUTPUT_DIR/cpp_analysis"
    else
        echo -e "${RED}✗ Failed to capture C++ trace. Skipping C++ analysis.${NC}"
    fi
fi

# 3. Create comparison dashboard
echo -e "${BLUE}3. Creating comparison dashboard${NC}"
cat > "$OUTPUT_DIR/comparison_dashboard.html" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Cross-Language Syscall Analysis Comparison</title>
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
            <h1>Cross-Language Syscall Analysis</h1>
            <p>Comparing C++, Rust, and Python implementations using CasaCore</p>
        </div>

        <div class="comparison-grid" id="comparison-grid">
            <!-- Analysis cards will be inserted here -->
        </div>

        <div class="insights">
            <h3>Key Insights</h3>
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
            { name: 'Rust - Casatables', dir: 'rust_analysis' },
            { name: 'Python - CasaCore', dir: 'python_analysis' },
            { name: 'C++ - CasaCore', dir: 'cpp_analysis' }
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
                                    <a href="${analysis.dir}/syscall_analysis.html" target="_blank">View Analysis</a>
                                    <a href="gdb_${analysis.dir.split('_')[0]}.txt" target="_blank">Raw Data</a>
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

# Print summary
echo -e "${GREEN}Cross-Language Syscall Analysis Complete!${NC}"
echo
echo -e "${BLUE}Generated files in $OUTPUT_DIR/:${NC}"
echo "  ├── strace_rust.txt              # Rust strace data"
echo "  ├── strace_python.txt            # Python strace data"
echo "  ├── strace_cpp.txt               # C++ strace data"
echo "  ├── rust_analysis/               # Rust analysis report"
echo "  ├── python_analysis/             # Python analysis report"
echo "  ├── cpp_analysis/                # C++ analysis report"
echo "  └── comparison_dashboard.html    # Interactive comparison dashboard"
echo
echo -e "${YELLOW}Analysis Features:${NC}"
echo "  - I/O Pattern Categorization (file I/O, memory, network, etc.)"
echo "  - Performance Metrics (timing, error rates, I/O efficiency)"
echo "  - Cross-Language Comparison using CasaCore"
echo "  - Interactive Visualizations"
echo "  - Stack Trace Integration"
echo
echo -e "${PURPLE}To explore the results:${NC}"
echo "  1. Open $OUTPUT_DIR/comparison_dashboard.html in your browser"
echo "  2. Click through individual analysis reports"
echo "  3. Compare syscall patterns across Rust, Python, and C++"
echo "  4. Analyze performance differences and optimization opportunities"
echo
echo -e "${BLUE}Analysis Tips:${NC}"
echo "  - Compare syscall counts between implementations"
echo "  - Look at I/O pattern distributions"
echo "  - Analyze timing differences"
echo "  - Check error rates and patterns"
