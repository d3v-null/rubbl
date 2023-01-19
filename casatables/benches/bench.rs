use criterion::{criterion_group, criterion_main, Criterion};
use ndarray::{Array1, Array2, Array3, Array4};
use rubbl_casatables::{
    Complex, GlueDataType, Table, TableCreateMode, TableDesc, TableDescCreateMode,
};
use std::path::PathBuf;
use tempfile::tempdir;

const N_ANTS: usize = 128;
const N_BLS: usize = N_ANTS * (N_ANTS + 1) / 2;
const N_TIMESTEPS: usize = 12;
const N_CHANNELS: usize = 24 * 32;
const N_POLS: usize = 4;

fn setup_main_table(table_path: PathBuf, n_rows: usize, data_shape: Vec<u64>) -> Table {
    let mut table_desc = TableDesc::new("", TableDescCreateMode::TDM_SCRATCH).unwrap();

    table_desc
        .add_scalar_column(GlueDataType::TpDouble, "TIME", None, true, false)
        .unwrap();

    table_desc
        .add_array_column(GlueDataType::TpFloat, "UVW", None, Some(&[3]), true, false)
        .unwrap();

    table_desc
        .add_array_column(
            GlueDataType::TpComplex,
            "DATA",
            None,
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
) -> (Array2<f64>, Array3<f32>, Array4<Complex<f32>>) {
    let time_array = Array2::from_shape_fn((shape.0, shape.1), |(timestep_idx, baseline_idx)| {
        (timestep_idx * shape.1 + baseline_idx) as f64
    });
    let uvw_array = Array3::from_shape_fn(
        (shape.0, shape.1, 3),
        |(timestep_idx, baseline_idx, uvw_idx)| {
            if uvw_idx == 0 {
                timestep_idx as f32
            } else if uvw_idx == 1 {
                baseline_idx as f32
            } else {
                1.
            }
        },
    );
    let vis_array = Array4::from_shape_fn(
        shape,
        |(timestep_idx, baseline_idx, chan_idx, pol_idx)| match pol_idx {
            0 => Complex::new(0., timestep_idx as _),
            1 => Complex::new(0., baseline_idx as _),
            2 => Complex::new(0., chan_idx as _),
            _ => Complex::new(0., 1.),
        },
    );
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
    (time_array, uvw_array, vis_array) // , weight_array, flag_array)
}

fn bench_table_put_cell_rowwise(crt: &mut Criterion) {
    let n_rows = N_BLS * N_TIMESTEPS;

    let tmp_dir = tempdir().unwrap();
    let table_path = tmp_dir.path().join("test.ms");

    let mut table = setup_main_table(table_path, n_rows, vec![N_CHANNELS as u64, N_POLS as u64]);
    let (times, uvws, data) = synthesize_test_data((N_TIMESTEPS, N_BLS, N_CHANNELS, N_POLS));
    let mut uvw_tmp = Array1::<f32>::zeros(3);
    let mut data_tmp = Array2::<Complex<f32>>::zeros((N_CHANNELS, N_POLS));

    crt.bench_function("casatables::Table::put_cell one at a time, row-wise", |bch| {
        bch.iter(|| {
            let mut row_idx = 0;

            // Write synthetic visibility data into the main table
            // timestep
            for ((times, uvws), data) in times
                .outer_iter()
                .zip(uvws.outer_iter())
                .zip(data.outer_iter())
            {
                // baseline
                for ((time, uvws), data) in times
                    .iter()
                    .zip(uvws.outer_iter())
                    .zip(data.outer_iter())
                {
                    table.put_cell("TIME", row_idx as _, time).unwrap();
                    uvw_tmp.assign(&uvws);
                    table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();
                    data_tmp.assign(&data);
                    table.put_cell("DATA", row_idx as _, &data_tmp).unwrap();

                    row_idx += 1;
                }
            }
        })
    });
}

fn bench_table_put_cell_columnwise(crt: &mut Criterion) {
    let n_rows = N_BLS * N_TIMESTEPS;

    let tmp_dir = tempdir().unwrap();
    let table_path = tmp_dir.path().join("test.ms");

    let mut table = setup_main_table(table_path, n_rows, vec![N_CHANNELS as u64, N_POLS as u64]);
    let (times, uvws, data) = synthesize_test_data((N_TIMESTEPS, N_BLS, N_CHANNELS, N_POLS));
    let mut uvw_tmp = Array1::<f32>::zeros(3);
    let mut data_tmp = Array2::<Complex<f32>>::zeros((N_CHANNELS, N_POLS));

    crt.bench_function("casatables::Table::put_cell one at a time, column-wise", |bch| {
        bch.iter(|| {
            let mut row_idx = 0;
            for times in times.outer_iter() {
                for times in times.iter() {
                    table.put_cell("TIME", row_idx as _, times).unwrap();
                    row_idx += 1;
                }
            }
            row_idx = 0;
            for uvws in uvws.outer_iter() {
                for uvws in uvws.outer_iter() {
                    uvw_tmp.assign(&uvws);
                    table.put_cell("UVW", row_idx as _, &uvw_tmp).unwrap();
                    row_idx += 1;
                }
            }
            row_idx = 0;
            for data in data.outer_iter() {
                for data in data.outer_iter() {
                    data_tmp.assign(&data);
                    table.put_cell("DATA", row_idx as _, &data_tmp).unwrap();
                    row_idx += 1;
                }
            }
        })
    });
}

fn bench_table_put_cells_rowwise(crt: &mut Criterion) {
    let n_rows = N_BLS * N_TIMESTEPS;

    let tmp_dir = tempdir().unwrap();
    let table_path = tmp_dir.path().join("test.ms");

    let mut table = setup_main_table(table_path, n_rows, vec![N_CHANNELS as u64, N_POLS as u64]);
    let (times, uvws, data) = synthesize_test_data((N_TIMESTEPS, N_BLS, N_CHANNELS, N_POLS));
    let mut times_tmp = Array1::<f64>::zeros(N_BLS);
    let mut uvws_tmp = Array2::<f32>::zeros((N_BLS, 3));
    let mut data_tmp = Array3::<Complex<f32>>::zeros((N_BLS, N_CHANNELS, N_POLS));

    crt.bench_function("casatables::Table::put_cells for each timestep, row-wise", |bch| {
        bch.iter(|| {
            let mut row_idx = 0;

            // Write synthetic visibility data into the main table
            for ((times, uvws), data) in times
                .outer_iter()
                .zip(uvws.outer_iter())
                .zip(data.outer_iter())
            {
                times_tmp.assign(&times);
                table.put_cells("TIME", row_idx as _, &times_tmp.to_vec()).unwrap();
                uvws_tmp.assign(&uvws);
                table.put_cells("UVW", row_idx as _, &uvws_tmp).unwrap();
                data_tmp.assign(&data);
                table.put_cells("DATA", row_idx as _, &data_tmp).unwrap();
                row_idx += N_BLS;
            }
        })
    });
}

fn bench_table_put_cells_columnwise(crt: &mut Criterion) {
    let n_rows = N_BLS * N_TIMESTEPS;

    let tmp_dir = tempdir().unwrap();
    let table_path = tmp_dir.path().join("test.ms");

    let mut table = setup_main_table(table_path, n_rows, vec![N_CHANNELS as u64, N_POLS as u64]);
    let (times, uvws, data) = synthesize_test_data((N_TIMESTEPS, N_BLS, N_CHANNELS, N_POLS));
    let mut uvws_tmp = Array2::<f32>::zeros((N_BLS, 3));
    let mut data_tmp = Array3::<Complex<f32>>::zeros((N_BLS, N_CHANNELS, N_POLS));

    crt.bench_function("casatables::Table::put_cells for each timestep, column-wise", |bch| {
        bch.iter(|| {
            let mut row_idx = 0;
            for times in times.outer_iter() {
                table.put_cells("TIME", row_idx as _, &times.to_vec()).unwrap();
                row_idx += N_BLS;
            }
            row_idx = 0;
            for uvws in uvws.outer_iter() {
                uvws_tmp.assign(&uvws);
                table.put_cells("UVW", row_idx as _, &uvws_tmp).unwrap();
                row_idx += N_BLS;
            }
            row_idx = 0;
            for data in data.outer_iter() {
                data_tmp.assign(&data);
                table.put_cells("DATA", row_idx as _, &data_tmp).unwrap();
                row_idx += N_BLS;
            }
        })
    });
}

fn bench_table_put_column(crt: &mut Criterion) {
    let n_rows = N_BLS * N_TIMESTEPS;

    let tmp_dir = tempdir().unwrap();
    let table_path = tmp_dir.path().join("test.ms");

    let mut table = setup_main_table(table_path, n_rows, vec![N_CHANNELS as u64, N_POLS as u64]);
    let (times, uvws, data) = synthesize_test_data((N_TIMESTEPS, N_BLS, N_CHANNELS, N_POLS));
    let times = times.into_shape((N_TIMESTEPS * N_BLS,)).unwrap();
    let uvws = uvws.into_shape((N_TIMESTEPS * N_BLS, 3)).unwrap();
    let data = data
        .into_shape((N_TIMESTEPS * N_BLS, N_CHANNELS, N_POLS))
        .unwrap();

    crt.bench_function("casatables::Table::put_column once", |bch| {
        bch.iter(|| {
            // Write synthetic visibility data into the main table
            table.put_column("TIME", &times).unwrap();
            table.put_column("UVW", &uvws).unwrap();
            table.put_column("DATA", &data).unwrap();
        })
    });
}

criterion_group!(
name = benches;
config = Criterion::default().sample_size(100);
targets =
    bench_table_put_cell_rowwise,
    bench_table_put_cells_rowwise,
    bench_table_put_column,
    bench_table_put_cell_columnwise,
    bench_table_put_cells_columnwise,
);
criterion_main!(benches);
