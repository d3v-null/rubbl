use ndarray::Array2;
use rubbl_casatables::{
    Complex, GlueDataType, Table, TableCreateMode, TableDesc, TableDescCreateMode, TableOpenMode,
};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

/// Example demonstrating syscall tracing for casatables operations
///
/// This example performs basic casatables operations that can be traced
/// with system tools like strace to analyze syscall patterns.
///
/// Usage:
///   strace -f -s 256 -k -o strace_output.txt cargo run --example syscall_tracer
///   # Then analyze with the analysis script
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Casatables Syscall Tracer");
    println!("This example performs basic operations that can be traced with strace");
    println!();

    // Configuration for the example
    let n_rows = 100;
    let data_shape = vec![32, 4];

    println!("📊 Configuration:");
    println!("  - Rows: {}", n_rows);
    println!("  - Data shape: {:?}", data_shape);
    println!();

    // Create temporary table
    let tmp_dir = tempfile::tempdir()?;
    let table_path = tmp_dir.path().join("syscall_tracer_example.ms");

    println!("🏗️  Creating table...");
    let mut table = create_test_table(&table_path, n_rows, &data_shape)?;

    // Prepare test data
    let data_template = create_test_data(&data_shape);
    let flags_template = create_test_flags(&data_shape);

    println!("✍️  Writing data...");
    // Select write mode via env: WRITE_MODE={put_cell|column_put}
    let mode = std::env::var("WRITE_MODE").unwrap_or_else(|_| "column_put".to_string());
    match mode.as_str() {
        "put_cell" => {
            // Note: TableRow writer path did not reduce syscalls in this environment; using put_cell.
            write_test_data_table_put_cell(
                &mut table,
                &data_shape,
                &data_template,
                &flags_template,
            )?;
        }
        "column_put" => {
            write_test_data_column_put(&mut table, &data_shape, &data_template, &flags_template)?;
        }
        other => {
            eprintln!("Unknown WRITE_MODE '{}', defaulting to column_put", other);
            write_test_data_column_put(&mut table, &data_shape, &data_template, &flags_template)?;
        }
    }

    println!("✅ Syscall tracer example completed successfully!");
    println!();
    println!("💡 To analyze syscalls, run:");
    println!("   strace -f -s 256 -k -o syscall_trace.txt cargo run --example syscall_tracer");
    println!("   python3 analyze_syscalls.py syscall_trace.txt");

    Ok(())
}

fn create_test_table(
    table_path: &PathBuf,
    n_rows: usize,
    data_shape: &[u64],
) -> Result<Table, Box<dyn std::error::Error>> {
    // Create a fresh table using the rubbl API that mirrors the C++ demo
    // Build the same schema as the C++ example using existing rubbl APIs
    let mut table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH)?;
    // Scalars
    table_desc.add_scalar_column(GlueDataType::TpDouble, "TIME", None, false, false)?;
    table_desc.add_scalar_column(GlueDataType::TpInt, "ANTENNA1", None, false, false)?;
    table_desc.add_scalar_column(GlueDataType::TpInt, "ANTENNA2", None, false, false)?;
    table_desc.add_scalar_column(GlueDataType::TpBool, "FLAG_ROW", None, false, false)?;
    // Fixed-shape arrays
    table_desc.add_array_column(
        GlueDataType::TpComplex,
        "DATA",
        Some("Visibility data"),
        Some(data_shape),
        false,
        false,
    )?;
    table_desc.add_array_column(
        GlueDataType::TpBool,
        "FLAG",
        Some("Data flags"),
        Some(data_shape),
        false,
        false,
    )?;

    let table = Table::new(table_path, table_desc, n_rows, TableCreateMode::New)?;
    Ok(table)
}

fn create_test_data(data_shape: &[u64]) -> Array2<Complex<f32>> {
    let shape = (data_shape[0] as usize, data_shape[1] as usize);
    let mut data = Array2::<Complex<f32>>::zeros(shape);

    // Fill with some pattern to make it interesting for analysis
    for ((i, j), elem) in data.indexed_iter_mut() {
        *elem = Complex::new(
            (i as f32 * 0.1).sin() * (j as f32 * 0.2).cos(),
            (i as f32 * 0.15).cos() * (j as f32 * 0.25).sin(),
        );
    }

    data
}

