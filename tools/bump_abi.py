#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
#
# FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
# Copyright (C) 2026 FreeCity4 contributors
#
# This program is free software: you can redistribute it and/or modify it
# under the terms of the GNU General Public License as published by the Free
# Software Foundation, either version 3 of the License, or (at your option)
# any later version. This program is distributed WITHOUT ANY WARRANTY; see
# the GNU General Public License for more details. You should have received a
# copy of the license along with this program (see LICENSE at the repository
# root); if not, see <https://www.gnu.org/licenses/>.

"""Bumps FC4_ABI_VERSION once per merge-train batch.

Every lane that changes the C ABI used to bump FC4_ABI_VERSION on its own
branch, so five lanes edited engine/capi/include/fc4.h at the same lines and
every merge conflicted. The merge train now bumps it once per batch instead:
run this script with the batch id and the names of the lanes in the batch, and
it increments the number by exactly one and writes one history line naming the
batch and all of its lanes.

Usage:
    python tools/bump_abi.py --batch <batch-id> --lanes laneA,laneB,laneC [--dry-run]
    python tools/bump_abi.py --self-test

Options:
    --batch ID     required; the merge-train batch being sealed (e.g. "re-answers25").
    --lanes LIST   required; comma-separated lane names in the batch. Order is kept
                   as given and every name is written to the history line.
    --note TEXT    optional; the rest of the history sentence. A default that
                   names the batch and its lanes is used when omitted.
    --dry-run      print what would change and change nothing.
    --force        allow running with a dirty working tree in fc4.h.
    --record PATH  batch record (default <repo>/build-runs/abi-batches.json).
    --self-test    run the checks against a temporary copy of the tree.

Safety:
  * refuses to run if fc4.h has uncommitted changes (use --force to override),
    so a bump never lands on top of a lane's uncommitted edit;
  * refuses to run twice for the same --batch, recording applied batches in
    build-runs/abi-batches.json (gitignored). Create that file if missing.
    It is per-worktree, so run the bump in the tree that merges the batch;
    pass --record to share one file across worktrees.
  * does NOT run git and does NOT commit. It edits files in place.

FC4_ABI_VERSION mirrors: the number is written in more than one place. This
script finds every one of them and rewrites each; the output lists what it
found. As of this writing the mirrors are engine/capi/include/fc4.h (the
#define) and engine/capi/src/lib.rs (the Rust const). Everything else refers to
the macro or the const by name and needs no edit; those are counted in the
output too. A new mirror that hardcodes the number is picked up by the scan
below - please keep the number only in the two places it is meant to live.
"""

import argparse
import json
import os
import re
import shutil
import sys
import tempfile
from datetime import datetime, timezone

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)

# Where FC4_ABI_VERSION is spelled out as a number, relative to the repo root.
# The scan (scan_mirrors) is what actually decides; these are the known homes
# and are used by --self-test as the expected set.
MIRROR_PATHS = (
    "engine/capi/include/fc4.h",
    "engine/capi/src/lib.rs",
)

# Lines that DEFINE or ASSIGN the version number. Anything else that mentions
# the symbol (a comparison, a doc comment, "FC4_ABI_VERSION 64): the generator")
# is a reference, not a mirror. Each pattern captures the number in group 2 so
# the rewrite is exact - `u32` in `FC4_ABI_VERSION: u32 = 68;` is a type, never
# the version.
#
#   #define FC4_ABI_VERSION 68u
#   pub const FC4_ABI_VERSION: u32 = 68;
#   FC4_ABI_VERSION = 68
#   "FC4_ABI_VERSION": 68
RE_MIRROR_PATTERNS = (
    re.compile(r"(#\s*define\s+FC4_ABI_VERSION\s+)(\d+)"),
    re.compile(r"(\bFC4_ABI_VERSION\b(?:\s*:\s*[A-Za-z_][\w:<>]*)?\s*=\s*)(\d+)"),
    re.compile(r"([\"']?FC4_ABI_VERSION[\"']?\s*:\s*)(\d+)"),
)

HISTORY_START = re.compile(r"/\*\s*ABI history\s*:")
DEFINE_LINE = re.compile(r"^(\s*#\s*define\s+FC4_ABI_VERSION\s+)(\d+)([uU]?)\s*$")
# ` * 68 = adds the prop manager ...` (the entry's first line)
HISTORY_ENTRY = re.compile(r"^\s*\*\s*(\d+)\s*=\s*")


