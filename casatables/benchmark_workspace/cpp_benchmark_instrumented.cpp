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

// Include casacore headers
#include <casacore/tables/Tables.h>
#include <casacore/tables/Tables/TableDesc.h>
#include <casacore/tables/Tables/ScalarColumn.h>
#include <casacore/tables/Tables/ArrayColumn.h>
#include <casacore/tables/Tables/TableRow.h>
#include <casacore/casa/Arrays/Vector.h>
#include <casacore/casa/Arrays/Array.h>

class CasacoreBenchmark {
private:
    std::string table_path;
    int num_rows;
    int num_cols;

public:
    CasacoreBenchmark(const std::string& path, int rows, int cols)
        : table_path(path), num_rows(rows), num_cols(cols) {}

    void runBenchmark() {
        std::cout << "C++ Casacore Benchmark - Direct implementation using casacore C++ API" << std::endl;
        std::cout << "  Table: " << table_path << std::endl;
        std::cout << "  Rows: " << num_rows << std::endl;
        std::cout << "  Columns: " << num_cols << std::endl;
        std::cout << "  Mode: column_put_bulk (replicating Rust's default behavior)" << std::endl;

        try {
            // Create table description (using rubbl_casacore namespace)
            rubbl_casacore::TableDesc tableDesc("", rubbl_casacore::TableDesc::TDOption::Scratch);

            // Add scalar columns (Double type to match Rust benchmark)
            for (int i = 0; i < num_cols; ++i) {
                std::string colName = "COL_" + std::to_string(i);
                int opt = rubbl_casacore::ColumnDesc::Direct;  // Direct storage option
                tableDesc.addColumn(rubbl_casacore::ScalarColumnDesc<rubbl_casacore::Double>(
                    colName,
                    "Column " + std::to_string(i),
                    opt
                ));
            }

            // Add array column for UVW data (3-element double array)
            rubbl_casacore::IPosition shape(1);
            shape(0) = 3;
            int opt = rubbl_casacore::ColumnDesc::Direct;  // Direct storage option
            tableDesc.addColumn(rubbl_casacore::ArrayColumnDesc<rubbl_casacore::Double>(
                "UVW",
                "UVW coordinates",
                shape,
                opt
            ));

            std::cout << "Table description created with " << num_cols << " scalar columns and 1 array column" << std::endl;

            // Create table
            rubbl_casacore::SetupNewTable newTable(table_path, tableDesc, rubbl_casacore::Table::New);
            rubbl_casacore::Table table(newTable, rubbl_casacore::Table::Plain, num_rows);

            std::cout << "Table created successfully" << std::endl;
            std::cout << "Starting write operations..." << std::endl;

            // Write scalar columns using bulk operations (like Rust's column_put_bulk mode)
            for (int col_idx = 0; col_idx < num_cols; ++col_idx) {
                std::string colName = "COL_" + std::to_string(col_idx);
                rubbl_casacore::ScalarColumn<rubbl_casacore::Double> column(table, colName);

                // Create data vector
                rubbl_casacore::Vector<rubbl_casacore::Double> columnData(num_rows);
                for (int row_idx = 0; row_idx < num_rows; ++row_idx) {
                    columnData(row_idx) = static_cast<rubbl_casacore::Double>(col_idx) * 1000.0 + static_cast<rubbl_casacore::Double>(row_idx);
                }

                // Put the entire column at once
                column.putColumn(columnData);
                std::cout << "  Wrote column " << colName << " with " << num_rows << " values" << std::endl;
            }

            // Write UVW array column using individual cell operations
            rubbl_casacore::ArrayColumn<rubbl_casacore::Double> uvwColumn(table, "UVW");
            for (int row_idx = 0; row_idx < num_rows; ++row_idx) {
                rubbl_casacore::Vector<rubbl_casacore::Double> uvwData(3);
                uvwData(0) = static_cast<rubbl_casacore::Double>(row_idx) * 0.1;
                uvwData(1) = static_cast<rubbl_casacore::Double>(row_idx) * 0.2;
                uvwData(2) = static_cast<rubbl_casacore::Double>(row_idx) * 0.3;

                uvwColumn.put(row_idx, uvwData);
            }
            std::cout << "  Wrote UVW array column with " << num_rows << " 3-element arrays" << std::endl;

            std::cout << "Starting read operations for verification..." << std::endl;

            // Read back data for verification and to generate more syscalls
            rubbl_casacore::Double total_checksum = 0.0;

            // Read scalar columns
            for (int col_idx = 0; col_idx < num_cols; ++col_idx) {
                std::string colName = "COL_" + std::to_string(col_idx);
                rubbl_casacore::ScalarColumn<rubbl_casacore::Double> column(table, colName);

                rubbl_casacore::Vector<rubbl_casacore::Double> columnData = column.getColumn();
                for (int row_idx = 0; row_idx < num_rows; ++row_idx) {
                    total_checksum += columnData(row_idx);
                }
            }

            // Read UVW array column
            rubbl_casacore::ArrayColumn<rubbl_casacore::Double> uvwColumnRead(table, "UVW");
            for (int row_idx = 0; row_idx < num_rows; ++row_idx) {
                rubbl_casacore::Vector<rubbl_casacore::Double> uvwData = uvwColumnRead(row_idx);
                for (size_t i = 0; i < uvwData.nelements(); ++i) {
                    total_checksum += uvwData(i);
                }
            }

            std::cout << "Benchmark completed. Checksum: " << total_checksum << std::endl;

        } catch (const rubbl_casacore::AipsError& e) {
            std::cerr << "Casacore error: " << e.getMesg() << std::endl;
            return;
        } catch (const std::exception& e) {
            std::cerr << "Standard error: " << e.what() << std::endl;
            return;
        } catch (...) {
            std::cerr << "Unknown error occurred" << std::endl;
            return;
        }
    }
};

int main(int argc, char* argv[]) {
    if (argc < 4) {
        std::cerr << "Usage: " << argv[0] << " <table_name> <num_rows> <num_cols>" << std::endl;
        std::cerr << "This C++ benchmark replicates the same high-level task as the Rust implementation using casacore C++ API directly" << std::endl;
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