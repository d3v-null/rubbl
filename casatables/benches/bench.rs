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
    let baseline_indices: Vec<usize> = (0..(127 * 128 / 2)).collect();
    let timestep_range: Range<usize> = 0..10;
    let fine_channel_range: Range<usize> = 0..(24 * 32);

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
            let data_shape = [fine_channel_range.len() as _, 4];
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
                .add_rows(baseline_indices.len() * timestep_range.len())
                .unwrap();

            // Write synthetic visibility data to the data column of the table.
            for (row_index, (baseline_index, timestep_index)) in
                iproduct!(timestep_range.clone().into_iter(), baseline_indices.iter()).enumerate()
            {
                let visibility_data = Array::<Complex<f32>, _>::from_shape_fn(
                    (data_shape[0] as usize, data_shape[1] as usize),
                    |(fine_channel_index, polarization_index): (usize, usize)| {
                        Complex::<f32>::new(
                            row_index as f32,
                            (fine_channel_index * fine_channel_range.len() + polarization_index)
                                as f32,
                        )
                    },
                );

                table
                    .put_cell(
                        &"DATA",
                        (baseline_index * baseline_indices.len() + timestep_index) as u64,
                        &visibility_data,
                    )
                    .unwrap();
            }
        });
    });
}

criterion_group!(benches, bench_write);
criterion_main!(benches);
