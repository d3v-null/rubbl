#include <iostream>
#include <string>
#include <vector>
#include <casacore/tables/Tables.h>
#include <casacore/casa/Arrays/Array.h>
#include <casacore/casa/Arrays/Vector.h>

using namespace casacore;

int main(int argc, char* argv[]) {
    if (argc < 4) {
        std::cerr << "Usage: " << argv[0] << " <table_name> <num_rows> <num_cols>" << std::endl;
        return 1;
    }
    
    std::string table_name = argv[1];
    int num_rows = std::stoi(argv[2]);
    int num_cols = std::stoi(argv[3]);
    
    std::string write_mode = getenv("WRITE_MODE") ? getenv("WRITE_MODE") : "column_put_bulk";
    
    std::cout << "Creating table with " << num_rows << " rows and " << num_cols << " columns" << std::endl;
    
    // Setup table description
    TableDesc td("", "1", TableDesc::Scratch);
    
    for (int i = 0; i < num_cols; i++) {
        td.addColumn(ScalarColumnDesc<Double>(String("COL_") + String::toString(i)));
    }
    td.addColumn(ArrayColumnDesc<Double>("UVW", IPosition(1, 3), ColumnDesc::FixedShape));
    
    // Create table
    SetupNewTable setup(table_name, td, Table::New);
    Table table(setup, num_rows);
    
    std::cout << "Starting write operations (mode: " << write_mode << ")" << std::endl;
    
    if (write_mode == "column_put_bulk") {
        // Column-wise operations
        for (int col_idx = 0; col_idx < num_cols; col_idx++) {
            std::string col_name = "COL_" + std::to_string(col_idx);
            ScalarColumn<Double> col(table, col_name);
            
            for (int row_idx = 0; row_idx < num_rows; row_idx++) {
                double value = col_idx * 1000.0 + row_idx;
                col.put(row_idx, value);
            }
        }
        
        // UVW column
        ArrayColumn<Double> uvw_col(table, "UVW");
        for (int row_idx = 0; row_idx < num_rows; row_idx++) {
            Vector<Double> uvw_data(3);
            uvw_data(0) = row_idx * 0.1;
            uvw_data(1) = row_idx * 0.2;
            uvw_data(2) = row_idx * 0.3;
            uvw_col.put(row_idx, uvw_data);
        }
    } else if (write_mode == "row_put_bulk") {
        // Row-wise operations
        TableRow row(table);
        for (int row_idx = 0; row_idx < num_rows; row_idx++) {
            TableRecord& rec = row.record();
            
            for (int col_idx = 0; col_idx < num_cols; col_idx++) {
                std::string col_name = "COL_" + std::to_string(col_idx);
                double value = col_idx * 1000.0 + row_idx;
                rec.define(col_name, value);
            }
            
            Vector<Double> uvw_data(3);
            uvw_data(0) = row_idx * 0.1;
            uvw_data(1) = row_idx * 0.2;
            uvw_data(2) = row_idx * 0.3;
            rec.define("UVW", uvw_data);
            
            row.put(row_idx);
        }
    }
    
    std::cout << "Starting read operations" << std::endl;
    
    // Read back for verification
    double total_checksum = 0.0;
    for (int row_idx = 0; row_idx < num_rows; row_idx++) {
        for (int col_idx = 0; col_idx < num_cols; col_idx++) {
            std::string col_name = "COL_" + std::to_string(col_idx);
            ScalarColumn<Double> col(table, col_name);
            total_checksum += col(row_idx);
        }
        
        ArrayColumn<Double> uvw_col(table, "UVW");
        Vector<Double> uvw_data = uvw_col(row_idx);
        for (int i = 0; i < 3; i++) {
            total_checksum += uvw_data(i);
        }
    }
    
    std::cout << "Benchmark completed. Checksum: " << total_checksum << std::endl;
    return 0;
}
