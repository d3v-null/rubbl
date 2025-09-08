#!/usr/bin/env python3
"""
Python version of the syscall tracer using python-casacore.
This creates real casacore table operations for accurate syscall comparison.
"""

import numpy as np
import tempfile
import os
import json
import sys

def check_casacore():
    """Check if python-casacore is available"""
    try:
        import casacore.tables as ct
        return True
    except ImportError:
        print("python-casacore not found. Install with: pip install python-casacore")
        return False

def main():
    if not check_casacore():
        sys.exit(1)

    import casacore.tables as ct

    print("Starting Python syscall tracer with casacore...")

    try:
        # Simple operations for syscall analysis
        n_rows = 100
        data_shape = [32, 4]

        with tempfile.TemporaryDirectory() as tmp_dir:
            table_path = os.path.join(tmp_dir, "syscall_test_python.ms")

            print("Creating casacore table...")

            # Create table description
            desc = ct.maketabdesc([
                ct.makescacoldesc("TIME", 0.0, comment="Observation time"),
                ct.makescacoldesc("ANTENNA1", 0, comment="First antenna"),
                ct.makescacoldesc("ANTENNA2", 0, comment="Second antenna"),
                ct.makescacoldesc("FLAG_ROW", False, comment="Row flag"),
                ct.makearrcoldesc("DATA", np.complex64(0), shape=data_shape, comment="Visibility data"),
                ct.makearrcoldesc("FLAG", False, shape=data_shape, comment="Data flags")
            ])

            # Create the table
            table = ct.table(table_path, desc, nrow=n_rows, readonly=False)

            print("Writing data...")

            # Prepare test data
            data_matrix = np.zeros(data_shape, dtype=np.complex64)
            flag_matrix = np.zeros(data_shape, dtype=bool)

            # Write a few rows
            for row_idx in range(min(10, n_rows)):  # Write fewer rows for efficiency
                # Create test data with some pattern
                for i in range(data_shape[0]):
                    for j in range(data_shape[1]):
                        idx = i * data_shape[1] + j
                        data_matrix[i, j] = complex(float(idx % 100), 0.0)
                        flag_matrix[i, j] = (idx + row_idx) % 13 == 0

                # Write to table
                table.putcell("TIME", row_idx, float(row_idx))
                table.putcell("ANTENNA1", row_idx, (row_idx % 128))
                table.putcell("ANTENNA2", row_idx, ((row_idx + 1) % 128))
                table.putcell("FLAG_ROW", row_idx, (row_idx % 2 == 0))
                table.putcell("DATA", row_idx, data_matrix)
                table.putcell("FLAG", row_idx, flag_matrix)

            print("Reading data...")

            # Read some data back
            for row_idx in range(min(5, n_rows)):
                time_val = table.getcell("TIME", row_idx)
                ant1_val = table.getcell("ANTENNA1", row_idx)
                ant2_val = table.getcell("ANTENNA2", row_idx)
                flag_row_val = table.getcell("FLAG_ROW", row_idx)
                data_matrix = table.getcell("DATA", row_idx)
                flag_matrix = table.getcell("FLAG", row_idx)

                # Some processing to generate syscalls
                processed_time = time_val * 2.0
                baseline = ant1_val + ant2_val
                combined_flag = flag_row_val or np.any(flag_matrix)

            print("Performing additional I/O operations...")

            # Additional file I/O operations
            test_file = os.path.join(tmp_dir, "test_data.npy")
            np.save(test_file, data_matrix)
            loaded_data = np.load(test_file)

            # JSON operations
            metadata = {
                "n_rows": n_rows,
                "data_shape": data_shape,
                "test_value": 42,
                "library": "python-casacore"
            }
            metadata_file = os.path.join(tmp_dir, "metadata.json")
            with open(metadata_file, 'w') as f:
                json.dump(metadata, f)

            with open(metadata_file, 'r') as f:
                loaded_metadata = json.load(f)

            # Close table
            table.close()

        print("Python syscall tracer with casacore completed successfully.")

    except Exception as e:
        print(f"Error in Python syscall tracer: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

if __name__ == "__main__":
    main()