def die(msg):
    sys.stderr.write("bump_abi: %s\n" % msg)
    return 2


def read(path):
    with open(path, "r", encoding="utf-8", newline="") as f:
        return f.read()


def write(path, text):
    with open(path, "w", encoding="utf-8", newline="") as f:
        f.write(text)


def find_repo(start=None):
    """The repo root: the directory holding engine/capi/include/fc4.h."""
    d = os.path.abspath(start or REPO)
    for _ in range(8):
        if os.path.isfile(os.path.join(d, "engine", "capi", "include", "fc4.h")):
            return d
        parent = os.path.dirname(d)
        if parent == d:
            break
        d = parent
    return None


def parse_version(fc4_text):
    """The FC4_ABI_VERSION number and the index of its #define line."""
    lines = fc4_text.split("\n")
    for i, line in enumerate(lines):
        m = DEFINE_LINE.match(line)
        if m:
            return int(m.group(2)), i
    raise ValueError("no `#define FC4_ABI_VERSION <n>` in fc4.h")


def match_mirror(line, version):
    """The mirror match on one line, or None.

    Returns (pattern_index, match). A match whose number is not `version` is
    still returned: the caller reports it rather than silently rewriting a
    stale literal.
    """
    for i, rx in enumerate(RE_MIRROR_PATTERNS):
        m = rx.search(line)
        if m:
            return i, m
    return None


def scan_mirrors(repo, version):
    """Every file that mentions FC4_ABI_VERSION.

    Returns (mirrors, references):
      mirrors    list of (relpath, lineno, line, pattern_index, match) for the
                 lines that define or assign the number;
      references list of (relpath, lineno, line) for the symbolic mentions.
    """
    mirrors, references = [], []
    roots = [os.path.join(repo, "engine"), os.path.join(repo, "docs")]
    for root in roots:
        if not os.path.isdir(root):
            continue
        for dirpath, dirnames, filenames in os.walk(root):
            dirnames[:] = sorted(
                d for d in dirnames
                if d not in ("build-x86", "build-x86-checked", "target", "__pycache__", "cargo")
            )
            for name in sorted(filenames):
                if not name.endswith((".h", ".hpp", ".c", ".cpp", ".rs", ".py", ".md", ".txt", ".inc", ".tsv")):
                    continue
                path = os.path.join(dirpath, name)
                rel = os.path.relpath(path, repo).replace("\\", "/")
                try:
                    text = read(path)
                except (OSError, UnicodeDecodeError):
                    continue
                for i, line in enumerate(text.split("\n")):
                    if "FC4_ABI_VERSION" not in line:
                        continue
                    hit = match_mirror(line, version)
                    if hit:
                        mirrors.append((rel, i + 1, line, hit[0], hit[1]))
                    else:
                        references.append((rel, i + 1, line.strip()))
    return mirrors, references


def history_block_bounds(fc4_text):
    """(start, end) line indices of the `/* ABI history: ... */` comment."""
    lines = fc4_text.split("\n")
    define_i = None
    for i, line in enumerate(lines):
        if DEFINE_LINE.match(line):
            define_i = i
            break
    if define_i is None:
        raise ValueError("no `#define FC4_ABI_VERSION <n>` in fc4.h")
    start = None
    for i in range(define_i - 1, -1, -1):
        if HISTORY_START.search(lines[i]):
            start = i
            break
    if start is None:
        raise ValueError("the `/* ABI history: */` block above FC4_ABI_VERSION was not found")
    return start, define_i


def last_history_entry(fc4_text):
    """(entry_number, first_line_index) of the newest entry in the history block."""
    start, end = history_block_bounds(fc4_text)
    lines = fc4_text.split("\n")
    found = None
    for i in range(start, end):
        m = HISTORY_ENTRY.match(lines[i])
        if m:
            found = (int(m.group(1)), i)
    if found is None:
        raise ValueError("the ABI history block has no `N = ...` entries")
    return found


