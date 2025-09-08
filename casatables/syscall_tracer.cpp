#include <iostream>
#include <vector>
#include <complex>
#include <fstream>
#include <filesystem>
#include <casacore/casa/aips.h>
#include <casacore/tables/Tables/Table.h>
#include <casacore/tables/Tables/TableDesc.h>
#include <casacore/tables/Tables/SetupNewTab.h>
#include <casacore/tables/Tables/ScalarColumn.h>
#include <casacore/tables/Tables/ArrayColumn.h>
#include <casacore/casa/Arrays/Vector.h>
#include <casacore/casa/Arrays/Matrix.h>
#include <casacore/casa/Arrays/IPosition.h>
#include <casacore/casa/BasicSL/Complex.h>

int main() {
    std::cout << "Starting C++ syscall tracer with casacore..." << std::endl;

    try {
        // Initialize casacore
        casa::initCasa();

        // Simple operations for syscall analysis
        casa::uInt n_rows = 10;
        casa::IPosition data_shape(2, 16, 4);  // 16x4 matrix

        // Create temporary directory
        auto tmp_dir = std::filesystem::temp_directory_path() / "syscall_test_cpp";
        std::filesystem::create_directory(tmp_dir);
        std::string table_path = (tmp_dir / "syscall_test_cpp.ms").string();

        std::cout << "Creating casacore table..." << std::endl;

        // Create table description
        casa::TableDesc td("syscall_test", "1", casa::TableDesc::Scratch);

        // Add scalar columns
        td.addColumn(casa::ScalarColumnDesc<casa::Double>("TIME", "Observation time"));
        td.addColumn(casa::ScalarColumnDesc<casa::Int>("ANTENNA1", "First antenna"));
        td.addColumn(casa::ScalarColumnDesc<casa::Int>("ANTENNA2", "Second antenna"));
        td.addColumn(casa::ScalarColumnDesc<casa::Bool>("FLAG_ROW", "Row flag"));

        // Add array columns
        td.addColumn(casa::ArrayColumnDesc<casa::Complex>("DATA", "Visibility data", data_shape, casa::ColumnDesc::FixedShape));
        td.addColumn(casa::ArrayColumnDesc<casa::Bool>("FLAG", "Data flags", data_shape, casa::ColumnDesc::FixedShape));

        // Create the table
        casa::SetupNewTable setup(table_path, td, casa::Table::New);
        casa::Table table(setup, n_rows);

        std::cout << "Writing data..." << std::endl;

        // Create column objects
        casa::ScalarColumn<casa::Double> time_col(table, "TIME");
        casa::ScalarColumn<casa::Int> ant1_col(table, "ANTENNA1");
        casa::ScalarColumn<casa::Int> ant2_col(table, "ANTENNA2");
        casa::ScalarColumn<casa::Bool> flag_row_col(table, "FLAG_ROW");
        casa::ArrayColumn<casa::Complex> data_col(table, "DATA");
        casa::ArrayColumn<casa::Bool> flag_col(table, "FLAG");

        // Write a few rows
        for (casa::uInt row_idx = 0; row_idx < 3; ++row_idx) {
            // Create test data
            casa::Matrix<casa::Complex> data_matrix(data_shape);
            casa::Matrix<casa::Bool> flag_matrix(data_shape, false);

            // Fill data matrix with test values
            for (casa::uInt i = 0; i < data_shape[0]; ++i) {
                for (casa::uInt j = 0; j < data_shape[1]; ++j) {
                    casa::uInt idx = i * data_shape[1] + j;
                    data_matrix(i, j) = casa::Complex(static_cast<float>(idx), 0.0f);
                    flag_matrix(i, j) = (idx % 13 == 0);  // Some pattern for flags
                }
            }

            // Write data to columns
            time_col.put(row_idx, static_cast<casa::Double>(row_idx));
            ant1_col.put(row_idx, static_cast<casa::Int>(row_idx % 128));
            ant2_col.put(row_idx, static_cast<casa::Int>((row_idx + 1) % 128));
            flag_row_col.put(row_idx, (row_idx % 2 == 0));
            data_col.put(row_idx, data_matrix);
            flag_col.put(row_idx, flag_matrix);
        }

        std::cout << "Reading data..." << std::endl;

        // Read some data back
        for (casa::uInt row_idx = 0; row_idx < 3; ++row_idx) {
            casa::Double time_val = time_col(row_idx);
            casa::Int ant1_val = ant1_col(row_idx);
            casa::Int ant2_val = ant2_col(row_idx);
            casa::Bool flag_row_val = flag_row_col(row_idx);
            casa::Matrix<casa::Complex> data_matrix = data_col(row_idx);
            casa::Matrix<casa::Bool> flag_matrix = flag_col(row_idx);

            // Some processing to generate syscalls
            casa::Double processed_time = time_val * 2.0;
            casa::Int baseline = ant1_val + ant2_val;
            casa::Bool combined_flag = flag_row_val || casa::anyTrue(flag_matrix);
        }

        std::cout << "Performing additional I/O operations..." << std::endl;

        // Additional file I/O operations
        std::string test_file = (tmp_dir / "test_data.bin").string();
        std::ofstream out_file(test_file, std::ios::binary);

        // Write some binary data
        std::vector<std::complex<float>> binary_data(64);
        for (size_t i = 0; i < binary_data.size(); ++i) {
            binary_data[i] = std::complex<float>(static_cast<float>(i), 1.0f);
        }
        out_file.write(reinterpret_cast<char*>(binary_data.data()),
                      binary_data.size() * sizeof(std::complex<float>));
        out_file.close();

        // Read it back
        std::ifstream in_file(test_file, std::ios::binary);
        std::vector<std::complex<float>> loaded_data(binary_data.size());
        in_file.read(reinterpret_cast<char*>(loaded_data.data()),
                    loaded_data.size() * sizeof(std::complex<float>));

        // Create a text metadata file
        std::string metadata_file = (tmp_dir / "metadata.txt").string();
        std::ofstream meta_file(metadata_file);
        meta_file << "n_rows: " << n_rows << std::endl;
        meta_file << "data_shape: [" << data_shape[0] << ", " << data_shape[1] << "]" << std::endl;
        meta_file << "test_value: 42" << std::endl;
        meta_file.close();

        // Read it back
        std::ifstream meta_in_file(metadata_file);
        std::string line;
        while (std::getline(meta_in_file, line)) {
            // Process metadata line
            size_t colon_pos = line.find(':');
            if (colon_pos != std::string::npos) {
                std::string key = line.substr(0, colon_pos);
                std::string value = line.substr(colon_pos + 1);
                // Some processing
                std::string processed_key = key + "_processed";
            }
        }

        // Clean up
        table.markForDelete();
        std::filesystem::remove_all(tmp_dir);

        std::cout << "C++ syscall tracer with casacore completed successfully." << std::endl;

    } catch (const casa::AipsError& e) {
        std::cerr << "CasaCore error: " << e.getMesg() << std::endl;
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "Standard error: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}
