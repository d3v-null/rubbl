use criterion::{criterion_group, criterion_main, Criterion};
use ndarray::{Array3, Array4, Array2};
use rubbl_casatables::{Complex, GlueDataType, Table, TableDesc, TableDescCreateMode, TableCreateMode};
use std::path::PathBuf;
use tempfile::tempdir;

const N_ANTS: usize = 128;
const N_TIMESTEPS: usize = 10;
const N_CHANNELS: usize = 24 * 32;
const N_POLS: usize = 4;

fn setup_main_table(
    table_path: PathBuf,
    n_rows : usize,
    data_shape: Vec<u64>
) -> Table {
    let mut table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();

    // table_desc
    //     .add_array_column(
    //         GlueDataType::TpDouble,
    //         "UVW",
    //         Some("Vector with uvw coordinates (in meters)"),
    //         Some(&[3]),
    //         true,
    //         false,
    //     )
    //     .unwrap();

    table_desc
        .add_array_column(
            GlueDataType::TpComplex,
            "DATA",
            Some("test data column"),
            Some(&data_shape),
            false,
            false,
        )
        .unwrap();

    // table_desc
    //     .add_array_column(
    //         GlueDataType::TpFloat,
    //         "WEIGHT_SPECTRUM",
    //         None,
    //         Some(&data_shape),
    //         false,
    //         false,
    //     )
    //     .unwrap();

    Table::new(&table_path, table_desc, n_rows, TableCreateMode::New).unwrap()
}

fn synthesize_test_data(
    shape: (usize, usize, usize, usize),
) ->
    // (Array4<Complex<f32>>, Array4<f32>, Array4<bool>)
    Array4<Complex<f32>>
{
    // let (N_TIMESTEPS, N_CHANNELS, n_baselines, N_POLS) = shape;
    // let vis_array =
    Array4::from_shape_fn(
        shape,
        |(timestep_idx, baseline_idx, chan_idx, pol_idx)| match pol_idx {
            0 => Complex::new(0., timestep_idx as _),
            1 => Complex::new(0., baseline_idx as _),
            2 => Complex::new(0., chan_idx as _),
            _ => Complex::new(0., 1.),
        },
    )
    // let weight_array = Array4::from_shape_fn(
    //     shape,
    //     |(timestep_idx, chan_idx, baseline_idx, pol_idx)| {
    //         (timestep_idx * shape.0 * shape.1 * shape.2 * shape.3
    //             + chan_idx * shape.0 * shape.1 * shape.2
    //             + baseline_idx * shape.0 * shape.1
    //             + pol_idx) as f32
    //     },
    // );
    // let flag_array =
    //     Array4::from_shape_fn(shape, |(timestep_idx, chan_idx, baseline_idx, pol_idx)| {
    //         (timestep_idx * shape.0 * shape.1 * shape.2 * shape.3
    //             + chan_idx * shape.0 * shape.1 * shape.2
    //             + baseline_idx * shape.0 * shape.1
    //             + pol_idx % 2
    //             == 0) as _
    //     });
    // (vis_array, weight_array, flag_array)
}

fn bench_table_put_cell_preload(crt: &mut Criterion) {
    let n_baselines = N_ANTS * (N_ANTS - 1) / 2;
    let n_rows = n_baselines * N_TIMESTEPS;

    let tmp_dir = tempdir().unwrap();
    let table_path = tmp_dir.path().join("test.ms");

    let mut table= setup_main_table(table_path, n_rows, vec![N_CHANNELS as u64, N_POLS as u64]);
    let data = synthesize_test_data((N_TIMESTEPS, n_baselines, N_CHANNELS, N_POLS));
    let mut data_tmp = Array2::<Complex<f32>>::zeros((N_CHANNELS, N_POLS));

    crt.bench_function("casatables::Table::put_cell one at a time", |bch| {
        bch.iter(|| {
            let mut row_idx = 0;

            // Write synthetic visibility data into the main table
            for data in data.outer_iter() {
                for data in data.outer_iter() {
                    data_tmp.assign(&data);
                    table.put_cell("DATA", row_idx as _, &data_tmp).unwrap();

                    row_idx+=1;
                }
            }
        })
    });
}

fn bench_table_put_cells_preload(crt: &mut Criterion) {
    let n_baselines = N_ANTS * (N_ANTS - 1) / 2;
    let n_rows = n_baselines * N_TIMESTEPS;

    let tmp_dir = tempdir().unwrap();
    let table_path = tmp_dir.path().join("test.ms");

    let mut table= setup_main_table(table_path, n_rows, vec![N_CHANNELS as u64, N_POLS as u64]);
    let data = synthesize_test_data((N_TIMESTEPS, n_baselines, N_CHANNELS, N_POLS));
    let mut data_tmp = Array3::<Complex<f32>>::zeros((n_baselines, N_CHANNELS, N_POLS));

    crt.bench_function("casatables::Table::put_cells for each timestep", |bch| {
        bch.iter(|| {
            let mut row_idx = 0;

            // Write synthetic visibility data into the main table
            for data in data.outer_iter() {
                data_tmp.assign(&data);
                table.put_cells("DATA", row_idx as _, &data_tmp).unwrap();
                row_idx+=n_baselines;
            }
        })
    });
}

fn bench_table_put_column_preload(crt: &mut Criterion) {
    let n_baselines = N_ANTS * (N_ANTS - 1) / 2;
    let n_rows = n_baselines * N_TIMESTEPS;

    let tmp_dir = tempdir().unwrap();
    let table_path = tmp_dir.path().join("test.ms");

    let mut table= setup_main_table(table_path, n_rows, vec![N_CHANNELS as u64, N_POLS as u64]);
    let data = synthesize_test_data((N_TIMESTEPS, n_baselines, N_CHANNELS, N_POLS));
    let data = data.into_shape((N_TIMESTEPS * n_baselines, N_CHANNELS, N_POLS)).unwrap();
    let mut data_tmp = Array3::<Complex<f32>>::zeros((N_TIMESTEPS * n_baselines, N_CHANNELS, N_POLS));

    crt.bench_function("casatables::Table::put_column once", |bch| {
        bch.iter(|| {

            // Write synthetic visibility data into the main table
            data_tmp.assign(&data);
            table.put_column("DATA", &data_tmp).unwrap();
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets =
        bench_table_put_cell_preload,
        bench_table_put_cells_preload,
        bench_table_put_column_preload,
    );
criterion_main!(benches);