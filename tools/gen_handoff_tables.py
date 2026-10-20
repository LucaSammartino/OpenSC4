#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Regenerate the generated tables that used to be hand-edited by every lane at merge time.

Canonical tables (the only ones this script owns):

  1. docs/lanes/INDEX.md, between ``<!-- BEGIN GENERATED: lanes -->`` and ``<!-- END GENERATED -->``:
     the four-column lane table::

         | Lane | Agent/worktree | State at stop | Next steps |

     This is the one canonical lane table. It lived in HANDOFF.md section 0 until 2026-10-20 and
     moved to its own file so the HANDOFF stays one page. It replaces both older HANDOFF tables:
     the five-column wind-down table ``| Lane | Worktree / branch | Agent | State at the
     wind-down | Next steps |`` and the roster ``| Lane | Agent ID | Worktree | Reserved |
     State |``. A lane's ABI/seam reservation from the roster is folded into its "State at
     stop" cell; a lane that had a roster row keeps it verbatim under "## Retired roster row"
     in its own ``docs/lanes/<lane>.md``, which this script never reads. If INDEX.md is missing
     it is created with a fixed preamble.

  2. HANDOFF.md, between ``<!-- BEGIN GENERATED: latest-run -->`` and ``<!-- END GENERATED -->``:
     the ``- **Latest in-game run:** ...`` bullet.

  3. docs/manual-checks.md, between ``<!-- BEGIN GENERATED: runs -->`` and
     ``<!-- END GENERATED -->``: the run table::

         | Tag | Run | Date | Lane | DLL | Pacing | Verdict | Switches | Result |

Inputs:

  * ``docs/lanes/<lane>.md`` -- exactly one data row of the four columns above (with the
    header and separator; a second column headed ``Worktree`` instead of ``Agent/worktree``
    and a ``| --- |`` separator are accepted too), optionally followed by a ``## Next steps`` paragraph whose text is
    appended to the row's Next steps cell. Anything else in the file is ignored.
  * ``docs/runs/<tag>.md`` -- one ``key: value`` per line. Required keys: ``tag``, ``run``,
    ``verdict``, ``switches``, ``result``. Recommended: ``date``, ``lane``, ``dll``,
    ``pacing``. Optional: ``summary`` and ``summary_note`` (fragments without a trailing
    period) for the "Latest in-game run" bullet. ``tag`` must equal the file name.

Order is deterministic: lanes by natural sort of the file name, runs by ``run`` number.
The script is idempotent -- running it twice produces the same bytes -- and it only ever
rewrites the text between the markers, so hand-written prose outside them survives.

