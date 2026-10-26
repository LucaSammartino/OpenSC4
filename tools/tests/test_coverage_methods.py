"""Method credit binds complete, exact house evidence to original function starts."""
import csv
import importlib.util
from pathlib import Path
import shutil
import subprocess
import tempfile
from types import SimpleNamespace
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("coverage_method_tests", ROOT / "tools/coverage.py")
cov = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cov)
methods = cov.coverage_methods


def line(address="0x0094C431", name="Owner::Method", calls=3, complete=3, mismatches=0, resyncs=0):
    return (f"2026-10-26 [pid 1 tid 2] fixture shadow method {address} {name} (month) 2026-04-01: "
            f"calls={calls} complete={complete} mismatches={mismatches}; resyncs={resyncs}")


class MethodTests(unittest.TestCase):
    def setUp(self):
        self.va = 0x94C431
        self.att = SimpleNamespace(funcs={self.va: 75}, name=lambda va: "Owner::Method")
        self.owners = {self.va: "Owner"}
        self.game = [(self.va, 75, 0, "framework")]
        self.reg = {"shadows": {"fixture": (2, 2)}, "resynced": {"fixture": 0}, "methods": {}}

    def credit(self, lines=(), rows=()):
        self.reg["methods"] = methods.read_lines(lines)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "methods.tsv"
            if rows:
                with path.open("w", encoding="utf-8", newline="") as stream:
                    writer = csv.DictWriter(stream, delimiter="\t", fieldnames=rows[0].keys())
                    writer.writeheader()
                    writer.writerows(rows)
            return methods.credit([path] if rows else [], self.att, self.owners, self.game, self.reg)

    def row(self, **changes):
        return dict(address="0094c431", function_name="Owner::Method", owner="Owner", bytes="75",
                    shadow_key="fixture_shadow", evidence_family="fixture", **changes)

    def test_positive_complete_and_exact_is_credited(self):
        self.assertEqual(self.credit([line()])[0], {self.va})

    def test_every_line_must_be_exact_and_counters_complete(self):
        for bad in (line(calls=0, complete=0), line(complete=0), line(complete=4),
                    line(mismatches=1), line(resyncs=1), line().replace("; resyncs=0", "")):
            with self.subTest(bad=bad):
                self.assertFalse(self.credit([bad])[0])
        for earlier in (line(mismatches=1), line(resyncs=1), line().replace("calls=3", "calls=bad")):
            self.assertFalse(self.credit([earlier, line(calls=6, complete=6)])[0])

    def test_last_counters_are_not_summed_or_maximised(self):
        good = [line(calls=2, complete=1), line(calls=6, complete=6)]
        record = next(iter(methods.read_lines(good).values()))
        self.assertEqual((record["calls"], record["complete"]), (6, 6))
        self.assertEqual(self.credit(good)[0], {self.va})
        self.assertFalse(self.credit([line(calls=6, complete=6), line()])[0])
        self.assertFalse(self.credit([line(), line(calls=6, complete=4)])[0])

    def test_unknown_interior_address_and_slot_are_rejected_with_warning(self):
        for address in ("0x0094C432", "0x12345678", "-"):
            credited, _, warnings = self.credit([line(address=address)])
            self.assertFalse(credited)
            self.assertTrue(warnings)

    def test_name_collision_voids_the_address(self):
        credited, _, warnings = self.credit([line(), line(name="Wrong::Method")])
        self.assertFalse(credited)
        self.assertIn("collision", " ".join(warnings))

    def test_tsv_family_requires_nonempty_all_exact_and_explicit_zero_resync(self):
        self.assertEqual(self.credit(rows=[self.row()])[0], {self.va})
        for counts, resets in (((0, 0), 0), ((1, 2), 0), ((2, 2), 1), ((2, 2), None)):
            self.reg["shadows"]["fixture"] = counts
            self.reg["resynced"]["fixture"] = resets
            self.assertFalse(self.credit(rows=[self.row()])[0])

    def test_tsv_size_owner_name_and_duplicate_collisions_fail_closed(self):
        for field, value in (("bytes", "74"), ("owner", "Wrong"), ("function_name", "Wrong::Method")):
            row = self.row(); row[field] = value
            credited, _, warnings = self.credit([line()], [row])
            self.assertFalse(credited)
            self.assertIn("collision", " ".join(warnings))
        self.assertFalse(self.credit(rows=[self.row(), self.row()])[0])
        self.assertFalse(self.credit([line(mismatches=1)], [self.row()])[0])
        bad_size = self.row(); bad_size["bytes"] = "unknown"
        self.assertFalse(self.credit([line()], [bad_size])[0])

    def test_two_addresses_with_one_name_are_distinct_and_never_double_counted(self):
        second = self.va + 100
        self.att.funcs[second] = 10
        self.owners[second] = "Owner"
        self.game.append((second, 10, 0, "framework"))
        self.assertEqual(self.credit([line(), line(address=f"0x{second:08X}")], [self.row()])[0],
                         {self.va, second})

    def test_library_bytes_are_never_added_and_shared_helpers_can_be(self):
        self.game = []
        credited, _, warnings = self.credit([line()])
        self.assertFalse(credited)
        self.assertIn("library", " ".join(warnings))
        self.game = [(self.va, 75, None, "shared")]
        self.owners[self.va] = "<shared>"
        self.assertEqual(self.credit([line()])[0], {self.va})

    def test_owner_verification_subsumes_method_credit(self):
        rows = [{"verified": True, "implemented": False}]
        self.assertFalse(methods.incremental(self.game, rows, {self.va}))
        rows[0]["verified"] = False
        self.assertEqual(methods.incremental(self.game, rows, {self.va}), [(self.va, 75)])
        self.assertTrue(methods.function_status(rows, 0, self.va, {self.va}, "verified"))
        self.assertFalse(methods.function_status(rows, 0, self.va, {self.va}, "implemented"))
        self.assertTrue(methods.function_status(rows, None, self.va, {self.va}, "verified"))

    def test_frozen_proposal_has_only_44_functions_and_4070_bytes(self):
        path = ROOT / "docs/specs/attribution/proposed-shadow-methods.tsv"
        with path.open(encoding="utf-8") as stream:
            rows = [r for r in csv.DictReader(stream, delimiter="\t") if r["address"] != "TOTAL"]
        identities = {int(r["address"], 16): r for r in rows}
        att = SimpleNamespace(funcs={va: int(r["bytes"]) for va, r in identities.items()},
                              name=lambda va: identities[va]["function_name"])
        owners = {va: r["owner"] for va, r in identities.items()}
        game = [(va, size, None, "shared") for va, size in att.funcs.items()]
        reg = {"shadows": {r["evidence_family"]: (72, 72) for r in rows},
               "resynced": {r["evidence_family"]: 0 for r in rows}}
        credited, _, warnings = methods.credit([path], att, owners, game, reg)
        self.assertEqual((len(credited), sum(att.funcs[va] for va in credited)), (44, 4070))
        self.assertEqual(warnings, [])
        for excluded in (0x449E60, 0x449F00, 0x802750, 0x94C4D8):
            self.assertNotIn(excluded, credited)


