use criterion::{criterion_group, criterion_main, Criterion};
use flate2::read::GzDecoder;
use itertools::{iproduct, izip};
use ndarray::{Array, Array3, Array4, Array2, s, Axis};
use rubbl_casatables::{Complex, GlueDataType, Table, TableOpenMode};
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

const N_ANTS: usize = 128;
const N_TIMESTEPS: usize = 10;
const N_CHANNELS: usize = 24 * 32;
const N_POLS: usize = 4;

fn setup_main_table(
    table_path: PathBuf,
    n_rows : usize,
    data_shape: Vec<u64>
) -> Table {
    // unpack a .tar.gz archive of the default tables.
    let tar = GzDecoder::new(&DEFAULT_TABLES_GZ[..]);
    let mut archive = Archive::new(tar);
    if !(table_path.exists() && table_path.is_dir()) {
        create_dir_all(&table_path).unwrap();
    }
    archive.unpack(&table_path).unwrap();

    // Open the table
    let mut table = Table::open(table_path, TableOpenMode::ReadWrite).unwrap();

    // Add DATA and WEIGHT_SPECTRUM array columns
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
        .add_array_column(
            GlueDataType::TpFloat,
            "WEIGHT_SPECTRUM",
            None,
            Some(&data_shape),
            false,
            false,
        )
        .unwrap();
    table.add_rows(n_rows).unwrap();

    return table;
}

fn bench_table_put_cell_on_fly(crt: &mut Criterion) {
    let n_baselines = N_ANTS * (N_ANTS - 1) / 2;
    let n_rows = n_baselines * N_TIMESTEPS;
    let data_shape = vec![N_CHANNELS as _, N_POLS];

    let mut uvw_tmp = vec![0.; 3];
    let mut data_tmp = Array2::<Complex<f32>>::zeros((N_CHANNELS, N_POLS));
    let mut weights_tmp = Array2::<f32>::zeros((N_CHANNELS, N_POLS));
    let mut weights_pols_tmp = vec![0 as f32; N_POLS];
    let mut flags_tmp = Array::from_elem((N_CHANNELS, N_POLS), false);
    let sigma_tmp: Vec<f32> = vec![1., 1., 1., 1.];

    crt.bench_function("casatables::Table::put_cell, on the fly", |bch| {
        bch.iter(|| {
            // Create a new temporary directory to write to each time
            let tmp_dir = tempfile::tempdir().unwrap();
            let table_path = tmp_dir.path().join("table.ms");
            let mut table = setup_main_table(table_path, n_rows, data_shape.iter().map(|&x| x as u64).collect());

            // Write synthetic visibility data to the data column of the table.
            for (row_idx, (timestep_idx, baseline_idx)) in iproduct!(0..N_TIMESTEPS, 0..n_baselines).enumerate() {
                // Calculate the uvw coordinates for this row.
                uvw_tmp[0] = row_idx as _;
                uvw_tmp[1] = baseline_idx as _;
                uvw_tmp[2] = timestep_idx as _;

                // Calculate the weights for this row.
                weights_tmp.column_mut(0).fill(row_idx as _);
                weights_tmp.column_mut(1).fill(baseline_idx as _);
                weights_tmp.column_mut(2).fill(timestep_idx as _);
                weights_tmp.column_mut(3).fill(1.);

                let antenna1 = (baseline_idx % N_ANTS) as i32;
                let antenna2 = (baseline_idx / N_ANTS) as i32;

                // each element in the visibility array is a complex number, whose real component is
                // the row index and whose imaginary component is the element index.
                data_tmp.iter_mut().enumerate().for_each(|(idx, elt)| {
                    *elt = Complex::new(row_idx as _, idx as _);
                });
                flags_tmp.iter_mut().enumerate().for_each(|(idx, elt)| {
                    *elt = idx % 2 == 0;
                });
                weights_pols_tmp.iter_mut().zip(weights_tmp.axis_iter(Axis(1))).for_each(|(elt, weights_pol)| {
                    *elt = weights_pol.sum();
                });

                table.put_cell("TIME", row_idx as _, &(timestep_idx as f64)).unwrap();
                table
                    .put_cell("TIME_CENTROID", row_idx as _, &(timestep_idx as f64))
                    .unwrap();
                table.put_cell("ANTENNA1", row_idx as _, &antenna1).unwrap();
                table.put_cell("ANTENNA2", row_idx as _, &antenna2).unwrap();
                table.put_cell("DATA_DESC_ID", row_idx as _, &(0 as i32)).unwrap();
                table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();
                table.put_cell("INTERVAL", row_idx as _, &(1. as f64)).unwrap();
                table.put_cell("EXPOSURE", row_idx as _, &(1. as f64)).unwrap();
                table.put_cell("PROCESSOR_ID", row_idx as _, &(-1. as i32)).unwrap();
                table.put_cell("SCAN_NUMBER", row_idx as _, &(1 as i32)).unwrap();
                table.put_cell("STATE_ID", row_idx as _, &(-1 as i32)).unwrap();
                table.put_cell("SIGMA", row_idx as _, &sigma_tmp).unwrap();
                table.put_cell("DATA", row_idx as _, &data_tmp).unwrap();
                table.put_cell("WEIGHT_SPECTRUM", row_idx as _, &weights_tmp).unwrap();
                table.put_cell("WEIGHT", row_idx as _, &weights_pols_tmp).unwrap();
                table.put_cell("FLAG", row_idx as _, &flags_tmp).unwrap();
                table.put_cell("FLAG_ROW", row_idx as _, &(row_idx % 2 == 0)).unwrap();
            }
        });
    });
}

