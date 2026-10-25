"""Read-only catalog of lot configuration exemplars from installed SC4 game data.

No executable/code inputs. Uses the DBPF reader in this directory, including QFS
and cohort inheritance. Output is deterministic TSV; category comes from the
lot's building OccupantGroups, not a guess based on a display name.
"""
import argparse
import collections
import csv
from dataclasses import dataclass
import io
import re
import struct
from pathlib import Path
import sys

from dbpf import DBPF, TYPE_COHORT, TYPE_EXEMPLAR, parse_exemplar

EXEMPLAR_TYPE, EXEMPLAR_NAME = 0x10, 0x20
OCCUPANT_GROUPS, LOT_SIZE = 0xAA1DD396, 0x88EDC790
OBJECT_FIRST, OBJECT_LAST = 0x88EDC900, 0x88EDCFFF
CATEGORIES = (
    ("power", {0x1400}), ("water", {0x1401}), ("garbage", {0x1402}),
    ("police", {0x1500, 0x1501}), ("fire", {0x1502}), ("health", {0x1507}),
    ("education", {0x1503, 0x1504, 0x1505, 0x1506}),
    ("landmarks", {0x150A}), ("rewards", {0x150B}),
    ("transit", {0x1003}),
)
FIELDS = ("iid", "name", "category", "group", "width", "depth", "building_iids", "occupant_groups", "source")
INSTALL = Path("C:/Program Files (x86)/GOG Galaxy/Games/SimCity 4 Deluxe Edition")


@dataclass
class Record:
    parent: tuple
    props: dict
    source: str


def values(props, key):
    return props.get(key, (None, []))[1]


def category(groups):
    found = [name for name, ids in CATEGORIES if ids & groups]
    if 0x1006 in groups and not {"landmarks", "rewards"}.intersection(found):
        found.append("parks")
    if 0x1004 in groups and not {"power", "water", "garbage", "transit"}.intersection(found):
        found.append("utilities")
    return ",".join(found) or "other"


def package_paths(install, plugins):
    # Base archives first, expansion overrides second, user plugins last.
    base = sorted(p for p in install.rglob("*.dat") if p.name.lower() not in {"ep1.dat", "unins000.dat"})
    expansion = sorted(install.rglob("EP1.dat"))
    user = sorted((p for p in plugins.rglob("*") if p.is_file() and p.suffix.lower() in
                   {".dat", ".sc4lot", ".sc4desc"}), key=lambda p: str(p).lower()) if plugins.exists() else []
    return [(p, "install/" + p.relative_to(install).as_posix()) for p in base + expansion] + [
        (p, "Plugins/" + p.relative_to(plugins).as_posix()) for p in user]


def read_records(paths):
    records, warnings = {}, []
    for path, label in paths:
        try:
            package = DBPF(path)
        except (OSError, ValueError, IndexError, struct.error) as exc:
            warnings.append(f"{label}: {exc}")
            continue
        for entry in package.entries:
            if entry[0] not in (TYPE_EXEMPLAR, TYPE_COHORT):
                continue
            try:
                data = package.read(entry)
                parent, props = parse_text_exemplar(data) if data[:4] in (b'EQZT', b'CQZT') else parse_exemplar(data)
            except (ValueError, NotImplementedError, KeyError, IndexError, struct.error) as exc:
                warnings.append(f"{label} {entry[2]:08X}: {exc}")
                continue
            records[entry[:3]] = Record(parent, props, label)
    return records, warnings


