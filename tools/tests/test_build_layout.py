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
        assert manifest["test"] == [{"name": "integration", "path": "tests/integration.rs"}]
        included = re.findall(r'#\[path = "([^"]+)"\]',
                              (directory / "tests/integration.rs").read_text())
        existing = {p.name for p in (directory / "tests").glob("*.rs") if p.name != "integration.rs"}
        assert set(included) == existing
        assert len(included) == len(existing)


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
