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
#include <casacore/casa/Arrays/Vector.h>
#include <casacore/casa/Arrays/Matrix.h>
#include <casacore/casa/Arrays/IPosition.h>
#include <casacore/casa/BasicSL/Complex.h>

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

        // Add array columns
        td.addColumn(ArrayColumnDesc<Complex>("DATA", "Visibility data", data_shape, ColumnDesc::FixedShape));
        td.addColumn(ArrayColumnDesc<Bool>("FLAG", "Data flags", data_shape, ColumnDesc::FixedShape));

        // Create the table
        SetupNewTable setup(table_path, td, Table::New);
        Table table(setup, n_rows);

        std::cout << "Writing data..." << std::endl;

        // Write mode via env: WRITE_MODE={column_put|column_put_bulk}
        const char *mode_env = std::getenv("WRITE_MODE");
        std::string write_mode = mode_env ? std::string(mode_env) : std::string("column_put");

        // Create column objects
        ScalarColumn<Double> time_col(table, "TIME");
        ScalarColumn<Int> ant1_col(table, "ANTENNA1");
        ScalarColumn<Int> ant2_col(table, "ANTENNA2");
        ScalarColumn<Bool> flag_row_col(table, "FLAG_ROW");
        ArrayColumn<Complex> data_col(table, "DATA");
        ArrayColumn<Bool> flag_col(table, "FLAG");

        if (write_mode == "column_put_bulk") {
            // Bulk path: write arrays for first 3 rows in one call each, mirroring Rust bulk
            const uInt rows_to_write = 3;
            IPosition arrShape(3, rows_to_write, data_shape[0], data_shape[1]);

            Array<Complex> data_arr(arrShape);
            Array<Bool> flag_arr(arrShape);

            // Fill arrays: layout is [row, i, j]
            {
                Array<Complex>::iterator it = data_arr.begin();
                for (uInt r = 0; r < rows_to_write; ++r) {
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
                for (uInt r = 0; r < rows_to_write; ++r) {
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
        } else {
            // Per-row path: write a few rows
            for (uInt row_idx = 0; row_idx < 3; ++row_idx) {
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
