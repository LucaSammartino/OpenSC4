#!/usr/bin/env python3
"""Instruction-doc checks: the rules agents follow stay consistent by test, not by prose.

Why this exists (2026-10-22 cleanup, docs/ops/cleanup-proposals.md): rules were copied into several docs, one copy
changed and the others went stale ("do not delete worktrees" after the user approved a cleanup; "poll every 30 s"
after the one-blocking-wait rule; two clean-room lists that disagreed). These checks catch that drift.

    python tools/doccheck.py check     # links + wall + stale; exit 1 on any problem
    python tools/doccheck.py links     # every relative link in an instruction doc resolves
    python tools/doccheck.py wall      # CONTRIBUTING.md, tools/lanes.py and the lane prompt forbid the same paths
    python tools/doccheck.py stale     # no superseded rule is back in an instruction doc
    python tools/doccheck.py bytes [--ref <git-ref>]   # instruction bytes one lane round reads, per kind

Pure standard library (node only for `bytes`, to render the lane prompt).
"""
import argparse
import ast
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The docs agents take instructions from. History, specs and generated logs are data, not instructions.
INSTRUCTION_GLOBS = ("README.md", "CONTRIBUTING.md", "docs/*.md", "docs/ops/*.md", "docs/lanes/README.md",
                     ".claude/agents/*.md")
NOT_INSTRUCTIONS = {"docs/manual-checks.md", "docs/coverage-report.md", "docs/token-ledger.md",
                    "docs/sdk-errata-draft.md", "docs/sim-engine.md"}

# Superseded rules: (pattern, the current rule). Add a row whenever a rule is replaced.
STALE = (
    (r"Do not clean up or delete old git worktrees",
     "worktrees of landed, clean, unparked lanes are removed after each train (docs/ops/rules.md 'Worktrees')"),
    (r"every 30 s or so",
     "wait in one blocking call: .claude/lane-bin/qwait.py (docs/harness.md 'How long to wait')"),
    (r"\*\*Commits:\*\* branch `main`",
     "a lane commits on its own branch; only the merge-train owner moves main (docs/ops/rules.md 'Commits')"),
    (r"take the merge lock|takes? `build-runs/main-merge\.lock`",
     "lanes enqueue and stop; there is no merge lock (docs/lanes/README.md 'How a lane lands')"),
)

# What one lane round reads besides its prompt: the agent file, the docs the prompt sends every lane to, and the
# kind's own reference. Lanes stopped reading docs/HANDOFF.md on 2026-10-22 (the coordinator's run status). Used by
# `bytes`.
ROUND_COMMON = ("CONTRIBUTING.md", "docs/ops/rules.md", "docs/ops/conventions.md",
                "docs/lanes/README.md", "docs/merge-train.md", "docs/specs/questions/README.md")
ROUND_KINDS = {
    "engine": (".claude/agents/fc4-lane.md", ("docs/harness.md", "docs/ops/lessons.md")),
    "re": (".claude/agents/fc4-lane-re.md", ("docs/re-workspace.md",)),
    "tool": (".claude/agents/fc4-lane.md", ("docs/lane-tools.md",)),
}

LINK = re.compile(r"\[[^\]\n]*\]\(([^)\s]+)\)")
FENCE = re.compile(r"^```.*?^```", re.S | re.M)


def instruction_docs(root):
    out = []
    for g in INSTRUCTION_GLOBS:
        for p in sorted(root.glob(g)):
            rel = p.relative_to(root).as_posix()
            if p.is_file() and rel not in NOT_INSTRUCTIONS and rel not in out:
                out.append(rel)
    return out


def read(root, rel):
    return (root / rel).read_text(encoding="utf-8", errors="replace")


def check_links(root):
    problems = []
    for rel in instruction_docs(root):
        text = FENCE.sub("", read(root, rel))
        for m in LINK.finditer(text):
            target = m.group(1)
            if re.match(r"[a-z]+:", target) or target.startswith("#"):
                continue
            path = target.split("#", 1)[0]
            if not (root / rel).parent.joinpath(path).exists():
                line = text.count("\n", 0, m.start()) + 1
                problems.append(f"{rel}: broken link {target} (near line {line}, code fences skipped)")
    return problems


