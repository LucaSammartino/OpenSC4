"""Rank unexecuted exact-shadow owners and measure a fixed-census reach delta.

python tools/suite_reach_gaps.py --baseline build-runs/suite/TAG --out build-runs/gaps
Add --after build-runs/suite/NEW to measure the union on the same attribution.
This uses suite_coverage's conservative, whole-census shadow verdicts.
"""
import argparse
from collections import defaultdict
import csv
import json
from pathlib import Path

import suite_coverage


def shadow_key(row, aliases):
    shadow, _, own = row.get('shadow', '-').partition(':')
    return own or aliases.get(shadow.removesuffix('_shadow'), shadow.removesuffix('_shadow'))


def measure(data, baseline, after=None, aliases=None):
    aliases = aliases or {}
    after = after or dict(reached=set(), exact=set(), scenarios=[])
    combined_exact = (baseline['exact'] | after['exact']) - {
        key for item in baseline['scenarios'] + after['scenarios']
        for key in item['ran'] - item['exact']}
    new_reached = after['reached'] - baseline['reached']
    groups = defaultdict(lambda: dict(implemented_bytes=0, gap_bytes=0, gap_functions=0,
                                     newly_executed_bytes=0, newly_verified_bytes=0))
    gaps, metrics = [], dict(game_bytes=0, before_verified_bytes=0, after_verified_bytes=0)
    for address, size, index, _ in data['game']:
        metrics['game_bytes'] += size
        if index is None or not data['rows'][index]['implemented']:
            continue
        row = data['rows'][index]
        key = shadow_key(row, aliases)
        before = address in baseline['reached'] and key in baseline['exact']
        verified = address in baseline['reached'] | after['reached'] and key in combined_exact
        metrics['before_verified_bytes'] += size * before
        metrics['after_verified_bytes'] += size * verified
        if key not in baseline['exact']:
            continue
        owner = data['owner'][address]
        group = groups[owner, key]
        group['implemented_bytes'] += size
        if address not in baseline['reached']:
            group['gap_bytes'] += size
            group['gap_functions'] += 1
            gaps.append(dict(address=f'{address:08x}', bytes=size, owner=owner, key=key,
                             newly_executed=int(address in new_reached)))
        if address in new_reached:
            group['newly_executed_bytes'] += size
            group['newly_verified_bytes'] += size * (key in combined_exact)
    owners = [dict(owner=owner, key=key, **counts,
                   verdict='exact' if key in combined_exact else 'different')
              for (owner, key), counts in groups.items()]
    owners.sort(key=lambda row: (-row['gap_bytes'], row['owner']))
    gaps.sort(key=lambda row: (-row['bytes'], row['address']))
    for prefix in ('before', 'after'):
        metrics[prefix + '_percent'] = 100 * metrics[prefix + '_verified_bytes'] / metrics['game_bytes']
    metrics['gain_points'] = metrics['after_percent'] - metrics['before_percent']
    return owners, gaps, metrics


def load_directory(directory):
    directory = directory.resolve()
    return suite_coverage.load(directory.parent.parent.parent, directory.name)


def contract_addresses(data, baseline, aliases):
    """Bound a lifecycle trace to implemented owners with baseline-exact shadows.

    This changes instrumentation footprint only; byte weights and credit still
    come from coverage.load_report_data and the raw positive entry counters.
    """
    return {address for address, _, index, _ in data['game']
            if index is not None and data['rows'][index]['implemented']
            and shadow_key(data['rows'][index], aliases) in baseline['exact']}


def write_contracts(source, destination, addresses):
    with source.open(encoding='utf-8-sig', newline='') as stream:
        reader = csv.DictReader(stream, delimiter='\t')
        columns = reader.fieldnames
        rows = [row for row in reader if int(row['address'], 16) in addresses]
    suite_coverage.sibling('suite').write_tsv(destination, columns, rows)
    return len(rows)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--after', type=Path)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--contracts', type=Path, help='write a reduced exact-owner entry roster for lifecycle runs')
    args = parser.parse_args(argv)
    coverage = suite_coverage.sibling('coverage')
    data = coverage.load_report_data()
    baseline = load_directory(args.baseline)
    after = load_directory(args.after) if args.after else None
    owners, gaps, metrics = measure(data, baseline, after, coverage.SHADOW_LABEL_ALIAS)
    args.out.mkdir(parents=True, exist_ok=True)
    if args.contracts:
        roster = Path(__file__).resolve().parents[1] / 'docs/abi/functions.tsv'
        count = write_contracts(roster, args.contracts,
                                contract_addresses(data, baseline, coverage.SHADOW_LABEL_ALIAS))
        print(f'exact-owner contracts: {count}; {args.contracts}')
    writer = suite_coverage.sibling('suite').write_tsv
    writer(args.out / 'owners.tsv', ['owner', 'key', 'implemented_bytes', 'gap_bytes', 'gap_functions',
                                   'newly_executed_bytes', 'newly_verified_bytes', 'verdict'], owners)
    writer(args.out / 'functions.tsv', ['address', 'bytes', 'owner', 'key', 'newly_executed'], gaps)
    mismatches = []
    if after:
        for item in after['scenarios']:
            parsed = suite_coverage.sibling('suite').parse_log(
                item['log'].read_text(encoding='utf-8-sig', errors='replace'), item['scenario']['end'])
            for key, counts in parsed['per_key'].items():
                if counts['first_mismatch']:
                    mismatches.append(dict(scenario=item['name'], key=key, log=str(item['log']),
                                           first_mismatch=counts['first_mismatch']))
    writer(args.out / 'mismatches.tsv', ['scenario', 'key', 'log', 'first_mismatch'], mismatches)
    metrics.update(baseline=str(args.baseline.resolve()), after=str(args.after.resolve()) if args.after else None,
                   baseline_scenarios=len(baseline['scenarios']),
                   after_scenarios=len(after['scenarios']) if after else 0)
    (args.out / 'metrics.json').write_text(json.dumps(metrics, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(metrics, sort_keys=True))
    for row in owners[:20]:
        print('{owner}\t{key}\tgap={gap_bytes}\tnew={newly_executed_bytes}\t{verdict}'.format(**row))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
