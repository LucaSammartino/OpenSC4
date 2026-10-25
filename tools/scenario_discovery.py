#!/usr/bin/env python3
"""Monthly discovery and every non-exact shadow, without deduplicating findings.

New keys are counted both per scenario and in manifest order across the sweep.
These are discovery measurements, not coverage credit or proof of causality.
"""
import argparse
from collections import Counter
from decimal import Decimal
import json
from pathlib import Path
import re

from scenario_runs import ACTION, FAMILY, FORMATS, ROOT, SHADOW, shadow_differs, write_tsv

DATE = re.compile(r"\b(\d{4}-\d\d-\d\d)\b")
NUMBER = r"-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?"
PAIR = re.compile(r"(\w+) orig/ours=(" + NUMBER + r")/(" + NUMBER + r")(?:!|(?=[; ]|$))")
RESYNC = re.compile(r"\bresynced:\s*(?!none\b|0\b)\S|\bresyncs?\s*[=:]\s*[1-9]\d*\b")


def differences(body):
    """Report only deltas actually present in the log, preserving opaque bytes."""
    fields = []
    for name, original, ours in PAIR.findall(body):
        delta = Decimal(ours) - Decimal(original)
        if delta:
            fields.append(f"{name}={original}/{ours} delta(ours-orig)={delta}")
    for name, cells, count, maximum in re.findall(
            r"(intrinsic|total|slope) cells=(\d+) mismatched=(\d+) max\|d\|=([^;]+)", body):
        if int(count):
            fields.append(f"{name}: {count}/{cells} cells differ, max_abs_delta={maximum}")
    for record, count, size, offset, original, ours in re.findall(
            r"(statistics \([^)]+\)): (\d+) of (\d+) bytes differ.*?first at \+(\d+): orig (\w+) ours (\w+)", body):
        fields.append(f"{record}: {count}/{size} bytes differ; +{offset} orig={original} ours={ours}")
    if not fields:
        # Some shadows report refusals or message order, with no scalar delta.
        fields.extend(re.findall(r"\b(?:refusals|mismatches|orig-only|msgorder|destroy)=[\d/]+", body))
        fields.append("no numeric field delta logged; see evidence")
    return "; ".join(fields)


def inspect(log, run, suite_keys):
    """Use execution chronology; a DBPF record date may precede the live month."""
    script = (ROOT / run["script"]).read_text(encoding="utf-8").splitlines()
    current = run["start"]
    months, samples, nonexact, seen, seen_bad = {}, [], [], set(), set()
    previous, actions = {}, []
    reached = None
    for number, line in enumerate(log.open(encoding="utf-8", errors="replace"), 1):
        body = line.partition("] ")[2] or line.rstrip()
        action = ACTION.search(line)
        shadow = SHADOW.search(line) or FAMILY.search(line) or FORMATS.search(line)
        # Avoid wall dates and dates in byte-difference examples. Only recognized
        # game events can advance the observed execution month.
        event_date = None
        if action:
            actual = re.search(r"\(run on (\d{4}-\d\d-\d\d)\)", body)
            event_date = actual[1] if actual else action[2]
            script_line = int(action[1])
            text = script[script_line - 1] if 0 < script_line <= len(script) else "-"
            actions.append((number, script_line, text, action[4]))
        elif shadow or "heartbeat (periodic):" in body or "AUTOTEST VERDICT:" in body:
            found = DATE.search(body)
            event_date = found[1] if found else None
        # Several planner shadows date their empty Julian sentinel as -4713.
        # It is a record value, never a simulation clock observation.
        if event_date and run["start"] <= event_date <= run["end"]:
            current = max(current, event_date)
        month = current[:7]
        row = months.setdefault(month, dict(scenario=run["scenario"], month=month,
            shadow_lines=0, shadow_exact_lines=0, new_shadow_keys=[], new_suite_keys=[],
            new_mismatch_keys=[], cumulative_shadow_keys=0, reach_functions="-", new_functions="-"))
        if body.startswith("reach summary "):
            count = re.search(r"\breached=(\d+)", body)
            if count:
                total = int(count[1])
                row["reach_functions"] = total
                row["new_functions"] = (0 if row["new_functions"] == "-" else row["new_functions"]) + total - (reached or 0)
                reached = total
        if not shadow:
            continue
        key = shadow[1].replace(" ", "_")
        different = shadow_differs(line)
        exact = not different and not RESYNC.search(line)
        row["shadow_lines"] += 1
        row["shadow_exact_lines"] += int(exact)
        if key not in seen:
            row["new_shadow_keys"].append(key)
            if key not in suite_keys:
                row["new_suite_keys"].append(key)
            samples.append(dict(scenario=run["scenario"], month=month, key=key,
                log_line=number, exact=int(exact), new_to_suite=int(key not in suite_keys)))
            seen.add(key)
            suite_keys.add(key)
        row["cumulative_shadow_keys"] = len(seen)
        if different or not exact:
            if key not in seen_bad:
                row["new_mismatch_keys"].append(key)
                seen_bad.add(key)
            start = previous.get(key, 0)
            window = [a for a in actions if a[0] > start]
            native_date = DATE.search(body)
            nonexact.append(dict(scenario=run["scenario"], seed=run["seed"], mix=run["mix"],
                city=run["city"], key=key, month=month,
                sample_date=native_date[1] if native_date else "-", log_line=number,
                kind="different" if different else "resynced", differences=differences(body),
                window_start_line=start, script_lines=",".join(str(a[1]) for a in window) or "-",
                actions=" | ".join(f"L{a[1]} {a[2]} -> {a[3]}" for a in window) or "none",
                evidence=line.strip()))
        previous[key] = number
    cumulative = 0
    for row in months.values():
        cumulative += len(row["new_shadow_keys"])
        row["cumulative_shadow_keys"] = cumulative
        for column in ("new_shadow_keys", "new_suite_keys", "new_mismatch_keys"):
            row[column] = ",".join(row[column]) or "-"
    return list(months.values()), samples, nonexact


def summarize(out, manifest):
    months, firsts, nonexact, suite_keys = [], [], [], set()
    for run in manifest["runs"]:
        log = ROOT / run["root"] / "build-runs/game-results" / (run["scenario"] + ".log")
        if not log.exists():
            continue
        rows, samples, issues = inspect(log, run, suite_keys)
        months.extend(rows)
        firsts.extend(samples)
        nonexact.extend(issues)
    write_tsv(out / "monthly.tsv", ("scenario", "month", "shadow_lines", "shadow_exact_lines",
        "new_shadow_keys", "new_suite_keys", "new_mismatch_keys", "cumulative_shadow_keys",
        "reach_functions", "new_functions"), months)
    write_tsv(out / "first_seen.tsv", ("scenario", "month", "key", "log_line", "exact", "new_to_suite"), firsts)
    write_tsv(out / "nonexact.tsv", ("scenario", "seed", "mix", "city", "key", "month", "sample_date",
        "log_line", "kind", "differences", "window_start_line", "script_lines", "actions", "evidence"), nonexact)
    counts = Counter(r["scenario"] for r in nonexact)
    print(f"discovery: {len(months)} scenario-months, {len(suite_keys)} shadow keys, "
          f"{len(nonexact)} non-exact lines in {len(counts)} scenarios; reach="
          + ("available" if any(r["reach_functions"] != "-" for r in months) else "unavailable"))
    return months, firsts, nonexact


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,24}", args.tag):
        parser.error("invalid tag")
    out = ROOT / "build-runs/suite" / args.tag
    summarize(out, json.loads((out / "manifest.json").read_text(encoding="utf-8")))


if __name__ == "__main__":
    main()
