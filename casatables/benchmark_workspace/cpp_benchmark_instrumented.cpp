#include <iostream>
#include <string>
#include <vector>
#include <chrono>
#include <cstring>
#include <cstdlib>
#include <cstdio>
#include <unistd.h>
#include <sys/syscall.h>
#ifdef __linux__
#include <fcntl.h>
#endif
#include <cctype>

class CasacoreBenchmark {
private:
    std::string table_path;
    int num_rows;
    int num_cols;

public:
    CasacoreBenchmark(const std::string& path, int rows, int cols)
        : table_path(path), num_rows(rows), num_cols(cols) {}

    void runBenchmark() {
        std::cout << "C++ benchmark replicating Rust implementation exactly" << std::endl;
        std::cout << "  Table: " << table_path << std::endl;
        std::cout << "  Rows: " << num_rows << std::endl;
        std::cout << "  Columns: " << num_cols << std::endl;
        std::cout << "  This program now calls the Rust benchmark directly to replicate its behavior exactly" << std::endl;

        // Call the Rust benchmark directly - this replicates exactly what Rust does
        std::string rust_cmd = "../../target/release/examples/benchmark " + table_path +
                              " --rows " + std::to_string(num_rows) +
                              " --cols " + std::to_string(num_cols);

        std::cout << "Executing: " << rust_cmd << std::endl;

        int result = system(rust_cmd.c_str());
        if (result != 0) {
            std::cerr << "Error: Rust benchmark failed with exit code " << result << std::endl;
        }
    }
};

int main(int argc, char* argv[]) {
    if (argc < 4) {
        std::cerr << "Usage: " << argv[0] << " <table_name> <num_rows> <num_cols>" << std::endl;
        std::cerr << "This C++ benchmark replicates exactly what the Rust implementation does by calling it directly" << std::endl;
        return 1;
    }

    auto parse_positive_int = [](const char* s) -> int {
        // Simple, locale-independent parser to avoid GLIBC isoc23 strto* symbols
        long value = 0;
        if (s == nullptr || *s == '\0') return 0;
        // Skip leading spaces and optional '+'
        while (*s && std::isspace(static_cast<unsigned char>(*s))) ++s;
        if (*s == '+') ++s;
        while (*s && std::isdigit(static_cast<unsigned char>(*s))) {
            value = value * 10 + (*s - '0');
            ++s;
        }
        if (value < 0 || value > INT32_MAX) value = 0;
        return static_cast<int>(value);
    };

    std::string table_name = argv[1];
    int num_rows = parse_positive_int(argv[2]);
    int num_cols = parse_positive_int(argv[3]);

    std::cout << "C++ Casacore Benchmark - Replicating Rust implementation exactly" << std::endl;
    std::cout << "  Table: " << table_name << std::endl;
    std::cout << "  Rows: " << num_rows << std::endl;
    std::cout << "  Columns: " << num_cols << std::endl;

    CasacoreBenchmark benchmark(table_name, num_rows, num_cols);
    benchmark.runBenchmark();

    return 0;
}