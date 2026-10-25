#!/usr/bin/env python3
"""Prepare/run/summarize generated FC4 Test scenarios with the existing game queue.

Discovery sweep companion to tools/suite.py. All generated content and evidence
stay in build-runs/suite/<tag>; Documents saves are opened read-only. Six request
roots let the existing per-root queue policy use six independently locked slots.
"""
import argparse
from collections import Counter, defaultdict
import csv
from datetime import date, timedelta
import hashlib
import json
from pathlib import Path
import re
import shutil
import struct
import subprocess

from dbpf import DBPF
from scenario_gen import MIXES, SIZES, add_months, generate

ROOT = Path(__file__).resolve().parents[1]
SUMMARY = ("scenario", "profile", "verdict", "end_reached", "days_per_second", "actions_ok",
           "actions_failed", "shadow_lines", "shadow_exact_lines", "worst_shadow",
           "compare_exact", "compare_different", "reach_functions", "log")
ACTION = re.compile(r"script: line (\d+) (\d{4}-\d\d-\d\d) (\w+).*? -> (.*)")
SHADOW = re.compile(r"\] ([a-z_]+(?: [a-z_]+)*) shadow [^:]*(?:\d{4}-\d\d-\d\d|julian=\d+)[^:]*: "
                    r".*(?:mismatch(?:es|ed)[= ]\d|: clean\b)")
FAMILY = re.compile(r"\] ([a-z0-9]+(?:-[a-z0-9]+)+)-shadow: .*mismatches=\d")
FORMATS = re.compile(r"\] (formats) shadow: [^:]+: \d+ exemplar entries, [1-9]\d* compared, "
                     r"\d+ exact, \d+ differ, \d+ rejected")


def write_tsv(path, fields, rows):
    with path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def city_facts(path):
    """Read only the documented regional header and single simulator entry."""
    db = DBPF(path)
    regional = db.get(0xCA027EDB, 0xCA027EE1, 0)
    helper, version, _, _, sx, sz = struct.unpack_from("<HHiiii", regional)
    if helper != 1 or version not in (9, 10, 13) or sx != sz or sx not in (1, 2, 4):
        raise ValueError("unsupported regional-city header: " + str(path))
    sim = db.get(0x2990C1E5, 0x299B2D1B, 0)
    sim_version = struct.unpack_from("<H", sim, 12)[0]
    # Class entry header 12, simulator fixed payload 79; old timer is
    # u8 units + u64 elapsed + u8 running, observed 10 bytes in shipped saves.
    expected = 91 if sim_version == 4 else 101
    if sim_version not in (3, 4) or len(sim) != expected or struct.unpack_from("<I", sim)[0] != len(sim):
        raise ValueError("unsupported simulator record: " + str(path))
    julian = struct.unpack_from("<I", sim, len(sim) - 4)[0]
    return {"size": {64: "small", 128: "medium", 256: "large"}[sx * 64],
            "saved_date": date.fromordinal(julian - 1721425).isoformat(),
            "source_sha256": hashlib.sha256(db.data).hexdigest()}


def shadow_profile():
    text = (ROOT / "tools/regress.ps1").read_text(encoding="utf-8")
    match = re.search(r'^\s+shadows\s*=\s*"([^"\r\n]+)"', text, re.MULTILINE)
    if not match:
        raise ValueError("cannot find regress.ps1's literal shadows profile")
    return match[1].replace("`n", "\n")


def discovery_request(text):
    """Use normal queue priority and close dialogs, including prepared v2 jobs."""
    head, separator, ini = text.partition("[ini]\n")
    if not separator:
        raise ValueError("discovery request has no [ini] section")
    head = re.sub(r"^\s*(?:priority|region)\s*=.*\n", "", head, flags=re.MULTILINE)
    ini = re.sub(r"^\s*close_dialogs\s*=.*\n?", "", ini, flags=re.MULTILINE)
    return head + "region = FC4 Test\npriority = normal\n[ini]\nclose_dialogs = 1\n" + ini


