#!/usr/bin/env python3
"""Quiet build and test for engine lanes: the full output goes to <WT>\\build-runs\\qbuild-<step>.log, the console
gets only the verdict, the counts and the first errors (capped).

Usage: python qbuild.py --wt <worktree> <step>... [--errors N]
  test     cargo test --workspace --target i686-pc-windows-msvc (from engine/): summed "test result:" counts,
           failed test names, the first lines of each panic, compile errors
  clippy   cargo clippy --workspace --all-targets -- -D warnings: error/warning count and the first diagnostics
  verify   build-runs\\coord-verify-lane.ps1 (CMake build + ctest; copied from the main checkout if missing):
           configure/build/ctest exit, BUILD ERROR lines, the ctest summary and failed tests
  fmt F..  rustfmt --check --edition 2021 on the given .rs files (paths relative to engine/ or absolute)
  all      test, clippy, verify
Cargo runs with CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0 (the disk is shared).
Exit 0 only if every step passed. Read more of a log with grep or Read offset/limit, never whole.
"""
import argparse
import os
import re
import shutil
import subprocess
import sys

MAIN = r"C:\Users\tlori\FreeCity4"


def run(cmd, cwd, log, env=None):
    with open(log, "w", encoding="utf-8", errors="replace") as f:
        p = subprocess.run(cmd, cwd=cwd, stdout=f, stderr=subprocess.STDOUT, env=env)
    with open(log, "rb") as f:
        text = f.read().replace(b"\x00", b"").decode("utf-8", errors="replace")
    return p.returncode, text.splitlines()


def cargo_env():
    env = dict(os.environ)
    env.update(CARGO_INCREMENTAL="0", CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0")
    return env


def compile_errors(lines, cap):
    out = []
    for i, l in enumerate(lines):
        if re.match(r"^error(\[E\d+\])?:", l) and not l.startswith("error: could not compile"):
            loc = next((m.strip() for m in lines[i + 1:i + 4] if m.strip().startswith("-->")), "")
            out.append(f"  {l.strip()[:200]} {loc}")
    return out[:cap], len(out)


def step_test(wt, logdir, cap):
    log = os.path.join(logdir, "qbuild-test.log")
    rc, lines = run(["cargo", "test", "--workspace", "--target", "i686-pc-windows-msvc"],
                    os.path.join(wt, "engine"), log, cargo_env())
    tot = [0, 0, 0]
    for l in lines:
        m = re.search(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", l)
        if m:
            for k in range(3):
                tot[k] += int(m.group(k + 1))
    failed = [l.split()[1] for l in lines if re.match(r"^test \S+ \.\.\. FAILED", l)]
    print(f"test: exit={rc} passed={tot[0]} failed={tot[1]} ignored={tot[2]}  (log {log})")
    errs, n = compile_errors(lines, cap)
    if errs:
        print(f"  {n} compile errors:")
        print("\n".join(errs))
    for name in failed[:cap]:
        print(f"  FAILED {name}")
        try:
            i = lines.index(f"---- {name} stdout ----")
            for l in lines[i + 1:i + 5]:
                if not l.strip() or l.startswith(("----", "test result:", "failures:")):
                    break
                print(f"    {l.strip()[:200]}")
        except ValueError:
            pass
    if len(failed) > cap:
        print(f"  ... {len(failed) - cap} more failed tests")
    return rc == 0 and tot[1] == 0


def step_clippy(wt, logdir, cap):
    log = os.path.join(logdir, "qbuild-clippy.log")
    rc, lines = run(["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
                    os.path.join(wt, "engine"), log, cargo_env())
    diags = []
    for i, l in enumerate(lines):
        if re.match(r"^(error|warning)(\[\w+\])?:", l) and "generated" not in l and "could not compile" not in l:
            loc = next((m.strip() for m in lines[i + 1:i + 4] if m.strip().startswith("-->")), "")
            diags.append(f"  {l.strip()[:200]} {loc}")
    print(f"clippy: exit={rc} diagnostics={len(diags)}  (log {log})")
    print("\n".join(diags[:cap]))
    if len(diags) > cap:
        print(f"  ... {len(diags) - cap} more")
    return rc == 0


def step_verify(wt, logdir, cap):
    ps1 = os.path.join(logdir, "coord-verify-lane.ps1")
    if not os.path.exists(ps1):
        shutil.copy(os.path.join(MAIN, "build-runs", "coord-verify-lane.ps1"), ps1)
    log = os.path.join(logdir, "qbuild-verify.log")
    rc, lines = run(["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", ps1,
                     "-Engine", os.path.join(wt, "engine")], wt, log)
    keep = {}
    for l in lines:
        for k in ("configure exit=", "build exit=", "ctest exit="):
            if l.startswith(k):
                keep[k] = l.strip()
    summary = [l.strip() for l in lines if re.search(r"tests passed, \d+ tests? failed out of \d+", l)]
    errs = [l.strip()[:220] for l in lines if l.startswith("BUILD ERROR LINE:")]
    failed = [l.strip()[:200] for l in lines if re.search(r"\(Failed\)|\*\*\*Failed|\(Not Run\)|\*\*\*Exception", l)]
    print(f"verify: {' '.join(keep.values()) or 'no exit lines (script failed?)'}  (log {log})")
    if summary:
        print(f"  ctest: {summary[-1]}")
    for l in errs[:cap]:
        print(f"  {l}")
    for l in failed[:cap]:
        print(f"  {l}")
    if len(failed) > cap:
        print(f"  ... {len(failed) - cap} more failed tests")
    ok = keep.get("build exit=", "").endswith("=0") and keep.get("ctest exit=", "").endswith("=0")
    return ok


def step_fmt(wt, files, cap):
    if not files:
        print("fmt: no files given")
        return False
    p = subprocess.run(["rustfmt", "--check", "--edition", "2021", *files], cwd=os.path.join(wt, "engine"),
                       capture_output=True, text=True, errors="replace")
    lines = (p.stdout + p.stderr).splitlines()
    print(f"fmt: exit={p.returncode} files={len(files)} diff_lines={len(lines)}")
    for l in lines[:cap]:
        print(f"  {l[:200]}")
    return p.returncode == 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--wt", required=True, help="the lane worktree")
    ap.add_argument("--errors", type=int, default=15, help="max error/diagnostic lines per step (default 15)")
    ap.add_argument("steps", nargs="+")
    a = ap.parse_args()
    wt = os.path.abspath(a.wt)
    logdir = os.path.join(wt, "build-runs")
    os.makedirs(logdir, exist_ok=True)
    steps = a.steps
    if steps[0] == "fmt":
        return 0 if step_fmt(wt, steps[1:], a.errors) else 1
    if steps == ["all"]:
        steps = ["test", "clippy", "verify"]
    fns = {"test": step_test, "clippy": step_clippy, "verify": step_verify}
    ok = True
    for s in steps:
        if s not in fns:
            sys.exit(f"qbuild: unknown step {s!r} (test, clippy, verify, fmt, all)")
        ok = fns[s](wt, logdir, a.errors) and ok
    print("qbuild: PASS" if ok else "qbuild: FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
