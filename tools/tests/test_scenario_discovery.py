"""Monthly novelty needs execution dates, all mismatches and bounded action windows."""
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import scenario_discovery as discovery


class DiscoveryTests(unittest.TestCase):
    def test_freeze_only_completed_findings_and_leave_suite_alone(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            out = root / "build-runs/suite/test"
            out.mkdir(parents=True)
            results = root / "worker/build-runs/game-results"
            results.mkdir(parents=True)
            (root / "input.script").write_text("2029-08-21 mark test\n", encoding="utf-8")
            run = dict(scenario="test", root="worker", script="input.script", city="New City", seed=2,
                       mix="edge", end="2031-08-21")
            manifest = dict(runs=[run], months=24, region="FC4 Test")
            issues = [dict(scenario="test", key="dbpf", log_line=27)]
            with patch.object(discovery, "ROOT", root):
                self.assertEqual(discovery.freeze_candidates(out, manifest, issues), [])
                (results / "test.verdict").write_text("verdict: clean\n", encoding="utf-8")
                proposals = discovery.freeze_candidates(out, manifest, issues)
            self.assertEqual(len(proposals), 1)
            self.assertEqual(proposals[0]["name"], "gen27-new-city-s2-edge")
            self.assertEqual(proposals[0]["sets"], "gen")
            self.assertEqual((out / "freeze/gen27-new-city-s2-edge.script").read_bytes(),
                             (root / "input.script").read_bytes())
            self.assertFalse((root / "engine").exists())

    def test_months_stale_dates_windows_and_union(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "case.script").write_text(
                "2029-08-23 terrain 0 0 4 4 270 0\n2029-09-02 save\n", encoding="utf-8")
            log = root / "case.log"
            log.write_text("\n".join([
                "2026-10-25 [pid 1] nettool plan shadow -4713-11-24: mismatches=0",
                "2026-10-25 [pid 1] script: line 1 2029-08-23 terrain (run on 2029-08-23) -> SetAltitude: 0 -> 270",
                "2026-10-25 [pid 1] plumbing shadow 2029-08-25: pipes orig/ours=33/0!; mismatches=1",
                "2026-10-25 [pid 1] script: line 2 2029-09-02 save (run on 2029-09-02) -> saved",
                "2026-10-25 [pid 1] plumbing shadow 2029-09-02: mismatches=0",
                "2026-10-25 [pid 1] dbpf shadow 2029-08-20: refusals=8 mismatches=8",
                "2026-10-25 [pid 1] dbpf shadow 2029-09-01: refusals=2 mismatches=2",
                "2026-10-25 [pid 1] plumbing shadow 2029-09-04: pipes orig/ours=34/0!; mismatches=1",
                "2026-10-25 [pid 1] aura shadow 2029-09-04: mismatches=0; resyncs=2",
            ]) + "\n", encoding="utf-8")
            run = dict(scenario="case", seed=0, mix="default", city="Tegel", script="case.script",
                       start="2029-08-21", end="2029-09-21")
            union = {"plumbing"}
            with patch.object(discovery, "ROOT", root):
                months, firsts, issues = discovery.inspect(log, run, union)
                again, _, _ = discovery.inspect(log, run, union)
            self.assertEqual([r["month"] for r in months], ["2029-08", "2029-09"])
            self.assertEqual([r["shadow_lines"] for r in months], [2, 5])
            self.assertEqual([r["shadow_exact_lines"] for r in months], [1, 1])
            self.assertEqual(months[0]["new_shadow_keys"], "nettool_plan,plumbing")
            self.assertEqual(months[0]["new_suite_keys"], "nettool_plan")
            self.assertTrue(all(r["new_suite_keys"] == "-" for r in again))
            self.assertEqual(len(firsts), 4)
            self.assertEqual(len(issues), 5)  # both DBPF lines and both pipe lines survive
            self.assertEqual(issues[0]["script_lines"], "1")
            self.assertEqual(issues[1]["sample_date"], "2029-08-20")
            self.assertEqual(issues[1]["month"], "2029-09")
            self.assertEqual(issues[2]["script_lines"], "-")
            self.assertEqual(issues[3]["script_lines"], "-")
            self.assertEqual(issues[4]["kind"], "resynced")
            self.assertTrue(all(r["reach_functions"] == "-" for r in months))

    def test_decimal_and_opaque_byte_differences(self):
        fields = discovery.differences(
            "meanAlt orig/ours=284.31512/282.47906!; pipes orig/ours=33/0!; average orig/ours=71/71;")
        self.assertIn("delta(ours-orig)=-1.83606", fields)
        self.assertIn("delta(ours-orig)=-33", fields)
        self.assertNotIn("average", fields)
        fields = discovery.differences(
            "record statistics (working): 36 of 544 bytes differ, first at +382: orig AE47813F ours 00000000")
        self.assertIn("36/544 bytes differ; +382 orig=AE47813F ours=00000000", fields)


if __name__ == "__main__":
    unittest.main()
