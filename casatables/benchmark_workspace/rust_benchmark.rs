use std::env;
use std::path::Path;
use tempfile::tempdir;
use rubbl_casatables::{
    ColumnDescription, GlueDataType, Table, TableCreateMode, 
    TableDesc, TableDescCreateMode, TableOpenMode
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <table_name> <num_rows> <num_cols>", args[0]);
        return Ok(());
    }
    
    let table_name = &args[1];
    let num_rows: usize = args[2].parse()?;
    let num_cols: usize = args[3].parse()?;
    
    let write_mode = env::var("WRITE_MODE").unwrap_or_else(|_| "column_put_bulk".to_string());
    
    println!("Creating table with {} rows and {} columns", num_rows, num_cols);
    
    // Create table description
    let mut table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH)?;
    
    // Add scalar columns for simple benchmark
    for i in 0..num_cols {
        table_desc.add_scalar_column(
            GlueDataType::TpDouble,
            &format!("COL_{}", i),
            Some(&format!("Column {}", i)),
            true,  // direct
            false, // undefined
        )?;
    }
    
    // Add array column for more complex operations
    table_desc.add_fixed_array_column(
        GlueDataType::TpDouble,
        "UVW",
        Some("UVW coordinates"),
        &[3],
        true,
        false,
    )?;
    
    // Create table
    let mut table = Table::new(table_name, table_desc, num_rows, TableCreateMode::New)?;
    
    println!("Starting write operations (mode: {})", write_mode);
    
    match write_mode.as_str() {
        "column_put_bulk" => {
            // Bulk column operations - write entire columns at once
            for col_idx in 0..num_cols {
                let col_name = format!("COL_{}", col_idx);
                for row_idx in 0..num_rows {
                    let value = (col_idx as f64) * 1000.0 + (row_idx as f64);
                    table.put_cell(&col_name, row_idx as u64, &value)?;
                }
            }
            
            // Write UVW data
            for row_idx in 0..num_rows {
                let uvw_data = vec![
                    row_idx as f64 * 0.1,
                    row_idx as f64 * 0.2, 
                    row_idx as f64 * 0.3
                ];
                table.put_cell("UVW", row_idx as u64, &uvw_data)?;
            }
        },
        "row_put_bulk" => {
            // Row-wise operations - write entire rows at once using TableRow
            for row_idx in 0..num_rows {
                let mut row = table.read_row(row_idx as u64, false)?;
                
                for col_idx in 0..num_cols {
                    let col_name = format!("COL_{}", col_idx);
                    let value = (col_idx as f64) * 1000.0 + (row_idx as f64);
                    row.put_cell(&col_name, &value)?;
                }
                
                let uvw_data = vec![
                    row_idx as f64 * 0.1,
                    row_idx as f64 * 0.2, 
                    row_idx as f64 * 0.3
                ];
                row.put_cell("UVW", &uvw_data)?;
                row.write(row_idx as u64)?;
            }
        },
        _ => {
            println!("Unknown write mode: {}", write_mode);
            return Ok(());
        }
    }
    
    println!("Starting read operations");
    
    // Read back data for verification and more syscalls
    let mut total_checksum = 0.0;
    for row_idx in 0..num_rows {
        for col_idx in 0..num_cols {
            let col_name = format!("COL_{}", col_idx);
            let value: f64 = table.get_cell(&col_name, row_idx as u64)?;
            total_checksum += value;
        }
        
        let uvw_data: Vec<f64> = table.get_cell_as_vec("UVW", row_idx as u64)?;
        total_checksum += uvw_data.iter().sum::<f64>();
    }
    
    println!("Benchmark completed. Checksum: {}", total_checksum);
    Ok(())
}
