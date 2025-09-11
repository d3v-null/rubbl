// Simple test to verify column caching behavior
use rubbl_casatables::{GlueDataType, Table, TableCreateMode, TableDesc, TableDescCreateMode};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let table_path1 = "/tmp/test_caching.ms";
    let table_path2 = "/tmp/test_no_caching.ms";

    // Clean up any existing tables
    for path in [table_path1, table_path2] {
        if Path::new(path).exists() {
            std::fs::remove_dir_all(path)?;
        }
    }

    // Test 1: With caching
    println!("=== Testing WITH column caching ===");
    {
        let mut desc = TableDesc::new("MAIN", TableDescCreateMode::TDM_SCRATCH)?;
        desc.add_scalar_column(
            GlueDataType::TpDouble,
            "TEST_COL",
            Some("Test column"),
            false,
            false,
        )?;

        let mut table = Table::new(table_path1, desc, 100, TableCreateMode::New, None)?;

        for i in 0..100 {
            table.put_cell_cached("TEST_COL", i, &(i as f64))?;
        }
        println!("Completed 100 cached puts successfully");
    }

    // Test 2: Without caching (regular put_cell)
    println!("\n=== Testing WITHOUT column caching ===");
    {
        let mut desc = TableDesc::new("MAIN", TableDescCreateMode::TDM_SCRATCH)?;
        desc.add_scalar_column(
            GlueDataType::TpDouble,
            "TEST_COL",
            Some("Test column"),
            false,
            false,
        )?;

        let mut table = Table::new(table_path2, desc, 100, TableCreateMode::New, None)?;

        for i in 0..100 {
            table.put_cell("TEST_COL", i, &(i as f64))?;
        }
        println!("Completed 100 regular puts successfully");
    }

    println!("\nComparison test completed!");
    Ok(())
}
