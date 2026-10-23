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
The native steps (quick, verify) go through tools/build_queue.py while its worker runs (main checkout
build-runs/build-queue-worker.alive is fresh): the Codex sandbox cannot run MSVC's CMake configure. Set
FC4_NATIVE_DIRECT=1 to run them here instead.
"""
import argparse
import os
import re
import subprocess
import sys
import time


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


def build_queue_alive():
    """True while tools/build_queue.py runs (it refreshes this file every few seconds)."""
    main = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    alive = os.path.join(main, "build-runs", "build-queue-worker.alive")
    return os.environ.get("FC4_NATIVE_DIRECT") != "1" and os.path.exists(alive) \
        and time.time() - os.path.getmtime(alive) < 60


def queued_fc4verify(wt, steps):
    """Hand the native steps to the build queue (outside the Codex sandbox, where MSVC configure hangs) and wait."""
    label = f"qbuild-{time.strftime('%Y%m%d-%H%M%S')}-{'-'.join(steps)}"
    qdir = os.path.join(wt, "build-runs", "build-queue")
    os.makedirs(qdir, exist_ok=True)
    with open(os.path.join(qdir, label + ".req"), "w", encoding="utf-8") as f:
        f.write(f"label = {label}\nsteps = {' '.join(steps)}\n")
    verdict = os.path.join(wt, "build-runs", "build-results", label + ".verdict")
    print(f"native steps queued to tools/build_queue.py as {label}; waiting (a game regress run delays it)")
    sys.stdout.flush()
    deadline = time.time() + 3 * 3600
    while not os.path.exists(verdict):
        if time.time() > deadline:
            print(f"build queue: no verdict for {label} after 3 h")
            return False
        time.sleep(10)
    time.sleep(1)
    with open(verdict, encoding="utf-8", errors="replace") as f:
        text = f.read()
    print(text.rstrip())
    return text.startswith("verdict: pass")


def fc4verify(wt, steps):
    if build_queue_alive():
        return queued_fc4verify(wt, steps)
    tool = os.path.join(os.path.dirname(os.path.abspath(__file__)), "fc4verify.py")
    p = subprocess.run([sys.executable, tool, "--wt", wt, *steps])
    return p.returncode == 0


def step_quick(wt, logdir, cap):
    return fc4verify(wt, ["quick"])


def step_verify(wt, logdir, cap):
    return fc4verify(wt, ["build", "ctest"])


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
    fns = {"test": step_test, "clippy": step_clippy, "quick": step_quick, "verify": step_verify}
    ok = True
    for s in steps:
        if s not in fns:
            sys.exit(f"qbuild: unknown step {s!r} (test, clippy, quick, verify, fmt, all)")
        ok = fns[s](wt, logdir, a.errors) and ok
    print("qbuild: PASS" if ok else "qbuild: FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
