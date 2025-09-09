// Copyright 2017-2023 Peter Williams <peter@newton.cx> and collaborators
// Licensed under the MIT License.

//! Benchmark for analyzing syscall patterns in table operations.

use anyhow::Error;
use clap::{Arg, Command};
use rubbl_casatables::{GlueDataType, Table, TableCreateMode, TableDesc, TableDescCreateMode};
use rubbl_core::{ctry, notify::ClapNotificationArgsExt};
use std::{env, path::PathBuf, process};

fn main() {
    let matches = Command::new("benchmark")
        .version("0.1.0")
        .rubbl_notify_args()
        .arg(
            Arg::new("TABLE-PATH")
                .value_parser(clap::value_parser!(PathBuf))
                .help("The path where the benchmark table will be created")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("rows")
                .short('r')
                .long("rows")
                .value_parser(clap::value_parser!(usize))
                .help("Number of rows to create")
                .default_value("1000"),
        )
        .arg(
            Arg::new("cols")
                .short('c')
                .long("cols")
                .value_parser(clap::value_parser!(usize))
                .help("Number of columns to create")
                .default_value("10"),
        )
        .get_matches();

    process::exit(rubbl_core::notify::run_with_notifications(
        matches,
        |matches, _nbe| -> Result<i32, Error> {
            let table_path = matches.get_one::<PathBuf>("TABLE-PATH").unwrap();
            let num_rows = *matches.get_one::<usize>("rows").unwrap();
            let num_cols = *matches.get_one::<usize>("cols").unwrap();

            let write_mode = env::var("WRITE_MODE").unwrap_or_else(|_| "column_put_bulk".to_string());

            println!("Creating table with {} rows and {} columns", num_rows, num_cols);

            // Create table description
            let mut table_desc = ctry!(
                TableDesc::new("", TableDescCreateMode::TDM_SCRATCH);
                "failed to create table description"
            );

            // Add scalar columns for simple benchmark
            for i in 0..num_cols {
                ctry!(
                    table_desc.add_scalar_column(
                        GlueDataType::TpDouble,
                        &format!("COL_{}", i),
                        Some(&format!("Column {}", i)),
                        true,  // direct
                        false, // undefined
                    );
                    "failed to add scalar column {}", i
                );
            }

            // Add array column for more complex operations
            ctry!(
                table_desc.add_array_column(
                    GlueDataType::TpDouble,
                    "UVW",
                    Some("UVW coordinates"),
                    Some(&[3]),
                    true,
                    false,
                );
                "failed to add UVW array column"
            );

            // Create table
            let mut table = ctry!(
                Table::new(table_path, table_desc, num_rows, TableCreateMode::New);
                "failed to create table at \"{}\"", table_path.display()
            );

            println!("Starting write operations (mode: {})", write_mode);

            match write_mode.as_str() {
                "column_put_bulk" => {
                    // Bulk column operations - write entire columns at once
                    for col_idx in 0..num_cols {
                        let col_name = format!("COL_{}", col_idx);
                        for row_idx in 0..num_rows {
                            let value = (col_idx as f64) * 1000.0 + (row_idx as f64);
                            ctry!(
                                table.put_cell_cached(&col_name, row_idx as u64, &value);
                                "failed to put cell value for column {} row {}", col_name, row_idx
                            );
                        }
                    }

                    // Write UVW data with caching
                    for row_idx in 0..num_rows {
                        let uvw_data = vec![
                            row_idx as f64 * 0.1,
                            row_idx as f64 * 0.2,
                            row_idx as f64 * 0.3,
                        ];
                        ctry!(
                            table.put_cell_cached("UVW", row_idx as u64, &uvw_data);
                            "failed to put UVW cell for row {}", row_idx
                        );
                    }
                }
                "row_put_bulk" => {
                    // Row-wise operations - write entire rows at once using TableRow
                    let mut row = ctry!(
                        table.get_row_writer();
                        "failed to get row writer"
                    );
                    
                    for row_idx in 0..num_rows {
                        ctry!(
                            table.read_row(&mut row, row_idx as u64);
                            "failed to read row {}", row_idx
                        );

                        for col_idx in 0..num_cols {
                            let col_name = format!("COL_{}", col_idx);
                            let value = (col_idx as f64) * 1000.0 + (row_idx as f64);
                            ctry!(
                                row.put_cell(&col_name, &value);
                                "failed to put row cell value for column {} row {}", col_name, row_idx
                            );
                        }

                        let uvw_data = vec![
                            row_idx as f64 * 0.1,
                            row_idx as f64 * 0.2,
                            row_idx as f64 * 0.3,
                        ];
                        ctry!(
                            row.put_cell("UVW", &uvw_data);
                            "failed to put UVW row cell for row {}", row_idx
                        );
                        
                        ctry!(
                            row.put(row_idx as u64);
                            "failed to write row {}", row_idx
                        );
                    }
                }
                _ => {
                    println!("Unknown write mode: {}", write_mode);
                    return Ok(1);
                }
            }

            println!("Starting read operations");

            // Read back data for verification and more syscalls
            let mut total_checksum = 0.0;
            for row_idx in 0..num_rows {
                for col_idx in 0..num_cols {
                    let col_name = format!("COL_{}", col_idx);
                    let value: f64 = ctry!(
                        table.get_cell(&col_name, row_idx as u64);
                        "failed to get cell value for column {} row {}", col_name, row_idx
                    );
                    total_checksum += value;
                }

                let uvw_data: Vec<f64> = ctry!(
                    table.get_cell_as_vec("UVW", row_idx as u64);
                    "failed to get UVW cell for row {}", row_idx
                );
                total_checksum += uvw_data.iter().sum::<f64>();
            }

            println!("Benchmark completed. Checksum: {}", total_checksum);
            Ok(0)
        },
    ));
}