def prepare(args, out):
    if (out / "manifest.json").exists():
        raise ValueError("tag already prepared; use a new tag (preserve earlier evidence)")
    dll = args.dll.resolve(strict=True)
    if dll.suffix.lower() != ".dll":
        raise ValueError("--dll must be a DLL")
    if not 24 <= args.months <= 48:
        raise ValueError("game runs must cover 24..48 months")
    if not 10 <= args.seconds <= 7200:
        raise ValueError("seconds must be 10..7200")
    # Probe every input before publishing any request.
    facts = {city: city_facts(args.region_path / ("City - " + city + ".sc4")) for city in args.cities}
    profile = shadow_profile()
    out.mkdir(parents=True, exist_ok=True)
    manifest = {"tag": args.tag, "region": "FC4 Test", "months": args.months, "cities": facts,
                "dll_source": str(dll), "dll_sha256": hashlib.sha256(dll.read_bytes()).hexdigest(),
                "shadow_profile": profile, "runs": []}
    roots = [out / ("worker" + str(i + 1)) for i in range(args.slots)]
    for root in roots:
        build = root / "build-runs"
        (build / "scripts").mkdir(parents=True)
        (build / "game-queue").mkdir()
        shutil.copy2(dll, build / "FreeCity4.dll")
    for city in args.cities:
        info = facts[city]
        start = date.fromisoformat(info["saved_date"]) + timedelta(days=1)
        end = add_months(start, args.months)
        slug = re.sub(r"[^a-z0-9]+", "-", city.lower()).strip("-")
        for seed in args.seeds:
            for mix in MIXES:
                label = "{}-{}-s{}-{}".format(args.tag, slug, seed, mix)
                if len(label) > 63:
                    raise ValueError("tag/city/seed produce a label longer than 63 characters")
                root = roots[len(manifest["runs"]) % len(roots)]
                script = root / "build-runs/scripts" / (label + ".script")
                script.write_text(generate(seed, info["size"], start, args.months, mix), encoding="utf-8")
                req = ["# scenario_runs: region FC4 Test (the legacy queue's default)",
                       "label = " + label, "dll = build-runs/FreeCity4.dll", "city = " + city,
                       "seconds = " + str(args.seconds), "timeout = " + str(args.seconds + 150),
                       "end_date = " + end.isoformat(), "turbo = 1", "[ini]", profile,
                       "script = " + script.relative_to(root).as_posix()]
                request = root / "build-runs/game-queue" / (label + ".req")
                tmp = request.with_suffix(".tmp")
                tmp.write_text(discovery_request("\n".join(req) + "\n"), encoding="utf-8")
                tmp.rename(request)
                manifest["runs"].append({"scenario": label, "city": city, "seed": seed, "mix": mix,
                                         "start": start.isoformat(), "end": end.isoformat(),
                                         "root": root.relative_to(ROOT).as_posix(),
                                         "script": script.relative_to(ROOT).as_posix()})
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    write_tsv(out / "suite.tsv", ("name", "region", "city", "script", "end", "sets", "notes"),
              ({"name": r["scenario"], "region": "FC4 Test", "city": r["city"], "script": r["script"],
                "end": r["end"], "sets": "gen", "notes": "seed={} mix={} start={} size={}".format(
                    r["seed"], r["mix"], r["start"], facts[r["city"]]["size"])} for r in manifest["runs"]))
    print("prepared {} requests in {} roots; DLL SHA256 {}; {}".format(
        len(manifest["runs"]), len(roots), manifest["dll_sha256"], out / "manifest.json"))
    for city, info in facts.items():
        print("{}: {} saved {} source {}".format(city, info["size"], info["saved_date"], info["source_sha256"]))


def run_queue(args, out, manifest):
    roots = sorted({str(ROOT / r["root"]) for r in manifest["runs"]})
    # Prepared requests predate close_dialogs and the region queue key. Update
    # only pending jobs; completed/claimed evidence must remain untouched.
    pending = 0
    for root in roots:
        for request in (Path(root) / "build-runs/game-queue").glob("*.req"):
            text = request.read_text(encoding="utf-8")
            request.write_text(discovery_request(text), encoding="utf-8")
            pending += 1
    # Include the coordinator root so waiting train regress jobs are always
    # considered before this tier-2 sweep, also by this private worker.
    main = Path(subprocess.check_output(
        ["git", "-C", str(ROOT), "rev-parse", "--git-common-dir"], text=True).strip()).resolve().parent
    roots.insert(0, str(main))
    cmd = ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(ROOT / "tools/game_queue.ps1"),
           "-Instance", "gen-" + args.tag, "-Once", "-Slots", str(args.slots), "-Roots", ",".join(roots)]
    print("running {} requests ({} pending, priority=normal, close_dialogs=1) via {} slot locks; {}".format(
        len(manifest["runs"]), pending, args.slots, out / "queue.out"), flush=True)
    with (out / "queue.out").open("a", encoding="utf-8") as stream:
        result = subprocess.run(cmd, stdout=stream, stderr=subprocess.STDOUT)
    print("queue exit=" + str(result.returncode), flush=True)
    return result.returncode


