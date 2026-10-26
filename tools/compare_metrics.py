"""Run-7 registry and value-only in-place comparison evidence (standard library only).

Never pool observations across logs or add cumulative report snapshots. A bad
observation in any supplied log vetoes credit; a passing log must independently
meet the thresholds. Natural and replay tiers are independent; matched needs
both in one log when positive replay evidence exists. Fixture compares must
also be exact, but do not satisfy the natural/corpus/dense thresholds.
"""
import csv
import re
from collections import defaultdict
from pathlib import Path

REGISTRY_FIELDS = ("name", "address", "class", "convention", "this", "stack_arg_dwords",
                   "callee_pop", "ret", "windows", "rust_entry", "mode")
COUNTERS = ("calls", "sampled", "compared", "exact", "float_only", "different", "outside",
            "refused", "nested", "mutation", "detected", "max_ulp")
SOURCES = ("natural", "fixture", "corpus", "dense")
BAD = ("float_only", "different", "outside", "mutation", "detected")
METRICS = ("bound", "matched-natural", "matched-dense", "matched", "replaced-fn")


def read_registry(directory):
    """Optional registry. Ambiguous/invalid rows fail closed, rather than picking one."""
    bindings, warnings = {}, []
    invalid = set()
    for path in sorted(Path(directory).glob("*.tsv")):
        with path.open(encoding="utf-8-sig", newline="") as stream:
            rows = csv.DictReader((s for s in stream if s.strip() and not s.startswith("#")), delimiter="\t")
            if not set(REGISTRY_FIELDS) <= set(rows.fieldnames or ()):
                raise ValueError(f"{path}: missing registry columns")
            for number, row in enumerate(rows, 2):
                address = row["address"] or ""
                if not re.fullmatch(r"[0-9a-f]{8}", address):
                    raise ValueError(f"{path}:{number}: invalid registry address {address!r}")
                va = int(address, 16)
                if va in bindings or va in invalid:
                    bindings.pop(va, None)
                    invalid.add(va)
                    warnings.append(f"{address}: duplicate registry rows; binding void")
                    continue
                row["file"] = str(path)
                bindings[va] = row
    return bindings, warnings


def text_lines(path):
    """Stream UTF-8 game logs and PowerShell's UTF-16 output without loading them whole."""
    with Path(path).open("rb") as stream:
        bom = stream.read(2)
    encoding = "utf-16" if bom in (b"\xff\xfe", b"\xfe\xff") else "utf-8-sig"
    with Path(path).open(encoding=encoding, errors="replace") as stream:
        yield from stream


def regress_logs(regress):
    """Only the house summary and its recorded raw logs, never unrelated lane runs."""
    path = Path(regress)
    return [p for p in (path, path.with_name(path.stem + "-shadows.log"),
                       path.with_name(path.stem + "-replace.log")) if p.is_file()]


def regress_build(regress):
    """The recorded DLL hash, never the currently staged (possibly newer) DLL."""
    hashes = set()
    for line in text_lines(regress):
        match = re.match(r"staged DLL: .*\(sha256 ([0-9a-fA-F]{8,64})\)", line)
        if match:
            hashes.add(match[1].lower())
    return next(iter(hashes)) if len(hashes) == 1 else None


def build_matches(actual, expected):
    if expected is None:
        return False
    actual, expected = actual.lower(), expected.lower()
    # regress.ps1 records eight hex characters; the harness may emit the full hash.
    return actual == expected or (re.fullmatch(r"[0-9a-f]{8,64}", actual) is not None
                                 and re.fullmatch(r"[0-9a-f]{8,64}", expected) is not None
                                 and (actual.startswith(expected) or expected.startswith(actual)))