def history_closing_index(fc4_text):
    """Index of the line that closes the `/* ABI history: ... */` comment.

    That line is the last line of the newest entry (`... renumbered at merge). */`)
    and is the one that must lose its ` */` when a new entry is appended.
    """
    start, end = history_block_bounds(fc4_text)
    lines = fc4_text.split("\n")
    for i in range(end - 1, start - 1, -1):
        if lines[i].strip():
            if "*/" not in lines[i]:
                raise ValueError("the ABI history block does not close with `*/`: %r" % lines[i])
            return i
    raise ValueError("the ABI history block is empty")


def wrap_history(text, first_prefix, cont_prefix, width=96):
    """Wrap a history sentence the way the block already does."""
    words = text.split()
    out, cur = [], first_prefix
    for w in words:
        if cur.strip() and len(cur) + 1 + len(w) > width:
            out.append(cur.rstrip())
            cur = cont_prefix + w
        else:
            cur = (cur + " " + w) if cur.strip() else (cur + w)
    out.append(cur.rstrip())
    return out


def build_history_lines(new_version, batch, lanes, note):
    """The new ` * 69 = ...` entry, ending in `. */`."""
    lane_txt = ", ".join(lanes)
    sentence = "%s (lanes %s): %s" % (batch, lane_txt, note)
    body = "%d = %s" % (new_version, sentence)
    lines = wrap_history(body, " * ", " *      ")
    # The block's last line closes the comment.
    lines[-1] = lines[-1].rstrip() + ". */"
    return lines


def close_previous_entry(last_line):
    """Turn the block's last line from `... . */` into `... ;` so the new entry
    can follow it. The sentence's own full stop becomes the separator."""
    stripped = last_line.rstrip()
    m = re.search(r"\*/\s*$", stripped)
    if not m:
        raise ValueError("the last ABI history line does not close the comment: %r" % last_line)
    body = stripped[: m.start()].rstrip()
    if body.endswith("."):
        body = body[:-1]
    if not body.endswith(";"):
        body += ";"
    return body


def apply_bump(fc4_text, batch, lanes, note):
    """New fc4.h text: version + 1, one history line appended.

    Returns (new_text, old_version, new_version, history_lines). Pure: the
    caller writes the file, so --dry-run and --self-test can run it freely.
    """
    old_version, _ = parse_version(fc4_text)
    new_version = old_version + 1
    lines = fc4_text.split("\n")
    start, end = history_block_bounds(fc4_text)
    close_i = history_closing_index(fc4_text)
    lines[close_i] = close_previous_entry(lines[close_i])
    entry = build_history_lines(new_version, batch, lanes, note)
    lines[end:end] = entry
    # The #define itself sits at `end` before the insert; shift and rewrite it.
    define_i = end + len(entry)
    m = DEFINE_LINE.match(lines[define_i])
    if not m:
        raise ValueError("FC4_ABI_VERSION #define moved unexpectedly")
    lines[define_i] = "%s%d%s" % (m.group(1), new_version, m.group(3))
    return "\n".join(lines), old_version, new_version, entry


def replace_mirror_num(line, pattern_index, old_version, new_version):
    """Rewrite the captured number on one mirror line.

    Returns (new_line, changed, found_version). `changed` is False when the
    line's number was not old_version (a stale mirror) or when the line has no
    number at all; `found_version` is what was there.
    """
    rx = RE_MIRROR_PATTERNS[pattern_index]
    m = rx.search(line)
    if not m:
        return line, False, None
    found = int(m.group(2))
    if found != old_version:
        return line, False, found
    new_line = line[: m.start(2)] + str(new_version) + line[m.end(2) :]
    return new_line, True, found


def default_note(batch, lanes):
    return (
        "the batch's ABI changes (one bump per merge-train batch rather than one "
        "per lane; see the batch's lanes for the individual entries)"
    )


def load_record(path):
    if not os.path.isfile(path):
        return {"batches": []}
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict) or "batches" not in data:
        raise ValueError("%s is not a bump_abi batch record" % path)
    return data


def save_record(path, data):
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, sort_keys=False)
        f.write("\n")


