#!/bin/bash

# Demonstration script for cross-language syscall analysis
# This script shows how to trace and compare syscalls across Rust, Python, and C++

set -e

echo "🔍 Cross-Language Syscall Analysis Demo"
echo "========================================"
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create output directory
OUTPUT_DIR="syscall_comparison_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$OUTPUT_DIR"

echo -e "${BLUE}Output directory: $OUTPUT_DIR${NC}"
echo

# Function to run analysis for a language
run_analysis() {
    local lang=$1
    local cmd=$2
    local output_file="$OUTPUT_DIR/strace_$lang.txt"

    echo -e "${YELLOW}Tracing $lang...${NC}"

    # Run strace with comprehensive options
    strace -f -s 256 -k -o "$output_file" $cmd

    echo -e "${GREEN}✓ $lang tracing completed${NC}"

    # Generate analysis
    python3 analyze_syscalls.py "$output_file" \
        --format html \
        --output "$OUTPUT_DIR/analysis_$lang" \
        --title "$lang Syscall Analysis"

    echo -e "${GREEN}✓ $lang analysis completed${NC}"
    echo
}

# 1. Rust Analysis
echo -e "${BLUE}1. Analyzing Rust (casatables)${NC}"
if command -v cargo &> /dev/null; then
    cd "$(dirname "$0")"  # Go to casatables directory
    run_analysis "rust" "cargo run --example syscall_tracer"
else
    echo -e "${RED}✗ Cargo not found, skipping Rust analysis${NC}"
fi

# 2. Python Analysis
echo -e "${BLUE}2. Analyzing Python${NC}"
if command -v python3 &> /dev/null; then
    run_analysis "python" "python3 syscall_tracer.py"
else
    echo -e "${RED}✗ Python3 not found, skipping Python analysis${NC}"
fi

# 3. C++ Analysis
echo -e "${BLUE}3. Analyzing C++${NC}"
if command -v g++ &> /dev/null && [ -f "syscall_tracer.cpp" ]; then
    # Compile C++ program
    echo "Compiling C++ program..."
    g++ -std=c++17 -o syscall_tracer_cpp syscall_tracer.cpp -lnlohmann_json

    if [ $? -eq 0 ]; then
        run_analysis "cpp" "./syscall_tracer_cpp"
        rm syscall_tracer_cpp
    else
        echo -e "${RED}✗ C++ compilation failed${NC}"
    fi
else
    echo -e "${RED}✗ g++ or syscall_tracer.cpp not found, skipping C++ analysis${NC}"
fi

# 4. Generate comparison report
echo -e "${BLUE}4. Generating comparison report${NC}"

# Create comparison HTML
cat > "$OUTPUT_DIR/comparison.html" << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Cross-Language Syscall Comparison</title>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            margin: 0;
            padding: 20px;
            background: #f5f5f5;
        }
        .container {
            max-width: 1200px;
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
        .language-card {
            background: #f8f9fa;
            margin: 20px 0;
            padding: 20px;
            border-radius: 8px;
            border-left: 4px solid #007bff;
        }
        .language-card h3 {
            margin: 0 0 15px 0;
            color: #333;
        }
        .stats {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 15px;
            margin-bottom: 15px;
        }
        .stat {
            background: white;
            padding: 10px;
            border-radius: 5px;
            text-align: center;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }
        .stat-number {
            font-size: 1.5em;
            font-weight: bold;
            color: #007bff;
        }
        .stat-label {
            font-size: 0.9em;
            color: #666;
            margin-top: 5px;
        }
        .links {
            margin-top: 15px;
        }
        .links a {
            color: #007bff;
            text-decoration: none;
            margin-right: 20px;
        }
        .links a:hover {
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🔍 Cross-Language Syscall Analysis</h1>
            <p>Comparing system call patterns across Rust, Python, and C++</p>
        </div>

        <div id="language-cards">
            <!-- Language cards will be inserted here by JavaScript -->
        </div>

        <div style="text-align: center; margin-top: 30px; padding-top: 20px; border-top: 2px solid #eee;">
            <h2>📊 Analysis Results</h2>
            <p>Detailed syscall analysis reports are available in the individual language directories.</p>
        </div>
    </div>

    <script>
        // Load analysis data for each language
        const languages = ['rust', 'python', 'cpp'];
        const languageCards = document.getElementById('language-cards');

        languages.forEach(lang => {
            // Try to load analysis data
            fetch(`analysis_${lang}/syscall_analysis.json`)
                .then(response => response.ok ? response.json() : null)
                .then(data => {
                    if (data) {
                        const card = document.createElement('div');
                        card.className = 'language-card';

                        const langName = lang.charAt(0).toUpperCase() + lang.slice(1);
                        const color = lang === 'rust' ? '#000000' :
                                    lang === 'python' ? '#3776ab' : '#00599c';

                        card.innerHTML = `
                            <h3 style="color: ${color}">${langName}</h3>
                            <div class="stats">
                                <div class="stat">
                                    <div class="stat-number">${data.total_syscalls.toLocaleString()}</div>
                                    <div class="stat-label">Total Syscalls</div>
                                </div>
                                <div class="stat">
                                    <div class="stat-number">${data.unique_syscalls}</div>
                                    <div class="stat-label">Unique Syscalls</div>
                                </div>
                                <div class="stat">
                                    <div class="stat-number">${Object.keys(data.syscalls).length}</div>
                                    <div class="stat-label">Analyzed</div>
                                </div>
                            </div>
                            <div class="links">
                                <a href="analysis_${lang}/syscall_analysis.html" target="_blank">📈 View Analysis</a>
                                <a href="strace_${lang}.txt" target="_blank">📄 Raw Strace</a>
                            </div>
                        `;

                        languageCards.appendChild(card);
                    }
                })
                .catch(() => {
                    // Language analysis not available
                    const card = document.createElement('div');
                    card.className = 'language-card';
                    card.innerHTML = `
                        <h3>${lang.charAt(0).toUpperCase() + lang.slice(1)}</h3>
                        <p style="color: #666; margin: 15px 0;">Analysis not available</p>
                    `;
                    languageCards.appendChild(card);
                });
        });
    </script>
</body>
</html>
EOF

echo -e "${GREEN}✓ Comparison report generated${NC}"
echo

# Print summary
echo -e "${GREEN}🎉 Analysis Complete!${NC}"
echo
echo -e "${BLUE}Generated files:${NC}"
echo "  📁 $OUTPUT_DIR/"
echo "    ├── comparison.html          # Cross-language comparison"
echo "    ├── strace_rust.txt         # Raw Rust strace output"
echo "    ├── strace_python.txt       # Raw Python strace output"
echo "    ├── strace_cpp.txt          # Raw C++ strace output"
echo "    ├── analysis_rust/          # Rust detailed analysis"
echo "    ├── analysis_python/        # Python detailed analysis"
echo "    └── analysis_cpp/           # C++ detailed analysis"
echo
echo -e "${YELLOW}To view results:${NC}"
echo "  1. Open $OUTPUT_DIR/comparison.html in your web browser"
echo "  2. Individual analysis reports are in the analysis_* directories"
echo "  3. Raw strace output is available for further analysis"
echo
echo -e "${BLUE}💡 Tips:${NC}"
echo "  - Compare syscall counts and patterns across languages"
echo "  - Look at stack traces to understand call chains"
echo "  - Consider memory allocation patterns (mmap, brk)"
echo "  - Analyze I/O patterns (read, write, open, close)"
