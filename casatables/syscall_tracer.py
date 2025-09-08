#!/usr/bin/env python3
"""
Python version of the syscall tracer for comparison with Rust casatables.
This creates mock casatables operations to compare syscall patterns.
"""

import numpy as np
import tempfile
import os
import json

class MockTable:
    def __init__(self, path, n_rows, data_shape):
        self.path = path
        self.n_rows = n_rows
        self.data_shape = data_shape
        self.data = {}
        print(f"Mock table created at {path}")

    def put_cell(self, column, row_idx, value):
        if column not in self.data:
            self.data[column] = [None] * self.n_rows
        self.data[column][row_idx] = value

    def get_cell(self, column, row_idx):
        return self.data[column][row_idx]

def main():
    print("Starting Python syscall tracer...")

    # Simple operations for syscall analysis
    n_rows = 10
    data_shape = [16, 4]

    with tempfile.TemporaryDirectory() as tmp_dir:
        table_path = os.path.join(tmp_dir, "syscall_test_python.ms")

        print("Creating table...")
        table = MockTable(table_path, n_rows, data_shape)

        data_tmp = np.zeros((16, 4), dtype=np.complex64)
        flags_tmp = np.zeros((16, 4), dtype=bool)

        # Fill with simple data
        for idx in range(data_tmp.size):
            data_tmp.flat[idx] = complex(idx, 0.0)

        print("Writing data...")
        # Write a few rows
        for row_idx in range(3):
            table.put_cell("DATA", row_idx, data_tmp.copy())
            table.put_cell("FLAG", row_idx, flags_tmp.copy())
            table.put_cell("TIME", row_idx, float(row_idx))

        print("Reading data...")
        # Read some data back
        for row_idx in range(3):
            _ = table.get_cell("DATA", row_idx)
            _ = table.get_cell("FLAG", row_idx)

        # Some file I/O operations
        test_file = os.path.join(tmp_dir, "test_data.npy")
        np.save(test_file, data_tmp)
        loaded_data = np.load(test_file)

        # JSON operations
        metadata = {
            "n_rows": n_rows,
            "data_shape": data_shape,
            "test_value": 42
        }
        metadata_file = os.path.join(tmp_dir, "metadata.json")
        with open(metadata_file, 'w') as f:
            json.dump(metadata, f)

        with open(metadata_file, 'r') as f:
            loaded_metadata = json.load(f)

    print("Python syscall tracer completed.")

if __name__ == "__main__":
    main()