def shadow_differs(line):
    if "] formats shadow:" in line:
        return bool(re.search(r"\b[1-9]\d* (?:differ|rejected)\b", line))
    totals = re.findall(r"; mismatches=(\d+)", line)
    if totals:
        return int(totals[-1]) != 0
    return bool(re.search(r"mismatch(?:es|ed)[= ][1-9]|differences [1-9]", line))


def action_ok(result):
    return not re.search(r"\bok=0\b|\berror=[1-9]|skipped:|structured exception|\bfailed\b|\bno effect:|^no ", result)


def analyze(log, run):
    actions, shadows, first, events = defaultdict(Counter), defaultdict(Counter), {}, []
    previous = 0
    dates = []
    dps = "-"
    complete = False
    with log.open(encoding="utf-8", errors="replace") as stream:
        for number, line in enumerate(stream, 1):
            match = ACTION.search(line)
            if match:
                previous = int(match[1])
                actions[match[3]]["run"] += 1
                actions[match[3]]["ok" if action_ok(match[4]) else "failed"] += 1
            if "script: all " in line and " have run" in line:
                complete = True
            shadow = SHADOW.search(line) or FAMILY.search(line) or FORMATS.search(line)
            if shadow:
                key = shadow[1].replace(" ", "_")
                shadows[key]["lines"] += 1
                different = shadow_differs(line)
                resynced = bool(re.search(r"\bresynced:\s*(?!none\b|0\b)\S|\bresyncs?\s*[=:]\s*[1-9]\d*\b", line))
                shadows[key]["exact"] += not different and not resynced
                shadows[key]["different"] += different
                shadows[key]["resynced"] += resynced
                if different and key not in first:
                    first[key] = (number, previous, line.strip())
            # Record each exception family once; a crash can occur inside an
            # action before its completion line, so previous is context only.
            issue = re.search(r"structured exception in|first-chance exception OUTSIDE FreeCity4\.dll|"
                              r"\b(?:Rust )?panic(?: at|:)|AUTOTEST VERDICT: (?:crash|corrupt)", line)
            if issue and issue[0] not in first:
                first[issue[0]] = (number, previous, line.strip())
            heartbeat = re.search(r"heartbeat \(periodic\): (\d{4}-\d\d-\d\d)", line)
            if heartbeat:
                dates.append(heartbeat[1])
            final_date = re.search(r"AUTOTEST VERDICT:.*\bdate=(\d{4}-\d\d-\d\d)", line)
            if final_date:
                dates.append(final_date[1])
            perf = re.search(r"days_per_second=([0-9.]+)", line)
            if perf:
                dps = perf[1]
    script_lines = (ROOT / run["script"]).read_text(encoding="utf-8").splitlines()
    for key, (number, prior, evidence) in first.items():
        events.append({"scenario": run["scenario"], "seed": run["seed"], "mix": run["mix"], "city": run["city"],
                       "key": key, "log_line": number, "preceding_script_line": prior,
                       "preceding_script_text": script_lines[prior - 1] if prior else "-", "evidence": evidence})
    return actions, shadows, events, dates, dps, complete