It exits non-zero, printing the file name, if any ``docs/lanes/*.md`` has no table row (or a
malformed one), or if a ``docs/runs/*.md`` is missing a required field.

Usage::

    python tools/gen_handoff_tables.py            # rewrite the generated regions
    python tools/gen_handoff_tables.py --check    # exit 1 if a region is out of date
"""

from __future__ import annotations

import argparse
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HANDOFF = os.path.join(ROOT, "docs", "HANDOFF.md")
LANES_INDEX = os.path.join(ROOT, "docs", "lanes", "INDEX.md")
MANUAL = os.path.join(ROOT, "docs", "manual-checks.md")
LANES_DIR = os.path.join(ROOT, "docs", "lanes")
RUNS_DIR = os.path.join(ROOT, "docs", "runs")

BEGIN = "<!-- BEGIN GENERATED: {name} -->"
END = "<!-- END GENERATED -->"

LANE_HEADER = "| Lane | Agent/worktree | State at stop | Next steps |"
LANE_SEP = "|---|---|---|---|"
LANE_COLUMNS = 4
# Header cells a lane file may use: some lanes wrote "Worktree" for the second column.
LANE_HEADER_CELLS = (("lane",), ("agent/worktree", "worktree"), ("state at stop",), ("next steps",))
LANE_SKIP = ("README.md", "INDEX.md")

RUN_HEADER = "| Tag | Run | Date | Lane | DLL | Pacing | Verdict | Switches | Result |"
RUN_SEP = "|---|---|---|---|---|---|---|---|---|"
RUN_FIELDS = ("tag", "run", "date", "lane", "dll", "pacing", "verdict", "switches", "result")
RUN_REQUIRED = ("tag", "run", "verdict", "switches", "result")

# docs/lanes/INDEX.md is created with this text when it is missing; the table goes between the markers.
INDEX_PREAMBLE = (
    "# Lane index (generated)",
    "",
    "One row per lane, generated from `docs/lanes/<lane>.md` by `python tools/gen_handoff_tables.py`.",
    "Do not edit the table: edit the lane's own file and regenerate. The row format and the merge-queue",
    "contract are in [README.md](README.md).",
    "",
    "Who reads this: the coordinator, and a lane that resumes or follows up another lane's work. A row's",
    "\"merged\" is the lane's own claim: verify it with `git merge-base --is-ancestor <lane> main`.",
    "",
    BEGIN.format(name="lanes"),
    END,
)

# No cell may hold a raw pipe: it would silently split the table.
PIPE = re.compile(r"(?<!\\)\|")


class Error(Exception):
    pass


# --------------------------------------------------------------------------- reading


def read_text(path):
    with open(path, "r", encoding="utf-8", newline="") as fh:
        return fh.read()


def newline_of(text):
    return "\r\n" if "\r\n" in text else "\n"


def split_lines(text):
    return text.split("\r\n" if "\r\n" in text else "\n")


def natural_key(name):
    """Sort 'lane2' before 'lane10' while staying a pure function of the name."""
    return [int(p) if p.isdigit() else p.lower()
            for p in re.split(r"(\d+)", os.path.splitext(name)[0])]


def table_cells(line):
    return [c.strip() for c in line.split("|")[1:-1]]


def is_lane_header(line):
    s = line.strip()
    if not (s.startswith("|") and s.endswith("|")):
        return False
    cells = [c.lower() for c in table_cells(s)]
    return len(cells) == LANE_COLUMNS and all(c in ok for c, ok in zip(cells, LANE_HEADER_CELLS))


def is_lane_sep(line):
    s = line.strip()
    if not (s.startswith("|") and s.endswith("|")):
        return False
    cells = table_cells(s)
    return len(cells) == LANE_COLUMNS and all(re.match(r"^:?-{3,}:?$", c) for c in cells)


def parse_lane_file(path):
    name = os.path.splitext(os.path.basename(path))[0]
    lines = split_lines(read_text(path))
    header_at = None
    for i, line in enumerate(lines):
        if is_lane_header(line):
            header_at = i
            break
    if header_at is None:
        raise Error("no table row (expected the header %r)" % LANE_HEADER)
    if header_at + 2 >= len(lines):
        raise Error("table row is missing")
    if not is_lane_sep(lines[header_at + 1]):
        raise Error("table separator is not %r" % LANE_SEP)
    row = lines[header_at + 2].strip()
    if not row.startswith("|") or not row.endswith("|"):
        raise Error("table row is not a markdown table row")
    cells = table_cells(row)
    if len(cells) != LANE_COLUMNS:
        raise Error("table row has %d cells, expected %d" % (len(cells), LANE_COLUMNS))
    for cell in cells:
        if PIPE.search(cell):
            raise Error("table cell holds an unescaped '|': %s" % cell[:60])

    # Optional "## Next steps" paragraph, appended to the row's Next steps cell.
    extra = []
    for i in range(header_at + 3, len(lines)):
        if lines[i].startswith("## "):
            if lines[i].strip().lower() == "## next steps":
                j = i + 1
                while j < len(lines) and not lines[j].startswith("## "):
                    extra.append(lines[j])
                    j += 1
                break
            break
    note = " ".join(x.strip() for x in extra if x.strip())
    if note:
        cells[3] = (cells[3].rstrip() + " " + note).strip() if cells[3] != "-" else note

    return {"slug": name, "cells": cells, "row": "| " + " | ".join(cells) + " |"}


def parse_run_file(path):
    stem = os.path.splitext(os.path.basename(path))[0]
    fields = {}
    for line in split_lines(read_text(path)):
        m = re.match(r"^([a-z_]+): (.*)$", line)
        if m:
            fields.setdefault(m.group(1), m.group(2))
    for key in RUN_REQUIRED:
        if key not in fields:
            raise Error("missing %r" % key)
    if fields["tag"] != stem:
        raise Error("tag %r does not match the file name %r" % (fields["tag"], stem))
    try:
        fields["run"] = int(fields["run"])
    except ValueError:
        raise Error("run is not a number: %r" % fields["run"])
    for key in ("switches", "result"):
        if PIPE.search(fields[key]):
            raise Error("%s holds an unescaped '|'" % key)
    return fields


def load_lanes():
    if not os.path.isdir(LANES_DIR):
        raise Error("no docs/lanes/ directory")
    lanes = []
    for fn in sorted(os.listdir(LANES_DIR), key=natural_key):
        if not fn.endswith(".md") or fn in LANE_SKIP:
            continue
        path = os.path.join(LANES_DIR, fn)
        try:
            lanes.append(parse_lane_file(path))
        except Error as exc:
            raise Error("%s: %s" % (path, exc))
    if not lanes:
        raise Error("no docs/lanes/*.md files")
    return lanes


def load_runs():
    if not os.path.isdir(RUNS_DIR):
        raise Error("no docs/runs/ directory")
    runs = []
    for fn in sorted(os.listdir(RUNS_DIR)):
        if not fn.endswith(".md"):
            continue
        path = os.path.join(RUNS_DIR, fn)
        try:
            runs.append(parse_run_file(path))
        except Error as exc:
            raise Error("%s: %s" % (path, exc))
    seen = {}
    for run in runs:
        if run["run"] in seen:
            raise Error("duplicate run number %d in %s.md and %s.md"
                        % (run["run"], seen[run["run"]], run["tag"]))
        seen[run["run"]] = run["tag"]
    runs.sort(key=lambda r: r["run"])
    return runs


# --------------------------------------------------------------------------- building


def build_lane_table(lanes):
    out = [LANE_HEADER, LANE_SEP]
    out += [lane["row"] for lane in lanes]
    return out


def build_run_table(runs):
    out = [RUN_HEADER, RUN_SEP]
    for run in runs:
        cells = [run.get(k, "-") or "-" for k in RUN_FIELDS]
        cells[1] = str(run["run"])
        out.append("| " + " | ".join(cells) + " |")
    return out


def build_latest_run(runs):
    marked = [r for r in runs if r.get("summary")]
    marked.sort(key=lambda r: r["run"], reverse=True)
    if not marked:
        return ["- **Latest in-game run:** none recorded. See `docs/manual-checks.md`."]
    parts = []
    for run in marked:
        text = "%d (%s)" % (run["run"], run["summary"])
        if run.get("summary_note"):
            text += " " + run["summary_note"]
        parts.append(text)
    return ["- **Latest in-game run:** " + "; ".join(parts)
            + ". See `docs/manual-checks.md`."]


# --------------------------------------------------------------------------- writing


def replace_region(text, name, content_lines):
    begin = BEGIN.format(name=name)
    start = text.find(begin)
    if start < 0:
        raise Error("%s: no %r marker" % (name, begin))
    start += len(begin)
    if text.startswith("\r\n", start):
        start += 2
    elif text.startswith("\n", start):
        start += 1
    stop = text.find(END, start)
    if stop < 0:
        raise Error("%s: no %r marker after the begin marker" % (name, END))
    nl = newline_of(text)
    block = "".join(line + nl for line in content_lines)
    return text[:start] + block + text[stop:]


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true",
                    help="do not write; exit 1 if a generated region is out of date")
    args = ap.parse_args(argv)

    try:
        lanes = load_lanes()
        runs = load_runs()
        jobs = [
            (LANES_INDEX, "lanes", build_lane_table(lanes)),
            (HANDOFF, "latest-run", build_latest_run(runs)),
            (MANUAL, "runs", build_run_table(runs)),
        ]
    except Error as exc:
        sys.stderr.write("gen_handoff_tables: %s\n" % exc)
        return 1

    # Apply every region to one in-memory copy per file before writing.
    dirty = []
    cache = {}
    original = {}
    for path, name, content in jobs:
        if path not in cache:
            if path == LANES_INDEX and not os.path.exists(path):
                original[path] = ""
                cache[path] = "".join(line + "\n" for line in INDEX_PREAMBLE)
            else:
                original[path] = read_text(path)
                cache[path] = original[path]
        before = cache[path]
        after = replace_region(before, name, content)
        cache[path] = after
        if before != after or not original[path]:
            dirty.append("%s: %s" % (os.path.relpath(path, ROOT), name))

    if args.check:
        if dirty:
            sys.stderr.write("gen_handoff_tables: out of date: %s\n" % "; ".join(dirty))
            return 1
        print("gen_handoff_tables: up to date (%d lanes, %d runs)" % (len(lanes), len(runs)))
        return 0

    for path, text in cache.items():
        if original[path] != text:
            with open(path, "w", encoding="utf-8", newline="") as fh:
                fh.write(text)
    print("gen_handoff_tables: %d lanes, %d runs; rewrote %s"
          % (len(lanes), len(runs), ", ".join(dirty) if dirty else "nothing"))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
