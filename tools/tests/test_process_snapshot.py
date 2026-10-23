"""Behavioral tests using a deliberately fallible psutil-shaped provider."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("process_snapshot", ROOT / ".claude/lane-bin/process_snapshot.py")
mod = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(mod)


class Denied(Exception):
    pass


class Gone(Exception):
    pass


class Zombie(Gone):
    pass


class Process:
    def __init__(self, pid=10, **overrides):
        self.pid = pid
        self.values = dict(name="python.exe", username="caller", cwd=os.path.abspath("foreign"),
                           exe=os.path.abspath("python.exe"), cmdline=[os.path.abspath("python.exe")],
                           create_time=100.0, ppid=1, open_files=[], is_running=True)
        self.values.update(overrides)

    def __getattr__(self, key):
        def read():
            value = self.values[key]
            if isinstance(value, Exception):
                raise value
            if callable(value):
                return value()
            return value
        return read


class Provider:
    AccessDenied, NoSuchProcess, ZombieProcess = Denied, Gone, Zombie
    __version__ = "test"

    def __init__(self, *processes):
        self.processes = processes

    def Process(self):
        return Process(pid=999)

    def process_iter(self):
        return iter(self.processes)


class SnapshotTests(unittest.TestCase):
    def setUp(self):
        (ROOT / "build-runs").mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=ROOT / "build-runs")
        self.addCleanup(self.temp.cleanup)
        self.target = mod.normalized(self.temp.name)

    def capture(self, *processes, identities=None):
        return mod.snapshot(self.target, identities or [], Provider(*processes))

    def test_attributable_target_and_parentage(self):
        result = self.capture(Process(cwd=self.target), Process(pid=11, cwd=self.target, ppid=10,
                                                              create_time=101.0))
        self.assertEqual(result["status"], "OBSERVED")
        parent = result["target_related"][0]
        self.assertEqual(parent["children"][0]["pid"], 11)
        self.assertEqual(parent["cwd"], self.target)
        self.assertIn("psutil.cwd within target", parent["evidence"])
        self.assertEqual(mod.datetime.fromisoformat(result["capture_local"]).tzinfo is not None, True)
        self.assertFalse(result["authoritative"])

    def test_no_target_is_observation_and_foreign_commands_private(self):
        result = self.capture(Process())
        self.assertEqual(result["status"], "NO_TARGET_OBSERVED")
        self.assertEqual(result["counts"], dict(target_related=0, foreign=1, unknown=0))
        self.assertNotIn("secret", json.dumps(result))
        self.assertNotIn("cwd", result["foreign"][0])

    def test_target_argv_reference_and_redaction(self):
        operand = os.path.join(self.target, "tools", "check.py")
        result = self.capture(Process(cmdline=["python", operand, "--token", "sensitive"]))
        row = result["target_related"][0]
        self.assertEqual(row["command"]["target_path_operands"], [operand])
        self.assertNotIn("sensitive", json.dumps(result))
        self.assertEqual(row["argv_sha256"], mod.argv_digest(["python", operand, "--token", "sensitive"]))

    def test_output_handle_reference_is_not_writer_ownership(self):
        handle = SimpleNamespace(path=os.path.join(self.target, "build-runs", "check.log"), fd=2)
        row = self.capture(Process(open_files=[handle]))["target_related"][0]
        self.assertIn("not prove writer", row["open_output_handles"][0]["output_intent"])
        self.assertEqual(row["open_output_handles"][0]["mode"], "UNKNOWN")

    def test_foreign_output_and_prefix_sibling_not_attributed(self):
        handle = SimpleNamespace(path=self.target + "-other/build-runs/check.log", fd=2)
        result = self.capture(Process(cwd=self.target + "-other", open_files=[handle]))
        self.assertEqual(result["counts"]["target_related"], 0)
        self.assertEqual(result["counts"]["foreign"], 1)

    def test_relative_or_embedded_argv_is_unknown_not_foreign(self):
        for argv in (["python", "relative.py"], ["/bin/sh", "-c", "embedded command"],
                     [os.path.abspath("python.exe"), "--option=" + self.target]):
            result = self.capture(Process(cmdline=argv))
            self.assertEqual(result["foreign"], [])
            self.assertEqual(result["status"], "UNKNOWN")
            self.assertTrue(result["unknown"][0]["opaque_argv"])

    def test_target_handle_outside_build_runs_retained(self):
        handle = SimpleNamespace(path=os.path.join(self.target, "engine", "build", "result.obj"))
        row = self.capture(Process(open_files=[handle]))["target_related"][0]
        self.assertEqual(row["open_output_handles"][0]["path"], handle.path)

    def test_denied_handles_keeps_positive_evidence_but_unknown_status(self):
        result = self.capture(Process(cwd=self.target, open_files=Denied("secret")))
        self.assertEqual(result["status"], "UNKNOWN")
        self.assertEqual(len(result["target_related"]), 1)
        self.assertIn("open_files", result["target_related"][0]["fields_unknown"])
        self.assertNotIn("secret", json.dumps(result))

    def test_denied_candidate_does_not_become_foreign(self):
        result = self.capture(Process(cwd=Denied(), cmdline=Denied()))
        self.assertEqual(result["counts"]["unknown"], 1)
        self.assertEqual(result["foreign"], [])

    def test_empty_scope_fields_cannot_prove_foreign(self):
        result = self.capture(Process(cwd="", exe=""))
        self.assertEqual(result["foreign"], [])
        self.assertEqual(result["status"], "UNKNOWN")

    def test_unreadable_custom_scope_paths_are_unknown(self):
        result = self.capture(Process(name="custom", cwd=Denied()))
        self.assertEqual(result["unknown"][0]["reason"], "scope_paths_unreadable")

    def test_child_creation_before_parent_not_linked(self):
        result = self.capture(Process(cwd=self.target), Process(pid=11, cwd=self.target, ppid=10,
                                                              create_time=50.0))
        self.assertEqual(result["target_related"][0]["children"], [])

    def test_unreadable_owner_custom_name_still_unknown(self):
        result = self.capture(Process(name="custom", username=Denied()))
        self.assertEqual(result["unknown"][0]["reason"], "owner_unreadable")
        self.assertEqual(result["status"], "UNKNOWN")

    def test_other_owner_not_inspected(self):
        def forbidden():
            self.fail("foreign user's details must not be read")
        result = self.capture(Process(username="someone_else", cwd=forbidden, cmdline=forbidden))
        self.assertEqual(result["target_related"], [])

    def test_pid_reuse_does_not_match_old_identity(self):
        result = self.capture(Process(cwd=self.target, create_time=200.0),
                              identities=[dict(pid=10, create_time=100.0)])
        self.assertEqual(result["recorded"][0]["state"], "stale_pid")

    def test_recorded_argv_and_creation_both_checked(self):
        identity = dict(pid=10, create_time=100.0, argv_sha256=mod.argv_digest(["old"]))
        self.assertEqual(self.capture(Process(), identities=[identity])["recorded"][0]["state"],
                         "argv_mismatch")
        identity["argv_sha256"] = mod.argv_digest([os.path.abspath("python.exe")])
        self.assertEqual(self.capture(Process(), identities=[identity])["recorded"][0]["state"], "match")

    def test_partial_read_race_cannot_attribute_or_match(self):
        times = iter([100.0, 200.0])
        result = self.capture(Process(cwd=self.target, create_time=lambda: next(times)),
                              identities=[dict(pid=10, create_time=100.0)])
        self.assertEqual(result["target_related"], [])
        self.assertEqual(result["unknown"][0]["stable_identity"], False)
        self.assertEqual(result["recorded"][0]["state"], "UNKNOWN")

    def test_exit_during_read_retained(self):
        result = self.capture(Process(cwd=self.target, create_time=Gone(), is_running=False))
        self.assertEqual(result["status"], "UNKNOWN")
        self.assertTrue(any(d["kind"] == "Gone" for d in result["diagnostics"]))
        self.assertEqual(result["target_related"], [])

    def test_empty_and_failed_inventory_are_unknown(self):
        self.assertEqual(self.capture()["status"], "UNKNOWN")
        class Broken(Provider):
            def process_iter(self):
                yield Process(cwd=self.target)  # deliberately replaced below
                raise Denied()
        provider = Broken()
        provider.target = self.target
        result = mod.snapshot(self.target, [], provider)
        self.assertEqual(result["status"], "UNKNOWN")
        self.assertTrue(any(d["field"] == "inventory" for d in result["diagnostics"]))

    def test_diagnostics_bounded(self):
        result = self.capture(*(Process(pid=i, cwd=Denied()) for i in range(1, 120)))
        self.assertEqual(len(result["diagnostics"]), 100)
        self.assertGreater(result["diagnostics_omitted"], 0)

    def test_inventory_limit_is_explicit_unknown(self):
        with patch.object(mod, "MAX_PROCESSES", 1):
            result = self.capture(Process(), Process(pid=11))
        self.assertEqual(result["status"], "UNKNOWN")
        self.assertTrue(any(d["kind"] == "process_limit" for d in result["diagnostics"]))

    def test_missing_recorded_pid_never_safe_absence(self):
        result = self.capture(Process(), identities=[dict(pid=99, create_time=20.0)])
        self.assertEqual(result["recorded"][0]["state"], "not_observed")
        self.assertIn("not proof of absence", result["recorded"][0]["reason"])

    def test_malformed_identities(self):
        path = Path(self.temp.name) / "identities.json"
        for value in ({}, [dict(pid=10)], [dict(pid=True, create_time=1)],
                      [dict(pid=10, create_time=float("nan"))],
                      [dict(pid=10, create_time=1, argv_sha256="bad")]):
            path.write_text(json.dumps(value), encoding="utf-8")
            with self.assertRaises(ValueError):
                mod.load_identities(path)
        path.write_text('[{"pid":10,"create_time":100.0}]', encoding="utf-8")
        self.assertEqual(mod.load_identities(path)[0]["pid"], 10)

    def run_main(self, **kwargs):
        # Route output under this test's temporary worktree-shaped directory.
        fake_file = Path(self.temp.name) / ".claude/lane-bin/process_snapshot.py"
        with patch.object(mod, "__file__", str(fake_file)), contextlib.redirect_stdout(io.StringIO()) as buf:
            code = mod.main(["--target", self.target], **kwargs)
        files = list((Path(self.temp.name) / "build-runs/process-snapshots").glob("*.json"))
        return code, buf.getvalue(), json.loads(files[-1].read_text(encoding="utf-8"))

    def test_cli_writes_snapshot_and_non_authority_status(self):
        code, printed, data = self.run_main(provider=Provider(Process(cwd=self.target)))
        self.assertEqual(code, 3)
        self.assertIn("OBSERVED", printed)
        self.assertEqual(data["target"], self.target)

    def test_platform_diagnostic(self):
        with patch.object(mod.sys, "platform", "unsupported"):
            code, _, data = self.run_main()
        self.assertEqual(code, 3)
        self.assertIn("unsupported platform", data["error"])

    def test_missing_dependency_diagnostic(self):
        with patch.dict("sys.modules", {"psutil": None}):
            code, _, data = self.run_main()
        self.assertEqual(code, 3)
        self.assertIn("psutil unavailable for this Python interpreter", data["error"])

    def test_provider_failure_diagnostic(self):
        class Broken(Provider):
            def process_iter(self):
                raise RuntimeError("do not disclose")
        code, _, data = self.run_main(provider=Broken())
        self.assertEqual(code, 3)
        self.assertEqual(data["error"], "provider failed: RuntimeError")

    def test_cli_bad_identity_input(self):
        path = Path(self.temp.name) / "bad.json"
        path.write_text("broken json", encoding="utf-8")
        with contextlib.redirect_stdout(io.StringIO()) as buf:
            code = mod.main(["--target", self.target, "--identities", str(path)])
        self.assertEqual(code, 2)
        self.assertIn("ERROR input: JSONDecodeError", buf.getvalue())

    def test_bad_target_diagnostic(self):
        with contextlib.redirect_stdout(io.StringIO()) as buf:
            code = mod.main(["--target", self.target + "/absent"])
        self.assertEqual(code, 2)
        self.assertIn("ERROR input", buf.getvalue())


if __name__ == "__main__":
    unittest.main()
