"""Measured reach, timing, saturation and deterministic tier selection for suite.py."""
from collections import Counter
import csv
from datetime import date
import fnmatch
import json
from pathlib import Path


def tsv(path):
    with path.open(encoding='utf-8-sig', newline='') as stream:
        return list(csv.DictReader((line for line in stream if not line.startswith('#')), delimiter='\t'))


def reach(path):
    """The reach27 tracer's address TSV; a skipped/counter-zero row is not coverage."""
    if not path.is_file():
        return set(), {}
    rows = tsv(path)
    found, first = set(), {}
    for row in rows:
        if int(row['entries']) > 0 and row.get('skipped', '') in ('', '-', '0'):
            address = f"{int(row['address'], 16):08x}"
            found.add(address)
            first[address] = int(row['first_window'])
    return found, first


def knee(windows, fraction=0.01):
    """First month adding <=1% of observed union, after at least one full month.

    This is an observed knee, not an extrapolation beyond the last traced window.
    Window 1 includes load/partial-month work and is never sufficient alone.
    """
    if len(windows) < 2:
        return None
    threshold = max(1, int(windows[-1]['total'] * fraction))
    return next((i for i, row in enumerate(windows[1:], 1) if row['new'] <= threshold), len(windows) - 1)


def wall_time(items, slots):
    """Conservative longest-first packing, including measured load and shutdown."""
    loads = [0.0] * slots
    for item in sorted(items, key=lambda r: (-r['wall_seconds'], r['name'])):
        slot = min(range(slots), key=lambda i: (loads[i], i))
        loads[slot] += item['wall_seconds']
    return max(loads)


def measurements(root, scenarios, tags=None):
    """One latest passing measurement per current scenario; never infer reach from counts."""
    import hashlib
    import suite
    wanted = {r['name']: r for r in scenarios}
    folders = ([root / 'build-runs/suite' / tag for tag in tags] if tags else
               sorted((root / 'build-runs/suite').glob('*'), key=lambda p: p.stat().st_mtime))
    found = {}
    for folder in folders:
        manifest_path = folder / 'manifest.json'
        if not manifest_path.is_file():
            continue
        manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
        for request in manifest['requests']:
            row = request['scenario']
            name = row['name']
            result_path = folder / (name + '.json')
            if name not in wanted or not result_path.is_file():
                continue
            current = wanted[name]
            if any(row[k] != current[k] for k in ('region', 'city', 'script', 'end')):
                continue
            source = current['script']
            if source != '-':
                staged = folder / 'scripts' / (name + Path(source).suffix)
                if not staged.is_file() or hashlib.sha256(staged.read_bytes()).digest() != hashlib.sha256((root / source).read_bytes()).digest():
                    continue
            parsed = json.loads(result_path.read_text(encoding='utf-8'))
            if not suite.successful({name: parsed}):
                continue
            wall = parsed.get('wall_seconds')
            if wall is None or wall <= 0:
                continue
            addresses, first = reach(folder / (name + '.reach.tsv'))
            if not addresses and not parsed.get('per_key'):
                continue
            found[name] = dict(name=name, wall_seconds=wall, addresses=addresses,
                first_windows=first, per_key=parsed['per_key'],
                load_seconds=parsed.get('load_seconds'),
                seconds_per_month=parsed.get('seconds_per_month'),
                windows=parsed.get('reach_windows', []),
                city_bytes=parsed.get('city_bytes') or 2**63, log=parsed.get('log', '-'),
                source=folder.name, only_load=suite.load_only(current))
    return list(found.values())


def targets(root, keys):
    """Map shadow/replace keys and owner names to function addresses, read-only.

    The ABI table is enough for class-named functions; the name export supplies
    later RE names without changing the coverage tool's ownership calculation.
    """
    requested = {key.strip().lower() for key in keys.split(',') if key.strip()}
    if not requested:
        return set(), set()
    rules = tsv(root / 'tools/coverage-map.tsv')
    patterns = set()
    for rule in rules:
        if requested & {rule['subsystem'].lower(), rule['shadow'].lower(), rule['replace'].lower()}:
            patterns.add(rule['pattern'].removeprefix('m:').removeprefix('fn:').lower())
    patterns.update(key + '*' for key in requested)
    addresses = set()
    paths = (root / 'docs/abi/functions.tsv', root / 're/names_windows.tsv')
    for path in paths:
        if not path.is_file():
            continue
        for row in tsv(path):
            name = row.get('name', '').lower()
            if any(fnmatch.fnmatchcase(name, pattern) or fnmatch.fnmatchcase(name.split('::')[0], pattern)
                   for pattern in patterns):
                addresses.add(f"{int(row.get('address') or row['va'], 16):08x}")
    # Log labels use *_shadow's base, with neighbours/landvalue aliases.
    labels = {key.removesuffix('_shadow') for key in requested}
    aliases = {'landvalue': 'land_value', 'neighbors': 'neighbours', 'nettool': 'nettool_plan'}
    labels |= {aliases[k] for k in labels if k in aliases}
    return addresses, labels


def choose(items, *, slots=6, budget=900, target_addresses=None, target_keys=None):
    """Greedy new-functions/wall-second cover, then AFL-style zero-marginal cull.

    With reach TSVs the objective is functions. Targeted selection may fall back
    to observed per-key activity when no tracer addresses exist, and says so.
    """
    target_addresses, target_keys = target_addresses or set(), target_keys or set()
    targeted = bool(target_addresses or target_keys)
    use_functions = any(item['addresses'] & target_addresses if targeted else item['addresses'] for item in items)
    candidates = []
    for item in items:
        features = ((item['addresses'] & target_addresses if targeted else item['addresses']) if use_functions else
                    {'key:' + k for k in item['per_key'] if k in target_keys})
        if features:
            candidates.append(dict(item, features=features))
    if not use_functions and not targeted:
        raise ValueError('tier 1 requires per-scenario reach TSVs; run calibration with --reach (or use --set smoke)')
    union = set().union(*(item['features'] for item in candidates))
    selected, covered = [], set()
    remaining = list(candidates)
    while remaining:
        ranked = sorted(remaining, key=lambda item: (
            -len(item['features'] - covered) / item['wall_seconds'],
            -sum(v['lines'] for k, v in item['per_key'].items() if k in target_keys) / item['wall_seconds'] if targeted else 0,
            item['city_bytes'], item['name']))
        best = next((item for item in ranked if item['features'] - covered
                     and wall_time(selected + [item], slots) <= budget), None)
        if best is None:
            break
        selected.append(best)
        covered.update(best['features'])
        remaining.remove(best)
    # A later choice can subsume an earlier one. Cull the most costly redundant
    # rows first; keep one deterministic representative when two are identical.
    for item in sorted(selected, key=lambda r: (-r['wall_seconds'], -r['city_bytes'], r['name'])):
        others = set().union(*(r['features'] for r in selected if r is not item))
        if item['features'] <= others:
            selected.remove(item)
    counts = Counter(feature for item in selected for feature in item['features'])
    for item in selected:
        item['marginal_functions'] = sum(counts[f] == 1 for f in item['features']) if use_functions else None
        item['knee_months'] = knee(item['windows'])
    return dict(selected=selected, objective='functions' if use_functions else 'observed-keys',
                union_reach=len(covered), full_union_reach=len(union),
                wall_seconds=wall_time(selected, slots), full_wall_seconds=wall_time(candidates, slots),
                unmeasured_note='Only completed, current, passing scenarios with measured wall time qualify.')


def serializable(plan):
    return json.loads(json.dumps(plan, default=lambda value: sorted(value) if isinstance(value, set) else value))
