"""Tests for tools/doccheck.py: the instruction docs stay consistent by test.

    python -m unittest discover -s tools/tests -v
"""

import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
sys.path.insert(0, str(REPO / "tools"))
import doccheck  # noqa: E402

CONTRIB = """# Contributing
| | Analysis | Implementation |
|---|---|---|
| Must not | write `engine/` | open `decomp/`, `ghidra/` (any part), `re/`, `tools/`, the game executable; or run scripts from `tools/` (`.claude/lane-bin/` may be run) |
"""
LANES = 'ENGINE_FORBIDDEN = ("re/", "tools/", "decomp", "ghidra/")\n'
PROMPT = "const ENGINE = `- Never open, list, grep or read: decomp/, ghidra/ (any part), re/, tools/, any game executable — in either checkout.`\n"


def fixture(tmp, contrib=CONTRIB, lanes=LANES, prompt=PROMPT, extra=None):
    root = Path(tmp)
    (root / "tools").mkdir()
    (root / ".claude" / "workflows").mkdir(parents=True)
    (root / "docs" / "ops").mkdir(parents=True)
    (root / "CONTRIBUTING.md").write_text(contrib, encoding="utf-8")
    (root / "tools" / "lanes.py").write_text(lanes, encoding="utf-8")
    (root / ".claude" / "workflows" / "fc4-lane.js").write_text(prompt, encoding="utf-8")
    (root / "docs" / "ops" / "rules.md").write_text("# Rules\nSee [contributing](../../CONTRIBUTING.md).\n",
                                                    encoding="utf-8")
    for rel, text in (extra or {}).items():
        (root / rel).write_text(text, encoding="utf-8")
    return root


class DocCheck(unittest.TestCase):
    def test_repo_passes(self):
        for name, fn in (("links", doccheck.check_links), ("wall", doccheck.check_wall),
                         ("stale", doccheck.check_stale)):
            self.assertEqual(fn(REPO), [], name)

    def test_clean_fixture_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = fixture(tmp)
            self.assertEqual(doccheck.check_links(root) + doccheck.check_wall(root) + doccheck.check_stale(root), [])

    def test_broken_link_found_and_fenced_link_ignored(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = fixture(tmp, extra={"docs/a.md": "[gone](gone.md) [web](https://x.y) [here](#top)\n"
                                                    "```\n[in a fence](nowhere.md)\n```\n"})
            problems = doccheck.check_links(root)
            self.assertEqual(len(problems), 1)
            self.assertIn("docs/a.md: broken link gone.md", problems[0])

    def test_wall_mismatch_found(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = fixture(tmp, contrib=CONTRIB.replace("`re/`, ", ""))
            problems = doccheck.check_wall(root)
            self.assertTrue(any("only in fc4-lane.js ['re/']" in p for p in problems), problems)
            self.assertTrue(any("ENGINE_FORBIDDEN has 're/'" in p for p in problems), problems)

    def test_stale_rule_found(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = fixture(tmp, extra={"docs/harness.md": "Check for the `.verdict` file every 30 s or so.\n"})
            problems = doccheck.check_stale(root)
            self.assertEqual(len(problems), 1)
            self.assertIn("docs/harness.md:1: superseded rule", problems[0])

    def test_history_is_not_an_instruction_doc(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = fixture(tmp)
            (root / "docs" / "history").mkdir()
            (root / "docs" / "history" / "old.md").write_text("Do not clean up or delete old git worktrees\n",
                                                              encoding="utf-8")
            self.assertEqual(doccheck.check_stale(root), [])

    def test_superseded_lane_rules_are_rejected(self):
        old_rules = (
            "Your working state lives in docs/lanes/<lane>.md under ## Handoff.",
            "While the lane works: a `## Handoff` section.",
            "A lane commits it after every milestone.",
            "Lanes read the brief and the one-page HANDOFF, not whole trees.",
            "May read `docs/HANDOFF.md`, `docs/ops/`.",
            "A lane cannot read files outside its worktree.",
            "A worktree lane cannot read files outside its worktree.",
            "The engine lane turns a difference into a precise question.",
            "questions for the analysis side go in docs/specs/questions/<spec>.md",
            "`args.maxRounds`, default 6",
            "Before enqueueing:\n`... build ctest`",
            "a request\nwith an existing label overwrites that label's results",
            "rebuild on current main and\nsend the request again",
            'Safe pattern: one lane (or the coordinator) opens the exe once.',
            'On "already started", call `close_binary` once and retry.',
            "Pass\n`args.unguarded: true` to run it anyway as a default subagent.",
            "**behaviour specs** in `docs/specs/` and interface facts in `docs/abi/`",
            "cap = min(16, logical CPUs * 3 // 4, game-queue slots * 2 + 4)",
        )
        with tempfile.TemporaryDirectory() as tmp:
            root = fixture(tmp)
            for rule in old_rules:
                with self.subTest(rule=rule):
                    (root / "docs/ops/rules.md").write_text(rule + "\n", encoding="utf-8")
                    problems = doccheck.check_stale(root)
                    self.assertEqual(len(problems), 1, problems)
                    self.assertIn("superseded rule", problems[0])

    def test_question_instructions_are_checked_but_archives_are_not(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = fixture(tmp)
            questions = root / "docs/specs/questions"
            (questions / "answered").mkdir(parents=True)
            old = "The engine lane turns a difference into a precise question.\n"
            (questions / "README.md").write_text(old, encoding="utf-8")
            (questions / "answered/old.md").write_text(old, encoding="utf-8")
            problems = doccheck.check_stale(root)
            self.assertEqual(len(problems), 1, problems)
            self.assertTrue(problems[0].startswith("docs/specs/questions/README.md:1:"))

    def test_current_rules_do_not_match_old_wording(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = fixture(tmp, extra={"docs/harness.md": (
                "Write the lane file once at Finish; on interruption commit a handoff.\n"
                "Use SPEC-GAP unless tasked to file questions; maxRounds defaults to 3.\n"
                "Run qbuild.py all once; each request uses a new label; written verdicts are final.\n"
                "Read RE sources from MAIN; edit only WT; proposed ABI rows go in build-runs/.\n"
            )})
            self.assertEqual(doccheck.check_stale(root), [])


if __name__ == "__main__":
    unittest.main()
