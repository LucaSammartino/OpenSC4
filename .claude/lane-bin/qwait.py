#!/usr/bin/env python3
"""Wait for a file in ONE blocking call (no polling turns), then print its first lines, capped.

Usage: python qwait.py <file> [--timeout 540] [--lines 20] [--fresh]
  --fresh   ignore a file older than the start of this wait (a previous run's verdict)
Prints "timeout" after --timeout seconds (default 540, under the 10-minute tool limit): call again.
A .verdict file only counts once it starts with "verdict:" and has not changed for 3 seconds.
"""
import argparse
import os
import sys
import time


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("file")
    ap.add_argument("--timeout", type=int, default=540)
    ap.add_argument("--lines", type=int, default=20)
    ap.add_argument("--fresh", action="store_true")
    a = ap.parse_args()
    start = time.time()
    while time.time() - start < a.timeout:
        try:
            st = os.stat(a.file)
        except OSError:
            time.sleep(5)
            continue
        if (a.fresh and st.st_mtime < start - 2) or time.time() - st.st_mtime < 3:
            time.sleep(3)
            continue
        with open(a.file, "rb") as f:
            text = f.read().replace(b"\x00", b"").decode("utf-8", errors="replace")
        if a.file.endswith(".verdict") and not text.startswith("verdict:"):
            time.sleep(3)
            continue
        for l in text.splitlines()[: a.lines]:
            print(l[:220])
        return 0
    print("timeout")
    return 1


if __name__ == "__main__":
    sys.exit(main())
