### Running the examples and benches

This document shows quick, copy-pasteable commands to run Rubbl examples and the casatables benches locally. It assumes you have the Birli repo checked out next to this repo so the test data are available.

- Birli repo path assumed: `../Birli`
- Test data path: `../Birli/tests/data/1119683928_picket`
- Output directory (created below): `testdata/`

### 1) Prepare a small Measurement Set with Birli

We'll convert the picket-fence FITS data into a small CASA Measurement Set (MS) for the casatables example/benches. Limiting to a single coarse-channel pair keeps it fast.

```bash
# From this repo root
mkdir -p testdata

# Generate a small MS (restrict to coarse channels 62-63)
cargo run --manifest-path "../Birli/Cargo.toml" -- \
  -m "../Birli/tests/data/1119683928_picket/1119683928.metafits" \
  --sel-chan-ranges 62-63 \
  -M "testdata/1119683928_picket.ms" \
  "../Birli/tests/data/1119683928_picket/1119683928_20150630071834_gpubox01_00.fits"

# Move the MS into benches for convenience
git mv testdata/1119683928_picket_ch62-63.ms casatables/benches/1119683928_picket_ch62-63.ms
```

### 2) casatables example: tableinfo (summarize an MS)

```bash
cargo run -p rubbl_casatables --example tableinfo -- \
  "casatables/benches/1119683928_picket_ch62-63.ms"
```

### 3) FITS examples: fitssummary and fitsdump

Run these directly on one of the GPU box FITS files from the picket dataset:

```bash
# Summarize HDUs
cargo run -p rubbl_fits --example fitssummary -- \
  "../Birli/tests/data/1119683928_picket/1119683928_20150630071834_gpubox01_00.fits"

# Low-level parse/dump (prints headers and indicates data blocks)
cargo run -p rubbl_fits --example fitsdump -- \
  "../Birli/tests/data/1119683928_picket/1119683928_20150630071834_gpubox01_00.fits"
```

### 4) MIRIAD examples (require a MIRIAD UV dataset)

The MIRIAD examples expect a MIRIAD dataset directory (not a FITS file, not an MS). If you have a dataset at `</path/to/miriad_uv_dataset>`:

```bash
# List items in the dataset
cargo run -p rubbl_miriad --example dsls -- \
  "/path/to/miriad_uv_dataset"

# Dump history
cargo run -p rubbl_miriad --example dshistory -- \
  "/path/to/miriad_uv_dataset"

# Print initial header variables
cargo run -p rubbl_miriad --example uvheadervars -- \
  "/path/to/miriad_uv_dataset"

# Diagnostic UV dump (verbose)
cargo run -p rubbl_miriad --example uvdump -- \
  "/path/to/miriad_uv_dataset"

# Throughput benchmark (reads UV data as fast as possible)
cargo run -p rubbl_miriad --example uvblast -- \
  "/path/to/miriad_uv_dataset"
```

### 5) Running the casatables benches

The casatables benchmarks synthesize data and write CASA tables to measure performance of `Table::put_cell` and `TableRow::put` in different access patterns. They do not require external data beyond the bundled `default_tables.tar.gz`.

```bash
# Run only casatables benches (release mode is automatic)
cargo bench -p rubbl_casatables
```

- Typical timings on a laptop (illustrative only):
  - casatables::Table::put_cell columnwise one at a time: ~2.0–2.3 s
  - casatables::Table::put_cell, on the fly: ~4.7–4.8 s
  - casatables::Table::put_cell slicing pre-loaded data: ~5.7–6.3 s
  - casatables::Table::put_cell izip views of pre-loaded data: ~5.8–6.3 s
  - casatables::TableRow::put slicing, using tablerow: ~9.8–10.9 s

Notes:
- Benchmarks may take minutes; Criterion will increase sample time automatically.
- Gnuplot is optional; plotters-based HTML reports are generated if enabled.
- If casacore links fail on macOS, ensure Homebrew libs are visible (e.g., `export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib`).
