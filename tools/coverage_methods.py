"""Address-bound method evidence for coverage; no name-only or inferred byte credit."""
import csv
import re
from collections import defaultdict


HEADER = re.compile(
    r"(?:^|\] )(?P<family>[a-z0-9_-]+(?: [a-z0-9_-]+)*) method "
    r"(?P<address>0x[0-9a-fA-F]+|[0-9a-fA-F]{8}|-) (?P<name>\S+) "
    r"\(month\) \d{4}-\d\d-\d\d: (?P<body>.*)$")
COUNTERS = re.compile(r"calls=(\d+) complete=(\d+) mismatches=(\d+); resyncs=(\d+)\s*$")
SUMMARY = re.compile(
    r"^\s+(?P<family>[a-z0-9_-]+) method (?P<address>0x[0-9a-fA-F]+|[0-9a-fA-F]{8}|-) "
    r"(?P<name>\S+): (?P<lines>\d+) lines, (?P<exact>\d+) exact, "
    r"calls=(?P<calls>\d+) complete=(?P<complete>\d+); "
    r"mismatches=(?P<mismatches>\d+); resyncs=(?P<resyncs>\d+); counters_valid=(?P<valid>[01])\s*$")


def family_label(value):
    return re.sub(r"(?:[ _-]shadow)$", "", value).replace(" ", "_")


def identity(match):
    address = match["address"]
    return (family_label(match["family"]), None if address == "-" else int(address, 16), match["name"])


def read_lines(lines, summary=False):
    """Keep all failures and the last cumulative counters, never their sum or maximum."""
    out = {}
    for line in lines:
        m = (SUMMARY if summary else HEADER).search(line.rstrip())
        if not m:
            continue
        key = identity(m)
        if summary:
            current = {k: int(m[k]) for k in
                       ("lines", "exact", "calls", "complete", "mismatches", "resyncs", "valid")}
            # Duplicate summaries are ambiguous (split runs must be summarised from their joined log).
            if key in out:
                current["valid"] = 0
            out[key] = current
            continue
        current = out.setdefault(key, dict(lines=0, exact=0, calls=0, complete=0,
                                          mismatches=0, resyncs=0, valid=1))
        current["lines"] += 1
        counters = COUNTERS.fullmatch(m["body"])
        if not counters:
            current["valid"] = 0
            continue
        calls, complete, mismatches, resyncs = map(int, counters.groups())
        if calls < current["calls"] or complete < current["complete"] or complete > calls:
            current["valid"] = 0
        current["calls"], current["complete"] = calls, complete
        current["mismatches"] += bool(mismatches)
        current["resyncs"] += bool(resyncs)
        current["exact"] += mismatches == resyncs == 0
    return out


def exact(record):
    return (record["valid"] == 1 and record["lines"] > 0 and record["exact"] == record["lines"]
            and record["calls"] > 0 and record["complete"] == record["calls"]
            and record["mismatches"] == record["resyncs"] == 0)


def credit(paths, att, owners, game, reg):
    """Return eligible addresses, provenance and warnings. Conflicts void an address across all sources.

    TSVs name canonical coverage identities (including folded bodies). Repeated names at distinct starts
    are fine; duplicate TSV addresses, conflicting names/owners/sizes and unknown starts fail closed.
    TOTAL rows are audit annotations. Library functions never enter the game-code denominator.
    """
    claims, warnings, blocked = defaultdict(list), [], set()
    game_addresses = {va for va, _, _, _ in game}
    tsv_seen = set()
    for path in paths:
        with path.open(encoding="utf-8-sig", newline="") as stream:
            reader = csv.DictReader(stream, delimiter="\t")
            required = {"address", "function_name", "owner", "bytes", "shadow_key", "evidence_family"}
            fields = set(reader.fieldnames or ())
            if not required <= fields:
                # This directory also holds owner/core/map audits; they make no method claims.
                if fields & {"shadow_key", "evidence_family"}:
                    warnings.append(f"method evidence `{path.name}`: missing columns {sorted(required - fields)}")
                continue
            for row in reader:
                address = row["address"].strip()
                if address == "TOTAL":
                    continue
                try:
                    va = int(address, 16)
                except (ValueError, TypeError):
                    warnings.append(f"method evidence `{path.name}`: invalid address/bytes `{address}`; not credited")
                    continue
                try:
                    size = int(row["bytes"])
                except (ValueError, TypeError):
                    blocked.add(va)
                    warnings.append(f"method evidence 0x{va:08X}: invalid byte size; not credited")
                    continue
                if va in tsv_seen:
                    blocked.add(va)
                    warnings.append(f"method evidence 0x{va:08X}: duplicate TSV address collision; not credited")
                tsv_seen.add(va)
                label, key = row["evidence_family"].strip(), row["shadow_key"].strip()
                clean, lines = reg["shadows"].get(label, (0, 0))
                proven = lines > 0 and clean == lines and reg.get("resynced", {}).get(label) == 0
                if not label or not re.fullmatch(r"[a-z0-9_]+_shadow", key):
                    proven = False
                    warnings.append(f"method evidence 0x{va:08X}: missing family or invalid shadow key; not credited")
                claims[va].append((row["function_name"], row["owner"], size, proven,
                                   f"{path.name}:{key}:{label}"))
    for (label, va, name), record in reg.get("methods", {}).items():
        if va is None:
            warnings.append(f"method evidence `{label}`/{name}: no address (interface slot); not credited")
            continue
        claims[va].append((name, owners.get(va), att.funcs.get(va), exact(record), f"log:{label}"))
        if not record["valid"]:
            warnings.append(f"method evidence 0x{va:08X}: malformed or decreasing counters; not credited")
        if not record["valid"] or record["mismatches"] or record["resyncs"]:
            blocked.add(va)
    eligible, provenance = set(), {}
    for va, entries in sorted(claims.items()):
        if va not in att.funcs:
            warnings.append(f"method evidence 0x{va:08X}: unknown function start; not credited")
            continue
        if va not in game_addresses:
            warnings.append(f"method evidence 0x{va:08X}: excluded library function; not credited")
            continue
        expected = (att.name(va), owners[va], att.funcs[va])
        if any(entry[:3] != expected for entry in entries):
            blocked.add(va)
            warnings.append(f"method evidence 0x{va:08X}: name/owner/bytes collision with coverage table; not credited")
        if va not in blocked and any(entry[3] for entry in entries):
            eligible.add(va)
            provenance[va] = sorted({entry[4] for entry in entries if entry[3]})
    return eligible, provenance, warnings


def function_status(rows, index, address, methods, status):
    return ((index is not None and rows[index][status])
            or (status == "verified" and address in methods))


def incremental(game, rows, methods):
    return [(va, size) for va, size, index, _ in game
            if va in methods and not (index is not None and rows[index]["verified"])]
