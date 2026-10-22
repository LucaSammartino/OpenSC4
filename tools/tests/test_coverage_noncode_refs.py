"""A string whose bytes equal a code address must not transfer its caller's ownership."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("coverage_noncode_refs", ROOT / "tools/coverage.py")
cov = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cov)


class NoncodeReferenceTests(unittest.TestCase):
    def attribution(self, exclusion="01000100\t00403000\tpublic native string proof\n", direct=False):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            files = {
                "functions.tsv": "address\tbody_bytes\tthunk\tname\n"
                    "00401000\t17\tfalse\tFUN_00401000\n"
                    "00402000\t11\tfalse\tFUN_00402000\n"
                    "00403000\t42\tfalse\tFUN_00403000\n",
                "function-details.tsv": "address\tranges\n"
                    "00401000\t00401000-00401010\n00402000\t00402000-0040200a\n"
                    "00403000\t00403000-00403029\n",
                "names.tsv": "va\tname\tsource\tconfidence\tnote\n"
                    "00401000\tcFixture::Run\tmanual\thigh\tfixture seed\n",
                "vtables.tsv": "vtable\ttarget\tclass\n",
                "interfaces.tsv": "vtable\ttarget\tgame_class\n",
                "libs.tsv": "kind\tvalue\tlibrary\n",
                "ranges.tsv": "range\towner\tevidence\n",
                "calls.tsv": "caller\ttarget\n" + ("00401000\t00403000\n" if direct else ""),
                "instructions.asm": "00401001\tPUSH\t0x01000000\n00401002\tPUSH\t0x01000100\n",
                "data-references.tsv": "from\tto\n01000000\t00402000\n01000100\t00403000\n",
                "noncode.tsv": "from\tto\tevidence\n" + exclusion,
            }
            for name, text in files.items():
                (root / name).write_text(text, encoding="utf-8")
            att = cov.Attribution(root, root / "names.tsv", root / "vtables.tsv", root / "interfaces.tsv",
                                  root / "libs.tsv", root / "ranges.tsv", 0x00400000, 0x00500000,
                                  root / "noncode.tsv")
            att.run()
            return att

    def test_proven_string_does_not_propagate_but_real_callback_does(self):
        baseline = self.attribution(exclusion="")
        self.assertEqual(baseline.owner[0x00403000], "cFixture")
        corrected = self.attribution()
        self.assertEqual(corrected.owner[0x00402000], "cFixture")
        self.assertIsNone(corrected.owner[0x00403000])
        self.assertEqual(corrected.funcs, baseline.funcs)  # No denominator manipulation.

    def test_other_real_edge_to_same_target_is_preserved(self):
        self.assertEqual(self.attribution(direct=True).owner[0x00403000], "cFixture")

    def test_unmatched_exclusion_fails_instead_of_silently_not_applying(self):
        with self.assertRaisesRegex(SystemExit, "absent from this export"):
            self.attribution("01000104\t00403000\tproof\n")

    def test_evidence_and_unique_well_formed_pair_required(self):
        for row in ("01000100\t00403000\t\n", "bad\t00403000\tproof\n",
                    "01000100\t00403000\tproof\n01000100\t00403000\tproof\n"):
            with self.subTest(row=row), self.assertRaises(SystemExit):
                self.attribution(row)