fn synthesize_test_data(
    shape: (usize, usize, usize, usize),
) -> (Array4<Complex<f32>>, Array4<f32>, Array4<bool>) {
    // let (N_TIMESTEPS, N_CHANNELS, n_baselines, N_POLS) = shape;
    let vis_array = Array4::from_shape_fn(
        shape,
        |(timestep_idx, chan_idx, baseline_idx, pol_idx)| match pol_idx {
            0 => Complex::new(0., timestep_idx as _),
            1 => Complex::new(0., chan_idx as _),
            2 => Complex::new(0., baseline_idx as _),
            _ => Complex::new(0., 1.),
        },
    );
    let weight_array = Array4::from_shape_fn(
        shape,
        |(timestep_idx, chan_idx, baseline_idx, pol_idx)| {
            (timestep_idx * shape.0 * shape.1 * shape.2 * shape.3
                + chan_idx * shape.0 * shape.1 * shape.2
                + baseline_idx * shape.0 * shape.1
                + pol_idx) as f32
        },
    );
    let flag_array =
        Array4::from_shape_fn(shape, |(timestep_idx, chan_idx, baseline_idx, pol_idx)| {
            (timestep_idx * shape.0 * shape.1 * shape.2 * shape.3
                + chan_idx * shape.0 * shape.1 * shape.2
                + baseline_idx * shape.0 * shape.1
                + pol_idx % 2
                == 0) as _
        });
    (vis_array, weight_array, flag_array)
}

fn bench_table_put_cell_preload_slice(crt: &mut Criterion) {
    let n_baselines = N_ANTS * (N_ANTS - 1) / 2;
    let n_rows = n_baselines * N_TIMESTEPS;
    let data_shape = vec![N_CHANNELS, N_POLS];
    let shape = (N_TIMESTEPS, N_CHANNELS, n_baselines, N_POLS);
    let (vis_array, weight_array, flag_array) = synthesize_test_data(shape);

    // Create arrays to store synthetic visiblity data, re-used each row.
    let mut uvw_tmp = vec![0.; 3];
    let mut data_tmp = Array2::<Complex<f32>>::zeros((N_CHANNELS, N_POLS));
    let mut weights_tmp = Array2::<f32>::zeros((N_CHANNELS, N_POLS));
    let mut weights_pols_tmp = vec![0 as f32; N_POLS];
    let mut flags_tmp = Array::from_elem((N_CHANNELS, N_POLS), false);
    let sigma_tmp: Vec<f32> = vec![1., 1., 1., 1.];

    crt.bench_function("casatables::Table::put_cell slicing pre-loaded data", |bch| {
        bch.iter(|| {
            // Create a new temporary directory to write to each time
            let tmp_dir = tempfile::tempdir().unwrap();
            let table_path = tmp_dir.path().join("table.ms");
            let mut table = setup_main_table(table_path, n_rows, data_shape.iter().map(|&x| x as u64).collect());

            // Write synthetic visibility data into the main table.
            for (row_idx, (timestep_idx, baseline_idx)) in iproduct!(0..N_TIMESTEPS, 0..n_baselines).enumerate() {

                // Calculate the uvw coordinates for this row.
                uvw_tmp[0] = row_idx as _;
                uvw_tmp[1] = baseline_idx as _;
                uvw_tmp[2] = timestep_idx as _;

                let antenna1 = (baseline_idx % N_ANTS) as i32;
                let antenna2 = (baseline_idx / N_ANTS) as i32;

                data_tmp.assign(&vis_array.slice(s![timestep_idx, .., baseline_idx, ..]));
                weights_tmp.assign(&weight_array.slice(s![timestep_idx, .., baseline_idx, ..]));
                flags_tmp.assign(&flag_array.slice(s![timestep_idx, .., baseline_idx, ..]));

                weights_pols_tmp.iter_mut().zip(weights_tmp.axis_iter(Axis(1))).for_each(|(elt, weights_pol)| {
                    *elt = weights_pol.sum();
                });

                table.put_cell("TIME", row_idx as _, &(timestep_idx as f64)).unwrap();
                table
                    .put_cell("TIME_CENTROID", row_idx as _, &(timestep_idx as f64))
                    .unwrap();
                table.put_cell("ANTENNA1", row_idx as _, &antenna1).unwrap();
                table.put_cell("ANTENNA2", row_idx as _, &antenna2).unwrap();
                table.put_cell("DATA_DESC_ID", row_idx as _, &(0 as i32)).unwrap();
                table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();
                table.put_cell("INTERVAL", row_idx as _, &(1. as f64)).unwrap();
                table.put_cell("EXPOSURE", row_idx as _, &(1. as f64)).unwrap();
                table.put_cell("PROCESSOR_ID", row_idx as _, &(-1. as i32)).unwrap();
                table.put_cell("SCAN_NUMBER", row_idx as _, &(1 as i32)).unwrap();
                table.put_cell("STATE_ID", row_idx as _, &(-1 as i32)).unwrap();
                table.put_cell("SIGMA", row_idx as _, &sigma_tmp).unwrap();
                table.put_cell("DATA", row_idx as _, &data_tmp).unwrap();
                table.put_cell("WEIGHT_SPECTRUM", row_idx as _, &weights_tmp).unwrap();
                table.put_cell("WEIGHT", row_idx as _, &weights_pols_tmp).unwrap();
                table.put_cell("FLAG", row_idx as _, &flags_tmp).unwrap();
                table.put_cell("FLAG_ROW", row_idx as _, &(row_idx % 2 == 0)).unwrap();
            }
        })
    });
}

