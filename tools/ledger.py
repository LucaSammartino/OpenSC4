#!/usr/bin/env python3
"""Run-7 per-game-function ledger, using coverage.py's attribution and statuses.

RE/coordinator side only. Output contains contracts, citations and measured
values, never disassembly. Defaults to build-runs/ledger/functions.tsv and a
short summary.md. Repeat --log for explicit game logs; otherwise use exactly
the same house regress summary and raw logs as the coverage report.
"""
import argparse
import csv
import importlib.util
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

try:
    from tools import compare_metrics, function_boundary, oracle_class
except ModuleNotFoundError:
    import compare_metrics
    import function_boundary
    import oracle_class

ROOT = Path(__file__).resolve().parents[1]
_spec = importlib.util.spec_from_file_location("fc4_ledger_coverage", ROOT / "tools/coverage.py")
cov = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(cov)
CONTRACT_FIELDS = ("size", "name", "convention", "this", "callee_pop", "stack_arg_dwords",
                   "reg_args", "ret", "ret_confidence", "confidence", "callers", "notes")
ADDRESS = re.compile(r"(?<!\w)(?:FUN_|sub_)?(?:0x([0-9a-f]{6,8})|([0-9a-f]{8}))(?!\w)", re.I)


def read_contracts(path):
    if not Path(path).is_file():
        return {}
    contracts = {}
    for row in cov.read_tsv(path):
        if not re.fullmatch(r"[0-9a-f]{8}", row.get("address", "")):
            raise ValueError(f"{path}: invalid entry-contract address")
        va = int(row["address"], 16)
        if va in contracts:
            raise ValueError(f"{path}: duplicate entry contract {va:08x}")
        contracts[va] = row
    return contracts


def spec_citations(directory, addresses):
    """Exact address mentions in tables or prose, with file, section and line.

    Scan specs only; coordinator question/history records are not inputs.
    Do not infer coverage of other addresses from an owner-level spec mapping.
    """
    citations = defaultdict(list)
    for path in sorted(Path(directory).rglob("*.md")):
        if "questions" in path.relative_to(directory).parts:
            continue
        sections = []
        fence = None
        with path.open(encoding="utf-8", errors="replace") as stream:
            for number, line in enumerate(stream, 1):
                marker = re.match(r"^\s*(`{3,}|~{3,})", line)
                if marker:
                    if fence is None:
                        fence = marker[1][0]
                    elif marker[1][0] == fence:
                        fence = None
                heading = re.match(r"^(#{1,6})\s+(.+?)\s*#*\s*$", line) if fence is None else None
                if heading:
                    level, title = len(heading[1]), heading[2]
                    sections = [(n, s) for n, s in sections if n < level] + [(level, title)]
                for va in sorted({int(m[1] or m[2], 16) for m in ADDRESS.finditer(line)} & addresses):
                    citations[va].append({"file": "docs/specs/" + path.relative_to(directory).as_posix(),
                                          "section": " > ".join(s for _, s in sections), "line": number})
    return citations


def shadow_reach(data, logs):
    """Positive named-method shadow counters, scoped by the coverage shadow key.

    Aggregate labels (queries/lists/calls) cannot identify a function. Never
    infer reachability for every function belonging to a shadowed owner.
    """
    candidates, reached = defaultdict(list), {}
    for va, _, index, _ in data["game"]:
        status = data["rows"][index] if index is not None else {}
        shadow = status.get("shadow", "-")
        name = data["att"].name(va).rsplit("::", 1)[-1].split("(", 1)[0]
        if shadow.endswith("_shadow"):
            candidates[(shadow[:-7], name)].append(va)
    pattern = re.compile(r"\b(\w+) shadow \(([^)]+)\).*?\bcalls=(\d+)\b")
    for path in logs:
        for line in compare_metrics.text_lines(path):
            match = pattern.search(line)
            if not match or int(match[3]) == 0:
                continue
            addresses = candidates.get((match[1], match[2]), [])
            if len(addresses) == 1:
                va = addresses[0]
                reached[va] = max(reached.get(va, 0), int(match[3]))
    return reached


