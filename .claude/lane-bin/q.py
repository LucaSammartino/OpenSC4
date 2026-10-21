#!/usr/bin/env python3
"""Run any command quietly: the full output goes to a log file, the console gets a capped tail and the exit code.

Usage: python q.py [--lines N] [--bytes N] [--width N] [--head] [--full] [--log FILE] [--] <cmd> [args...]
  One argument is a shell command line (run with bash -c, so pipes and && work); several are run as they are.
  --lines N   lines shown (default 40), the last ones unless --head
  --bytes N   bytes shown at most (default 4096); each line is also cut to --width characters (default 200)
  --head      show the first lines instead of the last (for listings; a log's verdict is usually at the end)
  --full      no caps
  --log FILE  where the full output goes (default build-runs/q/<time>-<pid>.log under the current directory if it
              has build-runs/, else under the temp directory; the last 50 are kept); read more of it with qgrep or
              Read offset/limit, never whole
The last line is always "q: exit=N, L lines (K KB), showed ...; full output in FILE". Exits with the command's code.
"""
import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import time


KEEP_LOGS = 50


def default_log():
    """build-runs/q/<time>-<pid>.log (one per run: lanes share a checkout's build-runs); the oldest are pruned."""
    d = os.path.join("build-runs" if os.path.isdir("build-runs") else tempfile.gettempdir(), "q")
    os.makedirs(d, exist_ok=True)
    old = sorted(n for n in os.listdir(d) if n.endswith(".log"))
    for n in old[:max(0, len(old) - KEEP_LOGS + 1)]:
        try:
            os.remove(os.path.join(d, n))
        except OSError:
            pass
    return os.path.join(d, time.strftime("%Y%m%d-%H%M%S") + f"-{os.getpid()}.log")


def cut(s, width):
    return s if len(s) <= width else s[:width] + f" ...[+{len(s) - width}]"


def pick(lines, n, nbytes, width, head):
    """The first (head) or last n lines that fit in nbytes, in their original order."""
    seq = lines if head else reversed(lines)
    out, size = [], 0
    for line in seq:
        line = cut(line, width)
        b = len(line.encode("utf-8", "replace")) + 1
        if len(out) >= n or size + b > nbytes:
            break
        out.append(line)
        size += b
    return out if head else out[::-1]


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--lines", type=int, default=40)
    ap.add_argument("--bytes", type=int, default=4096)
    ap.add_argument("--width", type=int, default=200)
    ap.add_argument("--head", action="store_true")
    ap.add_argument("--full", action="store_true")
    ap.add_argument("--log")
    ap.add_argument("cmd", nargs=argparse.REMAINDER)
    a = ap.parse_args(argv)
    cmd = a.cmd[1:] if a.cmd[:1] == ["--"] else a.cmd
    if not cmd:
        ap.error("no command given")
    if len(cmd) == 1:
        bash = shutil.which("bash")
        cmd = [bash, "-c", cmd[0]] if bash else cmd[0]
    log = os.path.abspath(a.log or default_log())
    os.makedirs(os.path.dirname(log), exist_ok=True)
    with open(log, "wb") as f:
        try:
            rc = subprocess.run(cmd, stdout=f, stderr=subprocess.STDOUT, shell=isinstance(cmd, str)).returncode
        except OSError as e:
            f.write(f"q: cannot run {cmd!r}: {e}\n".encode())
            rc = 127
    with open(log, "rb") as f:
        raw = f.read().replace(b"\x00", b"")
    lines = raw.decode("utf-8", errors="replace").splitlines()
    if a.full:
        shown = lines
    else:
        shown = pick(lines, a.lines, a.bytes, a.width, a.head)
    if not a.head and len(shown) < len(lines):
        print(f"... {len(lines) - len(shown)} earlier lines not shown")
    for line in shown:
        print(line)
    if a.head and len(shown) < len(lines):
        print(f"... {len(lines) - len(shown)} more lines not shown")
    where = "all" if len(shown) == len(lines) else ("first" if a.head else "last") + f" {len(shown)}"
    print(f"q: exit={rc}, {len(lines)} lines ({len(raw) / 1024:.1f} KB), showed {where}; full output in {log}")
    return rc


if __name__ == "__main__":
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except AttributeError:
        pass
    sys.exit(main())
