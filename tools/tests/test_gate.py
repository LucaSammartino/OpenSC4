"""Regression tests for the merge-train gate (tools/merge-train.py) and its parsers (.claude/lane-bin/fc4verify.py).

    python -m unittest discover -s tools/tests -v

The train10 case (merge e99f0ff1, 2026-10-20): the gate failed in `cargo test` (fixtures/gate-train10.out) and stopped
before the CMake build, the state file's gate entry was then set to `ok: true` by hand, the regress passed on a
FreeCity4.dll left over from an earlier build, and `land` trusted the flag. fc4.h and FC4Subsystems.cpp on main did
not compile until 3975a42e.
"""

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
FIX = HERE / "fixtures"
sys.path.insert(0, str(REPO / ".claude" / "lane-bin"))
import fc4verify  # noqa: E402

_spec = importlib.util.spec_from_file_location("merge_train", REPO / "tools" / "merge-train.py")
mt = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mt)

HEAD = "e99f0ff1e0ffa87e342ca80adaa484c6c2be3481"
OTHER = "a23223be3d4e0021de78fdc6aaef8160543fe5d9"
# train10's history entry in build-runs/merge-train/state.json, as it was landed
TRAIN10_STATE = {"id": "train10", "lanes": ["glue19"], "head": HEAD,
                 "gate": {"ok": True, "note": "cargo test + clippy pass (run 10); conformance via coord-verify-lane"},
                 "regress": {"ok": True, "verdict": "PASS"}}


def fixture(name):
    return (FIX / name).read_text(encoding="utf-8")


class Records:
    """Builds gate records whose step logs are real files, the way fc4verify writes them."""

    def __init__(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)

    def close(self):
        self.tmp.cleanup()

    def step(self, name, text, **extra):
        log = self.dir / f"{name}.log"
        log.write_text(text, encoding="utf-8")
        parsed = {"test": fc4verify.parse_cargo_test, "clippy": fc4verify.parse_clippy,
                  "build": fc4verify.parse_ninja}.get(name)
        res = parsed(text) if parsed else fc4verify.parse_ctest(text, extra.get("expected"))
        return {**res, **extra, "log": str(log), "seconds": 1.0}

    def record(self, head=HEAD, build=None, ctest=None, clippy=None, **extra):
        steps = {
            "clippy": self.step("clippy", clippy if clippy is not None else fixture("clippy-clean.log")),
            "build": self.step("build", build if build is not None else fixture("build-ok.log")),
            "ctest": self.step("ctest", ctest if ctest is not None else fixture("ctest-ok.log"),
                               expected=fc4verify.parse_ctest_total(fixture("ctest-list.log"))),
        }
        rec = {"tool": "fc4verify", "version": 1, "head": head, "tree": "t", "dirty": [], "steps": steps}
        rec.update(extra)
        return rec


class ParserTests(unittest.TestCase):
    def test_train10_gate_output_is_a_cargo_test_failure(self):
        r = fc4verify.parse_cargo_test(fixture("gate-train10.out"))
        self.assertFalse(r["ok"])
        self.assertEqual(r["failed"], 1)
        self.assertIn("sweep_automata_records", r["failed_tests"])

    def test_clean_cargo_test(self):
        text = ("running 3 tests\ntest a ... ok\n\ntest result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; "
                "0 filtered out; finished in 0.01s\n\n   Doc-tests fc4_core\n\ntest result: ok. 0 passed; 0 failed; "
                "1 ignored; 0 measured; 0 filtered out; finished in 0.00s\n")
        r = fc4verify.parse_cargo_test(text)
        self.assertTrue(r["ok"], r)
        self.assertEqual((r["passed"], r["ignored"], r["results"]), (3, 1, 2))

    def test_cargo_compile_error_with_no_results_fails(self):
        text = "error[E0425]: cannot find value `x` in this scope\nerror: could not compile `fc4-core`\n"
        self.assertFalse(fc4verify.parse_cargo_test(text)["ok"])

    def test_real_build_passes(self):
        r = fc4verify.parse_ninja(fixture("build-ok.log"))
        self.assertTrue(r["ok"], r)

    def test_train10_cxx_break_fails_the_build(self):
        # the C2078 train10 put in FC4Subsystems.cpp, rebuilt for real (fixtures/README.md)
        r = fc4verify.parse_ninja(fixture("build-c2078.log"))
        self.assertFalse(r["ok"])
        self.assertTrue(any("C2078" in e for e in r["errors"]), r["errors"])

    def test_build_cut_short_fails(self):
        self.assertFalse(fc4verify.parse_ninja("[1/174] Building CXX object a.obj\n[2/174] Building CXX object b.obj\n")["ok"])
        self.assertFalse(fc4verify.parse_ninja("")["ok"])

    def test_no_work_to_do_passes(self):
        self.assertTrue(fc4verify.parse_ninja("ninja: no work to do.\n")["ok"])

    def test_ctest_ok_and_count(self):
        expected = fc4verify.parse_ctest_total(fixture("ctest-list.log"))
        self.assertTrue(expected and expected > 1)
        self.assertTrue(fc4verify.parse_ctest(fixture("ctest-ok.log"), expected)["ok"])
        # the same summary with a test missing from the run is not a pass
        self.assertFalse(fc4verify.parse_ctest(fixture("ctest-ok.log"), expected + 1)["ok"])

    def test_ctest_not_run_fails(self):
        text = ("1/2 Test #1: cargo_test ...***Not Run   0.00 sec\n"
                "50% tests passed, 1 tests failed out of 2\n")
        self.assertFalse(fc4verify.parse_ctest(text, 2)["ok"])

    def test_clippy(self):
        self.assertTrue(fc4verify.parse_clippy(fixture("clippy-clean.log"))["ok"])
        self.assertFalse(fc4verify.parse_clippy("error: unused variable: `x`\n  --> core/src/a.rs:1:5\n"
                                                "error: could not compile `fc4-core`\n")["ok"])


