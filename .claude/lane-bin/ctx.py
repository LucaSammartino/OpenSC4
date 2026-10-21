#!/usr/bin/env python3
"""Measure a running lane's context: the input of its latest model call, read from its own transcript.

Usage: python ctx.py <lane> [--budget 150000] [--first]
  --first   the first call instead of the latest (the fixed cost: system prompt, tool schemas, the task)
Prints one line, e.g. "ctx 98k / 150k: ok" or "ctx 162k / 150k: OVER - write the Handoff, commit, return state
continue". Exit 0 when under the budget, 2 when over, 3 when the transcript is not found (keep working).

The fc4-lane workflow labels its agents "lane:<lane>" (a restart round: "lane:<lane>#<n>"); every workflow agent
has <projects>/<slug>/<session>/subagents/workflows/<run>/agent-<id>.jsonl with a .meta.json beside it whose
"description" is that label. The newest matching transcript is this lane's live one. A call's context is
input_tokens + cache_read_input_tokens + cache_creation_input_tokens of its usage (the GLM route reports it all as
input_tokens, the Anthropic route mostly as cache reads). Standard library only, read-only.
"""
import argparse
import glob
import json
import os
import sys
import time

TAIL = 512 * 1024  # bytes read from the end of the transcript; a usage line is a few hundred bytes


def find_transcript(lane, max_age_days=3):
    root = os.path.join(os.path.expanduser("~"), ".claude", "projects")
    cutoff = time.time() - max_age_days * 86400
    best = None
    for meta in glob.glob(os.path.join(root, "*", "*", "subagents", "workflows", "*", "agent-*.meta.json")):
        try:
            st = os.stat(meta)
        except OSError:
            continue
        if st.st_mtime < cutoff:
            continue
        try:
            with open(meta, encoding="utf-8") as f:
                desc = json.load(f).get("description") or ""
        except (OSError, ValueError):
            continue
        if desc != f"lane:{lane}" and not desc.startswith(f"lane:{lane}#"):
            continue
        tr = meta[: -len(".meta.json")] + ".jsonl"
        try:
            m = os.stat(tr).st_mtime
        except OSError:
            continue
        if best is None or m > best[0]:
            best = (m, tr)
    return best[1] if best else None


def call_context(path, first=False):
    size = os.path.getsize(path)
    with open(path, "rb") as f:
        f.seek(0 if first else max(0, size - TAIL))
        lines = f.read().decode("utf-8", errors="replace").splitlines()
    for line in (lines if first else reversed(lines)):
        if '"usage"' not in line:
            continue
        try:
            rec = json.loads(line)
        except ValueError:
            continue
        u = (rec.get("message") or {}).get("usage") or {}
        t = u.get("input_tokens", 0) + u.get("cache_read_input_tokens", 0) + u.get("cache_creation_input_tokens", 0)
        if t:
            return t
    return None


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("lane")
    ap.add_argument("--budget", type=int, default=150000)
    ap.add_argument("--first", action="store_true")
    a = ap.parse_args()
    tr = find_transcript(a.lane)
    if not tr:
        print(f"ctx: no transcript labelled lane:{a.lane} found; keep working")
        return 3
    t = call_context(tr, a.first)
    if t is None:
        print("ctx: no usage recorded yet; keep working")
        return 3
    if a.first:
        print(f"first call {t} tokens  ({tr})")
        return 0
    if t > a.budget:
        print(f"ctx {t // 1000}k / {a.budget // 1000}k: OVER - update the Handoff, commit, return state continue")
        return 2
    print(f"ctx {t // 1000}k / {a.budget // 1000}k: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