def read_comparisons(paths, image_base=0x400000, bindings=None, expected_build=None):
    """VA -> log -> source -> cumulative counter maxima, retaining every failure.

    S3 joins by RVA; S4 (no RVA) joins by a unique registry name. Reported byte
    counts are diagnostic only: all credited bytes come from the game table.
    Build-stamped lines must match the house DLL; stale lines cannot veto or
    grant credit. Unstamped historical evidence remains supported.
    """
    evidence, warnings = defaultdict(lambda: defaultdict(dict)), []
    by_name = defaultdict(list)
    for va, binding in (bindings or {}).items():
        by_name[binding["name"]].append(va)
    for path in dict.fromkeys(Path(p).resolve() for p in paths):
        for number, line in enumerate(text_lines(path), 1):
            if "inplace summary " not in line:
                continue
            fields = {}
            duplicate = False
            for item in line.split("inplace summary ", 1)[1].split():
                if "=" not in item:
                    continue
                key, value = item.split("=", 1)
                duplicate |= key in fields
                fields[key] = value
            if "build" in fields and not build_matches(fields["build"], expected_build):
                warnings.append(f"{path}:{number}: stale compare build={fields['build']} "
                                f"(house={expected_build or 'unknown'}); ignored")
                continue
            if "rva" in fields:
                if not re.fullmatch(r"[0-9a-fA-F]{6,8}", fields["rva"]):
                    warnings.append(f"{path}:{number}: invalid compare RVA; no credit")
                    continue
                va = image_base + int(fields["rva"], 16)
            else:
                candidates = by_name.get(fields.get("name", ""), [])
                if len(candidates) != 1:
                    warnings.append(f"{path}:{number}: missing/ambiguous compare name; no credit")
                    continue
                va = candidates[0]
            source = fields.get("source", "invalid")
            bucket = evidence[va][str(path)].setdefault(source, {
                **dict.fromkeys(COUNTERS, 0), "invalid": False, "names": set(),
                "sizes": set(), "installed": True, "reports": 0, "refusal_ratio": 0.0,
                "refusal_refused": 0, "refusal_sampled": 0})
            bucket["reports"] += 1
            try:
                counts = {key: int(fields[key]) for key in COUNTERS}
                size = int(fields["bytes"])
                valid = (not duplicate and source in SOURCES and size >= 0
                         and fields["installed"] in ("0", "1") and bool(fields["name"])
                         and all(n >= 0 for n in counts.values())
                         and counts["exact"] + counts["float_only"] + counts["different"] == counts["compared"]
                         and counts["compared"] <= counts["sampled"] <= counts["calls"])
                bucket["names"].add(fields["name"])
                bucket["sizes"].add(size)
                bucket["installed"] &= fields["installed"] == "1"
                for key, value in counts.items():
                    bucket[key] = max(bucket[key], value)
                if source == "natural":
                    ratio = (counts["refused"] / counts["sampled"] if counts["sampled"]
                             else None if counts["refused"] else 0.0)
                    if bucket["refusal_ratio"] is not None and (ratio is None or ratio >= bucket["refusal_ratio"]):
                        bucket.update(refusal_ratio=ratio, refusal_refused=counts["refused"],
                                      refusal_sampled=counts["sampled"])
            except (KeyError, ValueError):
                valid = False
            if not valid:
                bucket["invalid"] = True
                warnings.append(f"{path}:{number}: malformed compare summary for {va:08x}; no credit")
    return evidence, warnings