// Benchmark the data writing speed of casatables::Table::put_cell with synthetic visibility data.
// This benchmark generates all the synthetic at the start, and writes to the DATA,
fn bench_tablerow_put_preload_slice(crt: &mut Criterion) {
    let n_baselines = N_ANTS * (N_ANTS - 1) / 2;
    let n_rows = n_baselines * N_TIMESTEPS;
    let data_shape = vec![N_CHANNELS, N_POLS];
    let shape = (N_TIMESTEPS, N_CHANNELS, n_baselines, N_POLS);
    let (vis_array, weight_array, flag_array) = synthesize_test_data(shape);

    // Create arrays to store synthetic visiblity data, re-used each row.
    let mut uvw_tmp = vec![0.; 3];
    let mut data_tmp = Array2::<Complex<f32>>::zeros((N_CHANNELS, N_POLS));
    let mut weights_tmp = Array2::<f32>::zeros((N_CHANNELS, N_POLS));
    let mut weights_pols_tmp = vec![0 as f32; N_POLS];
    let mut flags_tmp = Array::from_elem((N_CHANNELS, N_POLS), false);
    let sigma_tmp: Vec<f32> = vec![1., 1., 1., 1.];
    let flag_category_tmp = Array::from_elem((1, 1, 1), false);

    crt.bench_function("casatables::TableRow::put slicing, using tablerow", |bch| {
        bch.iter(|| {
            // Create a new temporary directory to write to each time
            let tmp_dir = tempfile::tempdir().unwrap();
            let table_path = tmp_dir.path().join("table.ms");
            let mut table = setup_main_table(table_path, n_rows, data_shape.iter().map(|&x| x as u64).collect());

            // Write synthetic visibility data into the main table.
            for (row_idx, (timestep_idx, baseline_idx)) in iproduct!(0..N_TIMESTEPS, 0..n_baselines).enumerate() {

                // Calculate the uvw coordinates for this row.
                uvw_tmp[0] = row_idx as _;
                uvw_tmp[1] = baseline_idx as _;
                uvw_tmp[2] = timestep_idx as _;

                let antenna1 = (baseline_idx % N_ANTS) as i32;
                let antenna2 = (baseline_idx / N_ANTS) as i32;

                data_tmp.assign(&vis_array.slice(s![timestep_idx, .., baseline_idx, ..]));
                weights_tmp.assign(&weight_array.slice(s![timestep_idx, .., baseline_idx, ..]));
                flags_tmp.assign(&flag_array.slice(s![timestep_idx, .., baseline_idx, ..]));

                weights_pols_tmp.iter_mut().zip(weights_tmp.axis_iter(Axis(1))).for_each(|(elt, weights_pol)| {
                    *elt = weights_pol.sum();
                });

                let mut table_row = table.get_row_writer().unwrap();

                table_row.put_cell("TIME", &(timestep_idx as f64)).unwrap();
                table_row
                    .put_cell("TIME_CENTROID", &(timestep_idx as f64))
                    .unwrap();
                table_row.put_cell("FLAG_CATEGORY", &flag_category_tmp).unwrap();
                table_row.put_cell("ANTENNA1", &antenna1).unwrap();
                table_row.put_cell("ANTENNA2", &antenna2).unwrap();
                table_row.put_cell("DATA_DESC_ID", &(0 as i32)).unwrap();
                table_row.put_cell("UVW", &uvw_tmp).unwrap();
                table_row.put_cell("INTERVAL", &(1. as f64)).unwrap();
                table_row.put_cell("EXPOSURE", &(1. as f64)).unwrap();
                table_row.put_cell("PROCESSOR_ID", &(-1. as i32)).unwrap();
                table_row.put_cell("SCAN_NUMBER", &(1 as i32)).unwrap();
                table_row.put_cell("STATE_ID", &(-1 as i32)).unwrap();
                table_row.put_cell("SIGMA", &sigma_tmp).unwrap();
                table_row.put_cell("DATA", &data_tmp).unwrap();
                table_row.put_cell("WEIGHT_SPECTRUM", &weights_tmp).unwrap();
                table_row.put_cell("WEIGHT", &weights_pols_tmp).unwrap();
                table_row.put_cell("FLAG", &flags_tmp).unwrap();
                table_row.put_cell("FLAG_ROW", &(row_idx % 2 == 0)).unwrap();

                table_row.put(row_idx as _).unwrap();
            }
        })
    });
}


