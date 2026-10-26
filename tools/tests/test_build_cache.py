"""Cache correctness, including train278 with the real Windows toolchain."""
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tempfile

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / ".claude/lane-bin"))
import cache_compile
from cache_server import foreground_server
import fc4verify


@pytest.mark.skipif(os.name != "nt" or not fc4verify.VCVARS.exists() or not shutil.which("sccache"),
                    reason="requires Windows build tools")
def test_registry_crate_hits_across_worktree_cwds():
    import rust_cache
    manifests = sorted((Path.home() / ".cargo/registry/src").glob("*/adler2-*/Cargo.toml"))
    if not manifests:
        pytest.skip("adler2 registry source is not installed")
    (ROOT / "build-runs").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="rust-cache-", dir=ROOT / "build-runs") as tmp:
        tmp = Path(tmp)
        a, b = tmp / "a", tmp / "b"
        a.mkdir()
        b.mkdir()
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        env = fc4verify.build_env(ROOT, "registry-cache-test")
        env.update(SCCACHE_SERVER_PORT=str(port), SCCACHE_DIR=str(tmp / "cache"),
                   CARGO_MANIFEST_DIR=str(manifests[0].parent))
        env.pop("OUT_DIR", None)
        env.pop("CARGO_PRIMARY_PACKAGE", None)
        with foreground_server(env["FC4_SCCACHE"], env):
            for directory in (a, b):
                args = [sys.executable, rust_cache.__file__, shutil.which("rustc"),
                        "--crate-name", "adler2", "--crate-type=rlib", "--edition=2021",
                        "--emit=dep-info,metadata,link", "-Cdebuginfo=0", "--out-dir", str(directory),
                        str(manifests[0].parent / "src/lib.rs")]
                result = subprocess.run(args, cwd=directory, env=env, capture_output=True, text=True, timeout=60)
                assert result.returncode == 0, result.stdout + result.stderr
                assert (directory / "libadler2.rlib").is_file()
            stats = subprocess.check_output([env["FC4_SCCACHE"], "--show-stats", "--stats-format", "json"],
                                             env=env, text=True, timeout=15)
            counts = json.loads(stats)["stats"]
            assert counts["cache_hits"]["counts"].get("Rust") == 1
            assert counts["cache_misses"]["counts"].get("Rust") == 1


@pytest.mark.parametrize("path,expected", [
    (b"C:\\repo\\.claude\\worktrees\\removed\\engine\\header.h", b"D:/lane/engine/header.h"),
    (b"C:/repo/engine/header.h", b"D:/lane/engine/header.h"),
    (b"C:/repository/header.h", b"C:/repository/header.h"),
    (b"C:/Program Files/SDK/header.h", b"C:/Program Files/SDK/header.h"),
])
def test_include_mapping(path, expected):
    line = b"Note: including file:   " + path + b"\r\n"
    assert cache_compile.rewrite_include(line, "D:/lane", ["C:/repo"]) == (
        b"Note: including file:   " + expected + b"\r\n")
    assert cache_compile.rewrite_include(b"error: " + path, "D:/lane", ["C:/repo"]) == b"error: " + path


@pytest.mark.skipif(os.name != "nt" or not fc4verify.VCVARS.exists() or not shutil.which("sccache"),
                    reason="requires Windows build tools")
def test_real_msvc_replayed_include_prefix():
    """Exercise the mapper with real cached output, independently of Ninja IPC."""
    (ROOT / "build-runs").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="msvc-cache-", dir=ROOT / "build-runs") as tmp:
        tmp = Path(tmp)
        a, b = tmp / "a", tmp / "b"
        for directory in (a, b):
            directory.mkdir()
            (directory / "value.h").write_text("#define VALUE 1\n")
            (directory / "main.cpp").write_text('#include "value.h"\nint main(){return VALUE == 1 ? 0 : 1;}\n')
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        env = fc4verify.build_env(ROOT, "msvc-cache-test")
        env.update(SCCACHE_SERVER_PORT=str(port), SCCACHE_DIR=str(tmp / "cache"),
                   SCCACHE_BASEDIRS=os.pathsep.join(map(str, (a, b))))
        compiler = shutil.which("cl", path=env["PATH"])
        command = [env["FC4_SCCACHE"], compiler, "/nologo", "/showIncludes", "/c", "main.cpp", "/Fomain.obj"]
        with foreground_server(env["FC4_SCCACHE"], env):
            subprocess.run(command, cwd=a, env=env, capture_output=True, check=True, timeout=30)
            mapped = [sys.executable, cache_compile.__file__, "--root", str(b), "--from-root", str(a), "--",
                      *command]
            output = subprocess.check_output(mapped, cwd=b, env=env, timeout=30).replace(b"\\", b"/")
            assert (b / "value.h").as_posix().encode() in output
            assert (a / "value.h").as_posix().encode() not in output
            stats = json.loads(subprocess.check_output(
                [env["FC4_SCCACHE"], "--show-stats", "--stats-format", "json"], env=env, timeout=15))["stats"]
            assert stats["cache_hits"]["counts"].get("C/C++") == 1
            original = (b / "main.obj").read_bytes()
            (b / "value.h").write_text("#define VALUE 2\n")
            subprocess.run(mapped, cwd=b, env=env, capture_output=True, check=True, timeout=30)
            assert (b / "main.obj").read_bytes() != original


