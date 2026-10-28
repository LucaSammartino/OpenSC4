#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Compare FC4DrawCapture JSONL streams; 0 identical, 1 different, 2 invalid.

Only standard-library Python is needed. Both inputs are read through their
completion records, even after a difference. Empty, truncated or incomplete
captures can never establish equality. Hashes describe input bytes, not pointer
addresses or output-only destinations. See docs/draw-capture.md.
"""
from __future__ import annotations

import argparse
from dataclasses import dataclass
from itertools import zip_longest
import json
from pathlib import Path
import re
import sys
from typing import Any, Iterator


class InvalidStream(ValueError):
    pass


@dataclass
class Stats:
    frames: int = 0
    calls: int = 0
    draws: int = 0
    ui_frames: int = 0


@dataclass
class Difference:
    frame: Any
    call: Any
    method: str
    field: str
    left: Any
    right: Any


HASH = re.compile(r"[0-9a-f]{16}\Z")
STATUSES = {"output", "deferred-to-draw", "ignored-dx7"}
MAX_LINE = 1024 * 1024
MISSING = object()


def natural(value: Any) -> bool:
    return type(value) is int and value >= 0


def records(path: Path, stats: Stats) -> Iterator[dict]:
    """Validate framing and coverage counters without loading a stream in RAM."""
    expected_frames = None
    current = None
    calls = draws = nodes = 0
    ended = False
    line_number = 0

    def require(ok: bool, message: str) -> None:
        if not ok:
            raise InvalidStream(f"{path}:{line_number}: {message}")

    try:
        with path.open("r", encoding="utf-8") as source:
            while True:
                line = source.readline(MAX_LINE + 1)
                if not line:
                    break
                line_number += 1
                require(len(line) <= MAX_LINE, "record exceeds 1 MiB")
                require(not ended, "record after completion")
                try:
                    record = json.loads(line)
                except (ValueError, RecursionError) as exc:
                    raise InvalidStream(f"{path}:{line_number}: invalid JSON: {exc}") from exc
                require(isinstance(record, dict), "record must be an object")
                kind = record.get("type")
                if line_number == 1:
                    require(kind == "header" and record.get("schema") == "fc4.draw/1", "expected fc4.draw/1 header")
                    require(record.get("hash") == "fnv1a64", "unsupported hash")
                    expected_frames = record.get("frames")
                    require(natural(expected_frames) and expected_frames > 0, "expected frame count must be positive")
                    require(natural(record.get("scenarios")) and record["scenarios"] > 0, "invalid scenario count")
                elif kind == "frame":
                    require(current is None, "previous frame has no end_frame")
                    require(record.get("frame") == stats.frames, "nonconsecutive frame index")
                    require(record.get("paused") == 1 and record.get("hidden") == 0, "city is not visibly paused")
                    require(record.get("zoom") in range(1, 6) and record.get("rotation") in range(4), "invalid camera")
                    current = stats.frames
                    calls = draws = nodes = 0
                elif kind in {"call", "ui"}:
                    require(current is not None and record.get("frame") == current, "record outside its frame")
                    if kind == "call":
                        require(record.get("call") == calls, "nonconsecutive call index")
                        require(natural(record.get("slot")) and record["slot"] < 83, "invalid driver slot")
                        require(isinstance(record.get("method"), str) and bool(record["method"]), "missing method")
                        require(record.get("driver") in {"dx7", "opengl", "software"}, "unknown driver")
                        require(record.get("thread") in {0, 1}, "invalid thread class")
                        args = record.get("args")
                        require(isinstance(args, list) and all(natural(a) and a <= 0xffffffff for a in args), "invalid scalar arguments")
                        calls += 1
                        if record["slot"] in {3, 4}:
                            draws += 1
                    else:
                        require(record.get("node") == nodes, "nonconsecutive UI node index")
                        parent = record.get("parent")
                        require(type(parent) is int and (-1 if nodes == 0 else 0) <= parent < nodes, "invalid UI parent")
                        require(natural(record.get("id")) and record.get("visible") in {0, 1}, "invalid UI identity/visibility")
                        rect = record.get("rect")
                        require(isinstance(rect, list) and len(rect) == 4 and all(type(n) is int for n in rect), "invalid UI rectangle")
                        nodes += 1
                    buffers = record.get("buffers")
                    require(isinstance(buffers, dict), "missing buffer map")
                    for name, buffer in buffers.items():
                        require(isinstance(buffer, dict) and natural(buffer.get("bytes")), f"invalid buffer {name}")
                        if "status" in buffer:
                            require(buffer["status"] in STATUSES, f"incomplete buffer {name}: {buffer['status']}")
                            require("hash" not in buffer, f"ambiguous buffer {name}")
                        else:
                            require(isinstance(buffer.get("hash"), str) and HASH.fullmatch(buffer["hash"]) is not None,
                                    f"invalid buffer hash {name}")
                    if kind == "call" and record["slot"] in {3, 4}:
                        require("vertices" in buffers and "hash" in buffers["vertices"], "draw lacks vertex input hash")
                        if record["slot"] == 4:
                            require("indices" in buffers and "hash" in buffers["indices"], "draw lacks index input hash")
                    if kind == "ui":
                        require("text" in buffers and "hash" in buffers["text"], "UI node lacks text hash")
                elif kind == "end_frame":
                    require(current is not None and record.get("frame") == current, "end_frame outside its frame")
                    require(record.get("calls") == calls and record.get("draw_calls") == draws, "frame counters disagree")
                    require(draws > 0, "frame has no driver draws (minimized or software-only UI?)")
                    require(nodes > 0, "frame has no UI tree")
                    stats.frames += 1
                    stats.calls += calls
                    stats.draws += draws
                    stats.ui_frames += 1
                    current = None
                elif kind == "end":
                    require(current is None, "completion inside unfinished frame")
                    require(record.get("complete") is True and record.get("incomplete") == 0, "capture reports incomplete inputs")
                    require(stats.frames == expected_frames, "capture did not finish expected frames")
                    for key, value in vars(stats).items():
                        require(record.get("draw_calls" if key == "draws" else key) == value,
                                f"completion counter disagrees: {key}")
                    ended = True
                else:
                    require(False, f"unexpected record type: {kind!r}")
                yield record
        require(ended, "missing completion record (empty or truncated stream)")
    except (OSError, UnicodeError) as exc:
        raise InvalidStream(f"{path}: {exc}") from exc


def first_field(left: Any, right: Any, field: str = "") -> tuple | None:
    """Return the first differing leaf; object key order has no meaning."""
    if type(left) is not type(right):
        return field, left, right
    if isinstance(left, dict):
        # Identity and call arguments precede buffer details in diagnostics.
        priority = ("type", "frame", "call", "method", "slot", "driver", "args", "buffers")
        keys = list(dict.fromkeys([k for k in priority if k in left or k in right] + sorted(left.keys() | right.keys())))
        for key in keys:
            name = f"{field}.{key}" if field else key
            if key not in left or key not in right:
                return name, left.get(key, "<missing>"), right.get(key, "<missing>")
            difference = first_field(left[key], right[key], name)
            if difference:
                return difference
    elif isinstance(left, list):
        for i, (a, b) in enumerate(zip_longest(left, right, fillvalue=MISSING)):
            if a is MISSING or b is MISSING:
                return f"{field}[{i}]", "<missing>" if a is MISSING else a, "<missing>" if b is MISSING else b
            difference = first_field(a, b, f"{field}[{i}]")
            if difference:
                return difference
    elif left != right:
        return field, left, right
    return None


def compare(left: Path, right: Path) -> tuple[Difference | None, Stats]:
    a_stats, b_stats = Stats(), Stats()
    first = None
    for a, b in zip_longest(records(left, a_stats), records(right, b_stats), fillvalue=MISSING):
        if a is MISSING or b is MISSING:
            detail = ("record", "<end>" if a is MISSING else a, "<end>" if b is MISSING else b)
        else:
            detail = first_field(a, b)
        if first is None and detail:
            record = b if a is MISSING else a
            first = Difference(record.get("frame", "-"), record.get("call", record.get("node", "-")),
                               record.get("method", "UI.window" if record.get("type") == "ui" else record["type"]), *detail)
    return first, a_stats


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("left", type=Path)
    parser.add_argument("right", type=Path)
    parser.add_argument("--json", action="store_true", help="machine-readable result")
    args = parser.parse_args(argv)
    try:
        difference, stats = compare(args.left, args.right)
    except InvalidStream as exc:
        if args.json:
            print(json.dumps({"verdict": "invalid", "error": str(exc)}))
        else:
            print(f"INVALID {exc}", file=sys.stderr)
        return 2
    result = {"verdict": "different" if difference else "identical", **vars(stats)}
    if difference:
        result["difference"] = vars(difference)
    if args.json:
        print(json.dumps(result, sort_keys=True))
    elif difference:
        d = difference
        print(f"DIFFERENT frame={d.frame} call={d.call} method={d.method} field={d.field} "
              f"left={json.dumps(d.left)} right={json.dumps(d.right)}")
    else:
        print(f"IDENTICAL frames={stats.frames} calls={stats.calls} draws={stats.draws} ui_frames={stats.ui_frames}")
    return 1 if difference else 0


if __name__ == "__main__":
    sys.exit(main())
