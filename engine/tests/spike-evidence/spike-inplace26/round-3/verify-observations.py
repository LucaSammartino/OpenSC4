"""Recompute round-3 qualification, teeth, adapter costs and commit-time rates.

Usage: python verify-observations.py [--raw <worktree>/build-runs/game-results]
Unpublished body sizes are uncredited, not estimated from address spacing.
"""
import argparse
import csv
import json
import subprocess
from datetime import datetime
from pathlib import Path

from collect import FIELDS, collect

ROOT = Path(__file__).resolve().parent
WT = ROOT.parents[4]
START = datetime.fromisoformat("2026-10-25T13:36:50+00:00")
BASE = "6d75dfa87"
OLD_START = datetime.fromisoformat("2026-10-25T11:20:12+00:00")


def git(*args):
    return subprocess.check_output(["git", "-C", str(WT), *args], text=True).strip()


def rows(name):
    with (ROOT / name).open(newline="") as stream:
        return list(csv.DictReader(stream))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--raw", type=Path)
    options = parser.parse_args()
    functions = {row["name"]: row for row in rows("functions.csv")}
    assert len(functions) >= 10
    assert all(int(row["body_bytes"]) == 0 and row["body_status"] == "unpublished" for row in functions.values())
    observations = rows("observations.csv")
    mutations = rows("mutations.csv")
    runs = json.loads((ROOT / "runs.json").read_text())
    for run in runs:
        assert run["verdict"] == "clean" and run["ref_warnings"] == 0
        actual = [row for row in observations if row["run"] == run["label"]]
        assert {(row["name"], row["source"]) for row in actual} == {(name, source) for name in functions for source in ("natural", "fixture")}
        if options.raw:
            summary, locations = collect(options.raw / (run["label"] + ".log"))
            for row in actual:
                assert summary[(row["name"], row["source"])] == [row[field] for field in FIELDS]
            for row in mutations:
                if row["run"] == run["label"]:
                    assert locations[(row["name"], row["source"])] == [row[field] for field in ("window", "byte", "xor", "detected")]
            verdict = (options.raw / (run["label"] + ".verdict")).read_text()
            assert "verdict: clean" in verdict and "ref_warnings=0" in verdict
    qualified, outside, floating = [], [], []
    for name in functions:
        values = [row for row in observations if row["name"] == name]
        total = {field: sum(int(row[field]) for row in values) for field in FIELDS}
        for row in values:
            assert int(row["compared"]) == int(row["exact"]) + int(row["different"])
            assert int(row["sampled"]) == int(row["compared"]) + int(row["outside"]) + int(row["refused"])
            assert int(row["mutation"]) == int(row["detected"])
        if total["outside"]:
            outside.append(name)
        if total["float_only"]:
            floating.append(name)
        if total["compared"] and total["compared"] == total["exact"] and not (total["outside"] or total["refused"]):
            qualified.append(name)
    for row in mutations:
        assert int(row["xor"]) == int(row["detected"]) == 1 and int(row["byte"]) == 0
        expected_window = 2 if row["name"] == "lighting-vertices" else (1 if row["name"] in ("lighting-global", "lighting-shadow") else 6)
        assert int(row["window"]) == expected_window
        match = [value for value in observations if all(value[key] == row[key] for key in ("run", "name", "source"))]
        assert len(match) == 1 and int(match[0]["mutation"]) == 1
        assert int(match[0]["different"]) == 0  # excludes detections of pre-existing errors
    mutated = {row["name"] for row in mutations}
    assert len(mutated) >= 5
    assert sum(int(row["mutation"]) for row in observations) == len(mutations)
    # Include the final handoff/report time without a self-referential hash file.
    end_commit = git("log", "-1", "--format=%H", "--grep=^Finish timed round-3 S3 measurement$") or "HEAD"
    endpoint = git("show", "-s", "--format=%cI", end_commit)
    end = datetime.fromisoformat(endpoint)
    hours = (end - START).total_seconds() / 3600
    cumulative_hours = (end - OLD_START).total_seconds() / 3600
    body_bytes = sum(int(functions[name]["body_bytes"]) for name in qualified)
    code_paths = ["engine/plugin/FC4Inplace.cpp", "engine/plugin/FC4InplaceRound3.h", "engine/core/src/lighting/mod.rs", "engine/core/src/lighting/inplace.rs", "engine/capi/src/lib.rs", "engine/capi/src/inplace3.rs"]
    changes = git("diff", "--numstat", BASE, "HEAD", "--", *code_paths)
    added = sum(int(line.split()[0]) for line in changes.splitlines())
    costs = rows("adapters.csv")
    assert {row["name"] for row in costs} == set(functions)
    assert sum(int(row["entry_shell_lines"]) for row in costs) == len(functions)
    assert abs(sum(float(row["charged_added_lines"]) for row in costs) - added) < 1e-8
    assert not git("diff", "--name-only", BASE, "HEAD", "--", "engine/core/src/view_utilities", "engine/capi/src/viewutil_shadow.rs")
    marginal = body_bytes / 1000 / hours
    cumulative = (1821 + body_bytes) / 1000 / cumulative_hours
    kill_share = len(set(outside + floating)) / len(functions)
    verdict = "KILL" if kill_share > .3 else ("PASS" if len(qualified) / len(functions) >= .7 and marginal >= 11 else "INCONCLUSIVE")
    print(json.dumps(dict(verdict=verdict, endpoint_commit=end_commit, endpoint_utc=endpoint, lane_hours=hours,
        qualified=qualified, exact_share=len(qualified) / len(functions), outside=outside,
        float_only=floating, mutation_events=len(mutations), mutated_functions=len(mutated),
        known_qualified_body_bytes=body_bytes, marginal_kb_hour=None, cumulative_kb_hour=None,
        marginal_known_bytes_lower_bound_kb_hour=marginal,
        cumulative_known_bytes_lower_bound_kb_hour=cumulative, adapter_and_facility_added_lines=added,
        lines_per_new_function=added / len(functions)), indent=2))


if __name__ == "__main__":
    main()
