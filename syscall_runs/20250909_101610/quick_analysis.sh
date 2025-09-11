#!/bin/bash
# Quick analysis commands for lseek investigation

echo "=== LSEEK COUNTS COMPARISON ==="
echo "Configuration | Total lseeks | SEEK_SET lseeks"
echo "============================================="
echo "Baseline:     $(grep -c 'lseek(' rust.strace) | $(grep -c 'lseek.*SEEK_SET' rust.strace)"
echo "MMAP:         $(grep -c 'lseek(' rust_mmap.strace) | $(grep -c 'lseek.*SEEK_SET' rust_mmap.strace)"  
echo "Scratch:      $(grep -c 'lseek(' rust_scratch.strace) | $(grep -c 'lseek.*SEEK_SET' rust_scratch.strace)"
echo

echo "=== PHASE BREAKDOWN ==="
echo "Phase         | lseek count"
echo "========================"
echo "COMPLEX col:  $(grep -c 'lseek(' phase_complex.txt)"
echo "BOOL col:     $(grep -c 'lseek(' phase_bool.txt)"
echo "Setup/cleanup: $(($(grep -c 'lseek(' rust.strace) - $(grep -c 'lseek(' phase_complex.txt) - $(grep -c 'lseek(' phase_bool.txt)))"
echo

echo "=== TOP SYSCALLS ==="
awk '{print $2}' rust.strace | cut -d'(' -f1 | sort | uniq -c | sort -nr | head -10