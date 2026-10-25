#!/usr/bin/env python3
"""Join entry contracts to coverage.py's unchanged game/implemented population."""
import argparse
import csv
import importlib.util
import json
import random
from collections import Counter
from pathlib import Path

from abi_return_use import DECIDED
from re_query import main_checkout

ROOT = Path(__file__).resolve().parents[1]


def population(export, exe):
    spec = importlib.util.spec_from_file_location('fc4_abi_coverage', ROOT / 'tools/coverage.py')
    cov = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(cov)
    _, lo, hi = cov.pe_text(exe)
    att = cov.attribution_for_tree(ROOT, export, lo, hi)
    owner, _ = att.run()
    rows = cov.read_map(ROOT / 'tools/coverage-map.tsv')
    # Call the shared status implementation; no verified/replaced evidence is needed.
    cov.map_statuses(rows, [], [], {}, {'runs': {}, 'shadows': {}}, {}, 'abi-report')
    game, *_ = cov.map_functions(rows, att, owner)
    return [(f'{va:08x}', size, owner.get(va) or area,
             i is not None and rows[i]['implemented']) for va, size, i, area in game]


def read_table(path):
    with path.open(encoding='utf-8', newline='') as fh:
        return {r['address']: r for r in csv.DictReader(fh, delimiter='\t')}


def measure(table, game):
    out = {}
    for scope in ('implemented', 'game'):
        selected = [p for p in game if scope == 'game' or p[3]]
        stats, owners = Counter(), Counter()
        undecided = []
        for address, size, owner, _ in selected:
            r = table.get(address, {})
            stats['functions'] += 1
            stats['bytes'] += size
            decided = r.get('ret') in DECIDED
            high = r.get('ret_confidence') in ('exact', 'high')
            stats['decided_bytes'] += size * decided
            stats['decided_high_bytes'] += size * (decided and high)
            stats['convention_pop_high_bytes'] += size * (r.get('confidence') in ('exact', 'high')
                and r.get('convention') != 'unknown' and r.get('callee_pop', '') != '')
            stats['unresolved_pop_functions'] += r.get('callee_pop', '') == ''
            if not decided or not high:
                owners[owner] += size
                undecided.append({'address': address, 'bytes': size, 'owner': owner,
                                  'ret': r.get('ret'), 'ret_confidence': r.get('ret_confidence')})
        out[scope] = dict(stats)
        for key in ('decided', 'decided_high', 'convention_pop_high'):
            out[scope][key + '_pct'] = round(100 * stats[key + '_bytes'] / max(1, stats['bytes']), 3)
        out[scope]['undecided_top20_owners'] = owners.most_common(20)
        out[scope]['undecided_rows'] = undecided
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    main_root = main_checkout(ROOT)
    parser.add_argument('--export', type=Path, default=main_root / 'ghidra/export/windows')
    parser.add_argument('--exe', type=Path, default=main_root / 'bin/SimCity4.exe')
    parser.add_argument('--table', type=Path, default=ROOT / 'docs/abi/functions.tsv')
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--report', type=Path, default=ROOT / 'build-runs/abi-functions/round2-metrics.json')
    parser.add_argument('--sample', type=Path, default=ROOT / 'build-runs/abi-functions/return-sample.json')
    parser.add_argument('--seed', type=int, default=20261025)
    args = parser.parse_args()
    table = read_table(args.table)
    game = population(args.export, args.exe)
    report = measure(table, game)
    if args.baseline:
        before = read_table(args.baseline)
        report['baseline'] = measure(before, game)
        changed = sorted(a for a, _, _, impl in game if impl and a in before and a in table
                         and before[a]['ret'] != table[a]['ret'])
        report['changed_implemented_functions'] = len(changed)
        # Never resample after fixes: preserve the first deterministic draw.
        if not args.sample.exists():
            selected = sorted(random.Random(args.seed).sample(changed, min(50, len(changed))))
            args.sample.parent.mkdir(parents=True, exist_ok=True)
            args.sample.write_text(json.dumps({'seed': args.seed, 'population': len(changed),
                'addresses': selected, 'cases': [{'before': before[a], 'after': table[a]}
                                               for a in selected]}, indent=2) + '\n', encoding='utf-8')
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    for scope in ('implemented', 'game'):
        print(scope, json.dumps({k: v for k, v in report[scope].items()
                                 if not k.startswith('undecided_')}, sort_keys=True))


if __name__ == '__main__':
    main()
