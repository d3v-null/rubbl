#!/bin/bash
# Compile C++ syscall tracer

# Find casacore installation
CASACORE_ROOT="/usr"
if [ -d "/usr/local/include/casacore" ]; then
    CASACORE_ROOT="/usr/local"
fi

# Compile
g++ -std=c++17 -O2 \
    -I${CASACORE_ROOT}/include/casacore \
    -L${CASACORE_ROOT}/lib \
    -lcasa_tables -lcasa_casa \
    syscall_tracer.cpp -o syscall_tracer

echo "C++ syscall tracer compiled successfully"