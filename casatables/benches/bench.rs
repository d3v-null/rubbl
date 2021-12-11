use criterion::{criterion_group, criterion_main, Criterion};
use flate2::read::GzDecoder;
use itertools::{iproduct, izip};
use rubbl_casatables::{Array, Complex, GlueDataType, Table, TableOpenMode};
#[cfg(feature = "mwalib")]
use rubbl_casatables::{Table, TableOpenMode};
use std::{
    fs::create_dir_all,
    ops::Range,
    path::{Path, PathBuf},
    time::SystemTime,
};
use tar::Archive;

use lazy_static::lazy_static;

lazy_static! {
    static ref DEFAULT_TABLES_GZ: &'static [u8] = include_bytes!("data/default_tables.tar.gz");
}

// Benchmark the data writing speed of casatables::Table::put_cell with synthetic visibility data.
fn bench_write(crt: &mut Criterion) {
    let baselines = 127 * 128 / 2;
    let timesteps = 10;
    let n_rows = baselines * timesteps;
    let fine_channels = 24 * 32;

    crt.bench_function("casatables::Table::put_cell", |bch| {
        bch.iter(|| {
            // Create a new temporary directory to write to each time
            let tmp_dir = tempfile::tempdir().unwrap();
            let table_path = tmp_dir.path().join("table.ms");

            // unpack a .tar.gz archive of the default tables.
            let tar = GzDecoder::new(&DEFAULT_TABLES_GZ[..]);
            let mut archive = Archive::new(tar);
            if !(table_path.exists() && table_path.is_dir()) {
                create_dir_all(&table_path).unwrap();
            }
            archive.unpack(&table_path).unwrap();

            // Open the table
            let mut table = Table::open(table_path, TableOpenMode::ReadWrite).unwrap();

            // Add an array column for the data.
            let data_shape = [fine_channels, 4];
            let data_elements: u64 = data_shape.iter().product();
            table
                .add_array_column(
                    GlueDataType::TpComplex,
                    "DATA",
                    Some("test data column"),
                    Some(&data_shape),
                    false,
                    false,
                )
                .unwrap();
            table
                .add_rows(n_rows)
                .unwrap();

            // Write synthetic visibility data to the data column of the table.
            for row_index in 0..n_rows
            {
                // each element in the visibility array is a complex number, whose real component is
                // the row index and whose imaginary component is the element index.
                let visibility_data = Array::from_iter(
                    (0..data_elements)
                        .map(|d| Complex::<f32>::new(row_index as _, d as _)),
                )
                .into_shape((data_shape[0] as _, data_shape[1] as _))
                .unwrap();

                table
                    .put_cell(&"DATA", row_index as _, &visibility_data)
                    .unwrap();
            }
        });
    });
}

criterion_group!(benches, bench_write);
criterion_main!(benches);