class GateTests(unittest.TestCase):
    def setUp(self):
        self.r = Records()

    def tearDown(self):
        self.r.close()

    def test_train10_hand_set_ok_does_not_land(self):
        """The case that slipped through: the state file says the gate passed, there is no passing record."""
        why = mt.land_blockers(TRAIN10_STATE, HEAD, True, None, [])
        self.assertTrue(why)
        self.assertIn("no gate record", why[0])

    def test_train10_build_break_with_a_stale_ctest_pass_does_not_land(self):
        """A red build over a tree that still has the previous build's binaries: ctest passes on the old ones."""
        rec = self.r.record(build=fixture("build-c2078.log"), ctest=fixture("ctest-ok.log"))
        why = mt.land_blockers(TRAIN10_STATE, HEAD, True, rec, [])
        self.assertTrue(why)
        self.assertIn("build:", why[0])

    def test_train10_cargo_failure_inside_ctest_does_not_land(self):
        rec = self.r.record(ctest=fixture("ctest-cargo-fail.log"))
        ok, why = mt.judge_gate(rec, HEAD)
        self.assertFalse(ok)
        self.assertTrue(any(w.startswith("ctest:") for w in why), why)

    def test_record_edited_by_hand_is_judged_from_its_logs(self):
        rec = self.r.record(build=fixture("build-c2078.log"))
        rec["steps"]["build"]["ok"] = True
        rec["steps"]["build"]["summary"] = "174/174 edges"
        rec["ok"] = True
        ok, why = mt.judge_gate(rec, HEAD)
        self.assertFalse(ok)
        self.assertTrue(any(w.startswith("build:") for w in why), why)

    def test_record_whose_log_is_gone_fails(self):
        rec = self.r.record()
        Path(rec["steps"]["ctest"]["log"]).unlink()
        self.assertFalse(mt.judge_gate(rec, HEAD)[0])

    def test_record_for_another_head_does_not_land(self):
        rec = self.r.record(head=OTHER)
        why = mt.land_blockers(TRAIN10_STATE, HEAD, True, rec, [])
        self.assertTrue(why and "integration branch is at" in why[0], why)

    def test_record_of_a_dirty_tree_does_not_land(self):
        rec = self.r.record(dirty=["?? engine/plugin/FC4New.cpp"])
        self.assertFalse(mt.judge_gate(rec, HEAD)[0])

    def test_record_without_the_ctest_step_does_not_land(self):
        rec = self.r.record()
        del rec["steps"]["ctest"]
        self.assertFalse(mt.judge_gate(rec, HEAD)[0])

    def test_wrapper_style_output_is_not_a_record(self):
        rec = {"head": HEAD, "ok": True, "steps": {}}
        self.assertFalse(mt.judge_gate(rec, HEAD)[0])

    def test_main_moved_under_the_batch_does_not_land(self):
        why = mt.land_blockers(TRAIN10_STATE, HEAD, True, self.r.record(), ["engine/plugin/FC4Subsystems.cpp"])
        self.assertTrue(why and "main changed build files" in why[0], why)

    def test_passing_record_lands(self):
        rec = self.r.record()
        self.assertEqual(mt.judge_gate(rec, HEAD), (True, []))
        self.assertEqual(mt.land_blockers(TRAIN10_STATE, HEAD, True, rec, []), [])

    def test_docs_only_batch_needs_no_record(self):
        self.assertEqual(mt.land_blockers({"id": "t", "gate": {}}, HEAD, False, None, []), [])

    def test_record_round_trips_through_json(self):
        rec = self.r.record()
        self.assertTrue(mt.judge_gate(json.loads(json.dumps(rec)), HEAD)[0])


class CrateTests(unittest.TestCase):
    def test_touched_and_dependents(self):
        graph = fc4verify.crate_graph(REPO / "engine")
        self.assertEqual(graph["core"][0], "fc4-core")
        touched, deps = fc4verify.touched_crates(None, ["engine/formats/src/dbpf.rs", "docs/x.md"], graph)
        self.assertEqual(touched, ["formats"])
        self.assertIn("capi", deps)
        self.assertIn("conformance", deps)
        touched, deps = fc4verify.touched_crates(None, ["engine/capi/src/lib.rs"], graph)
        self.assertEqual((touched, deps), (["capi"], []))
        touched, _ = fc4verify.touched_crates(None, ["engine/Cargo.lock"], graph)
        self.assertEqual(sorted(touched), sorted(graph))
        self.assertEqual(fc4verify.touched_crates(None, ["engine/plugin/FC4Log.cpp"], graph), ([], []))


if __name__ == "__main__":
    unittest.main()
