#!/usr/bin/env python3
import argparse
import os
import re
import subprocess
from collections import defaultdict

syscall_re = re.compile(r"^\s*\d+\s*\d{2}:\d{2}:\d{2}\.\d+\s+(\w+)\(.*")
syscall_alt_re = re.compile(r"^\s*\d+\s*(\w+)\(.*")
frame_re = re.compile(r"^\s*>\s+(.+?)\(([^)+]+).*?\)\s*\[0x[0-9a-fA-F]+\]")

def parse_strace_k(path):
    per_syscall = defaultdict(list)
    current_frames = []
    current_sys = None
    with open(path, 'r', errors='ignore') as f:
        for line in f:
            line = line.rstrip('\n')
            m = frame_re.match(line)
            if m:
                # function name in group 2
                func = m.group(2)
                current_frames.append(func)
                continue
            m = syscall_re.match(line) or syscall_alt_re.match(line)
            if m:
                # flush previous block
                if current_sys and current_frames:
                    per_syscall[current_sys].append(list(current_frames))
                current_frames.clear()
                current_sys = m.group(1)
                continue
    # flush at EOF
    if current_sys and current_frames:
        per_syscall[current_sys].append(list(current_frames))
    return per_syscall

def write_folded(per_syscall, out_dir):
    os.makedirs(out_dir, exist_ok=True)
    outputs = []
    for sysc, stacks in per_syscall.items():
        if not stacks:
            continue
        folded_path = os.path.join(out_dir, f"{sysc}_folded.txt")
        with open(folded_path, 'w') as w:
            for frames in stacks:
                # strace -k prints top->bottom; flamegraph expects bottom->top
                folded = ';'.join(reversed(frames))
                w.write(f"{folded} 1\n")
        outputs.append((sysc, folded_path))
    return outputs

def generate_svgs(folded_files, flamegraph_pl, out_dir):
    svgs = []
    for sysc, folded in folded_files:
        svg = os.path.join(out_dir, f"{sysc}_flamegraph.svg")
        try:
            subprocess.run([flamegraph_pl, folded], check=True, stdout=open(svg, 'w'))
            svgs.append(svg)
        except Exception:
            pass
    return svgs

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('strace_k_file')
    ap.add_argument('--out-dir', required=True)
    ap.add_argument('--flamegraph', default='/home/dev/src/IO-ProfilingTools/FlameGraph/flamegraph.pl')
    args = ap.parse_args()

    per_sys = parse_strace_k(args.strace_k_file)
    folded = write_folded(per_sys, args.out_dir)
    svgs = generate_svgs(folded, args.flamegraph, args.out_dir)
    for p in svgs:
        print(p)

if __name__ == '__main__':
    main()


