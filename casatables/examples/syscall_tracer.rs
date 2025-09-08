use rubbl_casatables::{Complex, Table, TableOpenMode};
use ndarray::Array2;
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
    let n_rows = 100;  // Reasonable size for analysis
    let data_shape = vec![32, 4];  // Smaller data for faster execution

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
    write_test_data(&mut table, n_rows, &data_template, &flags_template)?;

    println!("📖 Reading data...");
    read_test_data(&mut table, n_rows)?;

    println!("🔄 Performing mixed operations...");
    perform_mixed_operations(&mut table, n_rows)?;

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
    _data_shape: &[u64],
) -> Result<Table, Box<dyn std::error::Error>> {
    // Use the same approach as the benchmarks - start with default tables
    use flate2::read::GzDecoder;
    use tar::Archive;
    use std::fs::create_dir_all;

    // Include the default tables archive (same as in bench.rs)
    static DEFAULT_TABLES_GZ: &[u8] = include_bytes!("../benches/data/default_tables.tar.gz");

    // Unpack the default tables archive
    let tar = GzDecoder::new(DEFAULT_TABLES_GZ);
    let mut archive = Archive::new(tar);
    if !(table_path.exists() && table_path.is_dir()) {
        create_dir_all(table_path)?;
    }
    archive.unpack(table_path)?;

    // Open the table
    let mut table = Table::open(table_path, TableOpenMode::ReadWrite)?;

    // Just add the rows we need - the default table already has the necessary columns
    table.add_rows(n_rows)?;
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
        *elem = (i + j) % 17 == 0;  // Arbitrary pattern
    }

    flags
}

fn write_test_data(
    table: &mut Table,
    n_rows: usize,
    _data_template: &Array2<Complex<f32>>,
    _flags_template: &ndarray::Array2<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Write to scalar columns that exist in the default table
    for row_idx in 0..n_rows {
        let row_u64 = row_idx as u64;

        // Write scalar columns that exist in the default table
        table.put_cell("TIME", row_u64, &(row_idx as f64 * 1.0))?;
        table.put_cell("ANTENNA1", row_u64, &((row_idx % 128) as i32))?;
        table.put_cell("ANTENNA2", row_u64, &(((row_idx + 1) % 128) as i32))?;
        table.put_cell("FLAG_ROW", row_u64, &(row_idx % 2 == 0))?;
    }

    Ok(())
}

fn read_test_data(
    table: &mut Table,
    n_rows: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read a subset of rows to simulate typical access patterns
    let read_rows = std::cmp::min(10, n_rows);

    for row_idx in 0..read_rows {
        let row_u64 = row_idx as u64;
        // Read scalar columns that exist in the default table
        let _: f64 = table.get_cell("TIME", row_u64)?;
        let _: i32 = table.get_cell("ANTENNA1", row_u64)?;
        let _: i32 = table.get_cell("ANTENNA2", row_u64)?;
        let _: bool = table.get_cell("FLAG_ROW", row_u64)?;
    }

    Ok(())
}

fn perform_mixed_operations(
    table: &mut Table,
    n_rows: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Simulate typical data processing operations

    // Update some scalar values
    for row_idx in (0..n_rows).step_by(10) {
        let row_u64 = row_idx as u64;
        let updated_time = (row_idx as f64 * 2.0) + 1000.0;
        table.put_cell("TIME", row_u64, &updated_time)?;
        table.put_cell("FLAG_ROW", row_u64, &((row_idx + 1) % 2 == 0))?;
    }

    // Read and process data
    for row_idx in (0..n_rows).step_by(5) {
        let row_u64 = row_idx as u64;
        let time: f64 = table.get_cell("TIME", row_u64)?;
        let ant1: i32 = table.get_cell("ANTENNA1", row_u64)?;
        let ant2: i32 = table.get_cell("ANTENNA2", row_u64)?;
        let flag_row: bool = table.get_cell("FLAG_ROW", row_u64)?;

        // Simulate some processing (just to create more syscalls)
        let _processed_time = time * 2.0;
        let _baseline = ant1 + ant2;
        let _flagged = !flag_row;
    }

    Ok(())
}