@unittest.skipUnless(shutil.which("powershell"), "requires Windows PowerShell")
class MethodSummaryTests(unittest.TestCase):
    def summarize(self, text):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "game.log"
            log.write_text(text, encoding="utf-8")
            result = subprocess.run(["powershell", "-NoProfile", "-File", str(ROOT / "tools/run_summary.ps1"),
                                     "-Log", str(log)], capture_output=True, text=True, check=True)
            summary = Path(tmp) / "regress-probe.out"
            summary.write_text("=== regress-probe-shadows (test)\n" + result.stdout, encoding="utf-8")
            parsed = cov.read_regress(summary)
            summary.with_name("regress-probe-shadows.log").write_text(text, encoding="utf-8")
            raw = cov.read_regress(summary)
            summary.write_text("=== regress-probe-replace (test)\n" + result.stdout, encoding="utf-8")
            self.assertFalse(cov.read_regress(summary, audit_legacy=False)["methods"])
            return parsed, raw, result.stdout

    def test_summary_roundtrip_preserves_all_failures_and_last_counts(self):
        text = "\n".join((line(calls=1, complete=0), line(calls=5, complete=5),
                          line(name="Owner::Bad", mismatches=1), line(name="Owner::Bad", calls=6, complete=6),
                          line(name="Owner::Reset", resyncs=1), line(name="Owner::Reset", calls=6, complete=6),
                          line(name="Owner::Rewind", calls=6, complete=6), line(name="Owner::Rewind"),
                          line(name="Owner::Missing").replace("; resyncs=0", ""),
                          line(address="-", name="Interface::slot3")))
        parsed, raw, output = self.summarize(text)
        self.assertEqual(parsed["methods"], raw["methods"])
        records = {name: r for (_, _, name), r in parsed["methods"].items()}
        self.assertTrue(methods.exact(records["Owner::Method"]))
        self.assertEqual((records["Owner::Method"]["calls"], records["Owner::Method"]["complete"]), (5, 5))
        for name in ("Bad", "Reset", "Rewind", "Missing"):
            self.assertFalse(methods.exact(records["Owner::" + name]))
        self.assertNotIn("fixture", parsed["shadows"])
        self.assertIn("2 lines, 2 exact, calls=5 complete=5", output)

    def test_hyphenated_evidence_families_survive_real_summary(self):
        text = ("2026-10-26 [pid 1 tid 2] viewutil-rotation shadow 2032-01-01: mismatches=0 resyncs=0\n"
                "2026-10-26 [pid 1 tid 2] modelinst-live shadow 2032-01-01: mismatches=0 resyncs=0\n")
        parsed, _, _ = self.summarize(text)
        for family in ("viewutil-rotation", "modelinst-live"):
            self.assertEqual(parsed["shadows"][family], (1, 1))
            self.assertEqual(parsed["resynced"][family], 0)


if __name__ == "__main__":
    unittest.main()
