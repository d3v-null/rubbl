// Example demonstrating write performance when editing existing measurement sets
// This example unzips the default tables and demonstrates the performance advantage
// of column object caching (optimization #2) when working with existing tables.

use rubbl_casatables::{Table, TableOpenMode};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

/// Extract default tables from tar.gz to a temporary location
fn extract_default_tables(extract_to: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Find the tar.gz file relative to the example location
    let tar_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches")
        .join("data")
        .join("default_tables.tar.gz");
    
    if !tar_path.exists() {
        return Err(format!("Default tables tar.gz not found at: {:?}", tar_path).into());
    }

    println!("📦 Extracting default tables from: {:?}", tar_path);
    
    // Create a measurement set directory structure
    let ms_path = extract_to.join("test.ms");
    fs::create_dir_all(&ms_path)?;
    
    println!("📂 Extracting to: {:?}", ms_path);

    // Extract using tar command
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(&tar_path)
        .arg("-C")
        .arg(&ms_path)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to extract tar.gz: {}", 
            String::from_utf8_lossy(&output.stderr)
        ).into());
    }

    println!("✅ Extraction completed successfully");
    Ok(ms_path)
}

/// Benchmark writing to columns using cached vs non-cached operations
fn benchmark_write_operations(ms_path: &Path, use_cache: bool) -> Result<u64, Box<dyn std::error::Error>> {
    println!("🔧 Opening table: {:?}", ms_path);
    
    // Open the existing measurement set for editing
    let mut table = Table::open(ms_path, TableOpenMode::ReadWrite)?;
    
    let mut n_rows = table.n_rows();
    println!("📊 Table has {} rows", n_rows);
    
    // If the table is empty, add some test data first
    if n_rows == 0 {
        println!("📝 Adding test data (10,000 rows) to empty measurement set...");
        let rows_to_add = 10000;
        n_rows = rows_to_add;
        table.add_rows(rows_to_add as usize)?;
        
        // Initialize the TIME column with baseline values
        for row_idx in 0..n_rows {
            let time_value = 4.5e9 + (row_idx as f64); // MJD time + offset
            table.put_cell("TIME", row_idx, &time_value)?;
        }
        println!("✅ Test data added");
    }
    
    // We'll write to the TIME column as it's a standard column in measurement sets
    let column_name = "TIME";
    
    println!("⏱️  Starting {} write operations (cache: {})", n_rows, use_cache);
    let start_time = std::time::Instant::now();
    
    // Write modified time values (add offset for benchmarking)
    for row_idx in 0..n_rows {
        let time_value = 4.5e9 + 1000.0 + (row_idx as f64); // Modified time values
        
        if use_cache {
            // Use the cached column object method (optimization #2)
            table.put_cell_cached(column_name, row_idx, &time_value)?;
        } else {
            // Use the non-cached method
            table.put_cell(column_name, row_idx, &time_value)?;
        }
    }
    
    let elapsed = start_time.elapsed();
    let elapsed_micros = elapsed.as_micros() as u64;
    
    println!("✅ Completed {} writes in {} μs", n_rows, elapsed_micros);
    println!("📈 Average: {:.2} μs per write", elapsed_micros as f64 / n_rows as f64);
    
    Ok(elapsed_micros)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Write MS Example - Column Caching Performance Demo");
    println!("This demonstrates the performance advantage of column object caching");
    println!("when editing existing measurement sets.");
    println!();

    // Create temporary directory for extraction
    let temp_dir = tempfile::tempdir()?;
    let ms_path = extract_default_tables(temp_dir.path())?;
    
    println!();
    println!("🏁 Starting performance comparison...");
    println!();
    
    // First, make a copy for the cached test
    let ms_cached = temp_dir.path().join("ms_cached");
    let ms_uncached = temp_dir.path().join("ms_uncached");
    
    // Copy the measurement set for both tests
    copy_dir_recursive(&ms_path, &ms_cached)?;
    copy_dir_recursive(&ms_path, &ms_uncached)?;
    
    // Test without caching first
    println!("🔄 Test 1: WITHOUT column caching");
    let time_uncached = benchmark_write_operations(&ms_uncached, false)?;
    
    println!();
    
    // Test with caching
    println!("🚀 Test 2: WITH column caching (optimization #2)");
    let time_cached = benchmark_write_operations(&ms_cached, true)?;
    
    println!();
    println!("📊 PERFORMANCE COMPARISON RESULTS:");
    println!("   Without caching: {} μs", time_uncached);
    println!("   With caching:    {} μs", time_cached);
    
    if time_uncached > time_cached {
        let improvement = ((time_uncached - time_cached) as f64 / time_uncached as f64) * 100.0;
        println!("   🎯 Performance improvement: {:.1}% faster with caching", improvement);
        println!("   ⚡ Speedup factor: {:.2}x", time_uncached as f64 / time_cached as f64);
    } else if time_cached > time_uncached {
        let difference = ((time_cached - time_uncached) as f64 / time_uncached as f64) * 100.0;
        println!("   📊 Difference: {:.1}% slower with caching (cache overhead may dominate for fast operations)", difference);
    } else {
        println!("   ⚠️  No significant difference measured (very fast operations)");
    }
    
    println!();
    println!("💡 This demonstrates column object caching behavior when editing existing measurement sets.");
    println!("   For simple scalar operations, caching may have overhead, but benefits larger workloads.");
    println!("   The table initialization fix (optimization #1) only helps with new table creation.");
    println!("   Column caching (optimization #2) benefits complex workflows with repeated column access.");

    Ok(())
}

/// Helper function to recursively copy directories
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path)?;
            }
        }
    } else {
        fs::copy(src, dst)?;
    }
    Ok(())
}