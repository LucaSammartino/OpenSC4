# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-only value registry compiler. Never reads the game or RE material."""
import argparse
import csv
import hashlib
import json
import re
from pathlib import Path

COLUMNS = "name address class convention this stack_arg_dwords callee_pop ret windows rust_entry mode".split()


def load(root):
    rows = []
    for path in sorted((root / "plugin/compare").glob("*.tsv")):
        with path.open(encoding="utf-8") as stream:
            reader = csv.DictReader(stream, delimiter="\t")
            if reader.fieldnames != COLUMNS:
                raise ValueError(f"{path}: expected {COLUMNS}")
            rows.extend(reader)
    names, addresses = set(), set()
    for row in rows:
        if not re.fullmatch(r"[a-zA-Z0-9_-]+", row["name"]) or row["name"] in names:
            raise ValueError("invalid/duplicate name")
        names.add(row["name"])
        if not re.fullmatch(r"[0-9a-f]{8}", row["address"]) or row["address"] in addresses:
            raise ValueError("invalid/duplicate address")
        addresses.add(row["address"])
        count, pop = int(row["stack_arg_dwords"]), int(row["callee_pop"])
        if not 0 <= count <= 16 or row["convention"] not in ("cdecl", "thiscall", "stdcall") or (row["convention"] == "cdecl" and count > 4):
            raise ValueError("unsupported convention/argument count")
        if pop != (0 if row["convention"] == "cdecl" else count * 4):
            raise ValueError("inconsistent callee cleanup")
        if row["this"] != ("ecx" if row["convention"] == "thiscall" else "none"):
            raise ValueError("inconsistent receiver")
        if row["ret"] not in ("void", "eax", "al", "x87-f32") or row["mode"] not in ("compare", "record", "replace"):
            raise ValueError("unsupported result/mode")
        if not row["windows"] or not row["rust_entry"]:
            raise ValueError("missing windows/adapter")
    return rows


def generate(root, output):
    rows = load(root)
    sizes = {}
    # Transitional published sizes; functions.tsv supersedes these when available.
    for path in (root / "tests/spike-evidence/spike-inplace26").glob("**/functions.csv"):
        with path.open(encoding="utf-8") as stream:
            for row in csv.DictReader(stream):
                sizes[row["name"]] = int(row["body_bytes"])
    contracts = root.parent / "docs/abi/functions.tsv"
    by_address = {}
    if contracts.exists():
        with contracts.open(encoding="utf-8") as stream:
            by_address = {r["address"]: r for r in csv.DictReader(stream, delimiter="\t")}
    lines = ["// Generated from compare/*.tsv; do not edit."]
    for i, row in enumerate(rows):
        lines.append(f'FC4_INPLACE_ENTRY(CompareEntry{i},{i},{row["callee_pop"]})')
    lines.append("Function functions[] = {")
    for i, row in enumerate(rows):
        contract = by_address.get(row["address"])
        size = int(contract["size"]) if contract else sizes.get(row["name"], 0)
        if contract:
            # Unknown facts stay unasserted; concrete conflicts fail the build.
            for column in ("convention", "callee_pop", "stack_arg_dwords"):
                # A method whose body never reads ECX publishes as stdcall/this=none; its callers still pass this,
                # so a thiscall row with the same cleanup describes the same boundary.
                if (column == "convention" and row[column] == "thiscall" and contract[column] == "stdcall"
                        and contract["this"] == "none"):
                    continue
                if contract[column] not in ("", "unknown", "?", row[column]):
                    raise ValueError(f'{row["name"]}: functions.tsv {column} conflicts')
        fingerprint = int.from_bytes(hashlib.sha256("\t".join(row[c] for c in COLUMNS if c != "mode").encode()).digest()[:4], "little")
        strings = [row[c] for c in ("name", "class", "ret", "windows", "rust_entry", "mode")]
        prefix = ",".join(json.dumps(s) for s in strings)
        lines.append(f'    {{{prefix},0x{row["address"]},{size},{row["stack_arg_dwords"]},{row["callee_pop"]},{str(row["this"] == "ecx").lower()},0x{fingerprint:08x},reinterpret_cast<void*>(CompareEntry{i})}},')
    lines.append("};")
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    generate(args.engine, args.output)
