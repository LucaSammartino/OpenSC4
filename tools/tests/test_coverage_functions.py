"""Independent tiny native metadata and literal accounting expectations, no game algorithms."""
import copy
import csv
import importlib.util
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest

ROOT = Path(__file__).resolve().parents[2]


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


cov = module("fixture_coverage", "tools/coverage.py")
tm = module("fixture_treemap", "tools/coverage_treemap.py")
ev = cov._function_module


class FunctionEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.tracked = set()
        # Real fixture weights: two partial-owner bodies 13 and 29 B; full owner
        # 7 B; library/shared/unowned bodies stay outside credits.
        self.att = SimpleNamespace(order=[16, 32, 48, 64, 80, 96],
                                   funcs={16: 13, 32: 29, 48: 7, 64: 11, 80: 17, 96: 19},
                                   vt={}, rng=[(16, 28, 16), (32, 60, 32), (64, 74, 64)])
        self.owners = {16: "cPartial", 32: "cPartial", 48: "cFull", 64: "lib:C", 80: cov.SHARED, 96: None}
        self.rules = {v: "name" for v in self.att.order}
        self.rows = [dict(pattern="cPartial", area="framework", subsystem="partial", **{s: False for s in cov.STAT}),
                     dict(pattern="cFull", area="framework", subsystem="full", **{s: True for s in cov.STAT})]
        self.artifact = {"exe_sha256": "original", "functions_sha256": "metadata", "details_sha256": "bodies"}
        self.write("docs/specs/fixture.md", "# Public contract\nComplete decision at exact entry.\n")
        self.write("engine/core/src/fixture.rs", "fn production_symbol() {}\n")
        self.write("engine/core/src/fixture_tests.rs", "fn independent_assertion() { assert!(literal_expected); }\n")
        self.write("docs/coverage-functions/fixture.log", "command: fixture-check\nresult=PASS\n"
                   + "production_sha256=" + self.ref("engine/core/src/fixture.rs")["sha256"] + "\n"
                   + "assertions_sha256=" + self.ref("engine/core/src/fixture_tests.rs")["sha256"] + "\n")
        self.claims = []

    def write(self, path, text):
        p = self.root / path
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(text.encode("utf-8"))
        self.tracked.add(path)
        return p

    def ref(self, path):
        return dict(path=path, sha256=cov.text_sha256(self.root / path), citation=path + "#correspondence")

    def claim(self, address="00000010", stages=None, owner="cPartial"):
        stages = stages or ["implemented"]
        paths = {"contract": "docs/specs/fixture.md", "qualification": "docs/coverage-functions/" + address + ".json"}
        if "implemented" in stages:
            paths.update(production="engine/core/src/fixture.rs", assertions="engine/core/src/fixture_tests.rs",
                         validation="docs/coverage-functions/fixture.log")
        q = dict(address=address, owner=owner, stages=stages,
                 review=dict(by="independent fixture reviewer", citation="fixture-review#accepted", approved=True),
                 normal_domain_complete=True, limitations="Synthetic fixture only; no native proof.",
                 decisions=[dict(contract="Complete decision", production="production_symbol" if "implemented" in stages else None,
                                 assertion="independent_assertion" if "implemented" in stages else None)], validation=None)
        if "implemented" in stages:
            q["validation"] = dict(command="fixture-check", result="PASS", provenance="fixture-review#actual-log",
                                   production_sha256=self.ref(paths["production"])["sha256"],
                                   assertions_sha256=self.ref(paths["assertions"])["sha256"],
                                   log_sha256=self.ref(paths["validation"])["sha256"])
        self.write(paths["qualification"], json.dumps(q))
        record = dict(address=address, owner=owner, stages=stages, artifact=dict(self.artifact),
                      evidence={role: self.ref(path) for role, path in paths.items()})
        self.claims.append(record)
        return record

    def resolve(self):
        p = self.write("tools/coverage-functions.json", json.dumps(dict(version=1, claims=self.claims)))
        return ev.resolve(self.att, self.owners, self.rules, self.rows, p, self.root,
                          cov.text_sha256, "original", self.artifact, self.tracked)

    def items(self):
        return [(v, self.att.funcs[v], 0, "framework") for v in self.att.order if v != 64]

    def test_empty_is_exact_legacy_baseline(self):
        flags, baseline, _ = self.resolve()
        self.assertEqual(flags, baseline)
        self.assertEqual(cov.tally_functions(self.items(), flags),
                         dict(n=5, b=85, specified_n=1, specified_b=7, implemented_n=1, implemented_b=7,
                              verified_n=1, verified_b=7, replaced_n=1, replaced_b=7))

    def test_exact_function_adds_only_13_actual_bytes(self):
        self.claim()
        flags, baseline, _ = self.resolve()
        t = cov.tally_functions(self.items(), flags)
        self.assertEqual((t["implemented_n"], t["implemented_b"], t["specified_b"]), (2, 20, 7))
        self.assertFalse(flags[32]["implemented"])
        self.assertEqual(cov.completion(self.items()[:2], flags, baseline, "implemented"),
                         "partial (1/2 fn; 13/42 B)")

    def test_stages_independent_union_once_order_deterministic(self):
        self.claim()
        self.claim("00000020", ["specified"])
        self.claim("00000030", ["specified", "implemented"], "cFull")
        first = self.resolve()
        self.claims.reverse()
        second = self.resolve()
        self.assertEqual(first[:2], second[:2])
        # Ledger digest changes with order, but flags and totals do not.
        t = cov.tally_functions(self.items(), first[0])
        self.assertEqual((t["specified_b"], t["implemented_b"], t["verified_b"], t["replaced_b"]), (36, 20, 7, 7))
        self.assertFalse(first[0][32]["implemented"])

    def test_all_entry_claims_still_do_not_promote_owner(self):
        self.claim()
        self.claim("00000020")
        flags, base, _ = self.resolve()
        self.assertEqual(cov.completion(self.items()[:2], flags, base, "implemented"), "partial (2/2 fn; 42/42 B)")

    def test_report_treemap_audit_exact_parity_and_split(self):
        self.claim()
        self.claim("00000020", ["specified"])
        flags, baseline, _ = self.resolve()
        report = cov.tally_functions(self.items(), flags)
        tree, totals = tm.collect_entries(self.att, self.owners, self.rows, flags)
        self.assertEqual(totals, dict(specified=36, implemented=20, verified=7, replaced=7))
        self.assertEqual(sorted(tree["framework"]["partial"]), [(13, 2, "cPartial"), (29, 1, "cPartial")])
        records = list(csv.DictReader(io.StringIO("\n".join(cov.audit_lines(
            self.att, self.owners, self.rules, dict.fromkeys(self.att.order, "fixture"), flags, baseline))), delimiter="\t"))
        for s in cov.STAT:
            self.assertEqual(sum(int(r["bytes"]) for r in records if r[s] == "1"), report[s + "_b"])
            self.assertEqual(totals[s], report[s + "_b"])

    def test_duplicate_and_conflicting_entries_fail(self):
        original = self.claim()
        for duplicate in (copy.deepcopy(original), {**original, "stages": ["specified"]}):
            self.claims = [original, duplicate]
            with self.assertRaisesRegex(ValueError, "duplicate/conflicting"):
                self.resolve()

    def test_bad_identity_and_stages_fail(self):
        original = self.claim()
        cases = [("stages", ["verified"]), ("stages", ["replaced"]), ("stages", ["implemented", "implemented"]),
                 ("stages", []), ("stages", "implemented"), ("owner", "cCaller"), ("address", "00000011"),
                 ("address", "0x000010"), ("artifact", {**self.artifact, "functions_sha256": "old"}),
                 ("evidence", {})]
        for field, value in cases:
            with self.subTest(field=field, value=value):
                self.claims = [{**original, field: value}]
                with self.assertRaises(ValueError):
                    self.resolve()

    def test_library_shared_unowned_folded_and_overlapping_fail(self):
        for address, owner in (("00000040", "lib:C"), ("00000050", cov.SHARED), ("00000060", "cPartial")):
            self.claims = []
            self.claim(address, owner=owner)
            with self.assertRaises(ValueError):
                self.resolve()
        self.claims = []
        self.claim()
        self.rules[16] = "name-folded"
        with self.assertRaises(ValueError):
            self.resolve()
        self.rules[16] = "name"
        self.att.vt[16] = {"cPartial", "cOther"}
        with self.assertRaises(ValueError):
            self.resolve()
        self.att.vt.clear()
        self.att.rng.append((20, 22, 32))
        with self.assertRaisesRegex(ValueError, "overlapping"):
            self.resolve()

    def test_all_current_evidence_modifications_withdraw_credit(self):
        record = self.claim()
        for role, ref in record["evidence"].items():
            path = self.root / ref["path"]
            old = path.read_bytes()
            path.write_bytes(old + b"\nchanged\n")
            with self.subTest(role=role), self.assertRaisesRegex(ValueError, "stale"):
                self.resolve()
            path.write_bytes(old)

    def test_path_escape_missing_untracked_and_citation_fail(self):
        original = self.claim()
        for path in ("../outside", "C:/outside", "/outside", "docs/specs/../outside", "docs/specs/missing", "docs\\specs\\fixture.md"):
            self.claims = [copy.deepcopy(original)]
            self.claims[0]["evidence"]["contract"]["path"] = path
            with self.subTest(path=path), self.assertRaises(ValueError):
                self.resolve()
        self.claims = [copy.deepcopy(original)]
        self.claims[0]["evidence"]["contract"]["citation"] = ""
        with self.assertRaisesRegex(ValueError, "citation"):
            self.resolve()
        self.claims = [original]
        self.tracked.remove("docs/specs/fixture.md")
        with self.assertRaisesRegex(ValueError, "untracked"):
            self.resolve()

    def test_incomplete_unreviewed_wrong_correspondence_or_validation_fails(self):
        record = self.claim()
        ref = record["evidence"]["qualification"]
        path = self.root / ref["path"]
        original = json.loads(path.read_text())
        for change in (lambda q: q.update(normal_domain_complete=False),
                       lambda q: q["review"].update(approved=False),
                       lambda q: q["decisions"][0].update(assertion="foreign_assertion"),
                       lambda q: q["validation"].update(result="FAIL"),
                       lambda q: q["validation"].update(production_sha256="old"),
                       lambda q: q.update(owner="cCaller")):
            q = copy.deepcopy(original)
            change(q)
            path.write_text(json.dumps(q), encoding="utf-8")
            record["evidence"]["qualification"] = self.ref(ref["path"])
            with self.assertRaises(ValueError):
                self.resolve()

    def test_crlf_lf_hash_and_claim_invariance(self):
        self.claim()
        before = self.resolve()
        for name in self.tracked:
            path = self.root / name
            path.write_bytes(path.read_bytes().replace(b"\n", b"\r\n"))
        after = self.resolve()
        self.assertEqual(before, after)

    def test_rehashed_generic_log_is_not_source_matched_proof(self):
        record = self.claim()
        self.write("docs/coverage-functions/fixture.log", "PASS 9000 tests\n")
        record["evidence"]["validation"] = self.ref("docs/coverage-functions/fixture.log")
        qpath = self.root / record["evidence"]["qualification"]["path"]
        q = json.loads(qpath.read_text())
        q["validation"]["log_sha256"] = record["evidence"]["validation"]["sha256"]
        qpath.write_text(json.dumps(q), encoding="utf-8")
        record["evidence"]["qualification"] = self.ref(record["evidence"]["qualification"]["path"])
        with self.assertRaisesRegex(ValueError, "retained validation lacks"):
            self.resolve()

    def test_duplicate_json_fields_and_malformed_input_fail(self):
        for text in ('{"version":1,"version":1,"claims":[]}', '{bad', '{"version":true,"claims":[]}'):
            path = self.write("tools/coverage-functions.json", text)
            with self.assertRaises(ValueError):
                ev.resolve(self.att, self.owners, self.rules, self.rows, path, self.root,
                           cov.text_sha256, "original", self.artifact, self.tracked)


if __name__ == "__main__":
    unittest.main()
