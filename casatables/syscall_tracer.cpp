#include <iostream>
#include <vector>
#include <complex>
#include <fstream>
#include <filesystem>
#include <cstdlib>
#include <casacore/casa/aips.h>
#include <casacore/tables/Tables/Table.h>
#include <casacore/tables/Tables/TableDesc.h>
#include <casacore/tables/Tables/SetupNewTab.h>
#include <casacore/tables/Tables/ScalarColumn.h>
#include <casacore/tables/Tables/ArrayColumn.h>
#include <casacore/tables/Tables/ScaColDesc.h>
#include <casacore/tables/Tables/ArrColDesc.h>
#include <casacore/tables/Tables/TableRow.h>
#include <casacore/tables/Tables/TableRecord.h>
#include <casacore/casa/Arrays/Vector.h>
#include <casacore/casa/Arrays/Matrix.h>
#include <casacore/casa/Arrays/IPosition.h>
#include <casacore/casa/BasicSL/Complex.h>
#include <casacore/tables/DataMan/TSMOption.h>

// Use the casacore namespace
using namespace casacore;

int main() {
    std::cout << "Starting C++ syscall tracer with casacore..." << std::endl;

    try {
        // CasaCore initialization is handled automatically

        // Simple operations for syscall analysis
        uInt n_rows = 100;
        IPosition data_shape(2, 32, 4);  // 32x4 matrix

        // Create temporary directory
        auto tmp_dir = std::filesystem::temp_directory_path() / "syscall_test_cpp";
        std::filesystem::create_directory(tmp_dir);
        std::string table_path = (tmp_dir / "syscall_test_cpp.ms").string();

        std::cout << "Creating casacore table..." << std::endl;

        // Create table description
        TableDesc td("syscall_test", "1", TableDesc::Scratch);

        // Add scalar columns
        td.addColumn(ScalarColumnDesc<Double>("TIME", "Observation time"));
        td.addColumn(ScalarColumnDesc<Int>("ANTENNA1", "First antenna"));
        td.addColumn(ScalarColumnDesc<Int>("ANTENNA2", "Second antenna"));
        td.addColumn(ScalarColumnDesc<Bool>("FLAG_ROW", "Row flag"));

        // Add array columns (default SSM)
        td.addColumn(ArrayColumnDesc<Complex>("DATA", "Visibility data", data_shape, ColumnDesc::FixedShape));
        td.addColumn(ArrayColumnDesc<Bool>("FLAG", "Data flags", data_shape, ColumnDesc::FixedShape));

        // Create the table with optional storage option via env STORAGE_MANAGER
        const char* sm_env = std::getenv("STORAGE_MANAGER");
        std::string sm = sm_env ? std::string(sm_env) : std::string("default");
        SetupNewTable setup(table_path, td, Table::New);
        // Map STORAGE_MANAGER to TSMOption and pass into Table constructor (consistent with Rust)
        TSMOption::Option opt = TSMOption::Default;
        if (sm == "mmap") opt = TSMOption::MMap;
        else if (sm == "buffer") opt = TSMOption::Buffer;
        else if (sm == "cache") opt = TSMOption::Cache;
        else if (sm == "aipsrc") opt = TSMOption::Aipsrc;
        TSMOption tsmOpt(opt);
        Table::EndianFormat endian = Table::LocalEndian;
        Bool initialize = False;
        Table table(setup, Table::Plain, n_rows, initialize, endian, tsmOpt);

        std::cout << "Writing data..." << std::endl;

        // Write mode via env: WRITE_MODE={table_put_row|table_put_cell|column_put|column_put_bulk}
        const char *mode_env = std::getenv("WRITE_MODE");
        std::string write_mode = mode_env ? std::string(mode_env) : std::string("column_put");

        // Create column objects
        ScalarColumn<Double> time_col(table, "TIME");
        ScalarColumn<Int> ant1_col(table, "ANTENNA1");
        ScalarColumn<Int> ant2_col(table, "ANTENNA2");
        ScalarColumn<Bool> flag_row_col(table, "FLAG_ROW");
        ArrayColumn<Complex> data_col(table, "DATA");
        ArrayColumn<Bool> flag_col(table, "FLAG");

        if (write_mode == "table_put_row") {
            // Row-wise path using TableRow and TableRecord
            TableRow row(table);
            for (uInt row_idx = 0; row_idx < n_rows; ++row_idx) {
                TableRecord &rec = row.record();
                rec.define("TIME", static_cast<Double>(row_idx));
                rec.define("ANTENNA1", static_cast<Int>(row_idx % 128));
                rec.define("ANTENNA2", static_cast<Int>((row_idx + 1) % 128));
                rec.define("FLAG_ROW", (row_idx % 2 == 0));

                Matrix<Complex> data_matrix(data_shape);
                Matrix<Bool> flag_matrix(data_shape, false);
                for (uInt i = 0; i < data_shape[0]; ++i) {
                    for (uInt j = 0; j < data_shape[1]; ++j) {
                        uInt idx = i * data_shape[1] + j;
                        data_matrix(i, j) = Complex(static_cast<float>(idx), 0.0f);
                        flag_matrix(i, j) = (idx % 13 == 0);
                    }
                }
                rec.define("DATA", data_matrix);
                rec.define("FLAG", flag_matrix);
                row.put(row_idx);
            }
        } else if (write_mode == "column_put_bulk") {
            // Bulk path: write arrays for first 3 rows in one call each, mirroring Rust bulk
            // Use Fortran ordering [i, j, row] to match putColumn expectation
            IPosition arrShape(3, data_shape[0], data_shape[1], n_rows);

            Array<Complex> data_arr(arrShape);
            Array<Bool> flag_arr(arrShape);

            // Fill arrays: layout is [row, i, j]
            {
                Array<Complex>::iterator it = data_arr.begin();
                for (uInt r = 0; r < n_rows; ++r) {
                    for (uInt i = 0; i < data_shape[0]; ++i) {
                        for (uInt j = 0; j < data_shape[1]; ++j) {
                            uInt idx = i * data_shape[1] + j;
                            *it++ = Complex(static_cast<float>(idx), 0.0f);
                        }
                    }
                }
            }
            {
                Array<Bool>::iterator it = flag_arr.begin();
                for (uInt r = 0; r < n_rows; ++r) {
                    for (uInt i = 0; i < data_shape[0]; ++i) {
                        for (uInt j = 0; j < data_shape[1]; ++j) {
                            uInt idx = i * data_shape[1] + j;
                            *it++ = (idx % 13 == 0);
                        }
                    }
                }
            }

            // Put arrays for columns at once
            data_col.putColumn(data_arr);
            flag_col.putColumn(flag_arr);

            // Note: to mirror Rust bulk, we skip scalar column writes here
        } else if (write_mode == "table_put_cell") {
            // Per-cell (by column, per row)
            for (uInt col_idx = 0; col_idx < (uInt)3; ++col_idx) {
                std::string colName = "COL_" + std::to_string(col_idx);
                ScalarColumn<Double> column(table, colName);
                for (uInt row_idx = 0; row_idx < n_rows; ++row_idx) {
                    double value = static_cast<Double>(col_idx) * 1000.0 + static_cast<Double>(row_idx);
                    column.put(row_idx, value);
                }
            }
            ArrayColumn<Complex> data_col(table, "DATA");
            ArrayColumn<Bool> flag_col(table, "FLAG");
            for (uInt row_idx = 0; row_idx < n_rows; ++row_idx) {
                Matrix<Complex> data_matrix(data_shape);
                Matrix<Bool> flag_matrix(data_shape, false);
                for (uInt i = 0; i < data_shape[0]; ++i) {
                    for (uInt j = 0; j < data_shape[1]; ++j) {
                        uInt idx = i * data_shape[1] + j;
                        data_matrix(i, j) = Complex(static_cast<float>(idx), 0.0f);
                        flag_matrix(i, j) = (idx % 13 == 0);
                    }
                }
                data_col.put(row_idx, data_matrix);
                flag_col.put(row_idx, flag_matrix);
            }
        } else if (write_mode == "column_put") {
            // Per-row path: write a few rows
            for (uInt row_idx = 0; row_idx < n_rows; ++row_idx) {
                // Create test data
                Matrix<Complex> data_matrix(data_shape);
                Matrix<Bool> flag_matrix(data_shape, false);

                // Fill data matrix with test values
                for (uInt i = 0; i < data_shape[0]; ++i) {
                    for (uInt j = 0; j < data_shape[1]; ++j) {
                        uInt idx = i * data_shape[1] + j;
                        data_matrix(i, j) = Complex(static_cast<float>(idx), 0.0f);
                        flag_matrix(i, j) = (idx % 13 == 0);  // Some pattern for flags
                    }
                }

                // Write data to columns
                time_col.put(row_idx, static_cast<Double>(row_idx));
                ant1_col.put(row_idx, static_cast<Int>(row_idx % 128));
                ant2_col.put(row_idx, static_cast<Int>((row_idx + 1) % 128));
                flag_row_col.put(row_idx, (row_idx % 2 == 0));
                data_col.put(row_idx, data_matrix);
                flag_col.put(row_idx, flag_matrix);
            }
        } else {
            std::cerr << "Unknown WRITE_MODE: " << write_mode << std::endl;
            return 1;
        }

        // Clean up
        table.markForDelete();
        std::filesystem::remove_all(tmp_dir);

        std::cout << "C++ syscall tracer with casacore completed successfully." << std::endl;

    } catch (const AipsError& e) {
        std::cerr << "CasaCore error: " << e.getMesg() << std::endl;
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "Standard error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