// Benchmark the data writing speed of casatables::Table::put_cell with synthetic visibility data.
// This benchmark generates all the synthetic at the start, and writes to the DATA,
fn bench_table_put_cell_preload_iter(crt: &mut Criterion) {
    let n_baselines = N_ANTS * (N_ANTS - 1) / 2;
    let n_rows = n_baselines * N_TIMESTEPS;
    let data_shape = vec![N_CHANNELS, N_POLS];
    let shape = (N_TIMESTEPS, N_CHANNELS, n_baselines, N_POLS);
    let (vis_array, weight_array, flag_array) = synthesize_test_data(shape);

    // Create arrays to store synthetic visiblity data, re-used each row.
    let mut uvw_tmp = vec![0.; 3];
    let mut data_tmp = Array2::<Complex<f32>>::zeros((N_CHANNELS, N_POLS));
    let mut weights_tmp = Array2::<f32>::zeros((N_CHANNELS, N_POLS));
    let mut weights_pols_tmp = vec![0 as f32; N_POLS];
    let mut flags_tmp = Array::from_elem((N_CHANNELS, N_POLS), false);
    let sigma_tmp: Vec<f32> = vec![1., 1., 1., 1.];

    crt.bench_function("casatables::Table::put_cell izip views of pre-loaded data", |bch| {
        bch.iter(|| {
            // Create a new temporary directory to write to each time
            let tmp_dir = tempfile::tempdir().unwrap();
            let table_path = tmp_dir.path().join("table.ms");
            let mut table = setup_main_table(table_path, n_rows, data_shape.iter().map(|&x| x as u64).collect());

            let mut row_idx = 0;
            // Write synthetic visibility data into the main table
            for (timestep_idx, vis_timestep_view, weight_timestep_view, flag_timestep_view) in izip!(
                0..N_TIMESTEPS, 
                vis_array.outer_iter(),
                weight_array.outer_iter(),
                flag_array.outer_iter()
            ) {
                for(baseline_idx, vis_baseline_view, weight_baseline_view, flag_baseline_view) in izip!(
                    0..n_baselines,
                    vis_timestep_view.axis_iter(Axis(1)),
                    weight_timestep_view.axis_iter(Axis(1)),
                    flag_timestep_view.axis_iter(Axis(1))
                ) {
                    
                    // Calculate the uvw coordinates for this row.
                    uvw_tmp[0] = row_idx as _;
                    uvw_tmp[1] = baseline_idx as _;
                    uvw_tmp[2] = timestep_idx as _;

                    let antenna1 = (baseline_idx % N_ANTS) as i32;
                    let antenna2 = (baseline_idx / N_ANTS) as i32;

                    data_tmp.assign(&vis_baseline_view);
                    weights_tmp.assign(&weight_baseline_view);
                    flags_tmp.assign(&flag_baseline_view);
    
                    weights_pols_tmp.iter_mut().zip(weights_tmp.axis_iter(Axis(1))).for_each(|(elt, weights_pol)| {
                        *elt = weights_pol.sum();
                    });
    
                    table.put_cell("TIME", row_idx as _, &(timestep_idx as f64)).unwrap();
                    table
                        .put_cell("TIME_CENTROID", row_idx as _, &(timestep_idx as f64))
                        .unwrap();
                    table.put_cell("ANTENNA1", row_idx as _, &antenna1).unwrap();
                    table.put_cell("ANTENNA2", row_idx as _, &antenna2).unwrap();
                    table.put_cell("DATA_DESC_ID", row_idx as _, &(0 as i32)).unwrap();
                    table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();
                    table.put_cell("INTERVAL", row_idx as _, &(1. as f64)).unwrap();
                    table.put_cell("EXPOSURE", row_idx as _, &(1. as f64)).unwrap();
                    table.put_cell("PROCESSOR_ID", row_idx as _, &(-1. as i32)).unwrap();
                    table.put_cell("SCAN_NUMBER", row_idx as _, &(1 as i32)).unwrap();
                    table.put_cell("STATE_ID", row_idx as _, &(-1 as i32)).unwrap();
                    table.put_cell("SIGMA", row_idx as _, &sigma_tmp).unwrap();
                    table.put_cell("DATA", row_idx as _, &data_tmp).unwrap();
                    table.put_cell("WEIGHT_SPECTRUM", row_idx as _, &weights_tmp).unwrap();
                    table.put_cell("WEIGHT", row_idx as _, &weights_pols_tmp).unwrap();
                    table.put_cell("FLAG", row_idx as _, &flags_tmp).unwrap();
                    table.put_cell("FLAG_ROW", row_idx as _, &(row_idx % 2 == 0)).unwrap();

                    row_idx+=1;
                }
            }
        })
    });
}