@pytest.mark.skipif(os.name != "nt" or not fc4verify.VCVARS.exists() or not shutil.which("sccache"),
                    reason="real MSVC/Ninja/sccache integration requires Windows build tools")
def test_train278_header_change_after_cross_worktree_hit():
    """First demonstrate stale success without mapping; then require a real rebuild.

    No fake compiler or cache: Ninja's deps database and the linked executable
    are inspected. This fails if the launcher is removed from the fixed case.
    """
    (ROOT / "build-runs").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="cache-regression-", dir=ROOT / "build-runs") as tmp:
        tmp = Path(tmp)
        a, b = tmp / "a", tmp / "b"
        a.mkdir()
        b.mkdir()
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        env = fc4verify.build_env(ROOT, "cache-regression")
        env.update(SCCACHE_SERVER_PORT=str(port), SCCACHE_DIR=str(tmp / "cache"),
                   SCCACHE_BASEDIRS=os.pathsep.join(map(str, (a, b))),
                   SCCACHE_ERROR_LOG=str(tmp / "sccache.log"), SCCACHE_LOG="debug")
        cache = fc4verify.sccache_exe()
        compiler = shutil.which("cl", path=env["PATH"])

        def run(command, cwd=tmp):
            command = [shutil.which(command[0], path=env["PATH"]) or command[0], *command[1:]]
            try:
                result = subprocess.run(command, cwd=cwd, env=env, capture_output=True,
                                        text=True, errors="replace", timeout=30)
            except subprocess.TimeoutExpired as exc:
                debug = "".join(debug_lines)
                raise AssertionError(f"compiler timed out: {exc.stdout!r}; {debug[-6000:]}") from exc
            assert result.returncode == 0, result.stdout + result.stderr
            return result.stdout

        def fixture(root, mapped):
            launcher = f'"{cache}"'
            if mapped:
                launcher = (f'"{sys.executable}" "{cache_compile.__file__}" --root "{root}" '
                            f'--from-root "{a}" --from-root "{b}" -- ' + launcher)
            # Ninja syntax uses forward slashes and escapes drive-colons only in build paths.
            rules = (f'rule cc\n  command = {launcher} "{compiler}" /nologo /showIncludes '
                     '/c main.cpp /Foobject.obj\n  deps = msvc\n'
                     f'rule link\n  command = "{compiler}" /nologo object.obj /Fesmoke.exe\n'
                     'build object.obj: cc main.cpp\nbuild smoke.exe: link object.obj\n'
                     'default smoke.exe\n')
            (root / "build.ninja").write_text(rules, encoding="utf-8", newline="\n")
            (root / "value.h").write_text("#define VALUE 1\n", encoding="utf-8")
            (root / "main.cpp").write_text('#include "value.h"\nint main() { return VALUE == 1 ? 0 : 1; }\n',
                                            encoding="utf-8")

        with foreground_server(cache, env) as debug_lines:
            fixture(a, False)
            run(["ninja"], a)
            fixture(b, False)
            run(["ninja"], b)
            stats = json.loads(run([cache, "--show-stats", "--stats-format", "json"]))["stats"]
            assert sum(stats["cache_hits"]["counts"].values()) >= 1, "must exercise a cache hit"
            unsafe_deps = run(["ninja", "-t", "deps"], b).replace("\\", "/").lower()
            assert (a / "value.h").as_posix().lower() in unsafe_deps
            (b / "value.h").write_text("#define VALUE 2\n", encoding="utf-8")
            assert "no work to do" in run(["ninja"], b)
            assert subprocess.run([str(b / "smoke.exe")]).returncode == 0, "reproduce stale success"
            # Reuse A's cached object, now through the production mapper.
            for name in ("object.obj", "smoke.exe", ".ninja_deps", ".ninja_log"):
                (b / name).unlink(missing_ok=True)
            fixture(b, True)
            run(["ninja"], b)
            deps = run(["ninja", "-t", "deps"], b).replace("\\", "/").lower()
            assert (b / "value.h").as_posix().lower() in deps
            assert (a / "value.h").as_posix().lower() not in deps
            obj_time = (b / "object.obj").stat().st_mtime_ns
            exe_time = (b / "smoke.exe").stat().st_mtime_ns
            (b / "value.h").write_text("#define VALUE 2\n", encoding="utf-8")
            output = run(["ninja"], b)
            assert "no work to do" not in output
            assert (b / "object.obj").stat().st_mtime_ns > obj_time
            assert (b / "smoke.exe").stat().st_mtime_ns > exe_time
            assert subprocess.run([str(b / "smoke.exe")]).returncode == 1
