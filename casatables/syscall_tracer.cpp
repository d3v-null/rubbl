#include <casa/aips.h>
#include <casa/Exceptions/Error.h>
#include <casa/Arrays/Vector.h>
#include <casa/Arrays/Array.h>
#include <casa/BasicSL/Complex.h>
#include <tables/Tables/TableDesc.h>
#include <tables/Tables/SetupNewTab.h>
#include <tables/Tables/Table.h>
#include <tables/Tables/ScalarColumn.h>
#include <tables/Tables/ArrayColumn.h>
#include <tables/Tables/TableRecord.h>
#include <iostream>
#include <cstdlib>
#include <string>

using namespace casacore;

enum class WriteMode {
    COLUMN_PUT_BULK,
    COLUMN_PUT_MMAP
};

WriteMode parseWriteMode() {
    const char* mode = std::getenv("WRITE_MODE");
    if (mode && std::string(mode) == "column_put_mmap") {
        return WriteMode::COLUMN_PUT_MMAP;
    }
    return WriteMode::COLUMN_PUT_BULK; // default
}

Table createTable(const std::string& tablePath, uInt nRows) {
    // Create table description
    TableDesc td("SYSCALL_TRACER", "1", TableDesc::Scratch);
    
    // Add COMPLEX column (array column, variable shape)
    td.addColumn(ArrayColumnDesc<Complex>("DATA_COMPLEX", "Complex data column", IPosition(), ColumnDesc::Direct));
    
    // Add BOOL column (scalar column)
    td.addColumn(ScalarColumnDesc<Bool>("FLAG_BOOL", "Boolean flag column"));
    
    // Setup and create table
    SetupNewTable newtab(tablePath, td, Table::New);
    Table table(newtab, nRows);
    
    return table;
}

void writeComplexColumn(Table& table, uInt nRows, WriteMode writeMode) {
    ArrayColumn<Complex> complexCol(table, "DATA_COMPLEX");
    
    for (uInt row = 0; row < nRows; row++) {
        Vector<Complex> complexData(2);
        switch (writeMode) {
            case WriteMode::COLUMN_PUT_BULK:
                complexData[0] = Complex(static_cast<Float>(row), static_cast<Float>(row + 1));
                complexData[1] = Complex(static_cast<Float>(row * 2), static_cast<Float>(row * 3));
                break;
            case WriteMode::COLUMN_PUT_MMAP:
                complexData[0] = Complex(static_cast<Float>(row) + 0.5f, static_cast<Float>(row + 1) + 0.5f);
                complexData[1] = Complex(static_cast<Float>(row * 2) + 0.5f, static_cast<Float>(row * 3) + 0.5f);
                break;
        }
        complexCol.put(row, complexData);
    }
}

void writeBoolColumn(Table& table, uInt nRows) {
    ScalarColumn<Bool> boolCol(table, "FLAG_BOOL");
    
    for (uInt row = 0; row < nRows; row++) {
        Bool boolData = (row % 2) == 0;
        boolCol.put(row, boolData);
    }
}

int main() {
    try {
        WriteMode writeMode = parseWriteMode();
        
        // Create temporary table path
        std::string tablePath = "/tmp/syscall_test_cpp.ms";
        uInt nRows = 1000; // Reasonable size for syscall analysis
        
        Table table = createTable(tablePath, nRows);
        
        // Write COMPLEX column
        writeComplexColumn(table, nRows, writeMode);
        
        // Write BOOL column  
        writeBoolColumn(table, nRows);
        
        std::cout << "C++ syscall tracer completed" << std::endl;
        
    } catch (const AipsError& e) {
        std::cerr << "Error: " << e.getMesg() << std::endl;
        return 1;
    }
    
    return 0;
}