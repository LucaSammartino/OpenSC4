"""Evidence must distinguish refusals, resynced shadows and incomplete scripts."""
import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
spec = importlib.util.spec_from_file_location("scenario_runs", ROOT / "tools/scenario_runs.py")
runs = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runs)


class ScenarioEvidenceTests(unittest.TestCase):
    def test_discovery_request_does_not_starve_regress_or_keep_dialogs(self):
        old = "label = case\npriority = high\nregion = Berlin\n[ini]\nclose_dialogs = 0\nscript = a.script\n"
        request = runs.discovery_request(old)
        self.assertIn("region = FC4 Test\npriority = normal\n[ini]\nclose_dialogs = 1\n", request)
        self.assertNotIn("high", request)
        self.assertEqual(request.count("close_dialogs ="), 1)
        self.assertEqual(request.count("region ="), 1)
        self.assertEqual(request, runs.discovery_request(request))
        self.assertIn("script = a.script\n", request)
        with self.assertRaises(ValueError):
            runs.discovery_request("label = no-ini\n")

    def test_native_refusals_and_exceptions_are_not_successes(self):
        for text in ("ok=0 error=5 (blocked)", "dry run: ok=0 error=2", "skipped: existing lot",
                     "SetAltitude: 270 -> 270 [structured exception in SetAltitude]", "no effect: disabled",
                     "no 24-hour clock"):
            self.assertFalse(runs.action_ok(text), text)
        for text in ("ok=1 error=0 (none)", "dry run: ok=1 error=0", "marked", "turbo off",
                     "transition notifications on; visual mode 2", "SetAltitude: 0 -> 5000"):
            self.assertTrue(runs.action_ok(text), text)

    def test_shadow_total_takes_priority_over_diagnostic_counts(self):
        self.assertFalse(runs.shadow_differs("live map total mismatched=391; mismatches=0"))
        self.assertTrue(runs.shadow_differs("mismatched=0; mismatches=2"))
        self.assertFalse(runs.shadow_differs("] formats shadow: file: 10 exemplar entries, 10 compared, 10 exact, 0 differ, 0 rejected"))
        self.assertTrue(runs.shadow_differs("] formats shadow: file: 10 exemplar entries, 10 compared, 9 exact, 1 differ, 0 rejected"))

    def test_actual_log_shapes_and_script_line_attribution(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "actions.script").write_text("# seed 7\n2026-02-01 zone 0 0 1 1 3\n", encoding="utf-8")
            log = root / "game.log"
            log.write_text(
                "2026-10-25 [pid 1 tid 2] script: line 2 2026-02-01 zone (run on 2026-02-02) -> ok=0 error=5\n"
                "2026-10-25 [pid 1 tid 2] script: line 3 2026-02-01 save: waits for the next framework tick\n"
                "2026-10-25 [pid 1 tid 2] aura shadow month 2026-02-01: mismatches=0\n"
                "2026-10-25 [pid 1 tid 2] aura shadow month 2026-03-01: mismatches=0; resyncs=2\n"
                "2026-10-25 [pid 1 tid 2] nettool plan shadow julian=2461101: mismatches=1\n"
                "2026-10-25 [pid 1 tid 2] flora-static-shadow: mismatches=0\n"
                "2026-10-25 [pid 1 tid 2] formats shadow: total: 10 compared, 10 exact, 0 differ\n"
                "2026-10-25 [pid 1 tid 2] formats shadow: file.dat: 0 exemplar entries, 0 compared, 0 exact, 0 differ, 0 rejected\n"
                "2026-10-25 [pid 1 tid 2] formats shadow: file.dat: 4 exemplar entries, 4 compared, 4 exact, 0 differ, 0 rejected\n"
                "2026-10-25 [pid 1 tid 2] heartbeat (periodic): 2026-04-01\n"
                "2026-10-25 [pid 1 tid 2] fail-fast reports on: a Rust panic, a CRT assert is logged\n"
                "2026-10-25 [pid 1 tid 2] counters: +0 foreign exceptions\n"
                "2026-10-25 [pid 1 tid 2] AUTOTEST VERDICT: clean (date=2028-01-01, days_per_second=17.25)\n"
                "2026-10-25 [pid 1 tid 2] script: all 2 action(s) of 'actions.script' have run\n", encoding="utf-8")
            run = {"scenario": "test", "seed": 7, "mix": "edge", "city": "fixture", "script": "actions.script"}
            with patch.object(runs, "ROOT", root):
                acts, shadows, findings, dates, dps, complete = runs.analyze(log, run)
            self.assertEqual(acts["zone"], {"run": 1, "failed": 1})
            self.assertNotIn("save", acts)  # waiting is not a completed action
            self.assertEqual(shadows["aura"]["exact"], 1)
            self.assertEqual(shadows["aura"]["resynced"], 1)
            self.assertEqual(sum(c["lines"] for c in shadows.values()), 5)
            self.assertEqual(shadows["formats"]["lines"], 1)
            self.assertEqual(findings[0]["key"], "nettool_plan")
            self.assertEqual(len(findings), 1)  # startup text and zero counters are not exceptions
            self.assertEqual(findings[0]["log_line"], 5)
            self.assertEqual(findings[0]["preceding_script_line"], 2)
            self.assertTrue(findings[0]["preceding_script_text"].endswith("zone 0 0 1 1 3"))
            self.assertEqual(dates, ["2026-04-01", "2028-01-01"])
            self.assertEqual(dps, "17.25")
            self.assertTrue(complete)

    def test_profile_uses_regress_combined_settings(self):
        profile = runs.shadow_profile()
        self.assertIn("disable = budget,ordinance", profile)
        self.assertIn("nettool_plan_shadow = 1", profile)
        self.assertIn("powerline_plan_shadow = 1", profile)
        self.assertNotIn("`n", profile)


if __name__ == "__main__":
    unittest.main()