def _contributing_wall(root):
    row = next((l for l in read(root, "CONTRIBUTING.md").splitlines() if l.startswith("| Must not |")), None)
    if row is None:
        return None
    cell = row.rstrip(" |").split("|")[-1]
    cell = cell.split("the game executable")[0]
    return {t.strip() for t in re.findall(r"`([^`]+)`", cell)}


def _lanes_wall(root):
    tree = ast.parse(read(root, "tools/lanes.py"))
    for node in ast.walk(tree):
        if isinstance(node, ast.Assign) and any(getattr(t, "id", "") == "ENGINE_FORBIDDEN" for t in node.targets):
            return set(ast.literal_eval(node.value))
    return None


def _prompt_wall(root):
    m = re.search(r"Never open, list, grep or read: (.+?) — in either checkout", read(root, ".claude/workflows/fc4-lane.js"))
    if not m:
        return None
    items = [i.replace("(any part)", "").strip() for i in m.group(1).split(",")]
    return {i for i in items if i and not i.startswith("any ")}


def check_wall(root):
    contrib, lanes, prompt = _contributing_wall(root), _lanes_wall(root), _prompt_wall(root)
    for name, val in (("CONTRIBUTING.md 'Must not' row", contrib), ("tools/lanes.py ENGINE_FORBIDDEN", lanes),
                      (".claude/workflows/fc4-lane.js ENGINE list", prompt)):
        if val is None:
            return [f"{name}: not found"]
    problems = []
    if contrib != prompt:
        problems.append(f"clean-room lists differ: only in CONTRIBUTING.md {sorted(contrib - prompt)}, "
                        f"only in fc4-lane.js {sorted(prompt - contrib)}")
    for e in sorted(lanes):  # lanes.py matches by prefix ('decomp' covers decomp/ and decomp-mac/)
        if not any(c.startswith(e) for c in contrib):
            problems.append(f"tools/lanes.py ENGINE_FORBIDDEN has {e!r}, which CONTRIBUTING.md does not forbid")
    return problems


def check_stale(root):
    problems = []
    for rel in instruction_docs(root):
        text = read(root, rel)
        for pat, now in STALE:
            for m in re.finditer(pat, text):
                problems.append(f"{rel}:{text.count(chr(10), 0, m.start()) + 1}: superseded rule {m.group(0)!r}; "
                                f"now: {now}")
    return problems


def round_bytes(root, ref=None):
    def size(rel):
        if ref:
            p = subprocess.run(["git", "-C", str(root), "show", f"{ref}:{rel}"], capture_output=True)
            return len(p.stdout) if p.returncode == 0 else 0
        p = root / rel
        return len(p.read_bytes().replace(b"\r\n", b"\n")) if p.exists() else 0

    def prompt(kind):
        req = {"args": {"kind": kind, "name": "measure", "task": "", "date": "2026-01-01"}}
        try:
            p = subprocess.run(["node", str(root / "tools" / "lane_prompt.mjs")], input=json.dumps(req).encode(),
                               capture_output=True, check=True)
            return len(json.loads(p.stdout)["prompt"].encode("utf-8"))
        except (OSError, subprocess.CalledProcessError, ValueError, KeyError):
            return 0

    out = {}
    for kind, (agent, extra) in ROUND_KINDS.items():
        files = {f: size(f) for f in (agent, *ROUND_COMMON, *extra)}
        out[kind] = {"prompt": prompt(kind), "files": files, "total": prompt(kind) + sum(files.values())}
    return out


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("cmd", choices=("check", "links", "wall", "stale", "bytes"))
    ap.add_argument("--ref", help="bytes: measure the files at this git ref (the prompt is always the working copy)")
    a = ap.parse_args(argv)
    if a.cmd == "bytes":
        for kind, r in round_bytes(ROOT, a.ref).items():
            print(f"{kind}: {r['total']} bytes = prompt {r['prompt']} + files {sum(r['files'].values())}")
            for f, n in r["files"].items():
                print(f"  {n:7d} {f}")
        return 0
    checks = {"links": check_links, "wall": check_wall, "stale": check_stale}
    names = list(checks) if a.cmd == "check" else [a.cmd]
    problems = [p for n in names for p in checks[n](ROOT)]
    for p in problems:
        print(p)
    print(f"doccheck {a.cmd}: {'FAIL, ' + str(len(problems)) + ' problem(s)' if problems else 'ok'}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