def git_dirty(repo, relpath):
    """True when `git status` reports uncommitted work on relpath. Absent git
    or an untracked file is treated as clean (nothing to protect)."""
    import subprocess

    try:
        r = subprocess.run(
            ["git", "status", "--porcelain", "--", relpath],
            cwd=repo,
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (OSError, subprocess.SubprocessError):
        return False
    return bool(r.stdout.strip())


def parse_lanes(raw):
    lanes = [x.strip() for x in raw.split(",")]
    lanes = [x for x in lanes if x]
    if not lanes:
        raise ValueError("--lanes needs at least one name")
    for x in lanes:
        if not re.match(r"^[A-Za-z0-9][A-Za-z0-9._-]*$", x):
            raise ValueError("lane name %r is not a plain identifier" % x)
    # keep order, drop duplicates
    seen, out = set(), []
    for x in lanes:
        if x not in seen:
            seen.add(x)
            out.append(x)
    return out


def do_bump(repo, batch, lanes, note, dry_run, force, record_path):
    fc4 = os.path.join(repo, "engine", "capi", "include", "fc4.h")
    if not os.path.isfile(fc4):
        return die("fc4.h not found at %s" % fc4)

    # --- guards -------------------------------------------------------------
    if not force and git_dirty(repo, "engine/capi/include/fc4.h"):
        return die(
            "fc4.h has uncommitted changes; commit or stash them first, or pass --force\n"
            "         (a bump on top of a lane's edit would mix two things in one commit)"
        )
    record = load_record(record_path)
    for b in record.get("batches", []):
        if b.get("batch") == batch:
            return die(
                "batch %r was already applied (ABI %s -> %s on %s); refusing to bump twice\n"
                "         record: %s" % (batch, b.get("old"), b.get("new"), b.get("applied_at"), record_path)
            )

    # --- what changes ------------------------------------------------------
    fc4_text = read(fc4)
    try:
        new_fc4, old_version, new_version, entry = apply_bump(fc4_text, batch, lanes, note)
    except ValueError as e:
        return die(str(e))

    mirrors, references = scan_mirrors(repo, old_version)
    rewrites = []
    for rel, lineno, line, pat_i, m in mirrors:
        new_line, changed, found = replace_mirror_num(line, pat_i, old_version, new_version)
        rewrites.append((rel, lineno, line, new_line, changed, found))

    # --- report ------------------------------------------------------------
    print("FC4_ABI_VERSION %d -> %d" % (old_version, new_version))
    print("batch: %s" % batch)
    print("lanes: %s" % ", ".join(lanes))
    print("history line(s) to append to engine/capi/include/fc4.h:")
    for l in entry:
        print("  %s" % l)
    print("mirrors found (%d):" % len(rewrites))
    for rel, lineno, old_line, new_line, changed, found in rewrites:
        if changed:
            print("  %-34s:%-5d updated" % (rel, lineno))
            print("      - %s" % old_line.strip())
            print("      + %s" % new_line.strip())
        else:
            print("  %-34s:%-5d NOT UPDATED (found %s)" % (rel, lineno, found))
            print("      ! %s" % old_line.strip())
    print("symbolic references (no number to change): %d" % len(references))
    for rel, lineno, line in references:
        print("  %-34s:%-5d %s" % (rel, lineno, line[:100]))

    stale = [r for r in rewrites if not r[4]]
    if stale:
        print("warning: %d mirror(s) did not hold version %d; fix them by hand" % (len(stale), old_version))

    if dry_run:
        print("dry run: nothing written")
        return 0

    # --- write -------------------------------------------------------------
    write(fc4, new_fc4)
    for rel, lineno, old_line, new_line, changed, found in rewrites:
        if not changed or rel == "engine/capi/include/fc4.h":
            continue  # fc4.h was written whole above
        path = os.path.join(repo, rel)
        text = read(path)
        lines = text.split("\n")
        if lines[lineno - 1] != old_line:
            return die("%s:%d changed under us; aborting" % (rel, lineno))
        lines[lineno - 1] = new_line
        write(path, "\n".join(lines))

    record.setdefault("batches", []).append(
        {
            "batch": batch,
            "lanes": lanes,
            "old": old_version,
            "new": new_version,
            "applied_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "note": note,
        }
    )
    save_record(record_path, record)
    print("recorded batch in %s" % record_path)
    print("FC4_ABI_VERSION is now %d. Commit the files above; do not bump again for this batch." % new_version)
    return 0


# --- self-test ---------------------------------------------------------------

SAMPLE_HISTORY = """\
/* ABI history: 1 = initial; 2 = adds fc4_sim_on_post_city_init;
 * 3 = adds fc4_sim_get_debug and fc4_sim_get_agent_info (diagnostics);
 * 67 = adds the disaster layer's mirror tooling (fc4_disasters_set_mirror_random,
 *      fc4_disasters_get_location, fc4_disasters_riot_locked, fc4_disasters_fire_ids): the plugin's
 *      disasters_shadow (a same-process differential of the original layer) (taken as 61, then 62
 *      to 66, on its lane; renumbered at merge);
 * 68 = adds the prop manager, lot configuration manager and tutorial system (fc4_prop_manager,
 *      fc4_lot_config, fc4_tutorial; prop-lotconfig.md, tutorial.md) (taken as 70 on its lane;
 *      renumbered at merge). */
#define FC4_ABI_VERSION 68u
"""


def self_test():
    failures = []

    def check(what, got, want):
        if got != want:
            failures.append("%s: got %r, want %r" % (what, got, want))
            print("  FAIL %s: got %r, want %r" % (what, got, want))
        else:
            print("  ok   %s" % what)

    print("self-test: bump_abi.py")

    # --- pure functions ---
    v, i = parse_version(SAMPLE_HISTORY)
    check("parse_version", v, 68)
    n, li = last_history_entry(SAMPLE_HISTORY)
    check("last_history_entry number", n, 68)
    check("last_history_entry first line", li > 0, True)
    close_i = history_closing_index(SAMPLE_HISTORY)
    check("history_closing_index", SAMPLE_HISTORY.split("\n")[close_i].rstrip().endswith(". */"), True)
    check("closes previous entry", close_previous_entry(SAMPLE_HISTORY.split("\n")[close_i]),
          " *      renumbered at merge);")

    new_text, old_v, new_v, entry = apply_bump(
        SAMPLE_HISTORY, "re-answers25", ["traffic", "lotdev", "weather"],
        default_note("re-answers25", ["traffic", "lotdev", "weather"]),
    )
    check("version incremented", (old_v, new_v), (68, 69))
    check("#define rewritten", "#define FC4_ABI_VERSION 69u" in new_text, True)
    check("old #define gone", "#define FC4_ABI_VERSION 68u" in new_text, False)
    check("previous entry closed", "renumbered at merge);" in new_text, True)
    check("comment still closes", new_text.rstrip().endswith("#define FC4_ABI_VERSION 69u"), True)
    check("entry names the batch", any("re-answers25" in l for l in entry), True)
    check("entry names every lane", any("traffic" in l and "lotdev" in l and "weather" in l for l in entry), True)
    check("entry ends the comment", entry[-1].rstrip().endswith(". */"), True)
    check("only one new entry", sum(1 for l in entry if HISTORY_ENTRY.match(l)), 1)
    # Idempotence of the pure step: applying twice to the same input is stable.
    again = apply_bump(SAMPLE_HISTORY, "re-answers25", ["a"], "x")[0]
    check("pure step is deterministic", again, apply_bump(SAMPLE_HISTORY, "re-answers25", ["a"], "x")[0])
    # Every history line inside the block still parses.
    check("history block still closed", " */" in new_text, True)

    # --- mirror rewrite ---
    # (source, pattern index, want_line, want_changed, want_found)
    cases = (
        ("#define FC4_ABI_VERSION 68u", 0, "#define FC4_ABI_VERSION 69u", True, 68),
        ("pub const FC4_ABI_VERSION: u32 = 68;", 1, "pub const FC4_ABI_VERSION: u32 = 69;", True, 68),
        ("FC4_ABI_VERSION = 68", 1, "FC4_ABI_VERSION = 69", True, 68),
        ('"FC4_ABI_VERSION": 68', 2, '"FC4_ABI_VERSION": 69', True, 68),
        # stale: a mirror holding some other number must not be rewritten
        ("pub const FC4_ABI_VERSION: u32 = 67;", 1, "pub const FC4_ABI_VERSION: u32 = 67;", False, 67),
    )
    for src, pat_i, want, want_ok, want_found in cases:
        got, ok, found = replace_mirror_num(src, pat_i, 68, 69)
        check("mirror %r" % src[:40], (got, ok, found), (want, want_ok, want_found))
    # symbolic: no definition/assignment shape, so nothing to rewrite
    check(
        "symbolic ref is not a mirror",
        match_mirror("CHECK(fc4_abi_version() == FC4_ABI_VERSION);", 68),
        None,
    )
    check(
        "doc prose is not a mirror",
        match_mirror(" * §6.3, FC4_ABI_VERSION 64): the generator", 68),
        None,
    )

    # --- end to end on a temp tree ---
    tmp = tempfile.mkdtemp(prefix="bump_abi_selftest_")
    try:
        for rel, body in (
            ("engine/capi/include/fc4.h", SAMPLE_HISTORY),
            ("engine/capi/src/lib.rs", "pub const FC4_ABI_VERSION: u32 = 68;\n"),
            ("engine/tests/capi_smoke.cpp", "CHECK(fc4_abi_version() == FC4_ABI_VERSION);\n"),
        ):
            p = os.path.join(tmp, rel.replace("/", os.sep))
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p, "w", encoding="utf-8") as f:
                f.write(body)
        rec = os.path.join(tmp, "build-runs", "abi-batches.json")
        rc = do_bump(tmp, "re-answers25", ["traffic", "lotdev"], "a note", False, True, rec)
        check("do_bump exit", rc, 0)
        fc4 = read(os.path.join(tmp, "engine", "capi", "include", "fc4.h"))
        check("e2e version", parse_version(fc4)[0], 69)
        check("e2e rust mirror", "FC4_ABI_VERSION: u32 = 69;" in read(
            os.path.join(tmp, "engine", "capi", "src", "lib.rs")), True)
        check("e2e symbolic ref untouched", "FC4_ABI_VERSION);" in read(
            os.path.join(tmp, "engine", "tests", "capi_smoke.cpp")), True)
        check("e2e record written", os.path.isfile(rec), True)
        check("e2e record has the batch", load_record(rec)["batches"][0]["batch"], "re-answers25")
        # A second run for the same batch must refuse.
        rc2 = do_bump(tmp, "re-answers25", ["traffic", "lotdev"], "again", False, True, rec)
        check("e2e refuses a second bump", rc2, 2)
        check("e2e version unchanged after refuse", parse_version(
            read(os.path.join(tmp, "engine", "capi", "include", "fc4.h")))[0], 69)
        # A different batch still works.
        rc3 = do_bump(tmp, "next-batch", ["a"], "another", False, True, rec)
        check("e2e next batch ok", rc3, 0)
        check("e2e next version", parse_version(
            read(os.path.join(tmp, "engine", "capi", "include", "fc4.h")))[0], 70)
        check("e2e record has two batches", len(load_record(rec)["batches"]), 2)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    print("self-test: %d check(s) failed" % len(failures))
    for f in failures:
        print("  FAILED: %s" % f)
    return 1 if failures else 0