def ledger_rows(data, states, bindings, evidence, classes, contracts, citations, boundaries, reached=None):
    for va, size, index, area in data["game"]:
        contract = contracts.get(va, {})
        binding = bindings.get(va, {})
        status = data["rows"][index] if index is not None else {}
        row = {"address": f"{va:08x}", "bytes": size, "name": data["att"].name(va),
               "owner": data["owner"][va] or "-", "rule": data["rule"][va],
               "area": area, "subsystem": data["fun_row"][va],
               **{key: int(bool(status.get(key))) for key in cov.STAT},
               "oracle_class": oracle_class.classify(classes.get(va, set())),
               "oracle_flags": ",".join(sorted(classes.get(va, set()))),
               **boundaries[va].columns(),
               "contract_present": int(bool(contract)),
               **{"contract_" + key: contract.get(key, "") for key in CONTRACT_FIELDS},
               "spec_citations": json.dumps(citations.get(va, []), ensure_ascii=False, sort_keys=True),
               **{"binding_" + key: binding.get(key, "") for key in compare_metrics.REGISTRY_FIELDS if key != "address"},
               "binding_file": Path(binding["file"]).relative_to(ROOT).as_posix() if binding else "",
               **{key: int(states[va][key]) for key in compare_metrics.METRICS},
               **{key: states[va][key] for key in ("natural_calls", "natural_refused", "natural_sampled", "refusal_ratio")},
               "shadow_calls": (reached or {}).get(va, 0),
               "match_reason": states[va]["reason"],
               "matched_logs": json.dumps(states[va]["matched_logs"]),
               "compare_evidence": json.dumps(evidence.get(va, {}), sort_keys=True, default=sorted)}
        yield row


def answering_cost(rows):
    implemented = {r["address"] for r in rows if r["implemented"]}
    targets, imports, slots = set(), set(), set()
    game, library, other = set(), set(), set()
    ready_bytes = ready_nonempty_bytes = 0
    for row in rows:
        if not row["implemented"]:
            continue
        direct = set()
        for kind, combined in (("game", game), ("library", library), ("other", other)):
            values = set(json.loads(row["direct_" + kind + "_targets"]))
            direct.update(values)
            combined.update(values)
        targets.update(direct)
        imports.update(json.loads(row["import_names"]))
        slots.update(json.loads(row["indirect_slots"]))
        if direct <= implemented:
            ready_bytes += row["bytes"]
            if direct:
                ready_nonempty_bytes += row["bytes"]
    return dict(direct=len(targets), game=len(game), library=len(library), other=len(other),
                imports=len(imports), slots=len(slots), ready_bytes=ready_bytes,
                ready_nonempty_bytes=ready_nonempty_bytes)


def pilot_picks(rows):
    """Five owners per boundary class, ranked only by reached unmatched bytes."""
    eligible = defaultdict(list)
    for row in rows:
        if (row["implemented"] and not row["matched"]
                and (row.get("natural_calls", 0) > 0 or row.get("shadow_calls", 0) > 0)):
            eligible[(row["boundary_class"], row["owner"])].append(row)
    picks = []
    for boundary in function_boundary.CLASSES:
        owners = [(owner, functions) for (cls, owner), functions in eligible.items() if cls == boundary]
        owners.sort(key=lambda pair: (-sum(r["bytes"] for r in pair[1]), pair[0]))
        for owner, functions in owners[:5]:
            example = min(functions, key=lambda r: (-r["bytes"], r["address"]))
            picks.append(dict(boundary=boundary, owner=owner, functions=len(functions),
                              bytes=sum(r["bytes"] for r in functions), example=example["address"]))
    return picks


