#!/usr/bin/env python3
"""Cargo wrapper: cache registry crates from their stable manifest directory.

Local/workspace crates retain their working directory and embedded source paths.
Generated-code crates (OUT_DIR) retain it too: reusing their cached dependency
files would otherwise record another worktree's generated paths.
"""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys


def registry_cwd(args, env):
    manifest = env.get("CARGO_MANIFEST_DIR", "")
    directory = Path(manifest)
    cargo_home = Path(env.get("CARGO_HOME") or Path.home() / ".cargo")
    try:
        directory.resolve().relative_to((cargo_home / "registry/src").resolve())
    except ValueError:
        return None
    if not manifest or env.get("OUT_DIR") or env.get("CARGO_PRIMARY_PACKAGE"):
        return None
    # Cargo uses absolute input/output/library paths for registry crates. Refuse
    # to relocate manual invocations with relative paths, even when they happen
    # to carry registry environment variables.
    for index, arg in enumerate(args):
        if arg.endswith(".rs") and not Path(arg).is_absolute():
            return None
        if arg in ("--out-dir", "-L", "--extern") and index + 1 < len(args):
            path = args[index + 1].split("=", 1)[-1]
            if not Path(path).is_absolute():
                return None
    return str(directory)


def main():
    compiler, *args = sys.argv[1:]
    # Clippy diagnostics must be produced on every clippy invocation.
    if Path(compiler).stem.lower() == "clippy-driver":
        return subprocess.call([compiler, *args])
    return subprocess.call([os.environ["FC4_SCCACHE"], compiler, *args],
                           cwd=registry_cwd(args, os.environ))


if __name__ == "__main__":
    raise SystemExit(main())
