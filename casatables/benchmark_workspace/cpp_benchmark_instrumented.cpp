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

// Simple standalone C++ benchmark that doesn't require casacore headers
// This will help us understand if the zero-writing is casacore-specific or FFI-specific

class SimpleBenchmark {
private:
    std::string table_path;
    int num_rows;
    int num_cols;

public:
    SimpleBenchmark(const std::string& path, int rows, int cols)
        : table_path(path), num_rows(rows), num_cols(cols) {}

    void createMockTable() {
        std::cout << "Creating mock table with " << num_rows << " rows and " << num_cols << " columns" << std::endl;

        // Create directory structure
        std::string mkdir_cmd = "mkdir -p " + table_path;
        system(mkdir_cmd.c_str());

        // Create some mock files to simulate casacore behavior
        std::vector<std::string> files = {
            table_path + "/table.dat",
            table_path + "/table.f0",
            table_path + "/table.info"
        };

        for (const auto& file : files) {
            std::cout << "Creating file: " << file << std::endl;

            FILE* fp = fopen(file.c_str(), "wb");
            if (fp) {
                // Simulate different allocation strategies
                if (file.find("table.f0") != std::string::npos) {
                    // This is where casacore typically stores column data
                    std::cout << "  Allocating space for column data file..." << std::endl;

                    // Calculate approximate size needed
                    size_t data_size = num_rows * num_cols * sizeof(double);
                    size_t block_size = 4096; // Typical block size
                    size_t total_blocks = (data_size + block_size - 1) / block_size;

                    std::cout << "  Data size: " << data_size << " bytes, blocks: " << total_blocks << std::endl;

                    // Strategy 1: Use ftruncate (efficient)
                    if (getenv("CPP_ALLOC_METHOD") && strcmp(getenv("CPP_ALLOC_METHOD"), "ftruncate") == 0) {
                        std::cout << "  Using ftruncate for efficient allocation" << std::endl;
                        ftruncate(fileno(fp), data_size);
                    }
                    // Strategy 2: Use fallocate (if available)
                    else if (getenv("CPP_ALLOC_METHOD") && strcmp(getenv("CPP_ALLOC_METHOD"), "fallocate") == 0) {
                        std::cout << "  Using fallocate for efficient allocation" << std::endl;
                        #ifdef __linux__
                        fallocate(fileno(fp), 0, 0, data_size);
                        #else
                        ftruncate(fileno(fp), data_size);
                        #endif
                    }
                    // Strategy 3: Write zeros (inefficient, like current Rust)
                    else {
                        std::cout << "  Writing zeros explicitly (inefficient method)" << std::endl;
                        std::vector<char> zero_block(block_size, 0);
                        for (size_t i = 0; i < total_blocks; i++) {
                            size_t write_size = std::min(block_size, data_size - i * block_size);
                            fwrite(zero_block.data(), 1, write_size, fp);
                        }
                    }
                } else {
                    // Other files - just create small metadata
                    std::string header = "Mock file for " + file + "\n";
                    fwrite(header.c_str(), 1, header.length(), fp);
                }

                fclose(fp);
            }
        }
    }

    void runBenchmark() {
        auto start = std::chrono::high_resolution_clock::now();

        createMockTable();

        // Simulate some data operations
        std::cout << "Simulating data operations..." << std::endl;
        double total = 0.0;
        for (int row = 0; row < num_rows; row++) {
            for (int col = 0; col < num_cols; col++) {
                total += row * col;
            }
        }

        auto end = std::chrono::high_resolution_clock::now();
        auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end - start);

        std::cout << "Mock benchmark completed in " << duration.count() << "ms" << std::endl;
        std::cout << "Checksum: " << total << std::endl;
    }
};

int main(int argc, char* argv[]) {
    if (argc < 4) {
        std::cerr << "Usage: " << argv[0] << " <table_name> <num_rows> <num_cols>" << std::endl;
        std::cerr << "Environment variables:" << std::endl;
        std::cerr << "  CPP_ALLOC_METHOD=ftruncate|fallocate|zeros" << std::endl;
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

    std::string alloc_method = getenv("CPP_ALLOC_METHOD") ? getenv("CPP_ALLOC_METHOD") : "zeros";
    std::cout << "C++ Mock Benchmark - Allocation method: " << alloc_method << std::endl;

    SimpleBenchmark benchmark(table_name, num_rows, num_cols);
    benchmark.runBenchmark();

    return 0;
}