def summary(rows, regress, logs, min_natural, min_dense, warnings, evidence_note=""):
    total = sum(r["bytes"] for r in rows)
    lines = ["# Run-7 function ledger", "",
             f"Game code: {len(rows):,} functions, {total:,} bytes. House regress: `{regress.name}`.",
             f"Compare logs: {len(logs)}; thresholds natural={min_natural}, corpus/dense={min_dense} per source in one log.", "",
             "| State | Functions | Bytes | Game bytes |", "|---|---:|---:|---:|"]
    for key in (*cov.STAT, *compare_metrics.METRICS):
        selected = [r for r in rows if r.get(key)]
        size = sum(r["bytes"] for r in selected)
        lines.append(f"| {key} | {len(selected):,} | {size:,} | {cov.pct(size, total)} |")
    lines += ["", "Matched needs natural plus positive replay tiers in the same log; zero-compare replay sources "
              "demand no threshold. Natural refused/sampled must be <=10%."]
    if evidence_note:
        lines.append(evidence_note)
    implemented_bytes = sum(r["bytes"] for r in rows if r["implemented"])
    for label, field, classes in (("Oracle", "oracle_class", oracle_class.CLASSES),
                                  ("Boundary", "boundary_class", function_boundary.CLASSES)):
        lines += ["", f"| {label} class | Functions | Bytes | Implemented functions | Implemented bytes | Share of implemented bytes |",
                  "|---|---:|---:|---:|---:|---:|"]
        for key in classes:
            selected = [r for r in rows if r[field] == key]
            impl = [r for r in selected if r["implemented"]]
            size = sum(r["bytes"] for r in impl)
            lines.append(f"| {key} | {len(selected):,} | {sum(r['bytes'] for r in selected):,} | "
                         f"{len(impl):,} | {size:,} | {cov.pct(size, implemented_bytes)} |")
    cost = answering_cost(rows)
    lines += ["", "Answering cost over implemented code (distinct targets/names/slot displacements):",
              f"- Direct callees: {cost['direct']:,} (game {cost['game']:,}, library/compiler {cost['library']:,}, other {cost['other']:,}).",
              f"- Imports: {cost['imports']:,}; indirect slot displacements: {cost['slots']:,}.",
              f"- Implemented bytes whose direct callees are all implemented: {cost['ready_bytes']:,} "
              f"({cov.pct(cost['ready_bytes'], implemented_bytes)}); with at least one direct callee: {cost['ready_nonempty_bytes']:,}.",
              "  Includes functions with no direct callees; library/compiler and unknown targets are not implemented.",
              "", "Direct counts are distinct callees; import/indirect counts are call sites in the function body only.",
              "Library/compiler means Ghidra-exported entries outside coverage's game set; imports are PE IAT calls.",
              "Slot displacements are nonnegative, dword-aligned single-register candidates (excluding ESP/EBP).",
              "Global flags cover memory accesses to PE .data/.rdata, including indexed globals, excluding LEA/immediates.",
              "", "Ten largest owners by implemented-but-unmatched bytes; both class splits cover ALL implemented owner bytes:", "",
              "| Owner | Implemented unmatched bytes | Implemented bytes | O:closed | O:globals | O:vtable | O:os | B:leaf | B:globals | B:direct | B:indirect | B:import |",
              "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"]
    owners, oracle_bytes, boundary_bytes = defaultdict(Counter), defaultdict(Counter), defaultdict(Counter)
    for row in rows:
        if row["implemented"]:
            owner = row["owner"]
            owners[owner]["implemented"] += row["bytes"]
            owners[owner]["unmatched"] += row["bytes"] if not row["matched"] else 0
            oracle_bytes[owner][row["oracle_class"]] += row["bytes"]
            boundary_bytes[owner][row["boundary_class"]] += row["bytes"]
    for owner in sorted(owners, key=lambda o: (-owners[o]["unmatched"], o))[:10]:
        split = [oracle_bytes[owner][key] for key in oracle_class.CLASSES]
        split += [boundary_bytes[owner][key] for key in function_boundary.CLASSES]
        lines.append(f"| {owner} | {owners[owner]['unmatched']:,} | {owners[owner]['implemented']:,} | " +
                     " | ".join(f"{size:,}" for size in split) + " |")
    picks = pilot_picks(rows)
    lines += ["", "Pilot picks: up to five owners per boundary class, ranked by implemented, unmatched bytes "
              "with positive natural compare or named-method house shadow calls. Aggregate shadows do not prove function reach.",
              "", "| Boundary | Owner | Reached functions | Reached unmatched bytes | Example address |",
              "|---|---|---:|---:|---|"]
    for pick in picks:
        lines.append(f"| {pick['boundary']} | {pick['owner']} | {pick['functions']} | {pick['bytes']:,} | {pick['example']} |")
    for cls in function_boundary.CLASSES:
        count = sum(p["boundary"] == cls for p in picks)
        if count < 5:
            lines.append(f"Shortfall {cls}: {count}/5 owners have identifiable positive function calls.")
    lines += ["", f"Entry contracts: {sum(r['contract_present'] for r in rows):,}; "
              f"functions with direct spec citations: {sum(r['spec_citations'] != '[]' for r in rows):,}.",
              f"Warnings: {len(warnings)} (full list in inputs.json)."]
    return "\n".join(lines) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--export")
    ap.add_argument("--exe")
    ap.add_argument("--regress")
    ap.add_argument("--log", action="append", help="explicit game log (repeatable); overrides house compare logs")
    ap.add_argument("--min-natural", type=int, default=20)
    ap.add_argument("--min-dense", type=int, default=200)
    ap.add_argument("--out", type=Path, default=ROOT / "build-runs/ledger/functions.tsv")
    args = ap.parse_args()
    if min(args.min_natural, args.min_dense) < 1:
        ap.error("compare thresholds must be positive")
    data = cov.load_report_data(args.export, args.exe, args.regress)
    house_logs = compare_metrics.regress_logs(data["regress"])
    house_build = compare_metrics.regress_build(data["regress"])
    logs = [Path(p) for p in args.log] if args.log else house_logs
    states, bindings, evidence, warnings = compare_metrics.collect_states(
        data["game"], ROOT / "engine/plugin/compare", logs, data["base"], args.min_natural, args.min_dense, house_build)
    exe = cov.find_input("bin/SimCity4.exe", args.exe)
    iat = oracle_class.read_iat(exe)
    classes = oracle_class.scan(data["export"], iat)
    boundaries = function_boundary.scan(data["export"], {va for va, *_ in data["game"]}, iat,
                                        function_boundary.data_sections(exe))
    contracts = read_contracts(ROOT / "docs/abi/functions.tsv")
    citations = spec_citations(ROOT / "docs/specs", {va for va, *_ in data["game"]})
    reached = shadow_reach(data, house_logs)
    rows = list(ledger_rows(data, states, bindings, evidence, classes, contracts, citations, boundaries, reached))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]), delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    warnings = data["warnings"] + warnings
    report = summary(rows, data["regress"], logs, args.min_natural, args.min_dense, warnings,
                     compare_metrics.evidence_note(states, warnings, house_build))
    args.out.with_name("summary.md").write_text(report, encoding="utf-8", newline="\n")
    paths = [data["export"] / name for name in ("functions.tsv", "function-details.tsv", "calls.tsv",
             "instructions.asm", "data-references.tsv", "export-summary.txt")]
    paths += list(data["inputs"].values()) + logs + house_logs
    paths += [ROOT / "docs/abi/functions.tsv"] + sorted((ROOT / "engine/plugin/compare").glob("*.tsv"))
    paths += sorted({ROOT / citation["file"] for refs in citations.values() for citation in refs})
    manifest = {"exe_sha256": data["exe_sha"], "min_natural": args.min_natural, "min_dense": args.min_dense,
                "logs": [str(p.resolve()) for p in logs], "house_build": house_build,
                "shadow_logs": [str(p.resolve()) for p in house_logs], "warnings": warnings,
                "text_sha256": {str(p.resolve()): cov.text_sha256(p) for p in paths if p.is_file()},
                "contracts_present": (ROOT / "docs/abi/functions.tsv").is_file(),
                "registry_files": [str(p) for p in sorted((ROOT / "engine/plugin/compare").glob("*.tsv"))]}
    args.out.with_name("inputs.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(report, end="")
    print(f"Wrote {args.out}")


if __name__ == "__main__":
    main()
