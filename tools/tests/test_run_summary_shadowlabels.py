"""Recorded comparison lines must survive summary generation and coverage parsing."""
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = Path(__file__).parent / "fixtures"
spec = importlib.util.spec_from_file_location("coverage_shadowlabels", ROOT / "tools/coverage.py")
cov = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cov)
LABELS = ("nettool_plan", "powerline_plan", "dbpf_bounds", "formats")


@unittest.skipUnless(shutil.which("powershell"), "requires Windows PowerShell")
class ShadowLabelTests(unittest.TestCase):
    def summarize(self, *fixtures, extra=""):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "game.log"
            log.write_text("".join((FIXTURES / name).read_text(encoding="utf-8")
                                   for name in fixtures) + extra, encoding="utf-8")
            result = subprocess.run(
                ["powershell", "-NoProfile", "-File", str(ROOT / "tools/run_summary.ps1"),
                 "-Log", str(log)], capture_output=True, text=True, check=True)
            summary = Path(tmp) / "regress-fixture.out"
            summary.write_text("=== regress-fixture-shadows (fixture)\n" + result.stdout,
                               encoding="utf-8")
            return cov.read_regress(summary), result.stdout

    def verified(self, reg, label):
        row = {"pattern": "Fixture", "spec": "-", "core": "-",
               "shadow": label + "_shadow", "replace": "-"}
        cov.map_statuses([row], [], [label + "_shadow"], {}, reg, {}, "fixture")
        return row["verified"]

    def test_all_four_keys_receive_exact_evidence(self):
        reg, _ = self.summarize("shadowlabels-clean.log", "shadowlabels-empty.log")
        for label in LABELS:
            with self.subTest(label=label):
                self.assertEqual(reg["shadows"][label], (1, 1))
                self.assertEqual(reg["resynced"][label], 0)
                self.assertTrue(self.verified(reg, label))

    def test_empty_files_and_aggregate_cannot_supply_comparisons(self):
        # Even an EXACT aggregate must not stand in for per-file comparisons.
        reg, _ = self.summarize("shadowlabels-empty.log", extra=(
            "2026-10-25 [pid 1 tid 2] formats shadow: total: 1 files (0 unreadable), "
            "1 exemplar entries, 1 compared, 1 exact, 0 differ, 0 rejected; EXACT\n"))
        self.assertNotIn("formats", reg["shadows"])
        self.assertFalse(self.verified(reg, "formats"))

    def test_formats_difference_and_rejection_each_void_exact_credit(self):
        reg, output = self.summarize("shadowlabels-clean.log", "shadowlabels-empty.log",
                                     "shadowlabels-differ.log")
        self.assertEqual(reg["shadows"]["formats"], (1, 3))
        self.assertFalse(self.verified(reg, "formats"))
        self.assertIn("differs: formats shadow: differ.dat:", output)
        self.assertIn("differs: formats shadow: rejected.dat:", output)

    def test_new_labels_keep_mismatch_and_reset_evidence(self):
        reg, _ = self.summarize("shadowlabels-clean.log", extra=(
            "2026-10-25 [pid 1 tid 2] nettool plan shadow julian=2461101: mismatches=1\n"
            "2026-10-25 [pid 1 tid 2] powerline plan shadow julian=2461101: mismatches=0; resyncs=1\n"
            "2026-10-25 [pid 1 tid 2] dbpf_bounds shadow 2026-02-01: mismatches=1\n"))
        self.assertEqual(reg["shadows"]["nettool_plan"], (1, 2))
        self.assertEqual(reg["shadows"]["powerline_plan"], (2, 2))
        self.assertEqual(reg["resynced"]["powerline_plan"], 1)
        self.assertEqual(reg["shadows"]["dbpf_bounds"], (1, 2))
        for label in LABELS[:-1]:
            self.assertFalse(self.verified(reg, label))

    def test_existing_label_counts_and_total_override_are_preserved(self):
        reg, _ = self.summarize(extra=(
            "2026-10-25 [pid 1 tid 2] landvalue shadow 2026-01-01: mismatched=391; mismatches=0\n"
            "2026-10-25 [pid 1 tid 2] flora-occupant-shadow: mismatches=0\n"
            "2026-10-25 [pid 1 tid 2] flora-occupant-shadow: mismatches=1\n"))
        self.assertEqual(reg["shadows"], {"landvalue": (1, 1), "flora-occupant": (1, 2)})
