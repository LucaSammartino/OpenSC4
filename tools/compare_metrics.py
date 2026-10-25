"""Run-7 registry and value-only in-place comparison evidence (standard library only).

Never pool observations across logs or add cumulative report snapshots. A bad
observation in any supplied log vetoes credit; a passing log must independently
meet the thresholds. Refused/nested calls are not comparisons. Fixture compares
must also be exact, but do not satisfy the natural/corpus/dense thresholds.
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


def read_comparisons(paths, image_base=0x400000):
    """VA -> log -> source -> cumulative counter maxima, retaining every failure.

    Required fields follow the S3 interface. Additional trailing key=value fields
    are ignored. Malformed evidence with an identifiable RVA is recorded as bad.
    """
    evidence, warnings = defaultdict(lambda: defaultdict(dict)), []
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
            rva = fields.get("rva", "")
            if not re.fullmatch(r"[0-9a-fA-F]{6,8}", rva):
                warnings.append(f"{path}:{number}: invalid compare RVA; no credit")
                continue
            va = image_base + int(rva, 16)
            source = fields.get("source", "invalid")
            bucket = evidence[va][str(path)].setdefault(source, {
                **dict.fromkeys(COUNTERS, 0), "invalid": False, "names": set(),
                "sizes": set(), "installed": True, "reports": 0})
            bucket["reports"] += 1
            try:
                counts = {key: int(fields[key]) for key in COUNTERS}
                size = int(fields["bytes"])
                valid = (not duplicate and source in SOURCES and size > 0
                         and fields["installed"] in ("0", "1") and bool(fields["name"])
                         and all(n >= 0 for n in counts.values())
                         and counts["exact"] + counts["float_only"] + counts["different"] == counts["compared"]
                         and counts["compared"] <= counts["sampled"] <= counts["calls"])
                bucket["names"].add(fields["name"])
                bucket["sizes"].add(size)
                bucket["installed"] &= fields["installed"] == "1"
                for key, value in counts.items():
                    bucket[key] = max(bucket[key], value)
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
    out = {"bound": bool(binding), "matched": False, "replaced-fn": False,
           "matched_logs": [], "reason": "unbound" if not binding else "no compare evidence"}
    if not binding or not logs:
        return out
    for sources in logs.values():
        for source, counts in sources.items():
            if (source not in SOURCES or counts["invalid"] or not counts["installed"]
                    or counts["names"] != {binding["name"]} or counts["sizes"] != {size}
                    or any(counts[key] for key in BAD)):
                out["reason"] = "non-exact, outside, invalid or incompatible evidence"
                return out
    # If any supplied log reports a corpus/dense source, require it in the
    # passing log too. A zero-compare source cannot be silently dropped.
    dense = {s for sources in logs.values() for s in sources if s in ("corpus", "dense")}
    for log, sources in sorted(logs.items()):
        natural = sources.get("natural", {})
        if natural.get("compared", 0) < min_natural:
            continue
        if any(sources.get(s, {}).get("compared", 0) < min_dense for s in dense):
            continue
        out["matched_logs"].append(log)
    out["matched"] = bool(out["matched_logs"])
    out["replaced-fn"] = out["matched"] and binding["mode"] == "replace"
    out["reason"] = "exact" if out["matched"] else "insufficient natural/corpus/dense compares in one log"
    return out


def collect_states(game, registry_dir, logs, image_base=0x400000, min_natural=20, min_dense=200):
    bindings, warnings = read_registry(registry_dir)
    evidence, parse_warnings = read_comparisons(logs, image_base)
    states = {va: function_state(bindings.get(va), evidence.get(va, {}), size, min_natural, min_dense)
              for va, size, *_ in game}
    return states, bindings, evidence, warnings + parse_warnings


def tally(game, states):
    """Byte-weighted function metrics, independent of owner-level coverage flags."""
    result = {key + suffix: 0 for key in ("bound", "matched", "replaced-fn") for suffix in ("_n", "_b")}
    for va, size, *_ in game:
        for key in ("bound", "matched", "replaced-fn"):
            if states[va][key]:
                result[key + "_n"] += 1
                result[key + "_b"] += size
    return result
