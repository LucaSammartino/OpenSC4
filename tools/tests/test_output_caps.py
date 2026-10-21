"""Tests for the lane output caps: re_query's capped stdout, qgrep's byte cap and --count, q.py's capped tail.

    python -m unittest discover -s tools/tests -v

None of these need the Ghidra export: re_query's cap is tested on its CappedStdout class alone.
"""

import io
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
LANE_BIN = REPO / ".claude" / "lane-bin"
sys.path.insert(0, str(REPO / "tools"))
sys.path.insert(0, str(LANE_BIN))
import q  # noqa: E402
import re_query  # noqa: E402


def run(script, *args, cwd=None):
    p = subprocess.run([sys.executable, str(LANE_BIN / script), *args], capture_output=True, text=True, cwd=cwd,
                       encoding="utf-8", errors="replace")
    return p.returncode, p.stdout.splitlines()


class CappedStdoutTest(unittest.TestCase):
    def capped(self, text, lines=60, nbytes=4096):
        buf = io.StringIO()
        out = re_query.CappedStdout(buf, lines, nbytes)
        out.write(text)
        out.finish()
        return buf.getvalue().splitlines()

    def test_under_the_cap_is_unchanged(self):
        self.assertEqual(self.capped("a\nb\nc\n"), ["a", "b", "c"])

    def test_line_cap_keeps_the_head_and_adds_one_footer(self):
        got = self.capped("".join(f"row {i}\n" for i in range(100)))
        self.assertEqual(len(got), 61)
        self.assertEqual(got[59], "row 59")
        self.assertTrue(got[60].startswith("... 40 more lines"))
        self.assertIn("--full", got[60])

    def test_byte_cap(self):
        got = self.capped("".join("x" * 99 + "\n" for _ in range(60)))  # 100 B per line: 40 fit in 4096
        self.assertEqual(len(got), 41)
        self.assertTrue(got[-1].startswith("... 20 more lines"))

    def test_nothing_after_the_first_hidden_line(self):
        got = self.capped("short\n" + "y" * 5000 + "\nshort again\n")
        self.assertEqual(got[0], "short")
        self.assertTrue(got[1].startswith("... 2 more lines"))

    def test_partial_writes_join_into_lines(self):
        buf = io.StringIO()
        out = re_query.CappedStdout(buf, 60, 4096)
        for piece in ("ab", "c\nd", "e"):
            out.write(piece)
        out.finish()
        self.assertEqual(buf.getvalue().splitlines(), ["abc", "de"])

    def test_count_mode_prints_the_row_count(self):
        buf, old = io.StringIO(), sys.stdout
        sys.stdout = buf
        try:
            re_query.more(895, 0)
            re_query.more(50, 40)
        finally:
            sys.stdout = old
        self.assertEqual(buf.getvalue().splitlines(), ["895 rows (--limit N to list them)", "... 10 more (--limit N)"])


class QgrepTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        for k in range(5):
            Path(self.tmp.name, f"f{k}.txt").write_text("".join(f"needle {k} {'z' * 150} {i}\n" for i in range(20)))

    def tearDown(self):
        self.tmp.cleanup()

    def test_byte_cap_and_footer(self):
        rc, out = run("qgrep.py", "needle", self.tmp.name)
        self.assertEqual(rc, 0)
        self.assertLessEqual(sum(len(line) + 1 for line in out[:-1]), 4096)
        self.assertTrue(out[-1].startswith("... "))
        self.assertIn("--full", out[-1])

    def test_count(self):
        rc, out = run("qgrep.py", "--count", "needle", self.tmp.name)
        self.assertEqual((rc, out), (0, ["100 matches in 5 files"]))

    def test_full_lists_everything(self):
        rc, out = run("qgrep.py", "--full", "needle", self.tmp.name)
        self.assertEqual(len(out), 100)

    def test_no_match(self):
        rc, out = run("qgrep.py", "haystack", self.tmp.name)
        self.assertEqual((rc, out), (1, ["qgrep: no matches"]))


class QTest(unittest.TestCase):
    def test_pick_tail_and_head(self):
        lines = [str(i) for i in range(100)]
        self.assertEqual(q.pick(lines, 3, 4096, 200, head=False), ["97", "98", "99"])
        self.assertEqual(q.pick(lines, 3, 4096, 200, head=True), ["0", "1", "2"])
        self.assertEqual(q.pick(["a" * 50] * 10, 40, 120, 200, head=False), ["a" * 50] * 2)
        self.assertEqual(q.pick(["b" * 300], 40, 4096, 200, head=True), ["b" * 200 + " ...[+100]"])

    def test_runs_a_command_and_reports_exit_and_log(self):
        with tempfile.TemporaryDirectory() as d:
            log = os.path.join(d, "out.log")
            code = "import sys; [print(i) for i in range(300)]; sys.exit(3)"
            rc, out = run("q.py", "--log", log, "--lines", "5", sys.executable, "-c", code)
            self.assertEqual(rc, 3)
            self.assertEqual(out[0], "... 295 earlier lines not shown")
            self.assertEqual(out[1:6], ["295", "296", "297", "298", "299"])
            self.assertTrue(out[6].startswith("q: exit=3, 300 lines"))
            self.assertEqual(len(Path(log).read_text().splitlines()), 300)

    def test_default_log_is_per_run_under_build_runs(self):
        with tempfile.TemporaryDirectory() as d:
            os.mkdir(os.path.join(d, "build-runs"))
            rc, out = run("q.py", sys.executable, "-c", "print('hi')", cwd=d)
            self.assertEqual((rc, out[0]), (0, "hi"))
            self.assertIn(os.path.join("build-runs", "q"), out[-1])
            self.assertEqual(len(os.listdir(os.path.join(d, "build-runs", "q"))), 1)

    def test_wt_runs_there_and_logs_there(self):
        with tempfile.TemporaryDirectory() as wt, tempfile.TemporaryDirectory() as elsewhere:
            Path(wt, "marker.txt").write_text("here\n")
            rc, out = run("q.py", "--wt", wt, sys.executable, "-c", "print(open('marker.txt').read().strip())",
                          cwd=elsewhere)
            self.assertEqual((rc, out[0]), (0, "here"))
            self.assertEqual(len(os.listdir(os.path.join(wt, "build-runs", "q"))), 1)


if __name__ == "__main__":
    unittest.main()
