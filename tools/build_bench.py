#!/usr/bin/env python3
"""Measure unchanged qbuild all gates in three isolated local source checkouts.

Uses only stdlib. Windows job accounting includes compiler/linker/test descendants;
system commit is sampled, not inferred from working sets. Results and full logs stay
under build-runs. Never cleans a user worktree or changes the source repository.
"""
from __future__ import annotations

import argparse
import ctypes
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import time


def gate_child(root, telemetry):
    """Own the cache server so Job CPU includes compilation done by sccache."""
    sys.path.insert(0, str(root / ".claude/lane-bin"))
    import fc4verify
    # The lifecycle helper belongs to the measurement tooling, including when
    # the measured checkout predates it.
    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / ".claude/lane-bin"))
    from cache_server import foreground_server
    env = fc4verify.build_env(root, "benchmark")
    exe = fc4verify.sccache_exe()
    ready = os.environ.get("FC4_BENCH_JOB_READY")
    if ready:
        deadline = time.monotonic() + 15
        while not Path(ready).exists():
            if time.monotonic() > deadline:
                raise RuntimeError("measurement Job was not assigned")
            time.sleep(.01)
    with foreground_server(exe, env):
        rc = subprocess.call([sys.executable, str(root / ".claude/lane-bin/qbuild.py"),
                              "--wt", str(root), "all"], cwd=root)
        stats = subprocess.check_output([exe, "--show-stats", "--stats-format", "json"], env=env, text=True)
        Path(telemetry).write_text(stats, encoding="utf-8")
    return rc


class Accounting(ctypes.Structure):
    _fields_ = [(n, ctypes.c_int64) for n in ("user", "kernel", "period_user", "period_kernel")]
    _fields_ += [(n, ctypes.c_uint32) for n in ("faults", "processes", "active", "terminated")]


class Performance(ctypes.Structure):
    _fields_ = [("cb", ctypes.c_uint32)]
    _fields_ += [(n, ctypes.c_size_t) for n in (
        "commit", "limit", "peak", "physical", "available", "cache", "kernel",
        "paged", "nonpaged", "page_size")]
    _fields_ += [(n, ctypes.c_uint32) for n in ("handles", "processes", "threads")]


def commit_percent():
    if os.name != "nt":
        return None
    info = Performance()
    info.cb = ctypes.sizeof(info)
    if not ctypes.windll.psapi.GetPerformanceInfo(ctypes.byref(info), info.cb):
        raise ctypes.WinError()
    return round(100 * info.commit / info.limit, 2)


def measured_run(cmd, cwd, env, log):
    job = None
    if os.name == "nt":
        api = ctypes.WinDLL("kernel32", use_last_error=True)
        api.CreateJobObjectW.restype = ctypes.c_void_p
        api.AssignProcessToJobObject.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
        api.QueryInformationJobObject.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_void_p,
                                                  ctypes.c_uint32, ctypes.c_void_p]
        api.CloseHandle.argtypes = [ctypes.c_void_p]
        job = api.CreateJobObjectW(None, None)
    started = time.monotonic()
    peak = commit_percent()
    try:
        with log.open("w", encoding="utf-8") as out:
            ready = log.with_suffix(".job-ready")
            ready.unlink(missing_ok=True)
            env = dict(env, FC4_BENCH_JOB_READY=str(ready))
            proc = subprocess.Popen(cmd, cwd=cwd, env=env, stdout=out, stderr=subprocess.STDOUT)
            if job and not api.AssignProcessToJobObject(job, int(proc._handle)):
                proc.terminate()
                proc.wait()
                raise ctypes.WinError(ctypes.get_last_error())
            ready.touch()
            while proc.poll() is None:
                value = commit_percent()
                if value is not None:
                    peak = max(peak, value)
                try:
                    proc.wait(timeout=0.5)
                except subprocess.TimeoutExpired:
                    pass
        cpu = None
        if job:
            info = Accounting()
            if not api.QueryInformationJobObject(job, 1, ctypes.byref(info), ctypes.sizeof(info), None):
                raise ctypes.WinError(ctypes.get_last_error())
            cpu = round((info.user + info.kernel) / 10_000_000, 2)
        text = log.read_text(encoding="utf-8", errors="replace")
        return {"seconds": round(time.monotonic() - started, 2), "cpu_seconds": cpu,
                "peak_commit_percent": peak, "exit": proc.returncode,
                "pass": "qbuild: PASS" in text and "qbuild: FAIL" not in text,
                "log": str(log)}
    finally:
        if job:
            api.CloseHandle(job)


def disk_bytes(root):
    return sum(p.stat().st_size for p in root.rglob("*") if p.is_file())


def percentile(values, fraction):
    values = sorted(values)
    position = (len(values) - 1) * fraction
    lo = int(position)
    return values[lo] + (values[min(lo + 1, len(values) - 1)] - values[lo]) * (position - lo)


