"""Ensure aggregation cannot silently lose a separately compiled Rust suite."""
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / ".claude/lane-bin"))
import qbuild
import rust_cache


def test_every_integration_suite_still_has_a_module():
    for crate in ("core", "formats", "conformance"):
        directory = ROOT / "engine" / crate
        manifest = tomllib.loads((directory / "Cargo.toml").read_text())
        assert manifest["package"]["autotests"] is False
        expected = [{"name": "integration", "path": "tests/integration.rs"}]
        # host_crt owns process-global rand() state and intentionally runs in
        # its own process rather than the aggregated integration executable.
        if crate == "core":
            expected.append({"name": "host_crt", "path": "tests/host_crt.rs"})
        assert manifest["test"] == expected
        separate = {Path(t["path"]).name for t in expected[1:]}
        included = re.findall(r'#\[path = "([^"]+)"\]',
                              (directory / "tests/integration.rs").read_text())
        existing = {p.name for p in (directory / "tests").glob("*.rs")
                    if p.name != "integration.rs" and p.name not in separate}
        assert all((directory / t["path"]).is_file() for t in expected)
        suites = [p for p in included if "/" not in p]
        assert set(suites) == existing
        assert len(suites) == len(existing)
        assert all((directory / "tests" / p).is_file() for p in included)


def test_qbuild_uses_cmake_target_directory():
    command = qbuild.cargo_args("C:/lane", "test", "--workspace")
    assert command[command.index("--target") + 1] == "i686-pc-windows-msvc"
    assert Path(command[command.index("--target-dir") + 1]).as_posix() == "C:/lane/engine/build-x86/cargo"


def test_registry_relocation_is_conservative(tmp_path):
    registry = tmp_path / "registry/src/index/example-1.0"
    registry.mkdir(parents=True)
    args = [str(registry / "lib.rs"), "--out-dir", str(tmp_path / "output")]
    env = {"CARGO_HOME": str(tmp_path), "CARGO_MANIFEST_DIR": str(registry)}
    assert rust_cache.registry_cwd(args, env) == str(registry)
    assert rust_cache.registry_cwd(args, dict(env, OUT_DIR="generated")) is None
    assert rust_cache.registry_cwd(args, dict(env, CARGO_PRIMARY_PACKAGE="1")) is None
    assert rust_cache.registry_cwd(["relative.rs"], env) is None
    assert rust_cache.registry_cwd([*args, "-L", "dependency=relative"], env) is None
    assert rust_cache.registry_cwd(args, dict(env, CARGO_MANIFEST_DIR=str(tmp_path / "local"))) is None
