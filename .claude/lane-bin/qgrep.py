#!/usr/bin/env python3
"""Capped search for lanes: at most --max matching lines (default 40), --per-file per file (default 8), each line
cut to --width characters (default 160), then a count of what was left out.

Usage: python qgrep.py [-i] [-l] [-C N] [--max N] [--per-file N] [--width N] [--glob *.rs] <regex> <path>...
  -l          file names with their match counts only
  -C N        N lines of context around each match (counts toward --max)
Skips .git, target, build* and __pycache__ directories. Standard library only (no ripgrep on this machine).
"""
import argparse
import fnmatch
import os
import re
import sys

SKIP_DIRS = {".git", "target", "__pycache__", "node_modules"}


def files_under(paths, pattern):
    for p in paths:
        if os.path.isfile(p):
            yield p
            continue
        for root, dirs, names in os.walk(p):
            dirs[:] = sorted(d for d in dirs if d not in SKIP_DIRS and not d.startswith("build"))
            for n in sorted(names):
                if pattern and not fnmatch.fnmatch(n, pattern):
                    continue
                yield os.path.join(root, n)


def cut(s, width):
    s = s.rstrip("\r\n")
    return s if len(s) <= width else s[:width] + f" ...[+{len(s) - width}]"


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("regex")
    ap.add_argument("paths", nargs="+")
    ap.add_argument("-i", action="store_true")
    ap.add_argument("-l", action="store_true")
    ap.add_argument("-C", type=int, default=0)
    ap.add_argument("--max", type=int, default=40)
    ap.add_argument("--per-file", type=int, default=8)
    ap.add_argument("--width", type=int, default=160)
    ap.add_argument("--glob")
    a = ap.parse_args()
    try:
        rx = re.compile(a.regex, re.I if a.i else 0)
    except re.error as e:
        sys.exit(f"qgrep: bad regex: {e}")
    shown = hidden = files = hidden_files = 0
    for path in files_under(a.paths, a.glob):
        try:
            with open(path, encoding="utf-8", errors="replace") as f:
                lines = f.readlines()
        except OSError:
            continue
        hits = [i for i, l in enumerate(lines) if rx.search(l)]
        if not hits:
            continue
        files += 1
        if a.l:
            if shown < a.max:
                print(f"{path}: {len(hits)}")
                shown += 1
            else:
                hidden_files += 1
            continue
        room = min(a.per_file, a.max - shown)
        if room <= 0:
            hidden += len(hits)
            hidden_files += 1
            continue
        last = -1
        for i in hits[:room]:
            lo, hi = max(0, i - a.C, last + 1), min(len(lines), i + a.C + 1)
            for j in range(lo, hi):
                print(f"{path}:{j + 1}{':' if j == i else '-'}{cut(lines[j], a.width)}")
            last = hi - 1
        shown += min(room, len(hits))
        hidden += max(0, len(hits) - room)
    if hidden or hidden_files:
        print(f"... {hidden} more matches, {hidden_files} more files not shown (narrow the regex or path, or -l)")
    if not files:
        print("qgrep: no matches")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
