#!/usr/bin/env python3
"""Run queue for native build requests (coordinator tooling). The Codex lanes' unelevated Windows sandbox cannot run
MSVC's CMake configure or ninja's compiler subprocesses (they hang), so lanes drop a request file into their own
worktree and this poller, started outside the sandbox, runs fc4verify on that worktree and writes the verdict back.
It is the native-build twin of tools/game_queue.ps1.

Usage (the coordinator starts it detached and leaves it running):
  python tools/build_queue.py [--jobs 1] [--poll 5] [--once] [--roots <dir> ...]
  python tools/build_queue.py --status
  python tools/build_queue.py --stop      ask the running worker to finish its builds and exit (restart it after)

Per root (the main checkout and every worktree in .claude/worktrees/): <root>/build-runs/build-queue/*.req, oldest
first. A request is key = value lines:
  label = verify-1        letters, digits, '.', '_', '-' (default: the file name)
  steps = build ctest     fc4verify steps: quick, test, clippy, build, ctest, cxxtest (default: build ctest)
Results: <root>/build-runs/build-results/<label>.verdict (first line "verdict: pass|fail|rejected", then the
fc4verify console output), <label>.running while it runs; the request moves to build-runs/build-queue/done/.
A native build under the game regress caused a machine-wide stall (2026-10-23), so the worker caps compile
parallelism instead of gating on game runs: with --jobs N each build gets CMAKE_BUILD_PARALLEL_LEVEL and
CARGO_BUILD_JOBS of cores/N, so N builds together spawn no more compilers than one uncapped build. (The old gate
counted every lane's game run in build-runs/game-results/, which is never empty under the game queue, so the worker
ran one build at a time with 20+ waiting, 2026-10-26.)
A worktree's waiting requests are served together: the build runs on the worktree as it is, so two requests from one
worktree would test the same tree. They run once with the union of their steps (quick and cxxtest are dropped when
build and ctest run: ctest includes cargo test --workspace), and every label gets the verdict.
Never kill the worker while a build runs: its fc4verify child outlives it and keeps the worktree's ninja files locked,
so the next build there fails at configure (2026-10-23). Use --stop, wait for "worker exit" in build-queue.log.
The worker writes build-runs/build-queue-worker.alive in the main checkout every poll; qbuild.py routes its verify
and quick steps here only while that file is fresh.
"""
import argparse
import os
import re
import subprocess
import sys
import threading
import time
from pathlib import Path

MAIN = Path(__file__).resolve().parent.parent
FC4VERIFY = MAIN / ".claude" / "lane-bin" / "fc4verify.py"
STEP_ORDER = ["quick", "test", "clippy", "build", "ctest", "cxxtest"]
STEPS = set(STEP_ORDER)
LABEL = re.compile(r"^[A-Za-z0-9._-]{1,80}$")


def roots(extra):
    if extra:
        return [Path(r) for r in extra]
    out = [MAIN]
    wt = MAIN / ".claude" / "worktrees"
    if wt.is_dir():
        out += sorted(p for p in wt.iterdir() if (p / ".git").exists())
    return out