def main(argv=None):
    # Windows consoles default to a legacy code page and die on `§` in fc4.h.
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            try:
                stream.reconfigure(encoding="utf-8", errors="replace")
            except (OSError, ValueError):
                pass
    ap = argparse.ArgumentParser(
        prog="bump_abi.py",
        description="Bump FC4_ABI_VERSION once per merge-train batch.",
    )
    ap.add_argument("--batch", help="merge-train batch id being sealed")
    ap.add_argument("--lanes", help="comma-separated lane names in the batch")
    ap.add_argument("--note", help="rest of the history sentence (a default names the batch and lanes)")
    ap.add_argument("--dry-run", action="store_true", help="print the change and write nothing")
    ap.add_argument("--force", action="store_true", help="allow a dirty fc4.h")
    ap.add_argument("--record", help="batch record JSON (default <repo>/build-runs/abi-batches.json)")
    ap.add_argument("--self-test", action="store_true", help="run the checks on a temporary copy")
    args = ap.parse_args(argv)

    if args.self_test:
        return self_test()

    if not args.batch or not args.lanes:
        ap.error("--batch and --lanes are required (or pass --self-test)")

    try:
        lanes = parse_lanes(args.lanes)
    except ValueError as e:
        return die(str(e))

    repo = find_repo()
    if repo is None:
        return die("engine/capi/include/fc4.h not found above %s" % REPO)

    record_path = args.record or os.path.join(repo, "build-runs", "abi-batches.json")
    note = args.note or default_note(args.batch, lanes)
    return do_bump(repo, args.batch, lanes, note, args.dry_run, args.force, record_path)


if __name__ == "__main__":
    sys.exit(main())
