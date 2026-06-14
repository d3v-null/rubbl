// Copyright 2017-2023 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

//! Syscall tracer for investigating casacore table operations.
//! This example creates tables and performs bulk column operations for syscall analysis.

use anyhow::Error;
use rubbl_casatables::{
    GlueDataType, Table, TableCreateMode, TableDesc, TableDescCreateMode,
};
use std::{env, path::PathBuf, process};
use rubbl_core::num_complex::Complex;

#[derive(Debug, Clone, Copy)]
enum TSMOption {
    TSMBuffer,
    TSMMmap,
}

#[derive(Debug, Clone, Copy)]
enum WriteMode {
    ColumnPutBulk,
    ColumnPutMmap,
}

fn parse_write_mode() -> WriteMode {
    match env::var("WRITE_MODE").as_deref() {
        Ok("column_put_bulk") => WriteMode::ColumnPutBulk,
        Ok("column_put_mmap") => WriteMode::ColumnPutMmap,
        _ => WriteMode::ColumnPutBulk, // default
    }
}

fn parse_tsm_option() -> Option<TSMOption> {
    match env::var("TSM_OPTION").as_deref() {
        Ok("TSM_BUFFER") => Some(TSMOption::TSMBuffer),
        Ok("TSM_MMAP") => Some(TSMOption::TSMMmap),
        _ => None, // TSM_DEFAULT
    }
}

fn is_scratch_mode() -> bool {
    env::var("SCRATCH").is_ok()
}

fn log_debug(msg: &str) {
    if env::var("RUBBL_CASATABLES_DEBUG").is_ok() {
        eprintln!("[rubbl_casatables] {}", msg);
    }
}

fn create_table(table_path: &PathBuf, n_rows: usize) -> Result<Table, Error> {
    let create_mode = if is_scratch_mode() {
        TableDescCreateMode::TDM_SCRATCH
    } else {
        TableDescCreateMode::TDM_SCRATCH // Use scratch to avoid writing .tabdsc files
    };

    let mut table_desc = TableDesc::new("SYSCALL_TRACER", create_mode)?;
    
    // Add COMPLEX column (array column, variable shape)
    table_desc.add_array_column(
        GlueDataType::TpComplex,
        "DATA_COMPLEX",
        None,
        None,
        false,
        false,
    )?;
    
    // Add BOOL column (scalar column)
    table_desc.add_scalar_column(
        GlueDataType::TpBool,
        "FLAG_BOOL",
        None,
        false,
        false,
    )?;

    log_debug(&format!("Creating table with {} rows, TSM: {:?}", n_rows, parse_tsm_option()));
    
    let table = Table::new(table_path, table_desc, n_rows, TableCreateMode::New)?;
    
    Ok(table)
}

fn write_complex_column(table: &mut Table, n_rows: usize, write_mode: WriteMode) -> Result<(), Error> {
    log_debug(&format!("array_column_put_column: n_rows={}, mode={:?}", n_rows, write_mode));
    
    match write_mode {
        WriteMode::ColumnPutBulk => {
            // Write bulk data to COMPLEX column
            for row in 0..n_rows {
                let complex_data = vec![
                    Complex::new(row as f32, (row + 1) as f32),
                    Complex::new((row * 2) as f32, (row * 3) as f32),
                ];
                table.put_cell("DATA_COMPLEX", row as u64, &complex_data)?;
            }
        },
        WriteMode::ColumnPutMmap => {
            // Alternative bulk write approach (if different implementation needed)
            for row in 0..n_rows {
                let complex_data = vec![
                    Complex::new(row as f32 + 0.5, (row + 1) as f32 + 0.5),
                    Complex::new((row * 2) as f32 + 0.5, (row * 3) as f32 + 0.5),
                ];
                table.put_cell("DATA_COMPLEX", row as u64, &complex_data)?;
            }
        },
    }
    
    log_debug("putColumn done");
    Ok(())
}

fn write_bool_column(table: &mut Table, n_rows: usize) -> Result<(), Error> {
    log_debug(&format!("array_column_put_column: n_rows={} (BOOL)", n_rows));
    
    // Write bulk data to BOOL column
    for row in 0..n_rows {
        let bool_data = (row % 2) == 0;
        table.put_cell("FLAG_BOOL", row as u64, &bool_data)?;
    }
    
    log_debug("putColumn done");
    Ok(())
}

fn main() {
    let write_mode = parse_write_mode();
    let _tsm_option = parse_tsm_option();
    
    let result = std::panic::catch_unwind(|| {
        let temp_dir = tempfile::tempdir().unwrap();
        let table_path = temp_dir.path().join("syscall_test.ms");
        
        let n_rows = 1000; // Reasonable size for syscall analysis
        
        log_debug(&format!("Starting syscall tracer, write_mode: {:?}", write_mode));
        
        let mut table = create_table(&table_path, n_rows).unwrap();
        
        // Write COMPLEX column
        write_complex_column(&mut table, n_rows, write_mode).unwrap();
        
        // Write BOOL column
        write_bool_column(&mut table, n_rows).unwrap();
        
        log_debug("Syscall tracer completed");
    });
    
    if let Err(_) = result {
        eprintln!("Error during syscall tracer execution");
        process::exit(1);
    }
}