### Running the examples

This document shows quick, copy-pasteable commands to run Rubbl examples locally. It assumes you have the Birli repo checked out next to this repo so the test data are available.

- Birli repo path assumed: `../Birli`
- Test data path: `../Birli/tests/data/1119683928_picket`
- Output directory (created below): `testdata/`

### 1) Prepare a small Measurement Set with Birli

We’ll convert the picket-fence FITS data into a small CASA Measurement Set (MS) for the casatables example. Limiting to a single coarse-channel pair keeps it fast.

```bash
# From this repo root
mkdir -p testdata

# Generate a small MS (restrict to coarse channels 62-63)
cargo run --manifest-path "../Birli/Cargo.toml" -- \
  -m "../Birli/tests/data/1119683928_picket/1119683928.metafits" \
  --sel-chan-ranges 62-63 \
  -M "testdata/1119683928_picket.ms" \
  "../Birli/tests/data/1119683928_picket/1119683928_20150630071834_gpubox01_00.fits"
```

Note: Birli appends the coarse-channel range to the MS filename; you should see something like `testdata/1119683928_picket_ch62-63.ms/` created.

### 2) casatables example: tableinfo (summarize an MS)

```bash
cargo run -p rubbl_casatables --example tableinfo -- \
  "testdata/1119683928_picket_ch62-63.ms"
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
dcargo run -p rubbl_miriad --example dsls -- \
  "/path/to/miriad_uv_dataset"

# Dump history
dcargo run -p rubbl_miriad --example dshistory -- \
  "/path/to/miriad_uv_dataset"

# Print initial header variables
dcargo run -p rubbl_miriad --example uvheadervars -- \
  "/path/to/miriad_uv_dataset"

# Diagnostic UV dump (verbose)
cargo run -p rubbl_miriad --example uvdump -- \
  "/path/to/miriad_uv_dataset"

# Throughput benchmark (reads UV data as fast as possible)
cargo run -p rubbl_miriad --example uvblast -- \
  "/path/to/miriad_uv_dataset"
```

### Notes
- The `testdata/` directory is for local, generated artifacts and is not intended to be checked into git.
- If you hit linker issues with casacore on macOS, ensure your Homebrew libraries are discoverable (e.g., `export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib`).