fn bench_table_put_cell_chj(crt: &mut Criterion) {
    let n_baselines = N_ANTS * (N_ANTS - 1) / 2;
    let n_rows = n_baselines * N_TIMESTEPS;
    let data_shape = vec![N_CHANNELS, N_POLS];
    let shape = (N_TIMESTEPS, N_CHANNELS, n_baselines, N_POLS);
    let (vis_array, weight_array, flag_array) = synthesize_test_data(shape);

    // Create arrays to store synthetic visiblity data, re-used each row.
    let mut uvw_tmp = vec![0.; 3];
    let mut data_tmp = Array2::<Complex<f32>>::zeros((N_CHANNELS, N_POLS));
    let mut weights_tmp = Array2::<f32>::zeros((N_CHANNELS, N_POLS));
    let mut weights_pols_tmp = vec![0 as f32; N_POLS];
    let mut flags_tmp = Array::from_elem((N_CHANNELS, N_POLS), false);
    let sigma_tmp: Vec<f32> = vec![1., 1., 1., 1.];

    crt.bench_function("casatables::Table::put_cell columnwise one at a time", |bch| {
        bch.iter(|| {
            // Create a new temporary directory to write to each time
            let tmp_dir = tempfile::tempdir().unwrap();
            let table_path = tmp_dir.path().join("table.ms");
            let mut table = setup_main_table(table_path, n_rows, data_shape.iter().map(|&x| x as u64).collect());

            let mut row_idx = 0;

            // Write synthetic visibility data into the main table
            for vis_timestep_view in vis_array.outer_iter() {
                for vis_baseline_view in vis_timestep_view.axis_iter(Axis(1)) {
                    data_tmp.assign(&vis_baseline_view);
                    table.put_cell("DATA", row_idx as _, &data_tmp).unwrap();

                    row_idx+=1;
                }
            }

            // // row_idx = 0;
            // // for (timestep_idx, weight_timestep_view) in izip!(
            // //     0..N_TIMESTEPS, 
            // //     weight_array.outer_iter(),
            // // ) {
            // //     for(baseline_idx, weight_baseline_view) in izip!(
            // //         0..n_baselines,
            // //         weight_timestep_view.axis_iter(Axis(1)),
            // //     ) {
            // //         weights_tmp.assign(&weight_baseline_view);    
            // //         weights_pols_tmp.iter_mut().zip(weights_tmp.axis_iter(Axis(1))).for_each(|(elt, weights_pol)| {
            // //             *elt = weights_pol.sum();
            // //         });
    
            // //         table.put_cell("WEIGHT_SPECTRUM", row_idx as _, &weights_tmp).unwrap();
            // //         table.put_cell("WEIGHT", row_idx as _, &weights_pols_tmp).unwrap();

            // //         row_idx+=1;
            // //     }
            // // }

            // row_idx = 0;
            // for weight_timestep_view in weight_array.outer_iter() {
            //     for weight_baseline_view in weight_timestep_view.axis_iter(Axis(1)) {
            //         weights_tmp.assign(&weight_baseline_view);    
            //         table.put_cell("WEIGHT_SPECTRUM", row_idx as _, &weights_tmp).unwrap();

            //         row_idx+=1;
            //     }
            // }

            // row_idx = 0;
            // for weight_timestep_view in weight_array.outer_iter() {
            //     for weight_baseline_view in weight_timestep_view.axis_iter(Axis(1)) {
            //         weights_pols_tmp.iter_mut().zip(weight_baseline_view.axis_iter(Axis(1))).for_each(|(elt, weights_pol)| {
            //             *elt = weights_pol.sum();
            //         });
            //         table.put_cell("WEIGHT", row_idx as _, &weights_pols_tmp).unwrap();

            //         row_idx+=1;
            //     }
            // }

            // row_idx = 0;
            // for flag_timestep_view in flag_array.outer_iter() {
            //     for flag_baseline_view in flag_timestep_view.axis_iter(Axis(1)) {
            //         flags_tmp.assign(&flag_baseline_view);
            //         table.put_cell("FLAG", row_idx as _, &flags_tmp).unwrap();

            //         row_idx+=1;
            //     }
            // }

            // row_idx = 0;
            // for timestep_idx in 0..N_TIMESTEPS {
            //     for _ in 0..n_baselines {
            //         table.put_cell("TIME", row_idx as _, &(timestep_idx as f64)).unwrap();
            //         row_idx+=1;
            //     }
            // }
            // row_idx = 0;
            // for timestep_idx in 0..N_TIMESTEPS {
            //     for _ in 0..n_baselines {
            //         table
            //             .put_cell("TIME_CENTROID", row_idx as _, &(timestep_idx as f64))
            //             .unwrap();
            //         row_idx+=1;
            //     }
            // }
            // row_idx = 0;
            // for _ in 0..N_TIMESTEPS {
            //     for baseline_idx in 0..n_baselines {
            //         let antenna1 = (baseline_idx % N_ANTS) as i32;
            //         let antenna2 = (baseline_idx / N_ANTS) as i32;
            //         table.put_cell("ANTENNA1", row_idx as _, &antenna1).unwrap();
            //         table.put_cell("ANTENNA2", row_idx as _, &antenna2).unwrap();
            //         row_idx+=1;
            //     }
            // }
            // row_idx = 0;
            // for timestep_idx in 0..N_TIMESTEPS {
            //     for baseline_idx in 0..n_baselines {
            //         table.put_cell("DATA_DESC_ID", row_idx as _, &0_i32).unwrap();

            //         row_idx+=1;
            //     }
            // }
            // row_idx = 0;
            // for timestep_idx in 0..N_TIMESTEPS {
            //     for baseline_idx in 0..n_baselines {
            //         // Calculate the uvw coordinates for this row.
            //         uvw_tmp[0] = row_idx as _;
            //         uvw_tmp[1] = baseline_idx as _;
            //         uvw_tmp[2] = timestep_idx as _;
            //         table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();

            //         row_idx+=1;
            //     }
            // }
            // row_idx = 0;
            // for timestep_idx in 0..N_TIMESTEPS {
            //     for baseline_idx in 0..n_baselines {
            //         table.put_cell("INTERVAL", row_idx as _, &(1. as f64)).unwrap();
            //         table.put_cell("EXPOSURE", row_idx as _, &(1. as f64)).unwrap();
            //         table.put_cell("PROCESSOR_ID", row_idx as _, &(-1. as i32)).unwrap();
            //         table.put_cell("SCAN_NUMBER", row_idx as _, &(1 as i32)).unwrap();
            //         table.put_cell("STATE_ID", row_idx as _, &(-1 as i32)).unwrap();
            //         table.put_cell("SIGMA", row_idx as _, &sigma_tmp).unwrap();

            //         row_idx+=1;
            //     }
            // }
            // // row_idx = 0;
            // // for timestep_idx in 0..N_TIMESTEPS {
            // //     for baseline_idx in 0..n_baselines {
            // //         // Calculate the uvw coordinates for this row.
            // //         uvw_tmp[0] = row_idx as _;
            // //         uvw_tmp[1] = baseline_idx as _;
            // //         uvw_tmp[2] = timestep_idx as _;

            // //         let antenna1 = (baseline_idx % N_ANTS) as i32;
            // //         let antenna2 = (baseline_idx / N_ANTS) as i32;
    
            // //         table.put_cell("TIME", row_idx as _, &(timestep_idx as f64)).unwrap();
            // //         table
            // //             .put_cell("TIME_CENTROID", row_idx as _, &(timestep_idx as f64))
            // //             .unwrap();
            // //         table.put_cell("ANTENNA1", row_idx as _, &antenna1).unwrap();
            // //         table.put_cell("ANTENNA2", row_idx as _, &antenna2).unwrap();
            // //         table.put_cell("DATA_DESC_ID", row_idx as _, &(0 as i32)).unwrap();
            // //         table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();
            // //         table.put_cell("INTERVAL", row_idx as _, &(1. as f64)).unwrap();
            // //         table.put_cell("EXPOSURE", row_idx as _, &(1. as f64)).unwrap();
            // //         table.put_cell("PROCESSOR_ID", row_idx as _, &(-1. as i32)).unwrap();
            // //         table.put_cell("SCAN_NUMBER", row_idx as _, &(1 as i32)).unwrap();
            // //         table.put_cell("STATE_ID", row_idx as _, &(-1 as i32)).unwrap();
            // //         table.put_cell("SIGMA", row_idx as _, &sigma_tmp).unwrap();

            // //         row_idx+=1;
            // //     }
            // // }
            // // row_idx = 0;
            // // for timestep_idx in 0..N_TIMESTEPS {
            // //     for baseline_idx in 0..n_baselines {
            // //         // Calculate the uvw coordinates for this row.
            // //         uvw_tmp[0] = row_idx as _;
            // //         uvw_tmp[1] = baseline_idx as _;
            // //         uvw_tmp[2] = timestep_idx as _;

            // //         let antenna1 = (baseline_idx % N_ANTS) as i32;
            // //         let antenna2 = (baseline_idx / N_ANTS) as i32;
    
            // //         table.put_cell("TIME", row_idx as _, &(timestep_idx as f64)).unwrap();
            // //         table
            // //             .put_cell("TIME_CENTROID", row_idx as _, &(timestep_idx as f64))
            // //             .unwrap();
            // //         table.put_cell("ANTENNA1", row_idx as _, &antenna1).unwrap();
            // //         table.put_cell("ANTENNA2", row_idx as _, &antenna2).unwrap();
            // //         table.put_cell("DATA_DESC_ID", row_idx as _, &(0 as i32)).unwrap();
            // //         table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();
            // //         table.put_cell("INTERVAL", row_idx as _, &(1. as f64)).unwrap();
            // //         table.put_cell("EXPOSURE", row_idx as _, &(1. as f64)).unwrap();
            // //         table.put_cell("PROCESSOR_ID", row_idx as _, &(-1. as i32)).unwrap();
            // //         table.put_cell("SCAN_NUMBER", row_idx as _, &(1 as i32)).unwrap();
            // //         table.put_cell("STATE_ID", row_idx as _, &(-1 as i32)).unwrap();
            // //         table.put_cell("SIGMA", row_idx as _, &sigma_tmp).unwrap();

            // //         row_idx+=1;
            // //     }
            // // }
            // // row_idx = 0;
            // // for timestep_idx in 0..N_TIMESTEPS {
            // //     for baseline_idx in 0..n_baselines {
            // //         // Calculate the uvw coordinates for this row.
            // //         uvw_tmp[0] = row_idx as _;
            // //         uvw_tmp[1] = baseline_idx as _;
            // //         uvw_tmp[2] = timestep_idx as _;

            // //         let antenna1 = (baseline_idx % N_ANTS) as i32;
            // //         let antenna2 = (baseline_idx / N_ANTS) as i32;
    
            // //         table.put_cell("TIME", row_idx as _, &(timestep_idx as f64)).unwrap();
            // //         table
            // //             .put_cell("TIME_CENTROID", row_idx as _, &(timestep_idx as f64))
            // //             .unwrap();
            // //         table.put_cell("ANTENNA1", row_idx as _, &antenna1).unwrap();
            // //         table.put_cell("ANTENNA2", row_idx as _, &antenna2).unwrap();
            // //         table.put_cell("DATA_DESC_ID", row_idx as _, &(0 as i32)).unwrap();
            // //         table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();
            // //         table.put_cell("INTERVAL", row_idx as _, &(1. as f64)).unwrap();
            // //         table.put_cell("EXPOSURE", row_idx as _, &(1. as f64)).unwrap();
            // //         table.put_cell("PROCESSOR_ID", row_idx as _, &(-1. as i32)).unwrap();
            // //         table.put_cell("SCAN_NUMBER", row_idx as _, &(1 as i32)).unwrap();
            // //         table.put_cell("STATE_ID", row_idx as _, &(-1 as i32)).unwrap();
            // //         table.put_cell("SIGMA", row_idx as _, &sigma_tmp).unwrap();

            // //         row_idx+=1;
            // //     }
            // // }
        })
    });
}

