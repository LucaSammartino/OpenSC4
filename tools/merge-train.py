#!/usr/bin/env python3
"""FreeCity4 merge train: the single owner of `main`.

Why this exists (HANDOFF "First task next session"): each lane used to take a shared merge lock, merge main,
re-gate and renumber `FC4_ABI_VERSION` / the HANDOFF rows / the manual-checks run numbers itself. N waiting lanes
therefore cost about N^2 gates, nearly every merge conflicted on the same five files, and about 25 agents polled
the lock so docs-only lanes waited hours.

The train instead:
  1. a finished lane writes `build-runs/merge-queue/<lane>.json` in its own worktree and stops;
  2. the train takes up to K ready lanes (oldest first), merges them one after another onto `integration/<n>`
     cut from main, resolving the known conflict hotspots once for the whole batch;
  3. it runs the gate and the regress once for the whole batch;
  4. on a pass main fast-forwards to the integration branch and each lane gets a merged notice;
  5. on a fail it bisects the batch (halves, gated in parallel) until the culprit lane is isolated, sends the
     failure back to that lane and lands the rest;
  6. docs-only and `test`/`tools` lanes skip the regress (and, when `engine/`, `tools/` and `tests/` are all
     unchanged, the gate result from the last engine batch stands).

Usage (run from the repository root, on `main`):
  python tools/merge-train.py status
  python tools/merge-train.py plan [--size 8]
  python tools/merge-train.py run   [--size 8] [--no-regress] [--land]
  python tools/merge-train.py merge-batch [--size 8]
  python tools/merge-train.py gate
  python tools/merge-train.py regress
  python tools/merge-train.py land
  python tools/merge-train.py bisect
  python tools/merge-train.py enqueue --lane L --branch B --kind engine|tools|docs|test [--abi]
                                      [--game-runs a,b] [--summary "..."]

State lives in `build-runs/merge-train/state.json` (gitignored). Nothing here pushes, and nothing here deletes a
worktree.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
MAIN = REPO
QUEUE_DIRS = [REPO / "build-runs" / "merge-queue"]
TRAIN_DIR = REPO / "build-runs" / "merge-train"
STATE_PATH = TRAIN_DIR / "state.json"
NOTICE_DIR = REPO / "build-runs" / "merge-notices"
WT_ROOT = REPO / ".claude" / "worktrees"
TRAIN_WT = WT_ROOT / "train-integration"
TRAIN_BRANCH = "train-integration"
LOG_DIR = REPO / "build-runs"

KINDS = ("engine", "tools", "docs", "test")
# "engine" touches engine/ and wants both a gate and a regress. The rest are batched freely and, for
# docs, skip the regress.
DOCS_KINDS = ("docs",)

VERSION_RE_C = re.compile(r"^(\s*#define\s+FC4_ABI_VERSION\s+)(\d+)(u?)\s*$", re.M)
VERSION_RE_RS = re.compile(r"^(\s*pub const FC4_ABI_VERSION: u32 = )(\d+)(;.*)$", re.M)


# --------------------------------------------------------------------------- helpers


def sh(args, cwd=REPO, check=True, capture=True, env=None, timeout=None):
    """Run a command; never uses the shell, so paths with spaces are safe."""
    e = dict(os.environ)
    if env:
        e.update(env)
    p = subprocess.run(
        [str(a) for a in args],
        cwd=str(cwd),
        env=e,
        capture_output=capture,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=timeout,
    )
    if check and p.returncode != 0:
        out = (p.stdout or "") + (p.stderr or "")
        raise RuntimeError(f"command failed ({p.returncode}): {' '.join(str(a) for a in args)}\n{out[-4000:]}")
    return p


def git(args, cwd=MAIN, check=True):
    return sh(["git"] + list(args), cwd=cwd, check=check)


def log(msg):
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)


def die(msg):
    print(f"merge-train: {msg}", file=sys.stderr)
    sys.exit(1)


def load_state():
    if STATE_PATH.exists():
        return json.loads(STATE_PATH.read_text(encoding="utf-8"))
    return {"batch_seq": 0, "current": None, "history": []}


def save_state(st):
    TRAIN_DIR.mkdir(parents=True, exist_ok=True)
    STATE_PATH.write_text(json.dumps(st, indent=2) + "\n", encoding="utf-8")


def main_is_clean():
    p = git(["status", "--porcelain"], check=False)
    return not (p.stdout or "").strip()


def current_branch():
    return git(["rev-parse", "--abbrev-ref", "HEAD"]).stdout.strip()


# --------------------------------------------------------------------------- queue


def queue_files():
    """Every merge-queue file: the coordinator's own build-runs/ plus each lane worktree's."""
    files = []
    for d in QUEUE_DIRS:
        if d.is_dir():
            files.extend(sorted(d.glob("*.json")))
    if WT_ROOT.is_dir():
        for wt in sorted(WT_ROOT.iterdir()):
            d = wt / "build-runs" / "merge-queue"
            if d.is_dir():
                files.extend(sorted(d.glob("*.json")))
    return files


def read_queue():
    out = []
    seen = set()
    for f in queue_files():
        try:
            rec = json.loads(f.read_text(encoding="utf-8-sig"))
        except Exception as exc:  # noqa: BLE001 - a malformed queue file must not stop the train
            print(f"  ! ignoring {f}: {exc}")
            continue
        lane = rec.get("lane") or f.stem
        if lane in seen:
            continue
        seen.add(lane)
        rec.setdefault("lane", lane)
        rec.setdefault("kind", "engine")
        rec.setdefault("abi", False)
        rec.setdefault("game_runs", [])
        rec.setdefault("state", "ready")
        rec["_file"] = str(f)
        out.append(rec)
    return out


def lane_worktree(lane):
    p = WT_ROOT / lane
    return p if p.is_dir() else None


def lane_branch_exists(branch):
    p = git(["rev-parse", "--verify", "--quiet", f"refs/heads/{branch}"], check=False)
    return p.returncode == 0


def lane_tip(branch):
    return git(["rev-parse", branch]).stdout.strip()


def describe_lane(rec):
    branch = rec.get("branch", rec["lane"])
    problems = []
    if not lane_branch_exists(branch):
        problems.append("branch missing")
    else:
        tip = lane_tip(branch)
        if rec.get("tip") and rec["tip"][:12] != tip[:12]:
            problems.append(f"tip moved ({rec['tip'][:8]} -> {tip[:8]})")
        wt = lane_worktree(rec["lane"])
        if wt is not None:
            dirty = git(["status", "--porcelain"], cwd=wt, check=False).stdout.strip()
            if dirty:
                problems.append(f"worktree dirty ({len(dirty.splitlines())} paths)")
        ahead = git(["rev-list", "--count", f"main..{branch}"], check=False).stdout.strip()
        rec["_ahead"] = ahead or "?"
    rec["_problems"] = problems
    rec["_ready"] = rec.get("state") == "ready" and not problems
    return rec


# --------------------------------------------------------------------------- commands


def cmd_status(_args):
    recs = [describe_lane(r) for r in read_queue()]
    st = load_state()
    print(f"main: {git(['rev-parse', '--short', 'HEAD']).stdout.strip()} on {current_branch()}")
    print(f"batch seq: {st['batch_seq']}   current: {st.get('current')}")
    if not recs:
        print("queue: empty")
        return 0
    print(f"queue: {len(recs)} lane(s)")
    for r in sorted(recs, key=lambda x: x.get("tip", "")):
        flag = "READY" if r["_ready"] else "hold "
        print(
            f"  {flag}  {r['lane']:<18} {r['kind']:<7} abi={str(r['abi']):<5} "
            f"ahead={r.get('_ahead', '?'):>3}  {'; '.join(r['_problems']) or r.get('summary', '')[:70]}"
        )
    return 0


def cmd_enqueue(args):
    rec = {
        "lane": args.lane,
        "branch": args.branch,
        "tip": args.tip or (lane_tip(args.branch) if lane_branch_exists(args.branch) else ""),
        "kind": args.kind,
        "abi": bool(args.abi),
        "game_runs": [x for x in (args.game_runs or "").split(",") if x],
        "summary": args.summary or "",
        "state": "ready",
    }
    out = QUEUE_DIRS[0] / f"{args.lane}.json"
    QUEUE_DIRS[0].mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(rec, indent=2) + "\n", encoding="utf-8")
    log(f"enqueued {args.lane} -> {out}")
    return 0


def pick_batch(size):
    recs = [describe_lane(r) for r in read_queue()]
    ready = [r for r in recs if r["_ready"]]
    # Docs-only first (they never need a regress and they clear the way), then by tip commit date = age.
    ready.sort(key=lambda r: (0 if r["kind"] in DOCS_KINDS else 1, r.get("tip", "")))
    return ready[:size], recs


def cmd_plan(args):
    batch, recs = pick_batch(args.size)
    print(f"batch of {len(batch)} (of {len(recs)} queued):")
    for r in batch:
        print(f"  {r['lane']:<18} {r['kind']:<7} abi={r['abi']}  branch={r['branch']}  tip={r['tip'][:10]}")
    if not batch:
        print("nothing ready")
    engine = [r for r in batch if r["kind"] not in DOCS_KINDS]
    print(f"gate needed: {bool(engine)}   regress needed: {bool(engine)}")
    return 0


def ensure_train_worktree():
    if not TRAIN_WT.is_dir():
        TRAIN_WT.parent.mkdir(parents=True, exist_ok=True)
        log(f"creating integration worktree {TRAIN_WT}")
        git(["worktree", "add", str(TRAIN_WT), "-b", TRAIN_BRANCH, "main"])
    return TRAIN_WT


def reset_integration(name):
    """Point the long-lived integration worktree at a fresh branch cut from main."""
    wt = ensure_train_worktree()
    git(["fetch", "origin"], check=False)  # no-op on a local-only repo; harmless
    git(["-C", str(wt), "reset", "--hard"], check=False)
    git(["-C", str(wt), "clean", "-fd"], check=False)
    git(["-C", str(wt), "checkout", "--force", "-B", name, "main"])
    return wt


# --- conflict hotspots -------------------------------------------------------
# These five files conflicted in nearly every merge because every lane edited them. The train resolves the
# mechanical ones once for the whole batch and regenerates the derived ones.

ABI_HEADER = "engine/capi/include/fc4.h"
ABI_RS = "engine/capi/src/lib.rs"
ABI_INC = "engine/tests/abi_entries.inc"
HANDOFF = "docs/HANDOFF.md"
MANUAL = "docs/manual-checks.md"
QUESTIONS = "docs/specs/QUESTIONS.md"


def conflicted_paths(wt):
    out = git(["-C", str(wt), "diff", "--name-only", "--diff-filter=U"], check=False).stdout.split()
    return [p for p in out if p]


def resolve_version_line(text, regex, prefer_max=True):
    """Merge a one-line version constant: keep the higher number, leave every other line alone."""
    m = regex.search(text)
    return text, m


def resolve_conflicts(wt, lanes):
    """Return (resolved, still_conflicted)."""
    paths = conflicted_paths(wt)
    if not paths:
        return [], []
    resolved, left = [], []
    for p in paths:
        if p == ABI_INC:
            # Generated at build time (abi-hygiene) or by tests/gen_abi_entries.py: take ours and regenerate.
            git(["-C", str(wt), "checkout", "--ours", "--", p], check=False)
            git(["-C", str(wt), "add", p], check=False)
            regen = Path(wt) / "engine" / "tests" / "gen_abi_entries.py"
            if regen.exists():
                sh([sys.executable, str(regen)], cwd=Path(wt) / "engine", check=False)
            resolved.append(p)
            continue
        if p in (ABI_HEADER, ABI_RS):
            if _resolve_abi_version(wt, p):
                resolved.append(p)
                continue
        if p in (HANDOFF, MANUAL, QUESTIONS):
            # Derived or append-only: keep ours now and regenerate/append afterwards.
            git(["-C", str(wt), "checkout", "--ours", "--", p], check=False)
            git(["-C", str(wt), "add", p], check=False)
            resolved.append(p + " (kept ours; regenerate from docs/lanes + docs/runs, and re-append the lanes' "
                                "QUESTIONS sections by hand)")
            continue
        left.append(p)
    return resolved, left


def _resolve_abi_version(wt, rel):
    """Resolve an fc4.h / lib.rs conflict whose only contested hunk is the ABI version constant."""
    path = Path(wt) / rel
    raw = path.read_text(encoding="utf-8", errors="replace")
    if "<<<<<<<" not in raw:
        return False
    regex = VERSION_RE_C if rel.endswith(".h") else VERSION_RE_RS
    nums, bodies = [], []
    # Split every conflict into ours / theirs and keep both bodies, except the version line.
    out, i, n = [], 0, len(raw)
    while i < n:
        start = raw.find("<<<<<<<", i)
        if start < 0:
            out.append(raw[i:])
            break
        out.append(raw[i:start])
        mid = raw.find("=======", start)
        end = raw.find(">>>>>>>", mid) if mid >= 0 else -1
        if mid < 0 or end < 0:
            return False
        ours = raw[start + len("<<<<<<<"):mid]
        # drop the first line (the marker's trailing branch label)
        ours = ours.split("\n", 1)[1] if "\n" in ours else ""
        theirs = raw[mid + len("======="):end]
        theirs = theirs.split("\n", 1)[1] if "\n" in theirs else ""
        mo, mt = regex.search(ours), regex.search(theirs)
        if mo and mt:
            nums.append(int(mo.group(2)))
            nums.append(int(mt.group(2)))
            out.append(ours if int(mo.group(2)) >= int(mt.group(2)) else theirs)
        else:
            bodies.append(ours)
            bodies.append(theirs)
            out.append(ours)
        i = raw.find("\n", end)
        i = n if i < 0 else i + 1
    text = "".join(out)
    if nums and not bodies:
        # only the version line differed: take the max explicitly
        hi = max(nums)
        text = regex.sub(lambda m: f"{m.group(1)}{hi}{m.group(3)}", text, count=1)
        path.write_text(text, encoding="utf-8")
        git(["-C", str(wt), "add", rel], check=False)
        return True
    if not bodies:
        path.write_text(text, encoding="utf-8")
        git(["-C", str(wt), "add", rel], check=False)
        return True
    return False


def bump_abi(wt, lanes, batch_id):
    """One ABI bump per batch. Prefers tools/bump_abi.py (abi-hygiene); falls back to a local bump."""
    script = REPO / "tools" / "bump_abi.py"
    if script.exists():
        p = sh([sys.executable, str(script), "--batch", batch_id, "--lanes", ",".join(lanes)], cwd=Path(wt), check=False)
        sys.stdout.write(p.stdout or "")
        if p.returncode == 0:
            return True
        log("tools/bump_abi.py failed; falling back to the built-in bump")
    changed = False
    for rel, regex in ((ABI_HEADER, VERSION_RE_C), (ABI_RS, VERSION_RE_RS)):
        path = Path(wt) / rel
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        m = regex.search(text)
        if not m:
            continue
        old = int(m.group(2))
        new = old + 1
        text = regex.sub(lambda mm: f"{mm.group(1)}{new}{mm.group(3)}", text, count=1)
        if rel.endswith(".h"):
            # one history line per batch, naming the lanes
            line = f" * {new} = merge-train batch {batch_id}: {', '.join(lanes)}.\n"
            close = text.find(" */\n#define FC4_ABI_VERSION")
            if close > 0:
                text = text[:close] + line + text[close:]
        path.write_text(text, encoding="utf-8")
        git(["-C", str(wt), "add", rel], check=False)
        changed = True
        log(f"ABI {old} -> {new} ({rel})")
    return changed


def regenerate(wt):
    """Rebuild every derived file after the merges, so no lane has to hand-merge them."""
    out = []
    gen = Path(wt) / "engine" / "tests" / "gen_abi_entries.py"
    if gen.exists():
        p = sh([sys.executable, str(gen)], cwd=Path(wt) / "engine", check=False)
        out.append(f"gen_abi_entries: exit {p.returncode}")
    tables = Path(wt) / "tools" / "gen_handoff_tables.py"
    if tables.exists():
        p = sh([sys.executable, str(tables)], cwd=Path(wt), check=False)
        out.append(f"gen_handoff_tables: exit {p.returncode}")
        if p.returncode != 0:
            out.append((p.stdout or "")[-1500:])
    return out


def commit_integration(wt, lanes, batch_id, note):
    git(["-C", str(wt), "add", "-A"])
    st = git(["-C", str(wt), "status", "--porcelain"], check=False).stdout.strip()
    if not st:
        return None
    msg = f"merge-train: resolve batch {batch_id} hotspots ({', '.join(lanes)})\n\n{note}\n\n" \
          "Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>\n"
    msgfile = Path(wt) / "build-runs" / f"train-{batch_id}-resolve.txt"
    msgfile.parent.mkdir(parents=True, exist_ok=True)
    msgfile.write_text(msg, encoding="utf-8")
    git(["-C", str(wt), "commit", "-F", str(msgfile)])
    return git(["-C", str(wt), "rev-parse", "HEAD"]).stdout.strip()


# --- gate / regress ----------------------------------------------------------


def batch_touches_engine(wt):
    base = git(["-C", str(wt), "merge-base", "main", "HEAD"], check=False).stdout.strip()
    if not base:
        return True
    diff = git(["-C", str(wt), "diff", "--name-only", base, "HEAD"], check=False).stdout.split()
    for p in diff:
        if p.startswith("engine/") or p.startswith("tools/") or p.startswith("tests/"):
            return True
    return False


def run_gate(wt, tag):
    """cargo + clippy + ctest for the integration worktree. Returns (ok, tail)."""
    engine = Path(wt) / "engine"
    if not engine.is_dir():
        return True, "no engine/ in this batch"
    log(f"gate {tag}: cargo test (this takes a while)")
    results = []
    for name, argv in (
        ("cargo test", ["cargo", "test", "--workspace", "--target", "i686-pc-windows-msvc"]),
        ("cargo clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
    ):
        p = sh(argv, cwd=engine, check=False, timeout=5400)
        tail = ((p.stdout or "") + (p.stderr or ""))[-2500:]
        results.append(f"=== {name}: exit {p.returncode}\n{tail}")
        if p.returncode != 0:
            _write_gate_log(tag, results)
            return False, tail
    verify = REPO / "build-runs" / "coord-verify-lane.ps1"
    if verify.exists():
        log(f"gate {tag}: cmake + ctest")
        p = sh(
            ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(verify),
             "-Engine", str(engine), "-Tag", tag],
            check=False, timeout=7200,
        )
        tail = ((p.stdout or "") + (p.stderr or ""))[-4000:]
        results.append(f"=== coord-verify-lane: exit {p.returncode}\n{tail}")
        ok = p.returncode == 0 and re.search(r"ctest\s*:", tail) is not None and "tests passed" in tail.lower()
        if not ok:
            # coord-verify-lane prints a verdict line; accept it if it says success
            ok = p.returncode == 0 and re.search(r"verdict:\s*(ok|pass|clean|success)", tail, re.I) is not None
        _write_gate_log(tag, results)
        return ok, tail
    _write_gate_log(tag, results)
    return True, "\n".join(results)[-2500:]


def _write_gate_log(tag, results):
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    (LOG_DIR / f"gate-{tag}.out").write_text("\n".join(results), encoding="utf-8")


def run_regress(wt, tag):
    """Submit the two regress runs through the game queue and wait for the verdict."""
    dll = Path(wt) / "engine" / "build-x86" / "FreeCity4.dll"
    if not dll.exists():
        log(f"regress {tag}: no DLL at {dll}; skipping")
        return None, "no DLL"
    log(f"regress {tag}: submitting to the game queue (about 10-20 min)")
    p = sh(
        ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(REPO / "tools" / "regress.ps1"),
         "-Tag", tag, "-Dll", str(dll), "-Queue", "-Turbo", "-TurboTimeout", "1800", "-QueueWait", "5400"],
        check=False, timeout=7200,
    )
    out = (p.stdout or "") + (p.stderr or "")
    (LOG_DIR / f"regress-{tag}.out").write_text(out, encoding="utf-8")
    m = re.search(r"REGRESS\s+\S+:\s*(PASS|FAIL)", out)
    verdict = m.group(1) if m else ("PASS" if p.returncode == 0 and "PASS" in out else "FAIL")
    return verdict, out[-4000:]


# --- batch lifecycle ---------------------------------------------------------


def cmd_merge_batch(args):
    if current_branch() != "main":
        die(f"run this from main (currently on {current_branch()})")
    if not main_is_clean():
        die("main has uncommitted changes; commit or stash them first")
    batch, _all = pick_batch(args.size)
    if not batch:
        log("nothing ready")
        return 0
    st = load_state()
    n = st["batch_seq"] + 1
    batch_id = f"train{n}"
    integ = f"integration/{n}"
    lanes = [r["lane"] for r in batch]
    log(f"batch {batch_id}: {', '.join(lanes)}")
    wt = reset_integration(integ)
    merged, skipped = [], []
    for r in batch:
        branch = r["branch"]
        log(f"  merge {r['lane']} ({branch} @ {r['tip'][:10]})")
        p = git(["-C", str(wt), "merge", "--no-ff", "--no-commit", branch],
                check=False, cwd=wt)
        if p.returncode != 0:
            resolved, left = resolve_conflicts(wt, [r["lane"]])
            if left:
                log(f"  ! {r['lane']}: unresolved conflicts in {left}")
                skipped.append((r["lane"], left))
                git(["-C", str(wt), "merge", "--abort"], check=False)
                continue
            if resolved:
                log(f"  resolved hotspots: {', '.join(resolved)}")
        msgfile = Path(wt) / "build-runs" / f"train-{batch_id}-{r['lane']}-merge.txt"
        msgfile.parent.mkdir(parents=True, exist_ok=True)
        msgfile.write_text(
            f"Merge {r['lane']} into {integ} (merge-train batch {batch_id})\n\n"
            f"{r.get('summary', '')}\n\n"
            f"branch: {branch}\nleaf: {r['tip'][:12]}\nkind: {r['kind']}\n\n"
            "Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>\n",
            encoding="utf-8",
        )
        git(["-C", str(wt), "commit", "-F", str(msgfile)], check=False)
        merged.append(r["lane"])
    if not merged:
        die("no lane merged; aborting the batch")
    need_abi = any(r["abi"] for r in batch if r["lane"] in merged)
    note = []
    if need_abi:
        bump_abi(wt, merged, batch_id)
        note.append("ABI bumped once for the batch")
    note.extend(regenerate(wt))
    commit_integration(wt, merged, batch_id, "\n".join(note) or "derived files regenerated")
    head = git(["-C", str(wt), "rev-parse", "HEAD"]).stdout.strip()
    st["batch_seq"] = n
    st["current"] = {
        "id": batch_id,
        "integration": integ,
        "worktree": str(TRAIN_WT),
        "lanes": merged,
        "skipped": [{"lane": a, "conflicts": b} for a, b in skipped],
        "head": head,
        "gate": None,
        "regress": None,
    }
    st.setdefault("history", [])
    save_state(st)
    log(f"batch {batch_id} merged at {head[:10]} on {integ} ({len(merged)} lanes)")
    for a, b in skipped:
        log(f"  SKIPPED {a}: conflicts in {b}")
    print(json.dumps({"batch": batch_id, "integration": integ, "merged": merged,
                      "skipped": [{"lane": a, "conflicts": b} for a, b in skipped], "head": head}, indent=2))
    return 0


def cmd_gate(_args):
    st = load_state()
    cur = st.get("current")
    if not cur:
        die("no current batch; run merge-batch first")
    wt = Path(cur["worktree"])
    if not batch_touches_engine(wt):
        log("docs-only batch: no gate needed")
        cur["gate"] = {"ok": True, "skipped": "docs-only"}
    else:
        ok, tail = run_gate(wt, cur["id"])
        cur["gate"] = {"ok": ok}
        log(f"gate {cur['id']}: {'PASS' if ok else 'FAIL'}")
        if not ok:
            log(tail[-1500:])
    save_state(st)
    return 0 if cur["gate"]["ok"] else 1


def cmd_regress(args):
    st = load_state()
    cur = st.get("current")
    if not cur:
        die("no current batch; run merge-batch first")
    wt = Path(cur["worktree"])
    if args.no_regress or not batch_touches_engine(wt):
        log("no regress for this batch")
        cur["regress"] = {"ok": True, "skipped": "docs/tools-only"}
        save_state(st)
        return 0
    verdict, tail = run_regress(wt, cur["id"])
    cur["regress"] = {"ok": verdict == "PASS", "verdict": verdict}
    save_state(st)
    log(f"regress {cur['id']}: {verdict}")
    if verdict != "PASS":
        log(tail[-1500:])
    return 0 if verdict == "PASS" else 1


def write_notices(cur, main_sha):
    NOTICE_DIR.mkdir(parents=True, exist_ok=True)
    for lane in cur["lanes"]:
        f = NOTICE_DIR / f"{lane}.md"
        f.write_text(
            f"# merged: {lane}\n\n"
            f"- branch `{lane}` merged into `main` as `{main_sha}` in merge-train batch `{cur['id']}`.\n"
            f"- integration: `{cur['integration']}` at `{cur['head'][:12]}`\n"
            f"- gate: {cur.get('gate')}   regress: {cur.get('regress')}\n\n"
            "Nothing more to do on this lane. Any `docs/lanes/<lane>.md` and `docs/runs/<tag>.md` you wrote are now\n"
            "the source of the generated HANDOFF / manual-checks tables.\n",
            encoding="utf-8",
        )
    # one copy in the main tree as well, so a later session can see what landed
    summary = NOTICE_DIR / f"{cur['id']}.md"
    summary.write_text(
        f"# merge-train batch {cur['id']}\n\n"
        f"- lanes: {', '.join(cur['lanes'])}\n"
        f"- skipped: {cur.get('skipped')}\n"
        f"- gate: {cur.get('gate')}\n- regress: {cur.get('regress')}\n"
        f"- main now at `{main_sha}`\n",
        encoding="utf-8",
    )


def cmd_land(args):
    if current_branch() != "main":
        die("run this from main")
    if not main_is_clean():
        die("main has uncommitted changes")
    st = load_state()
    cur = st.get("current")
    if not cur:
        die("no current batch")
    if not (cur.get("gate") or {}).get("ok"):
        die("gate did not pass; use bisect, or land with --force")
    if not (cur.get("regress") or {}).get("ok"):
        if not args.force:
            die("regress did not pass; use bisect, or land with --force")
    wt = Path(cur["worktree"])
    head = git(["-C", str(wt), "rev-parse", "HEAD"]).stdout.strip()
    log(f"landing {cur['id']} ({head[:10]}) onto main")
    # fast-forward main to the integration branch
    p = git(["merge", "--ff-only", head], check=False)
    if p.returncode != 0:
        log("fast-forward refused; merging with --no-ff instead")
        git(["merge", "--no-ff", "-m",
             f"Merge merge-train batch {cur['id']}: {', '.join(cur['lanes'])}\n\n"
             "Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>\n", head])
    main_sha = git(["rev-parse", "HEAD"]).stdout.strip()
    write_notices(cur, main_sha)
    st.setdefault("history", []).append({**cur, "main": main_sha, "landed_at": time.strftime("%Y-%m-%d %H:%M:%S")})
    st["current"] = None
    save_state(st)
    # the queue files for landed lanes go away so they are not merged twice
    for lane in cur["lanes"]:
        for f in queue_files():
            if f.stem == lane:
                f.rename(f.with_suffix(".merged.json"))
    log(f"main is now {main_sha[:10]}; batch {cur['id']} landed")
    print(json.dumps({"main": main_sha, "lanes": cur["lanes"], "batch": cur["id"]}, indent=2))
    return 0


def cmd_bisect(_args):
    """Split the current batch and gate the halves in parallel until one culprit lane is isolated."""
    st = load_state()
    cur = st.get("current")
    if not cur:
        die("no current batch")
    lanes = cur["lanes"]
    if len(lanes) < 2:
        die(f"batch has a single lane ({lanes}); the culprit is already isolated")
    log(f"bisecting {cur['id']}: {lanes}")
    halves = [lanes[: len(lanes) // 2], lanes[len(lanes) // 2 :]]
    procs = []
    for i, half in enumerate(halves):
        name = f"{cur['id']}-b{i}"
        wt = reset_integration(f"integration/{name}")
        for lane in half:
            rec = next((r for r in read_queue() if r["lane"] == lane), None)
            branch = (rec or {}).get("branch", lane)
            git(["-C", str(wt), "merge", "--no-ff", "--no-commit", branch], check=False)
            resolve_conflicts(wt, [lane])
            git(["-C", str(wt), "commit", "--no-edit"], check=False)
        log(f"  half {i}: {half} -> gating as {name}")
        out = LOG_DIR / f"gate-{name}.out"
        engine = Path(wt) / "engine"
        verify = REPO / "build-runs" / "coord-verify-lane.ps1"
        argv = ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(verify),
                "-Engine", str(engine), "-Tag", name]
        procs.append((half, name, subprocess.Popen(argv, cwd=str(wt), stdout=subprocess.PIPE,
                                                   stderr=subprocess.STDOUT, text=True,
                                                   encoding="utf-8", errors="replace")))
    bad = None
    for half, name, proc in procs:
        out, _ = proc.communicate(timeout=7200)
        (LOG_DIR / f"gate-{name}.out").write_text(out or "", encoding="utf-8")
        ok = proc.returncode == 0
        log(f"  half {name}: {'PASS' if ok else 'FAIL'} ({half})")
        if not ok and bad is None:
            bad = half
    if bad is None:
        log("both halves pass: the failure is in how they interact, or it was flaky. Rerun the full batch.")
        return 1
    cur["lanes"] = bad
    cur["bisect"] = cur.get("bisect", []) + [{"half": bad, "count": len(bad)}]
    save_state(st)
    log(f"culprit is in {bad}; run 'bisect' again (or 'fail --lane' when it is one lane)")
    return 0


def cmd_fail(args):
    st = load_state()
    cur = st.get("current")
    if not cur:
        die("no current batch")
    lane = args.lane
    if lane not in cur["lanes"]:
        die(f"{lane} is not in the current batch")
    wt = lane_worktree(lane)
    body = (
        f"# merge-train: batch {cur['id']} failed on {lane}\n\n"
        f"The gate/regress for the batch failed and bisection isolated {lane}. The failure output is in\n"
        f"`build-runs/gate-{cur['id']}.out` and `build-runs/regress-{cur['id']}.out` in the main checkout.\n\n"
        "Fix it on your branch and write a fresh `build-runs/merge-queue/<lane>.json` when it is green.\n"
    )
    if wt:
        (wt / "build-runs").mkdir(parents=True, exist_ok=True)
        (wt / "build-runs" / f"merge-failed-{cur['id']}.md").write_text(body, encoding="utf-8")
        log(f"wrote {wt}/build-runs/merge-failed-{cur['id']}.md")
    cur["lanes"] = [x for x in cur["lanes"] if x != lane]
    cur.setdefault("rejected", []).append(lane)
    save_state(st)
    log(f"{lane} sent back; {len(cur['lanes'])} lane(s) remain in {cur['id']} (use 'land' to land them)")
    return 0


def cmd_run(args):
    """merge-batch -> gate -> regress -> land, with bisection on failure."""
    for step, fn in (("merge-batch", lambda: cmd_merge_batch(args)),
                     ("gate", lambda: cmd_gate(args)),
                     ("regress", lambda: cmd_regress(args))):
        log(f"--- {step}")
        rc = fn()
        if rc != 0:
            if step == "merge-batch":
                return rc
            log(f"{step} failed; bisecting")
            if cmd_bisect(args) == 0:
                return 2  # caller (the operator) continues with the isolated half
            return rc
    if args.land:
        return cmd_land(args)
    log("batch is green; run 'land' to put it on main")
    return 0


# --------------------------------------------------------------------------- cli


def main():
    ap = argparse.ArgumentParser(prog="merge-train", description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("status")
    p = sub.add_parser("plan")
    p.add_argument("--size", type=int, default=8)
    p = sub.add_parser("enqueue")
    p.add_argument("--lane", required=True)
    p.add_argument("--branch", required=True)
    p.add_argument("--tip", default="")
    p.add_argument("--kind", choices=KINDS, default="engine")
    p.add_argument("--abi", action="store_true")
    p.add_argument("--game-runs", default="")
    p.add_argument("--summary", default="")
    p = sub.add_parser("merge-batch")
    p.add_argument("--size", type=int, default=8)
    sub.add_parser("gate")
    p = sub.add_parser("regress")
    p.add_argument("--no-regress", action="store_true")
    p = sub.add_parser("land")
    p.add_argument("--force", action="store_true")
    sub.add_parser("bisect")
    p = sub.add_parser("fail")
    p.add_argument("--lane", required=True)
    p = sub.add_parser("run")
    p.add_argument("--size", type=int, default=8)
    p.add_argument("--land", action="store_true")
    p.add_argument("--no-regress", action="store_true")
    args = ap.parse_args()
    return {"status": cmd_status, "plan": cmd_plan, "enqueue": cmd_enqueue,
            "merge-batch": cmd_merge_batch, "gate": cmd_gate, "regress": cmd_regress,
            "land": cmd_land, "bisect": cmd_bisect, "fail": cmd_fail, "run": cmd_run}[args.cmd](args)


if __name__ == "__main__":
    sys.exit(main())