def parse_text_exemplar(data):
    """Read the text data form used by some shipped Maxis landmark packages."""
    text = data.decode('latin1')
    parent_match = re.search(r'ParentCohort=Key:\{([^}]+)\}', text)
    if not parent_match:
        raise ValueError('text exemplar has no ParentCohort')
    group, instance, kind = (int(v.strip(), 0) for v in parent_match[1].split(','))
    props = {}
    pattern = r'^(0x[0-9a-fA-F]+):\{".*?"\}=(\w+):\d+:\{(.*)\}\s*$'
    for line in text.splitlines()[3:]:
        if not line.strip():
            continue
        match = re.match(pattern, line)
        if not match:
            raise ValueError('unsupported text exemplar property')
        pid, ptype, raw = match.groups()
        if ptype == 'String':
            val = raw.strip().strip('"')
        else:
            val = []
            for token in filter(None, (part.strip() for part in raw.split(','))):
                if ptype.startswith('Float'):
                    val.append(float(token))
                elif ptype == 'Bool':
                    val.append(token.lower() in ('true', '1', '0x01'))
                else:
                    val.append(int(token, 0))
        props[int(pid, 16)] = (ptype, val)
    return (kind, group, instance), props


def catalog(records):
    cache = {}

    def inherited(tgi, trail=()):
        if tgi in cache:
            return cache[tgi]
        if tgi in trail:
            raise ValueError(f"cohort cycle at {tgi}")
        rec = records.get(tgi)
        if rec is None:
            return {}
        result = dict(inherited(rec.parent, trail + (tgi,)))
        result.update(rec.props)
        cache[tgi] = result
        return result

    buildings, families = collections.defaultdict(list), collections.defaultdict(list)
    for tgi in records:
        props = inherited(tgi)
        if tgi[0] == TYPE_EXEMPLAR and values(props, EXEMPLAR_TYPE) == [2]:
            groups = set(values(props, OCCUPANT_GROUPS))
            buildings[tgi[2]].append(groups)
            for family in values(props, 0x27812870):  # Building/prop family IDs
                families[family].append(groups)
    rows = []
    for tgi, rec in sorted(records.items(), key=lambda item: (item[0][2], item[0][1], item[0][0])):
        props = inherited(tgi)
        if tgi[0] != TYPE_EXEMPLAR or values(props, EXEMPLAR_TYPE) != [16]:
            continue
        iids, groups = set(), set()
        for pid, (_, obj) in props.items():
            if OBJECT_FIRST <= pid <= OBJECT_LAST and len(obj) >= 13 and obj[0] == 0:
                iids.update(obj[12:])  # building object: IDs start after the 12 fixed words
        for iid in iids:
            for group_set in buildings.get(iid, []) + families.get(iid, []):
                groups.update(group_set)
        size = values(props, LOT_SIZE)
        rows.append(dict(iid=f"0x{tgi[2]:08X}", name=values(props, EXEMPLAR_NAME) or "",
                         category=category(groups), group=f"0x{tgi[1]:08X}",
                         width=size[0] if size else "", depth=size[1] if len(size) > 1 else "",
                         building_iids=",".join(f"0x{i:08X}" for i in sorted(iids)),
                         occupant_groups=",".join(f"0x{g:08X}" for g in sorted(groups)), source=rec.source))
    return rows


def render(rows):
    out = io.StringIO(newline="")
    writer = csv.DictWriter(out, FIELDS, delimiter="\t", lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
    return out.getvalue()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--install", type=Path, default=INSTALL)
    parser.add_argument("--plugins", type=Path, default=Path.home() / "Documents/SimCity 4/Plugins")
    parser.add_argument("--output", type=Path, help="TSV file (otherwise stdout)")
    args = parser.parse_args(argv)
    paths = package_paths(args.install, args.plugins)
    if not paths:
        parser.error("no DBPF game data packages found")
    records, warnings = read_records(paths)
    rows = catalog(records)
    for warning in warnings:
        print("warning: " + warning, file=sys.stderr)
    text = render(rows)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text, encoding="utf-8", newline="")
    else:
        sys.stdout.write(text)
    print(f"lotconfigs: {len(rows)} lots, {len(paths)} packages, {len(warnings)} warnings; "
          f"categories={dict(sorted(collections.Counter(r['category'] for r in rows).items()))}", file=sys.stderr)
    return 1 if warnings or not rows else 0


if __name__ == "__main__":
    raise SystemExit(main())