fn create_test_flags(data_shape: &[u64]) -> ndarray::Array2<bool> {
    let shape = (data_shape[0] as usize, data_shape[1] as usize);
    let mut flags = ndarray::Array2::<bool>::from_elem(shape, false);

    // Set some flags to create realistic patterns
    for ((i, j), elem) in flags.indexed_iter_mut() {
        *elem = (i + j) % 17 == 0; // Arbitrary pattern
    }

    flags
}

fn write_test_data_table_put_cell(
    table: &mut Table,
    data_shape: &[u64],
    _data_template: &Array2<Complex<f32>>,
    _flags_template: &ndarray::Array2<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let rows_to_write = 3u64;
    let n0 = data_shape[0] as usize;
    let n1 = data_shape[1] as usize;
    for row_u64 in 0..rows_to_write {
        // Scalars
        table.put_cell("TIME", row_u64, &(row_u64 as f64))?;
        table.put_cell("ANTENNA1", row_u64, &((row_u64 as i32) % 128))?;
        table.put_cell("ANTENNA2", row_u64, &((row_u64 as i32 + 1) % 128))?;
        table.put_cell("FLAG_ROW", row_u64, &((row_u64 % 2) == 0))?;

        // Arrays
        let mut data_matrix = Array2::<Complex<f32>>::default((n0, n1));
        let mut flag_matrix = ndarray::Array2::<bool>::from_elem((n0, n1), false);
        for i in 0..n0 {
            for j in 0..n1 {
                let idx = (i * n1 + j) as u32;
                data_matrix[(i, j)] = Complex::new(idx as f32, 0.0);
                flag_matrix[(i, j)] = (idx % 13) == 0;
            }
        }
        table.put_cell("DATA", row_u64, &data_matrix)?;
        table.put_cell("FLAG", row_u64, &flag_matrix)?;
    }
    Ok(())
}

fn write_test_data_column_put(
    table: &mut Table,
    data_shape: &[u64],
    _data_template: &Array2<Complex<f32>>,
    _flags_template: &ndarray::Array2<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    use rubbl_casatables::GlueDataType::*;

    let rows_to_write = 3u64;
    let n0 = data_shape[0] as usize;
    let n1 = data_shape[1] as usize;

    // Open column handles once
    let mut time_col = table.open_scalar_column("TIME", TpDouble)?;
    let mut ant1_col = table.open_scalar_column("ANTENNA1", TpInt)?;
    let mut ant2_col = table.open_scalar_column("ANTENNA2", TpInt)?;
    let mut flagrow_col = table.open_scalar_column("FLAG_ROW", TpBool)?;
    let mut data_col = table.open_array_column("DATA", TpArrayComplex)?;
    let mut flag_col = table.open_array_column("FLAG", TpArrayBool)?;

    for row_u64 in 0..rows_to_write {
        // Scalars
        time_col.put(row_u64, &(row_u64 as f64))?;
        ant1_col.put(row_u64, &((row_u64 as i32) % 128))?;
        ant2_col.put(row_u64, &((row_u64 as i32 + 1) % 128))?;
        flagrow_col.put(row_u64, &((row_u64 % 2) == 0))?;

        // Arrays
        let mut data_matrix = Array2::<Complex<f32>>::default((n0, n1));
        let mut flag_matrix = ndarray::Array2::<bool>::from_elem((n0, n1), false);
        for i in 0..n0 {
            for j in 0..n1 {
                let idx = (i * n1 + j) as u32;
                data_matrix[(i, j)] = Complex::new(idx as f32, 0.0);
                flag_matrix[(i, j)] = (idx % 13) == 0;
            }
        }
        data_col.put(row_u64, &data_matrix)?;
        flag_col.put(row_u64, &flag_matrix)?;
    }

    Ok(())
}

// No readback: benchmark only creates and writes the table
