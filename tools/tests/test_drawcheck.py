"""Draw oracle: differences, malformed evidence, and completed-frame accounting."""
import contextlib
import copy
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import drawcheck


def capture():
    return [
        dict(type="header", schema="fc4.draw/1", hash="fnv1a64", frames=1, scenarios=1, schedule="fixed"),
        dict(type="frame", frame=0, scenario=0, zoom=1, rotation=0, paused=1, hidden=0),
        dict(type="ui", frame=0, node=0, parent=-1, id=123, rect=[0, 0, 800, 600], visible=1,
             buffers={"text": dict(bytes=5, hash="a430d84680aabd0b")}),
        dict(type="call", frame=0, call=0, driver="dx7", slot=3, method="DrawArrays", thread=0,
             args=[0, 0, 3], buffers={"vertices": dict(bytes=60, hash="0123456789abcdef")}),
        dict(type="end_frame", frame=0, calls=1, draw_calls=1),
        dict(type="end", frames=1, calls=1, draw_calls=1, ui_frames=1, incomplete=0, complete=True),
    ]


class DrawCheckTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.left = Path(self.temp.name) / "a.jsonl"
        self.right = Path(self.temp.name) / "b.jsonl"

    def write(self, left, right):
        for path, records in ((self.left, left), (self.right, right)):
            path.write_text("".join(json.dumps(r) + "\n" for r in records), encoding="utf-8")

    def test_identical_and_json_cli(self):
        self.write(capture(), capture())
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            self.assertEqual(drawcheck.main([str(self.left), str(self.right), "--json"]), 0)
        result = json.loads(output.getvalue())
        self.assertEqual(result["verdict"], "identical")
        self.assertEqual(result["draws"], 1)

    def test_first_scalar_difference(self):
        a, b = capture(), capture()
        b[3]["args"][2] = 6
        b[3]["buffers"]["vertices"]["hash"] = "f" * 16
        self.write(a, b)
        difference, _ = drawcheck.compare(self.left, self.right)
        self.assertEqual((difference.frame, difference.call, difference.method, difference.field),
                         (0, 0, "DrawArrays", "args[2]"))

    def test_vertex_and_ui_hash_difference(self):
        for record, buffer, method in ((3, "vertices", "DrawArrays"), (2, "text", "UI.window")):
            with self.subTest(buffer=buffer):
                a, b = capture(), capture()
                b[record]["buffers"][buffer]["hash"] = "f" * 16
                self.write(a, b)
                difference, _ = drawcheck.compare(self.left, self.right)
                self.assertEqual(difference.field, f"buffers.{buffer}.hash")
                self.assertEqual(difference.method, method)

    def test_call_insertion_reports_method(self):
        a, b = capture(), capture()
        inserted = dict(type="call", frame=0, call=0, driver="dx7", slot=42,
                        method="LoadIdentity", thread=0, args=[], buffers={})
        b.insert(3, inserted)
        b[4]["call"] = 1
        b[5]["calls"] = b[6]["calls"] = 2
        self.write(a, b)
        difference, _ = drawcheck.compare(self.left, self.right)
        self.assertEqual(difference.field, "method")

    def test_incomplete_evidence_never_compares_equal(self):
        variants = [[], capture()[:-1], capture()[1:]]
        for field, value in (("complete", False), ("incomplete", 1), ("frames", 2), ("calls", 3)):
            records = capture()
            records[-1][field] = value
            variants.append(records)
        for records in variants:
            with self.subTest(records=records):
                self.write(records, records)
                with self.assertRaises(drawcheck.InvalidStream):
                    drawcheck.compare(self.left, self.right)

    def test_malformed_or_omitted_buffers(self):
        for buffer in ({}, {"vertices": dict(bytes=60, hash="bad")},
                       {"vertices": dict(bytes=0, status="unknown-extent")}):
            records = capture()
            records[3]["buffers"] = buffer
            self.write(records, records)
            with self.assertRaises(drawcheck.InvalidStream):
                drawcheck.compare(self.left, self.right)

    def test_invalid_tail_after_first_difference_is_not_valid_difference(self):
        a, b = capture(), capture()
        b[3]["args"][0] = 1
        b[-1]["complete"] = False
        self.write(a, b)
        with self.assertRaises(drawcheck.InvalidStream):
            drawcheck.compare(self.left, self.right)

    def test_empty_draw_frame_and_skipped_call_are_rejected(self):
        records = capture()
        records[3]["call"] = 1
        self.write(records, records)
        with self.assertRaises(drawcheck.InvalidStream):
            drawcheck.compare(self.left, self.right)
        records = capture()
        del records[3]
        records[3]["calls"] = records[3]["draw_calls"] = 0
        records[4]["calls"] = records[4]["draw_calls"] = 0
        self.write(records, records)
        with self.assertRaisesRegex(drawcheck.InvalidStream, "no driver draws"):
            drawcheck.compare(self.left, self.right)

    def test_dict_order_is_irrelevant_and_output_buffers_are_not_inputs(self):
        a = capture()
        a[3]["buffers"]["destination"] = dict(bytes=16, status="output")
        b = [dict(reversed(list(record.items()))) for record in copy.deepcopy(a)]
        self.write(a, b)
        self.assertIsNone(drawcheck.compare(self.left, self.right)[0])

    def test_trailing_garbage_and_bad_utf8(self):
        self.write(capture(), capture())
        with self.right.open("ab") as out:
            out.write(b"\xff\n")
        with self.assertRaises(drawcheck.InvalidStream):
            drawcheck.compare(self.left, self.right)
