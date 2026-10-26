#!/usr/bin/env python3
"""MSVC/sccache launcher: repair cached /showIncludes before Ninja records it.

Sccache hashes preprocessed MSVC input correctly but replays the producing tree's
absolute dependency paths. Only include records are translated; compiler errors,
system headers, exit status and object bytes are untouched. Root mappings are
captured in CMake's launcher arguments, so they also work in standalone builds.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import subprocess
import sys


def rewrite_include(line, root, roots):
    match = re.match(rb"^(Note: including file:\s*)(.*?)(\r?\n)?$", line)
    if not match:
        return line
    path = match[2].replace(b"\\", b"/")
    for foreign in sorted(roots, key=len, reverse=True):
        prefix = os.fsencode(foreign).replace(b"\\", b"/").rstrip(b"/")
        if path.lower().startswith(prefix.lower() + b"/"):
            suffix = path[len(prefix):]
            # Also covers cache entries from lanes that have since been removed.
            suffix = re.sub(rb"^/\.claude/worktrees/[^/]+/", b"/", suffix, flags=re.I)
            mapped = os.fsencode(root).replace(b"\\", b"/").rstrip(b"/") + suffix
            return match[1] + mapped + (match[3] or b"")
    return line


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--root", required=True)
    ap.add_argument("--from-root", action="append", default=[])
    ap.add_argument("command", nargs=argparse.REMAINDER)
    a = ap.parse_args()
    command = a.command[1:] if a.command[:1] == ["--"] else a.command
    if not command:
        ap.error("compiler command required")
    # MSVC normally emits /showIncludes on stdout. Merge stderr too: a replay
    # must never leave uncorrected dependency lines on either stream.
    proc = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    for line in proc.stdout:
        sys.stdout.buffer.write(rewrite_include(line, a.root, a.from_root))
        sys.stdout.buffer.flush()
    return proc.wait()


if __name__ == "__main__":
    raise SystemExit(main())