fn bench_table_put_cell_chj2(crt: &mut Criterion) {
    let n_baselines = N_ANTS * (N_ANTS - 1) / 2;
    let n_rows = n_baselines * N_TIMESTEPS;
    let data_shape = vec![N_CHANNELS, N_POLS];
    let shape = (N_TIMESTEPS, N_CHANNELS, n_baselines, N_POLS);
    let (vis_array, weight_array, flag_array) = synthesize_test_data(shape);

    // Create arrays to store synthetic visiblity data, re-used each row.
    let mut uvw_tmp = vec![0.; 3];
    let mut data_tmp = Array2::<Complex<f32>>::zeros((N_CHANNELS, N_POLS));
    let mut weights_tmp = Array2::<f32>::zeros((N_CHANNELS, N_POLS));
    let mut weights_pols_tmp = vec![0 as f32; N_POLS];
    let mut flags_tmp = Array::from_elem((N_CHANNELS, N_POLS), false);
    let sigma_tmp: Vec<f32> = vec![1., 1., 1., 1.];

    crt.bench_function("casatables::Table::put_cell columnwise one at a time iter", |bch| {
        bch.iter(|| {
            // Create a new temporary directory to write to each time
            let tmp_dir = tempfile::tempdir().unwrap();
            let table_path = tmp_dir.path().join("table.ms");
            let mut table = setup_main_table(table_path, n_rows, data_shape.iter().map(|&x| x as u64).collect());

            let mut row_idx = 0;

            // Write synthetic visibility data into the main table
            vis_array.outer_iter().for_each(|vis_timestep_view|  {
                vis_timestep_view.axis_iter(Axis(1)).for_each(|vis_baseline_view| {
                    data_tmp.assign(&vis_baseline_view);
                    table.put_cell("DATA", row_idx as _, &data_tmp).unwrap();
                    row_idx += 1;
                });
            });

        //     // row_idx = 0;
        //     // for (timestep_idx, weight_timestep_view) in izip!(
        //     //     0..N_TIMESTEPS, 
        //     //     weight_array.outer_iter(),
        //     // ) {
        //     //     for(baseline_idx, weight_baseline_view) in izip!(
        //     //         0..n_baselines,
        //     //         weight_timestep_view.axis_iter(Axis(1)),
        //     //     ) {
        //     //         weights_tmp.assign(&weight_baseline_view);    
        //     //         weights_pols_tmp.iter_mut().zip(weights_tmp.axis_iter(Axis(1))).for_each(|(elt, weights_pol)| {
        //     //             *elt = weights_pol.sum();
        //     //         });
    
        //     //         table.put_cell("WEIGHT_SPECTRUM", row_idx as _, &weights_tmp).unwrap();
        //     //         table.put_cell("WEIGHT", row_idx as _, &weights_pols_tmp).unwrap();

        //     //         row_idx+=1;
        //     //     }
        //     // }

        //     row_idx = 0;
        //     for weight_timestep_view in weight_array.outer_iter() {
        //         for weight_baseline_view in weight_timestep_view.axis_iter(Axis(1)) {
        //             weights_tmp.assign(&weight_baseline_view);    
        //             table.put_cell("WEIGHT_SPECTRUM", row_idx as _, &weights_tmp).unwrap();

        //             row_idx+=1;
        //         }
        //     }

        //     row_idx = 0;
        //     for weight_timestep_view in weight_array.outer_iter() {
        //         for weight_baseline_view in weight_timestep_view.axis_iter(Axis(1)) {
        //             weights_pols_tmp.iter_mut().zip(weight_baseline_view.axis_iter(Axis(1))).for_each(|(elt, weights_pol)| {
        //                 *elt = weights_pol.sum();
        //             });
        //             table.put_cell("WEIGHT", row_idx as _, &weights_pols_tmp).unwrap();

        //             row_idx+=1;
        //         }
        //     }

        //     row_idx = 0;
        //     for flag_timestep_view in flag_array.outer_iter() {
        //         for flag_baseline_view in flag_timestep_view.axis_iter(Axis(1)) {
        //             flags_tmp.assign(&flag_baseline_view);
        //             table.put_cell("FLAG", row_idx as _, &flags_tmp).unwrap();

        //             row_idx+=1;
        //         }
        //     }

        //     row_idx = 0;
        //     for timestep_idx in 0..N_TIMESTEPS {
        //         for _ in 0..n_baselines {
        //             table.put_cell("TIME", row_idx as _, &(timestep_idx as f64)).unwrap();
        //             row_idx+=1;
        //         }
        //     }
        //     row_idx = 0;
        //     for timestep_idx in 0..N_TIMESTEPS {
        //         for _ in 0..n_baselines {
        //             table
        //                 .put_cell("TIME_CENTROID", row_idx as _, &(timestep_idx as f64))
        //                 .unwrap();
        //             row_idx+=1;
        //         }
        //     }
        //     row_idx = 0;
        //     for _ in 0..N_TIMESTEPS {
        //         for baseline_idx in 0..n_baselines {
        //             let antenna1 = (baseline_idx % N_ANTS) as i32;
        //             let antenna2 = (baseline_idx / N_ANTS) as i32;
        //             table.put_cell("ANTENNA1", row_idx as _, &antenna1).unwrap();
        //             table.put_cell("ANTENNA2", row_idx as _, &antenna2).unwrap();
        //             row_idx+=1;
        //         }
        //     }
        //     row_idx = 0;
        //     for timestep_idx in 0..N_TIMESTEPS {
        //         for baseline_idx in 0..n_baselines {
        //             table.put_cell("DATA_DESC_ID", row_idx as _, &0_i32).unwrap();

        //             row_idx+=1;
        //         }
        //     }
        //     row_idx = 0;
        //     for timestep_idx in 0..N_TIMESTEPS {
        //         for baseline_idx in 0..n_baselines {
        //             // Calculate the uvw coordinates for this row.
        //             uvw_tmp[0] = row_idx as _;
        //             uvw_tmp[1] = baseline_idx as _;
        //             uvw_tmp[2] = timestep_idx as _;
        //             table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();

        //             row_idx+=1;
        //         }
        //     }
        //     row_idx = 0;
        //     for timestep_idx in 0..N_TIMESTEPS {
        //         for baseline_idx in 0..n_baselines {
        //             table.put_cell("INTERVAL", row_idx as _, &(1. as f64)).unwrap();
        //             table.put_cell("EXPOSURE", row_idx as _, &(1. as f64)).unwrap();
        //             table.put_cell("PROCESSOR_ID", row_idx as _, &(-1. as i32)).unwrap();
        //             table.put_cell("SCAN_NUMBER", row_idx as _, &(1 as i32)).unwrap();
        //             table.put_cell("STATE_ID", row_idx as _, &(-1 as i32)).unwrap();
        //             table.put_cell("SIGMA", row_idx as _, &sigma_tmp).unwrap();

        //             row_idx+=1;
        //         }
        //     }
        //     // row_idx = 0;
        //     // for timestep_idx in 0..N_TIMESTEPS {
        //     //     for baseline_idx in 0..n_baselines {
        //     //         // Calculate the uvw coordinates for this row.
        //     //         uvw_tmp[0] = row_idx as _;
        //     //         uvw_tmp[1] = baseline_idx as _;
        //     //         uvw_tmp[2] = timestep_idx as _;

        //     //         let antenna1 = (baseline_idx % N_ANTS) as i32;
        //     //         let antenna2 = (baseline_idx / N_ANTS) as i32;
    
        //     //         table.put_cell("TIME", row_idx as _, &(timestep_idx as f64)).unwrap();
        //     //         table
        //     //             .put_cell("TIME_CENTROID", row_idx as _, &(timestep_idx as f64))
        //     //             .unwrap();
        //     //         table.put_cell("ANTENNA1", row_idx as _, &antenna1).unwrap();
        //     //         table.put_cell("ANTENNA2", row_idx as _, &antenna2).unwrap();
        //     //         table.put_cell("DATA_DESC_ID", row_idx as _, &(0 as i32)).unwrap();
        //     //         table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();
        //     //         table.put_cell("INTERVAL", row_idx as _, &(1. as f64)).unwrap();
        //     //         table.put_cell("EXPOSURE", row_idx as _, &(1. as f64)).unwrap();
        //     //         table.put_cell("PROCESSOR_ID", row_idx as _, &(-1. as i32)).unwrap();
        //     //         table.put_cell("SCAN_NUMBER", row_idx as _, &(1 as i32)).unwrap();
        //     //         table.put_cell("STATE_ID", row_idx as _, &(-1 as i32)).unwrap();
        //     //         table.put_cell("SIGMA", row_idx as _, &sigma_tmp).unwrap();

        //     //         row_idx+=1;
        //     //     }
        //     // }
        //     // row_idx = 0;
        //     // for timestep_idx in 0..N_TIMESTEPS {
        //     //     for baseline_idx in 0..n_baselines {
        //     //         // Calculate the uvw coordinates for this row.
        //     //         uvw_tmp[0] = row_idx as _;
        //     //         uvw_tmp[1] = baseline_idx as _;
        //     //         uvw_tmp[2] = timestep_idx as _;

        //     //         let antenna1 = (baseline_idx % N_ANTS) as i32;
        //     //         let antenna2 = (baseline_idx / N_ANTS) as i32;
    
        //     //         table.put_cell("TIME", row_idx as _, &(timestep_idx as f64)).unwrap();
        //     //         table
        //     //             .put_cell("TIME_CENTROID", row_idx as _, &(timestep_idx as f64))
        //     //             .unwrap();
        //     //         table.put_cell("ANTENNA1", row_idx as _, &antenna1).unwrap();
        //     //         table.put_cell("ANTENNA2", row_idx as _, &antenna2).unwrap();
        //     //         table.put_cell("DATA_DESC_ID", row_idx as _, &(0 as i32)).unwrap();
        //     //         table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();
        //     //         table.put_cell("INTERVAL", row_idx as _, &(1. as f64)).unwrap();
        //     //         table.put_cell("EXPOSURE", row_idx as _, &(1. as f64)).unwrap();
        //     //         table.put_cell("PROCESSOR_ID", row_idx as _, &(-1. as i32)).unwrap();
        //     //         table.put_cell("SCAN_NUMBER", row_idx as _, &(1 as i32)).unwrap();
        //     //         table.put_cell("STATE_ID", row_idx as _, &(-1 as i32)).unwrap();
        //     //         table.put_cell("SIGMA", row_idx as _, &sigma_tmp).unwrap();

        //     //         row_idx+=1;
        //     //     }
        //     // }
        //     // row_idx = 0;
        //     // for timestep_idx in 0..N_TIMESTEPS {
        //     //     for baseline_idx in 0..n_baselines {
        //     //         // Calculate the uvw coordinates for this row.
        //     //         uvw_tmp[0] = row_idx as _;
        //     //         uvw_tmp[1] = baseline_idx as _;
        //     //         uvw_tmp[2] = timestep_idx as _;

        //     //         let antenna1 = (baseline_idx % N_ANTS) as i32;
        //     //         let antenna2 = (baseline_idx / N_ANTS) as i32;
    
        //     //         table.put_cell("TIME", row_idx as _, &(timestep_idx as f64)).unwrap();
        //     //         table
        //     //             .put_cell("TIME_CENTROID", row_idx as _, &(timestep_idx as f64))
        //     //             .unwrap();
        //     //         table.put_cell("ANTENNA1", row_idx as _, &antenna1).unwrap();
        //     //         table.put_cell("ANTENNA2", row_idx as _, &antenna2).unwrap();
        //     //         table.put_cell("DATA_DESC_ID", row_idx as _, &(0 as i32)).unwrap();
        //     //         table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();
        //     //         table.put_cell("INTERVAL", row_idx as _, &(1. as f64)).unwrap();
        //     //         table.put_cell("EXPOSURE", row_idx as _, &(1. as f64)).unwrap();
        //     //         table.put_cell("PROCESSOR_ID", row_idx as _, &(-1. as i32)).unwrap();
        //     //         table.put_cell("SCAN_NUMBER", row_idx as _, &(1 as i32)).unwrap();
        //     //         table.put_cell("STATE_ID", row_idx as _, &(-1 as i32)).unwrap();
        //     //         table.put_cell("SIGMA", row_idx as _, &sigma_tmp).unwrap();

        //     //         row_idx+=1;
        //     //     }
        //     // }
        })
    });
}

// criterion_group!(benches, bench_table_put_cell_data, bench_table_put_cell_main);
criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = 
        bench_table_put_cell_chj,    
        bench_table_put_cell_chj2,
        bench_table_put_cell_on_fly,
        bench_table_put_cell_preload_slice,
        bench_table_put_cell_preload_iter,
        bench_tablerow_put_preload_slice,
    );
criterion_main!(benches);