def prepare(source, dest, revision):
    if (dest / ".prepared-v2").exists():
        actual = subprocess.check_output(["git", "-C", str(dest), "rev-parse", "HEAD"], text=True).strip()
        if actual != revision:
            raise ValueError(f"{dest}: prepared revision {actual} != {revision}; choose a fresh --out")
        return
    common = subprocess.check_output(["git", "-C", str(source), "rev-parse", "--path-format=absolute",
                                      "--git-common-dir"], text=True).strip()
    # A local alternate avoids transport helpers (and does not change shared refs).
    subprocess.run(["git", "init", "--quiet", str(dest)], check=True)
    (dest / ".git/objects/info/alternates").write_text(
        (Path(common) / "objects").as_posix() + "\n", encoding="utf-8", newline="\n")
    subprocess.run(["git", "-C", str(dest), "sparse-checkout", "set", "--no-cone",
                    "/engine/", "/docs/specs/", "/docs/abi/", "/.claude/lane-bin/",
                    "/tools/build_queue.py"], check=True)
    subprocess.run(["git", "-C", str(dest), "checkout", "--quiet", "--detach", revision], check=True)
    # Submodule content is public SDK data, shared by the normal lane builder too.
    sdk = Path(common).parent / "engine/third_party/gzcom-dll"
    shutil.copytree(sdk, dest / "engine/third_party/gzcom-dll", dirs_exist_ok=True,
                    ignore=shutil.ignore_patterns(".git"))
    (dest / ".prepared-v2").touch()


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--source", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--label", required=True)
    ap.add_argument("--revision", help="pin the measured source revision (default HEAD)")
    ap.add_argument("--lld", action="store_true", help="validate rust-lld for every i686 Rust binary")
    ap.add_argument("--jobs", type=int, help="compiler jobs per gate; leaves defaults unchanged if omitted")
    ap.add_argument("--count", type=int, default=5)
    ap.add_argument("--overlay", action="store_true", help="copy current build tooling/config into snapshots")
    a = ap.parse_args()
    source, out = a.source.resolve(), a.out.resolve()
    if "build-runs" not in out.parts or out == source or source in out.parents and out.name == "build-runs":
        ap.error("--out must be a dedicated directory below build-runs")
    out.mkdir(parents=True, exist_ok=True)
    revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", a.revision or "HEAD"], text=True).strip()
    common = subprocess.check_output(["git", "-C", str(source), "rev-parse", "--path-format=absolute",
                                      "--git-common-dir"], text=True).strip()
    roots = [out / ("wt-" + str(i)) for i in range(3)]
    for root in roots:
        prepare(source, root, revision)
        if a.overlay:
            shutil.copytree(source / ".claude/lane-bin", root / ".claude/lane-bin", dirs_exist_ok=True)
            for relative in ("engine/CMakeLists.txt", "engine/.cargo/config.toml",
                             "engine/core/Cargo.toml", "engine/formats/Cargo.toml", "engine/conformance/Cargo.toml",
                             "engine/core/tests/integration.rs", "engine/formats/tests/integration.rs",
                             "engine/conformance/tests/integration.rs"):
                shutil.copy2(source / relative, root / relative)
    results = []
    for index in range(a.count):
        root = roots[index % 3]
        # Only these disposable checkout artifacts, after canonical containment checks.
        for name in ("target", "build-x86"):
            artifact = (root / "engine" / name).resolve()
            if out not in artifact.parents or root.resolve() not in artifact.parents:
                raise ValueError("artifact escaped benchmark checkout")
            if artifact.exists():
                shutil.rmtree(artifact)
        env = dict(os.environ, FC4_NATIVE_DIRECT="1",
                   SCCACHE_DIR=str(out / "shared-cache"),
                   FC4_CACHE_ROOTS=os.pathsep.join(map(str, [source, Path(common).parent, *roots])))
        # A benchmark launched by CTest inherits the outer gate's wrappers and
        # server port. Let each pinned qbuild choose its own configuration.
        for key in ("RUSTC_WRAPPER", "FC4_SCCACHE", "FC4_CACHE_PYTHON", "SCCACHE_SERVER_PORT",
                    "SCCACHE_BASEDIRS", "_MSPDBSRV_ENDPOINT_"):
            env.pop(key, None)
        if a.jobs:
            env.update(CARGO_BUILD_JOBS=str(a.jobs), CMAKE_BUILD_PARALLEL_LEVEL=str(a.jobs))
        if a.lld:
            sysroot = Path(subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip())
            linkers = list((sysroot / "lib/rustlib").glob("*/bin/rust-lld.exe"))
            if len(linkers) != 1:
                raise RuntimeError(f"expected one rust-lld, found {linkers}")
            env["CARGO_TARGET_I686_PC_WINDOWS_MSVC_LINKER"] = str(linkers[0])
        telemetry = out / f"{a.label}-{index}-cache.json"
        result = measured_run([sys.executable, str(Path(__file__).resolve()), "--gate-child", str(root),
                               str(telemetry)], root, env, out / f"{a.label}-{index}.log")
        if telemetry.exists():
            stats = json.loads(telemetry.read_text(encoding="utf-8"))["stats"]
            result["cache"] = {k: stats[k] for k in ("cache_hits", "cache_misses", "not_cached") if k in stats}
        result.update(worktree=str(root), index=index, label=a.label,
                      revision=revision, lld=a.lld, jobs=a.jobs,
                      disk_bytes=sum(disk_bytes(root / "engine" / n) for n in ("target", "build-x86")))
        results.append(result)
        (out / (a.label + ".json")).write_text(json.dumps(results, indent=2), encoding="utf-8")
        print(json.dumps(result), flush=True)
        if not result["pass"]:
            return 1
    times = [r["seconds"] for r in results]
    print(json.dumps({"label": a.label, "median": statistics.median(times),
                      "p90": percentile(times, .9), "gates": len(results)}), flush=True)
    return 0


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--gate-child":
        raise SystemExit(gate_child(Path(sys.argv[2]), sys.argv[3]))
    raise SystemExit(main())