def summarize(out, manifest):
    rows, action_rows, findings = [], [], []
    by_key = defaultdict(list)
    for run in manifest["runs"]:
        label = run["scenario"]
        results = ROOT / run["root"] / "build-runs/game-results"
        log = results / (label + ".log")
        verdict_file = results / (label + ".verdict")
        verdict_text = verdict_file.read_text(encoding="utf-8", errors="replace") if verdict_file.exists() else ""
        verdict = verdict_text.splitlines()[0].removeprefix("verdict: ") if verdict_text else "pending"
        acts, shadows, issues, dates, dps, complete = analyze(log, run) if log.exists() else ({}, {}, [], [], "-", False)
        findings.extend(issues)
        for op, counts in sorted(acts.items()):
            action_rows.append({"scenario": label, "action": op, **{k: counts[k] for k in ("run", "ok", "failed")}})
        for key, counts in shadows.items():
            by_key[key].append((run, counts, next((f for f in issues if f["key"] == key), None)))
        total = sum(c["lines"] for c in shadows.values())
        exact = sum(c["exact"] for c in shadows.values())
        worst = max(shadows, key=lambda k: shadows[k]["lines"] - shadows[k]["exact"]) if shadows else "-"
        reached = bool(dates and max(dates) >= run["end"])
        # The native end-date announcement is more precise than the periodic
        # heartbeat (which need not land on the final day).
        reached = reached or bool(re.search(r"end date " + re.escape(run["end"]) + r" reached\b", verdict_text))
        rows.append(dict(zip(SUMMARY, (label, "shadows", verdict, str(reached).lower(), dps,
                    sum(c["ok"] for c in acts.values()), sum(c["failed"] for c in acts.values()),
                    total, exact, worst, "-", "-", "-", log.relative_to(ROOT).as_posix()))))
        if not complete and verdict != "pending":
            findings.append({"scenario": label, "seed": run["seed"], "mix": run["mix"], "city": run["city"],
                             "key": "script_incomplete", "log_line": "-", "preceding_script_line": "-",
                             "preceding_script_text": "-", "evidence": "script completion marker missing; verdict=" + verdict})
    write_tsv(out / "summary.tsv", SUMMARY, rows)
    write_tsv(out / "actions.tsv", ("scenario", "action", "run", "ok", "failed"), action_rows)
    write_tsv(out / "findings.tsv", ("scenario", "seed", "mix", "city", "key", "log_line",
                                    "preceding_script_line", "preceding_script_text", "evidence"), findings)
    write_tsv(out / "per_key.tsv", ("key", "scenarios_run", "scenarios_exact", "first_mismatch"),
              ({"key": key, "scenarios_run": len(items),
                "scenarios_exact": sum(c["lines"] == c["exact"] for _, c, _ in items),
                "first_mismatch": next((r["scenario"] + ":" + str(issue["log_line"])
                                        for r, _, issue in items if issue), "-")} for key, items in sorted(by_key.items())))
    print("runs={} verdicts={} actions_ok={} actions_failed={} shadows={}/{} exact findings={}".format(
        len(rows), dict(Counter(r["verdict"] for r in rows)), sum(r["actions_ok"] for r in rows),
        sum(r["actions_failed"] for r in rows), sum(r["shadow_exact_lines"] for r in rows),
        sum(r["shadow_lines"] for r in rows), len(findings)))
    print("per run: {}; per action: {}; findings: {}".format(out / "summary.tsv", out / "actions.tsv", out / "findings.tsv"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("prepare", "run", "summarize"))
    parser.add_argument("--tag", required=True)
    parser.add_argument("--dll", type=Path)
    parser.add_argument("--region-path", type=Path, default=Path.home() / "Documents/SimCity 4/Regions/FC4 Test")
    parser.add_argument("--cities", nargs="+", default=["Konradshohe", "Tegel", "Spandau", "New City"])
    parser.add_argument("--seeds", type=int, nargs="+", default=[0, 1, 2])
    parser.add_argument("--months", type=int, default=24)
    parser.add_argument("--slots", type=int, choices=range(1, 7), default=6)
    parser.add_argument("--seconds", type=int, default=1200)
    args = parser.parse_args()
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,24}", args.tag):
        parser.error("tag must be 1..25 lowercase letters/digits/hyphens")
    out = ROOT / "build-runs/suite" / args.tag
    try:
        if args.command == "prepare":
            if not args.dll:
                parser.error("prepare requires --dll")
            prepare(args, out)
        else:
            manifest = json.loads((out / "manifest.json").read_text(encoding="utf-8"))
            code = run_queue(args, out, manifest) if args.command == "run" else 0
            summarize(out, manifest)
            return code
    except (ValueError, OSError, struct.error) as error:
        parser.error(str(error))


if __name__ == "__main__":
    raise SystemExit(main())