def function_state(binding, logs, size, min_natural=20, min_dense=200):
    """Bound, matched, replaced-fn, passing logs and a reviewable rejection reason."""
    if min_natural < 1 or min_dense < 1:
        raise ValueError("compare thresholds must be positive")
    out = {**dict.fromkeys(METRICS, False), "bound": bool(binding),
           "matched_logs": [], "natural_logs": [], "dense_logs": [],
           "natural_calls": 0, "refusal_ratio": 0.0, "natural_refused": 0, "natural_sampled": 0,
           "reason": "unbound" if not binding else "no compare evidence"}
    for sources in logs.values():
        natural = sources.get("natural", {})
        if natural.get("invalid", True):
            continue
        out["natural_calls"] = max(out["natural_calls"], natural["calls"])
        ratio = natural["refusal_ratio"]
        if out["refusal_ratio"] is not None and (ratio is None or ratio >= out["refusal_ratio"]):
            out.update(refusal_ratio=ratio, natural_refused=natural["refusal_refused"],
                       natural_sampled=natural["refusal_sampled"])
    if not binding or not logs:
        return out
    bad_natural = bad_dense = False
    for sources in logs.values():
        for source, counts in sources.items():
            if (source not in SOURCES or counts["invalid"] or not counts["installed"]
                    or counts["names"] != {binding["name"]}):
                out["reason"] = "non-exact, outside, invalid or incompatible evidence"
                return out
            if any(counts[key] for key in BAD):
                bad_natural |= source in ("natural", "fixture")
                bad_dense |= source in ("corpus", "dense", "fixture")
    refusal_bad = out["refusal_ratio"] is None or out["refusal_ratio"] > 0.1
    # Unused replay sources (compared=0) do not demand a threshold. Positive
    # sources must pass together; evidence from different logs is never pooled.
    dense = {s for sources in logs.values() for s, c in sources.items()
             if s in ("corpus", "dense") and c["compared"] > 0}
    for log, sources in sorted(logs.items()):
        natural = sources.get("natural", {})
        if not bad_natural and not refusal_bad and natural.get("compared", 0) >= min_natural:
            out["natural_logs"].append(log)
        if dense and not bad_dense and all(sources.get(s, {}).get("compared", 0) >= min_dense for s in dense):
            out["dense_logs"].append(log)
    out["matched-natural"] = bool(out["natural_logs"])
    out["matched-dense"] = bool(out["dense_logs"])
    out["matched_logs"] = [log for log in out["natural_logs"] if not dense or log in out["dense_logs"]]
    # A zero-compare replay line with an outside/error signal still vetoes matching.
    if bad_dense:
        out["matched_logs"] = []
    out["matched"] = bool(out["matched_logs"])
    out["replaced-fn"] = out["matched"] and binding["mode"] == "replace"
    out["reason"] = ("exact" if out["matched"] else "natural refusal ratio exceeds 10%" if refusal_bad
                     else "non-exact or outside evidence" if bad_natural or bad_dense
                     else "insufficient natural/corpus/dense compares in one log")
    return out


def collect_states(game, registry_dir, logs, image_base=0x400000, min_natural=20, min_dense=200,
                   expected_build=None):
    bindings, warnings = read_registry(registry_dir)
    evidence, parse_warnings = read_comparisons(logs, image_base, bindings, expected_build)
    states = {va: function_state(bindings.get(va), evidence.get(va, {}), size, min_natural, min_dense)
              for va, size, *_ in game}
    return states, bindings, evidence, warnings + parse_warnings


def tally(game, states):
    """Byte-weighted function metrics, independent of owner-level coverage flags."""
    result = {key + suffix: 0 for key in METRICS for suffix in ("_n", "_b")}
    for va, size, *_ in game:
        for key in METRICS:
            if states[va][key]:
                result[key + "_n"] += 1
                result[key + "_b"] += size
    return result


def evidence_note(states, warnings, expected_build):
    """Small shared audit line; per-function ratios and counters are in the ledger."""
    ratios = list(states.values())
    worst = max(ratios, key=lambda s: (s["refusal_ratio"] is None, s["refusal_ratio"] or 0), default={})
    ratio = worst.get("refusal_ratio", 0)
    display = "undefined" if ratio is None else f"{ratio:.1%}"
    refused, sampled = worst.get("natural_refused", 0), worst.get("natural_sampled", 0)
    blocked = sum(s["refusal_ratio"] is None or s["refusal_ratio"] > 0.1 for s in ratios)
    stale = sum(": stale compare build=" in w for w in warnings)
    return (f"House DLL build: {expected_build or 'unknown'}; stale compare summaries ignored: {stale}. "
            f"Natural refused/sampled: maximum {display} ({refused}/{sampled}); "
            f"above 10% or undefined with refusals: {blocked} functions.")
