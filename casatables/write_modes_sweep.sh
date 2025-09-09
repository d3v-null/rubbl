#!/bin/bash

set -uo pipefail

STRACE_DIR="../strace_investigation"
mkdir -p "${STRACE_DIR}"

MODES=(table_put_row table_put_cell column_put column_put_bulk)
SMS=(default mmap buffer cache tsm) # aipsrc segfaults

echo "Building binaries..." >/dev/stderr
(cd .. && cargo build --release --examples >/dev/null)
make >/dev/null || true

# Determine workspace root and rust example path
ROOT_DIR="$(cd .. && pwd)"
rust_bin="${ROOT_DIR}/target/release/examples/syscall_tracer"
if [ ! -x "${rust_bin}" ]; then
  echo "Error: Rust example binary not found at ${rust_bin}" >&2
  exit 1
fi
cpp_bin="./syscall_tracer"

if [ ! -x "${cpp_bin}" ]; then
  echo "Warning: ${cpp_bin} not available; C++ sweep will be skipped" >/dev/stderr
fi

run_one() {
  local lang="$1"; shift
  local mode="$1"; shift
  local bin="$1"; shift
  local suffix="$1"; shift || true
  local log="${STRACE_DIR}/${lang}_${mode}${suffix}.strace"
  local output_log="${STRACE_DIR}/${lang}_${mode}${suffix}.output"
  rm -f "${log}" "${output_log}"
  timeout 30 bash -c "WRITE_MODE=\"${mode}\" CASACORE_SKIP_ZERO_INIT=\"${CASACORE_SKIP_ZERO_INIT:-}\" strace -e trace=file,desc -o \"${log}\" \"${bin}\" >\"${output_log}\" 2>&1"
  local exit_code=$?
  if grep -q "Unknown WRITE_MODE" "${output_log}" 2>/dev/null; then
    return 1
  fi
  return $exit_code
}

summarize_one() {
  local lang="$1"; shift
  local mode="$1"; shift
  local sm="$1"; shift
  local exit_status="$1"; shift
  local suffix="$1"; shift || true
  local log="${STRACE_DIR}/${lang}_${mode}${suffix}.strace"
  local output_log="${STRACE_DIR}/${lang}_${mode}${suffix}.output"
  if [ ! -f "${log}" ]; then
    printf "%s | %-16s | %-7s | %5s | %5s | %6s | %9s\n" "${lang}" "${mode}" "${sm}" "-" "-" "-" "-"
    return
  fi
  if [ "${exit_status}" -ne 0 ]; then
    printf "%s | %-16s | %-7s | %5s | %5s | %6s | %9s\n" "${lang}" "${mode}" "${sm}" "-" "-" "-" "-"
    # Clean up output log
    rm -f "${output_log}"
    return
  fi
  local zeros writes seeks ftrunc
  zeros=$(grep -c 'write.*\\0' "${log}" 2>/dev/null || echo 0)
  writes=$(grep -cE '\b(write|pwrite64|pwrite|writev)\(' "${log}" 2>/dev/null || echo 0)
  seeks=$(grep -c 'lseek' "${log}" 2>/dev/null || echo 0)
  ftrunc=$(grep -c 'ftruncate' "${log}" 2>/dev/null || echo 0)
  printf "%s | %-16s | %-7s | %5s | %5s | %6s | %9s\n" "${lang}" "${mode}" "${sm}" "${zeros}" "${writes}" "${seeks}" "${ftrunc}"
  # Clean up output log
  rm -f "${output_log}"
}

echo
for sm in "${SMS[@]}"; do
  echo "=== Sweep Results (Rust) [SM=${sm} default zero-init] ==="
  echo "Lang | Mode              | SM      | Zeros | Writes | Lseek  | Ftruncate"
  echo "-----+-------------------+--------+-------+--------+--------+----------"
  export STORAGE_MANAGER="${sm}"
  for m in "${MODES[@]}"; do
    run_one rust "$m" "$rust_bin" "_${sm}"
    exit_status=$?
    summarize_one rust "$m" "$sm" "$exit_status" "_${sm}"
  done
done

if [ -x "${cpp_bin}" ]; then
  for sm in "${SMS[@]}"; do
    echo
    echo "=== Sweep Results (C++) [SM=${sm} default zero-init] ==="
    echo "Lang | Mode              | SM      | Zeros | Writes | Lseek  | Ftruncate"
    echo "-----+-------------------+--------+-------+--------+--------+----------"
    export STORAGE_MANAGER="${sm}"
    for m in "${MODES[@]}"; do
      run_one cpp "$m" "$cpp_bin" "_${sm}"
      exit_status=$?
      summarize_one cpp "$m" "$sm" "$exit_status" "_${sm}"
    done
  done
fi

echo
echo "=== Sweep Results (Rust) [CASACORE_SKIP_ZERO_INIT=1] ==="
export CASACORE_SKIP_ZERO_INIT=1
for sm in "${SMS[@]}"; do
  echo "Lang | Mode              | SM      | Zeros | Writes | Lseek  | Ftruncate"
  echo "-----+-------------------+--------+-------+--------+--------+----------"
  export STORAGE_MANAGER="${sm}"
  for m in "${MODES[@]}"; do
    run_one rust "$m" "$rust_bin" "_${sm}_noinit"
    exit_status=$?
    summarize_one rust "$m" "$sm" "$exit_status" "_${sm}_noinit"
  done
done

if [ -x "${cpp_bin}" ]; then
  echo
  echo "=== Sweep Results (C++) [CASACORE_SKIP_ZERO_INIT=1] ==="
  for sm in "${SMS[@]}"; do
    echo "Lang | Mode              | SM      | Zeros | Writes | Lseek  | Ftruncate"
    echo "-----+-------------------+--------+-------+--------+--------+----------"
    export STORAGE_MANAGER="${sm}"
    for m in "${MODES[@]}"; do
      run_one cpp "$m" "$cpp_bin" "_${sm}_noinit"
      exit_status=$?
      summarize_one cpp "$m" "$sm" "$exit_status" "_${sm}_noinit"
    done
  done
fi

unset CASACORE_SKIP_ZERO_INIT

echo
echo "Sweep complete. Logs: ${STRACE_DIR}/<lang>_<mode>[ _noinit].strace"


