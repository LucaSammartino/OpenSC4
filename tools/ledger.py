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
    from tools import compare_metrics, oracle_class
except ModuleNotFoundError:
    import compare_metrics
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


def ledger_rows(data, states, bindings, evidence, classes, contracts, citations):
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
               "contract_present": int(bool(contract)),
               **{"contract_" + key: contract.get(key, "") for key in CONTRACT_FIELDS},
               "spec_citations": json.dumps(citations.get(va, []), ensure_ascii=False, sort_keys=True),
               **{"binding_" + key: binding.get(key, "") for key in compare_metrics.REGISTRY_FIELDS if key != "address"},
               "binding_file": Path(binding["file"]).relative_to(ROOT).as_posix() if binding else "",
               **{key: int(states[va][key]) for key in ("bound", "matched", "replaced-fn")},
               "match_reason": states[va]["reason"],
               "matched_logs": json.dumps(states[va]["matched_logs"]),
               "compare_evidence": json.dumps(evidence.get(va, {}), sort_keys=True, default=sorted)}
        yield row


def summary(rows, regress, logs, min_natural, min_dense, warnings):
    total = sum(r["bytes"] for r in rows)
    lines = ["# Run-7 function ledger", "",
             f"Game code: {len(rows):,} functions, {total:,} bytes. House regress: `{regress.name}`.",
             f"Compare logs: {len(logs)}; thresholds natural={min_natural}, corpus/dense={min_dense} per source in one log.", "",
             "| State | Functions | Bytes | Game bytes |", "|---|---:|---:|---:|"]
    for key in (*cov.STAT, "bound", "matched", "replaced-fn"):
        selected = [r for r in rows if r[key]]
        size = sum(r["bytes"] for r in selected)
        lines.append(f"| {key} | {len(selected):,} | {size:,} | {cov.pct(size, total)} |")
    lines += ["", "| Oracle class | Functions | Bytes |", "|---|---:|---:|"]
    for key in oracle_class.CLASSES:
        selected = [r for r in rows if r["oracle_class"] == key]
        lines.append(f"| {key} | {len(selected):,} | {sum(r['bytes'] for r in selected):,} |")
    lines += ["", "Ten largest owners by implemented-but-unmatched bytes (oracle split in bytes):", "",
              "| Owner | Bytes | Closed | Globals | Vtable | OS |", "|---|---:|---:|---:|---:|---:|"]
    owners = defaultdict(Counter)
    for row in rows:
        if row["implemented"] and not row["matched"]:
            owners[row["owner"]][row["oracle_class"]] += row["bytes"]
    for owner in sorted(owners, key=lambda o: (-sum(owners[o].values()), o))[:10]:
        split = " | ".join(f"{owners[owner][key]:,}" for key in oracle_class.CLASSES)
        lines.append(f"| {owner} | {sum(owners[owner].values()):,} | {split} |")
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
    logs = [Path(p) for p in args.log] if args.log else compare_metrics.regress_logs(data["regress"])
    states, bindings, evidence, warnings = compare_metrics.collect_states(
        data["game"], ROOT / "engine/plugin/compare", logs, data["base"], args.min_natural, args.min_dense)
    classes = oracle_class.scan(data["export"], oracle_class.read_iat(cov.find_input("bin/SimCity4.exe", args.exe)))
    contracts = read_contracts(ROOT / "docs/abi/functions.tsv")
    citations = spec_citations(ROOT / "docs/specs", {va for va, *_ in data["game"]})
    rows = list(ledger_rows(data, states, bindings, evidence, classes, contracts, citations))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]), delimiter="\t", lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    warnings = data["warnings"] + warnings
    report = summary(rows, data["regress"], logs, args.min_natural, args.min_dense, warnings)
    args.out.with_name("summary.md").write_text(report, encoding="utf-8", newline="\n")
    paths = [data["export"] / name for name in ("functions.tsv", "function-details.tsv", "calls.tsv",
             "instructions.asm", "data-references.tsv", "export-summary.txt")]
    paths += list(data["inputs"].values()) + logs
    paths += [ROOT / "docs/abi/functions.tsv"] + sorted((ROOT / "engine/plugin/compare").glob("*.tsv"))
    paths += sorted({ROOT / citation["file"] for refs in citations.values() for citation in refs})
    manifest = {"exe_sha256": data["exe_sha"], "min_natural": args.min_natural, "min_dense": args.min_dense,
                "logs": [str(p.resolve()) for p in logs], "warnings": warnings,
                "text_sha256": {str(p.resolve()): cov.text_sha256(p) for p in paths if p.is_file()},
                "contracts_present": (ROOT / "docs/abi/functions.tsv").is_file(),
                "registry_files": [str(p) for p in sorted((ROOT / "engine/plugin/compare").glob("*.tsv"))]}
    args.out.with_name("inputs.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(report, end="")
    print(f"Wrote {args.out}")


if __name__ == "__main__":
    main()
