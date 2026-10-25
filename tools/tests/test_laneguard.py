"""Tests for the lane guard (.claude/lane-bin/laneguard.py, the PreToolUse hook in .claude/settings.json), the STOP
flag and parking in tools/lanes.py, and the fc4-lane workflow's gate (a dry run with a mock agent, no model).

    python -m unittest discover -s tools/tests -v

Everything runs against a fixture main checkout in a temp directory (FC4_LANEGUARD_MAIN / lanes.py --repo): fake
subagent transcripts laid out the way Claude Code 2.1.304 writes them (<projects>/<slug>/<session>.jsonl, and
<session>/subagents/workflows/<run>/agent-<id>.jsonl + .meta.json whose description is the workflow label), and a
real git repository as the lane's worktree. The hook input mirrors what was captured from a live probe on
2026-10-21: transcript_path is the PARENT session's transcript, agent_id / agent_type name the subagent.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
GUARD = REPO / ".claude" / "lane-bin" / "laneguard.py"
LANES = REPO / "tools" / "lanes.py"
sys.path.insert(0, str(REPO / ".claude" / "lane-bin"))
import laneguard  # noqa: E402

SESSION = "11111111-2222-3333-4444-555555555555"
LANE = "re-fixture1"


def git(cwd, *args):
    subprocess.run(["git", "-C", str(cwd), *args], check=True, capture_output=True,
                   env=dict(os.environ, GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@t", GIT_COMMITTER_NAME="t",
                            GIT_COMMITTER_EMAIL="t@t"))


class Fixture:
    def __init__(self, budget=150000, calls=5):
        self.tmp = Path(tempfile.mkdtemp(prefix="laneguard-"))
        self.main = self.tmp / "main"
        (self.main / "build-runs").mkdir(parents=True)
        self.wt = self.main / ".claude" / "worktrees" / LANE
        (self.wt / "docs" / "lanes").mkdir(parents=True)
        git(self.wt, "init", "-q", "-b", LANE)
        (self.wt / "docs" / "lanes" / f"{LANE}.md").write_text("| row |\n\n## Handoff\n", encoding="utf-8")
        git(self.wt, "add", "-A")
        git(self.wt, "commit", "-q", "-m", "start")
        self.proj = self.tmp / "projects" / "C--fixture"
        self.parent = self.proj / f"{SESSION}.jsonl"
        self.parent.parent.mkdir(parents=True)
        self.parent.write_text("", encoding="utf-8")
        self.run = self.proj / SESSION / "subagents" / "workflows" / "wf_fixture"
        self.run.mkdir(parents=True)
        self.budget, self.calls = budget, calls

    def agent(self, agent_id, label=f"lane:{LANE}", ctx=50000, agent_type="fc4-lane-re"):
        (self.run / f"agent-{agent_id}.meta.json").write_text(json.dumps(
            {"agentType": agent_type, "description": label, "workflowPhase": "Lane"}), encoding="utf-8")
        self.set_ctx(agent_id, ctx)
        return agent_id

    def set_ctx(self, agent_id, ctx):
        prompt = f"## Your lane\n- lane guard: budget={self.budget} calls={self.calls}. The harness enforces both."
        recs = [{"type": "user", "agentId": agent_id, "message": {"role": "user", "content": prompt}},
                {"type": "assistant", "agentId": agent_id,
                 "message": {"role": "assistant", "content": [{"type": "text", "text": "x"}],
                             "usage": {"input_tokens": 10, "cache_read_input_tokens": ctx - 10,
                                       "cache_creation_input_tokens": 0}}}]
        (self.run / f"agent-{agent_id}.jsonl").write_text("\n".join(json.dumps(r) for r in recs) + "\n",
                                                          encoding="utf-8")

    def call(self, agent_id, tool="Bash", tool_input=None, agent_type="fc4-lane-re"):
        data = {"session_id": SESSION, "transcript_path": str(self.parent), "cwd": str(self.main),
                "permission_mode": "bypassPermissions", "agent_id": agent_id, "agent_type": agent_type,
                "hook_event_name": "PreToolUse", "tool_name": tool,
                "tool_input": tool_input if tool_input is not None else {"command": "ls"}}
        p = subprocess.run([sys.executable, str(GUARD), "hook"], input=json.dumps(data), capture_output=True,
                           text=True, env=dict(os.environ, FC4_LANEGUARD_MAIN=str(self.main)), timeout=60)
        if p.returncode != 0:
            raise AssertionError(f"hook exit {p.returncode}: {p.stderr}")
        if not p.stdout.strip():
            return "allow", ""
        o = json.loads(p.stdout)["hookSpecificOutput"]
        return o["permissionDecision"], o["permissionDecisionReason"]

    def lanes(self, *args):
        p = subprocess.run([sys.executable, str(LANES), "--repo", str(self.main), *args], capture_output=True,
                           text=True, timeout=120)
        return p.returncode, p.stdout

    def commit_cmd(self):
        w = str(self.wt).replace("\\", "/")
        return {"command": f"git -C {w} add docs/lanes/{LANE}.md && git -C {w} commit -F {w}/build-runs/msg-{LANE}-1.txt"}

    def make_commit(self, name="f.txt"):
        (self.wt / name).write_text(name, encoding="utf-8")
        git(self.wt, "add", name)
        git(self.wt, "commit", "-q", "-m", name)

    def close(self):
        shutil.rmtree(self.tmp, ignore_errors=True)


class Base(unittest.TestCase):
    def setUp(self):
        self.f = Fixture()

    def tearDown(self):
        self.f.close()


class PassThrough(Base):
    def test_other_agent_types_and_sessions_pass(self):
        a = self.f.agent("a0", ctx=900000)
        self.assertEqual(self.f.call(a, agent_type="general-purpose")[0], "allow")
        self.assertEqual(self.f.call(a, agent_type=None)[0], "allow")
        self.assertFalse((self.f.main / "build-runs" / "lane-guard").exists(), "a pass-through writes no state")

    def test_gate_agent_is_not_a_lane(self):
        a = self.f.agent("g1", label=f"gate:{LANE}#1", ctx=900000, agent_type="fc4-lane")
        for _ in range(10):
            self.assertEqual(self.f.call(a, agent_type="fc4-lane")[0], "allow")


class Budget(Base):
    def test_over_budget_denies_all_but_committing_the_lane_file(self):
        a = self.f.agent("b1", ctx=162000)
        d, msg = self.f.call(a)
        self.assertEqual(d, "deny")
        self.assertIn("CONTEXT BUDGET", msg)
        self.assertIn("162k", msg)
        self.assertIn('"continue"', msg)
        w = str(self.f.wt).replace("\\", "/")
        self.assertEqual(self.f.call(a, "Bash", self.f.commit_cmd())[0], "allow")
        self.assertEqual(self.f.call(a, "Write", {"file_path": f"{w}/docs/lanes/{LANE}.md", "content": "x"})[0], "allow")
        self.assertEqual(self.f.call(a, "Write", {"file_path": f"{w}/build-runs/msg-{LANE}-2.txt", "content": "m"})[0],
                         "allow")
        self.assertEqual(self.f.call(a, "Read", {"file_path": f"{w}/docs/specs/automata.md"})[0], "deny")
        self.assertEqual(self.f.call(a, "Write", {"file_path": f"{w}/docs/specs/automata.md", "content": "x"})[0], "deny")
        self.assertEqual(self.f.call(a, "mcp__rea__procedure_pseudo_code", {"address": "0x401000"})[0], "deny")
        self.assertEqual(self.f.call(a, "StructuredOutput", {"lane": LANE, "tip": "abc", "state": "continue",
                                                             "summary": "handoff"})[0], "allow")
        log = (self.f.main / "build-runs" / "lane-guard" / f"{LANE}.log").read_text(encoding="utf-8")
        self.assertIn("deny budget", log)

    def test_under_budget_allows(self):
        a = self.f.agent("b2", ctx=98000)
        self.assertEqual(self.f.call(a)[0], "allow")

    def test_budget_comes_from_the_prompt_line(self):
        self.f.budget = 80000
        a = self.f.agent("b3", ctx=98000)
        d, msg = self.f.call(a)
        self.assertEqual(d, "deny")
        self.assertIn("budget 80k", msg)


class Watchdog(Base):
    def test_calls_without_commit_trip_and_stay_tripped_for_the_agent(self):
        a = self.f.agent("w1")
        for i in range(self.f.calls):
            self.assertEqual(self.f.call(a)[0], "allow", f"call {i + 1}")
        d, msg = self.f.call(a)
        self.assertEqual(d, "deny")
        self.assertIn("NO PROGRESS", msg)
        self.assertIn(f"{self.f.calls} tool calls since this worktree's last commit", msg)
        self.assertIn('"blocked"', msg)
        self.assertEqual(self.f.call(a, "Bash", self.f.commit_cmd())[0], "allow", "committing findings is allowed")
        self.f.make_commit()
        self.assertEqual(self.f.call(a)[0], "deny", "the trip lasts for the agent's life, even after a commit")
        st = json.loads((self.f.main / "build-runs" / "lane-guard" / f"{LANE}.json").read_text(encoding="utf-8"))
        self.assertEqual(st["calls"], 0, "the commit reset the lane's counter")
        b = self.f.agent("w2")
        self.assertEqual(self.f.call(b)[0], "allow", "a new round after the commit starts from zero")

    def test_commits_reset_the_count(self):
        a = self.f.agent("w3")
        for i in range(3 * self.f.calls):
            if i and i % self.f.calls == 0:
                self.f.make_commit(f"m{i}.txt")
            self.assertEqual(self.f.call(a)[0], "allow", f"call {i + 1}")

    def test_count_spans_rounds_without_a_commit(self):
        a = self.f.agent("w4")
        for _ in range(self.f.calls):
            self.f.call(a)
        b = self.f.agent("w5")
        self.assertEqual(self.f.call(b)[0], "deny", "a restart without a commit does not get a fresh allowance")


class Stop(Base):
    def test_stop_flag_denies_lanes_and_refuses_spawns(self):
        a = self.f.agent("s1")
        self.assertEqual(self.f.call(a)[0], "allow")
        rc, out = self.f.lanes("stop", "wind-down: land what is on disk")
        self.assertEqual(rc, 0, out)
        d, msg = self.f.call(a)
        self.assertEqual(d, "deny")
        self.assertIn("STOP", msg)
        self.assertIn("wind-down: land what is on disk", msg)
        self.assertEqual(self.f.call(a, "Bash", self.f.commit_cmd())[0], "allow")
        rc, out = self.f.lanes("slots")
        self.assertEqual(rc, 1)
        self.assertIn("STOP: wind-down", out)
        rc, out = self.f.lanes("check", "eng-new1", "--kind", "engine")
        self.assertEqual(rc, 1)
        self.assertIn("STOP flag set", out)
        code, line = laneguard.preflight(str(self.f.main), "eng-new1", "engine")
        self.assertEqual((code, line.split()[0]), (1, "STOP"))
        # a result returned under STOP does not count toward parking
        self.f.call(a, "StructuredOutput", {"lane": LANE, "tip": "x", "state": "blocked", "summary": "STOP: here"})
        rc, out = self.f.lanes("go")
        self.assertIn("STOP cleared", out)
        self.assertEqual(laneguard.preflight(str(self.f.main), "eng-new1", "engine"), (0, "GO"))
        b = self.f.agent("s2")
        self.assertEqual(self.f.call(b)[0], "allow")


class Park(Base):
    def blocked(self, agent_id, questions, summary="blocked"):
        a = self.f.agent(agent_id)
        self.f.call(a)
        self.f.call(a, "StructuredOutput", {"lane": LANE, "tip": "t", "state": "blocked", "questions": questions,
                                            "summary": summary})

    def test_two_rounds_blocked_on_the_same_question_park_the_lane(self):
        self.blocked("p1", "NO-12 filed in docs/specs/questions/network-occupants.md", "REA down, need NO-12")
        self.assertIsNone(laneguard.park_reason(str(self.f.main), LANE), "one blocked round does not park")
        self.blocked("p2", "NO-12 still open", "a different wording of the same block")
        self.assertIn("NO-12", laneguard.park_reason(str(self.f.main), LANE))
        code, line = laneguard.preflight(str(self.f.main), LANE, "engine")
        self.assertEqual((code, line.split()[0]), (1, "PARK"))
        c = self.f.agent("p3")
        d, msg = self.f.call(c)
        self.assertEqual(d, "deny")
        self.assertIn("PARKED", msg)
        rc, out = self.f.lanes("check", LANE, "--kind", "engine")
        self.assertIn("lane parked", out)
        rc, out = self.f.lanes("unpark", LANE)
        self.assertIn("released", out)
        self.assertEqual(laneguard.preflight(str(self.f.main), LANE, "engine"), (0, "GO"))
        self.assertEqual(self.f.call(self.f.agent("p4"))[0], "allow")

    def test_different_questions_do_not_park(self):
        self.blocked("p5", "NO-12")
        self.blocked("p6", "NO-13")
        self.assertIsNone(laneguard.park_reason(str(self.f.main), LANE))

    def test_two_watchdog_trips_park(self):
        for aid in ("p7", "p8"):
            a = self.f.agent(aid)
            for _ in range(self.f.calls + 1):
                self.f.call(a)
            self.f.call(a, "StructuredOutput", {"lane": LANE, "tip": "t", "state": "blocked", "questions": "none",
                                                "summary": f"explored, nothing found ({aid})"})
            self.f.make_commit(f"{aid}.txt")
        self.assertIn("watchdog trip", laneguard.park_reason(str(self.f.main), LANE) or "")

    def test_a_ready_round_in_between_resets(self):
        self.blocked("p9", "NO-12")
        a = self.f.agent("p10")
        self.f.call(a, "StructuredOutput", {"lane": LANE, "tip": "t", "state": "ready", "summary": "done"})
        self.blocked("p11", "NO-12")
        self.assertIsNone(laneguard.park_reason(str(self.f.main), LANE))


class BashAllowList(unittest.TestCase):
    W = "C:/x/main/.claude/worktrees/lane1"

    def ok(self, cmd):
        return laneguard.bash_allowed(cmd, self.W)

    def test_allowed(self):
        self.assertTrue(self.ok(f"git -C {self.W} add docs/lanes/lane1.md && git -C {self.W} commit -F {self.W}/build-runs/msg-lane1-3.txt"))
        self.assertTrue(self.ok(f"git -C {self.W} -c submodule.recurse=false status --short | head -n 30"))
        self.assertTrue(self.ok(f"git -C '{self.W}' log --oneline -5; git -C {self.W} rev-parse --short HEAD"))
        self.assertTrue(self.ok(f"git -C {self.W.replace('/', chr(92))} diff --stat 2>&1"))

    def test_denied(self):
        self.assertFalse(self.ok("ls"))
        self.assertFalse(self.ok(f"git -C {self.W} add x; rm -rf /"))
        self.assertFalse(self.ok(f"git -C {self.W} commit -m \"$(cat /etc/passwd)\""))
        self.assertFalse(self.ok(f"git -C {self.W} log > out.txt"))
        self.assertFalse(self.ok(f"git -C C:/x/main add -A && git -C C:/x/main commit -m x"))
        self.assertFalse(self.ok(f"git -C {self.W} push"))
        self.assertFalse(self.ok(f"git -C {self.W} log | python -c 'x'"))
        self.assertFalse(self.ok(f"cat > {self.W}/build-runs/msg.txt <<'EOF'\nm\nEOF"))


class WorkflowDryRun(unittest.TestCase):
    """fc4-lane.js run under node with a mock agent(): the gate agent's command runs for real against the fixture."""

    @classmethod
    def setUpClass(cls):
        cls.node = shutil.which("node")

    def setUp(self):
        if not self.node:
            self.skipTest("node not on PATH")
        self.f = Fixture()

    def tearDown(self):
        self.f.close()

    def run_wf(self, lanes, args=None, expect_fail=False, **extra):
        sc = {"args": {"kind": "tool", "name": LANE, "task": "dry run", "maxRounds": 3, **(args or {})},
              "guardMain": str(self.f.main), "lanes": lanes, **extra}
        p = self.f.tmp / "scenario.json"
        p.write_text(json.dumps(sc), encoding="utf-8")
        o = subprocess.run([self.node, str(HERE / "fc4_lane_dryrun.mjs"), str(p)], capture_output=True, text=True,
                           timeout=180, env=dict(os.environ, FC4_TEST_PYTHON=sys.executable))
        if expect_fail:
            self.assertNotEqual(o.returncode, 0, o.stdout)
            return o.stderr
        self.assertEqual(o.returncode, 0, o.stderr)
        return json.loads(o.stdout)

    def test_refuses_to_run_unguarded_when_the_agent_type_is_unknown(self):
        err = self.run_wf([{"lane": LANE, "tip": "x", "state": "ready", "summary": "must not run"}],
                          expect_fail=True, unknownAgentType=True)
        self.assertIn("without the lane guard", err)
        r = self.run_wf([{"lane": LANE, "tip": "x", "state": "ready", "summary": "done"}],
                        args={"unguarded": True}, unknownAgentType=True)
        self.assertEqual(r["result"]["state"], "ready")
        self.assertTrue(any("WITHOUT the lane guard" in m for m in r["logs"]), r["logs"])

    def test_refuses_to_start_with_stop(self):
        self.f.lanes("stop", "wind-down")
        r = self.run_wf([{"lane": LANE, "tip": "x", "state": "ready", "summary": "must not run"}])
        self.assertEqual(r["agents"], [f"gate:{LANE}#1"], "no lane agent may start")
        self.assertEqual(r["result"]["state"], "blocked")
        self.assertIn("not started: STOP", r["result"]["summary"])

    def test_starts_without_stop(self):
        r = self.run_wf([{"lane": LANE, "tip": "x", "state": "ready", "summary": "done"}])
        self.assertEqual(r["agents"], [f"gate:{LANE}#1", f"lane:{LANE}"])
        self.assertEqual(r["result"]["state"], "ready")

    def test_refuses_to_restart_after_stop_mid_run(self):
        r = self.run_wf([{"lane": LANE, "tip": "a1", "state": "continue", "summary": "budget"},
                         {"lane": LANE, "tip": "a2", "state": "ready", "summary": "must not run"}],
                        setStopAfterRound=1)
        self.assertEqual(r["agents"], [f"gate:{LANE}#1", f"lane:{LANE}", f"gate:{LANE}#2"])
        self.assertIn("round 2 not started: STOP", r["result"]["summary"])
        self.assertEqual(r["result"]["tip"], "a1")

    def test_restarts_on_continue(self):
        r = self.run_wf([{"lane": LANE, "tip": "a1", "state": "continue", "summary": "budget"},
                         {"lane": LANE, "tip": "a2", "state": "ready", "summary": "done"}])
        self.assertEqual(r["agents"], [f"gate:{LANE}#1", f"lane:{LANE}", f"gate:{LANE}#2", f"lane:{LANE}#2"])
        self.assertEqual(r["result"]["state"], "ready")

    def test_parked_lane_does_not_start(self):
        led = self.f.main / "build-runs" / "lane-guard" / f"{LANE}.rounds.jsonl"
        led.parent.mkdir(parents=True)
        led.write_text("".join(json.dumps({"agent_id": f"x{i}", "label": f"lane:{LANE}", "state": "blocked",
                                           "questions": "NO-12", "summary": "s", "guard": None}) + "\n"
                               for i in range(2)), encoding="utf-8")
        r = self.run_wf([{"lane": LANE, "tip": "x", "state": "ready", "summary": "must not run"}])
        self.assertEqual(r["agents"], [f"gate:{LANE}#1"])
        self.assertIn("not started: PARK", r["result"]["summary"])

    def test_prompt_carries_the_guard_limits(self):
        src = (REPO / ".claude" / "workflows" / "fc4-lane.js").read_text(encoding="utf-8")
        self.assertIn("lane guard: budget=${BUDGET} calls=${MAX_CALLS}", src)
        m = laneguard.LIMITS.search("- lane guard: budget=120000 calls=90. The harness")
        self.assertEqual(m.groups(), ("120000", "90"))


class Settings(unittest.TestCase):
    def test_settings_wire_the_hook(self):
        s = json.loads((REPO / ".claude" / "settings.json").read_text(encoding="utf-8"))
        cmds = [h["command"] for e in s["hooks"]["PreToolUse"] for h in e["hooks"]]
        self.assertTrue(any("laneguard.py" in c and c.endswith(" hook") for c in cmds), cmds)
        for name in laneguard.LANE_TYPES:
            self.assertTrue((REPO / ".claude" / "agents" / f"{name}.md").exists(), name)
            self.assertIn(f"name: {name}\n", (REPO / ".claude" / "agents" / f"{name}.md").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