def parse(req):
    kv = {}
    for n, line in enumerate(req.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            return None, f"line {n}: expected key = value"
        k, v = (s.strip() for s in line.split("=", 1))
        kv[k] = v
    label = kv.pop("label", req.stem)
    steps = kv.pop("steps", "build ctest").split()
    if kv:
        return None, f"unknown key(s): {', '.join(sorted(kv))}"
    if not LABEL.match(label):
        return None, f"bad label {label!r}"
    bad = [s for s in steps if s not in STEPS]
    if bad or not steps:
        return None, f"unknown step(s): {', '.join(bad) or '(none)'}"
    return (label, steps), None


def finish(root, req, label, word, text):
    res = root / "build-runs" / "build-results"
    res.mkdir(parents=True, exist_ok=True)
    (res / f"{label}.verdict").write_text(f"verdict: {word}\n{text}", encoding="utf-8")
    (res / f"{label}.running").unlink(missing_ok=True)
    done = req.parent / "done"
    done.mkdir(exist_ok=True)
    os.replace(req, done / req.name)


def union_steps(steps):
    """Steps in STEP_ORDER; quick and cxxtest are covered by build + ctest."""
    got = set(steps)
    if {"build", "ctest"} <= got:
        got -= {"quick", "cxxtest"}
    return [s for s in STEP_ORDER if s in got]


def run_one(root, reqs, steps, log, par):
    """reqs: [(req, label)] from one worktree, run once with steps; each label gets the verdict."""
    res = root / "build-runs" / "build-results"
    res.mkdir(parents=True, exist_ok=True)
    labels = [label for _, label in reqs]
    for label in labels:
        (res / f"{label}.running").write_text(time.strftime("%H:%M:%S"), encoding="utf-8")
    log(f"start {root.name}/{'+'.join(labels)}: {' '.join(steps)}")
    env = dict(os.environ)
    if par:
        env["CMAKE_BUILD_PARALLEL_LEVEL"] = env["CARGO_BUILD_JOBS"] = str(par)
    p = subprocess.run([sys.executable, str(FC4VERIFY), "--wt", str(root), *steps],
                       capture_output=True, text=True, errors="replace", env=env)
    out = p.stdout + p.stderr
    word = "pass" if re.search(r"^fc4verify: PASS", out, re.M) else "fail"
    for req, label in reqs:
        others = ", ".join(x for x in labels if x != label)
        note = f"(served together with {others}; steps {' '.join(steps)})\n" if others else ""
        finish(root, req, label, word, note + out)
    log(f"end {root.name}/{'+'.join(labels)}: {word}")


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--jobs", type=int, default=1)
    ap.add_argument("--poll", type=float, default=5)
    ap.add_argument("--once", action="store_true")
    ap.add_argument("--roots", nargs="*")
    ap.add_argument("--status", action="store_true")
    ap.add_argument("--stop", action="store_true")
    a = ap.parse_args()
    logf = MAIN / "build-runs" / "build-queue.log"
    alive = MAIN / "build-runs" / "build-queue-worker.alive"
    stop = MAIN / "build-runs" / "build-queue-worker.stop"

    if a.stop:
        stop.write_text(time.strftime("%H:%M:%S"), encoding="utf-8")
        print("stop requested: the worker takes no new builds and exits when its running ones end")
        return 0

    if a.status:
        print(f"worker alive: {alive.exists() and time.time() - alive.stat().st_mtime < 60}")
        for r in roots(a.roots):
            for f in sorted((r / "build-runs" / "build-results").glob("*.running")):
                print(f"running {r.name}/{f.stem}")
            for f in sorted((r / "build-runs" / "build-queue").glob("*.req")):
                print(f"waiting {r.name}/{f.name}")
        return 0

    lock = threading.Lock()

    def log(msg):
        with lock, open(logf, "a", encoding="utf-8") as f:
            f.write(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] {msg}\n")

    busy = {}  # root -> thread; one request per root at a time
    stop.unlink(missing_ok=True)
    par = max(2, (os.cpu_count() or 8) // a.jobs) if a.jobs > 1 else 0
    log(f"worker start: jobs={a.jobs} parallel level per build={par or 'uncapped'}")
    while True:
        alive.write_text(str(os.getpid()), encoding="utf-8")
        busy = {r: t for r, t in busy.items() if t.is_alive()}
        if stop.exists():
            if not busy:
                stop.unlink(missing_ok=True)
                alive.unlink(missing_ok=True)
                log("worker exit (--stop)")
                return 0
            time.sleep(a.poll)
            continue
        heads = []  # each root's waiting requests; roots served oldest first so no worktree starves
        for r in roots(a.roots):
            reqs = sorted((r / "build-runs" / "build-queue").glob("*.req"), key=lambda p: p.stat().st_mtime)
            if reqs:
                heads.append((reqs[0].stat().st_mtime, r, reqs))
        waiting = bool(heads)
        for _, r, reqs in sorted(heads, key=lambda h: h[0]):
            if r in busy or len(busy) >= a.jobs:
                continue
            good, steps = [], []
            for req in reqs:
                parsed, err = parse(req)
                if err:
                    finish(r, req, req.stem if LABEL.match(req.stem) else "rejected", "rejected", err + "\n")
                    log(f"rejected {r.name}/{req.name}: {err}")
                    continue
                good.append((req, parsed[0]))
                steps += parsed[1]
            if not good:
                continue
            t = threading.Thread(target=run_one, args=(r, good, union_steps(steps), log, par), daemon=True)
            busy[r] = t
            t.start()
        if a.once and not waiting and not busy:
            log("worker exit (--once)")
            return 0
        time.sleep(a.poll)


if __name__ == "__main__":
    sys.exit(main())
