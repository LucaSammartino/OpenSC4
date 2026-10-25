#!/usr/bin/env python3
"""Reproducible ABI source comparisons and a seeded cdecl review population.

The report keeps strict agreement separate from explained disagreements. It is
analysis-side evidence, not permission to install an uncertain entry contract.
"""
import argparse
import csv
import io
import json
import random
import subprocess
from collections import Counter
from pathlib import Path

import abi_functions as abi

ROOT = Path(__file__).resolve().parents[1]
S3_FILES = ('engine/tests/spike-evidence/spike-inplace26/functions.csv',
            'engine/tests/spike-evidence/spike-inplace26/round-3/functions.csv')


def spike_contracts(repo=ROOT, revision='77d2819fa'):
    out = []
    for path in S3_FILES:
        text = subprocess.run(['git', '-C', str(repo), 'show', f'{revision}:{path}'],
                              capture_output=True, text=True, check=True).stdout
        for r in csv.DictReader(io.StringIO(text)):
            if 'rva' in r:
                out.append({'name': r['name'], 'address': f'{int(r["rva"], 16) + 0x400000:08x}',
                            'pop': int(r['arg_bytes']) if r['convention'] != 'cdecl' else 0,
                            'args': int(r['arg_bytes']) // 4, 'convention': r['convention'],
                            'source': path})
            else:
                out.append({'name': r['name'], 'address': r['va'].lower(), 'pop': int(r['pop']),
                            'args': int(r['args']), 'source': path})
    return out


def compare(expected, actual, fields):
    return {key: {'expected': str(expected[key]), 'actual': str(actual.get(column, 'missing'))}
            for key, column in fields.items() if str(expected[key]) != str(actual.get(column, 'missing'))}


def summary(items):
    bad = [r for r in items if r['differences']]
    explained = [r for r in bad if r.get('explanation')]
    return {'checked': len(items), 'strict': len(items) - len(bad), 'explained': len(explained),
            'unexplained': len(bad) - len(explained), 'disagreements': bad}


def validate(export, table, evidence, report_path, sample_path, seed=20261025):
    rows = abi.read_tsv(table)
    by = {r['address']: r for r in rows}
    observed = json.loads(evidence.read_text(encoding='utf-8'))
    facts = observed['functions_evidence']
    errors = []
    if len(rows) != len(by):
        errors.append('duplicate addresses')
    if list(rows[0]) != abi.COLUMNS:
        errors.append('wrong columns')
    for row in rows:
        if len(row['address']) != 8 or any(c not in '0123456789abcdef' for c in row['address']):
            errors.append('invalid address: ' + row['address'])
    source = abi.read_tsv(export / 'function-details.tsv')
    for f in source:
        addr = f'{int(f["address"], 16):08x}'
        ranges = [p.split('-') for p in f['ranges'].split(';') if p]
        size = sum(int(b, 16) - int(a, 16) + 1 for a, b in ranges)
        if addr not in by or int(by[addr]['size']) != size:
            errors.append(f'{addr}: missing or wrong size')
    vt_checks = []
    for r in abi.read_tsv(ROOT / 'docs/abi/vtables.tsv'):
        addr = f'{int(r["target"], 16):08x}'
        actual = by.get(addr, {})
        diff = compare(r, actual, {'callee_pop': 'callee_pop', 'ret': 'ret'})
        explain = ''
        notes = actual.get('notes', '')
        if diff and 'vtable-conflict=' in notes:
            explain = 'same target has conflicting reviewed and inferred vtable return labels; high-confidence row retained'
        elif diff and not r['callee_pop'] and set(diff) == {'callee_pop'}:
            explain = 'vtable source leaves pop unspecified; export supplies an independently observed pop'
        vt_checks.append({'address': addr, 'class': r['class'], 'slot': r['slot'],
                          'differences': diff, 'explanation': explain})
    direct_checks = []
    for d in json.loads((ROOT / 'tools/abi_direct_contracts.json').read_text(encoding='utf-8')):
        actual = by.get(d['address'], {})
        fields = {'convention': 'convention', 'pop': 'callee_pop', 'args': 'stack_arg_dwords'}
        if d['ret'] != 'discarded':
            fields['ret'] = 'ret'
        diff = compare(d, actual, fields)
        explanation = ''
        if diff and set(diff) == {'ret'} and 'source=vtables.tsv' in actual.get('notes', ''):
            if actual.get('ret') == 'eax-or-void' and d['ret'] in ('al', 'eax', 'void'):
                explanation = 'vtable EAX-or-void is the machine channel; direct-call source specifies the semantic low-byte/value/void use'
        direct_checks.append({'address': d['address'], 'differences': diff, 'explanation': explanation})
    s3_checks = []
    for d in spike_contracts():
        fields = {'pop': 'callee_pop', 'args': 'stack_arg_dwords'}
        if 'convention' in d:
            fields['convention'] = 'convention'
        actual = by.get(d['address'], {})
        diff = compare(d, actual, fields)
        explanation = ''
        if diff and 's3-disagreement=' in actual.get('notes', ''):
            explanation = actual['notes'].split('s3-disagreement=', 1)[1].split(';', 1)[0]
        s3_checks.append({'address': d['address'], 'name': d['name'], 'differences': diff,
                          'explanation': explanation, 'fields_checked': list(fields)})
    ghidra_checks, categories = [], Counter()
    for f in source:
        if f['calling_convention'] == 'unknown':
            continue
        addr = f'{int(f["address"], 16):08x}'
        actual = by[addr]
        expected = f['calling_convention'].lstrip('_')
        category = ''
        if expected != actual['convention']:
            live = set(facts[addr]['reg_args'])
            if expected == 'thiscall' and 'ecx' not in live:
                category = 'Ghidra-thiscall-without-observed-ECX-use'
            elif expected == 'thiscall' and 'edx' in live:
                category = 'additional-EDX-live-in'
            elif actual['convention'] == 'unknown' and live:
                category = 'nonstandard-register-live-ins'
            elif actual['convention'] == 'unknown':
                category = 'no-stack-arguments-or-unresolved-pop'
            elif actual['convention'] in ('thiscall', 'fastcall'):
                category = 'observed-register-arguments'
            else:
                category = 'stack-cleanup-convention-disagrees'
            categories[category] += 1
        ghidra_checks.append({'address': addr, 'differences': {} if not category else
                              {'convention': {'expected': expected, 'actual': actual['convention']}},
                              'explanation': category})
    # Restrict to cdecls with a direct caller so both requested sources can be inspected.
    population = sorted(r['address'] for r in rows if r['convention'] == 'cdecl' and int(r['callers']))
    selected = sorted(random.Random(seed).sample(population, min(50, len(population))))
    sample = {'seed': seed, 'population': len(population), 'addresses': selected,
              'cases': [{**by[a], 'raw': facts[a]} for a in selected]}
    sample_path.parent.mkdir(parents=True, exist_ok=True)
    sample_path.write_text(json.dumps(sample, indent=2) + '\n', encoding='utf-8')
    report = {'rows': len(rows), 'exported': len(source), 'vtable_targets': len({r['address'] for r in vt_checks}),
              'schema_errors': errors, 'vtables': summary(vt_checks), 'direct_calls': summary(direct_checks),
              'spike_s3': summary(s3_checks), 'ghidra': summary(ghidra_checks),
              'ghidra_disagreement_categories': dict(categories), 'sample_seed': seed,
              'sample_population': len(population), 'sample_count': len(selected),
              'extraction_seconds': observed['seconds']}
    report_path.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    for name in ('vtables', 'direct_calls', 'spike_s3', 'ghidra'):
        s = report[name]
        print(f'{name}: checked={s["checked"]} strict={s["strict"]} explained={s["explained"]} '
              f'unexplained={s["unexplained"]}')
    print(f'rows={len(rows)} exported={len(source)} schema_errors={len(errors)} '
          f'sample={len(selected)}/{len(population)} seed={seed} seconds={observed["seconds"]}')
    return 1 if errors or any(report[k]['unexplained'] for k in ('vtables', 'direct_calls', 'spike_s3')) else 0


def main():
    from re_query import main_checkout
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--export', type=Path, default=main_checkout(ROOT) / 'ghidra/export/windows')
    parser.add_argument('--table', type=Path, default=ROOT / 'docs/abi/functions.tsv')
    parser.add_argument('--evidence', type=Path, default=ROOT / 'build-runs/abi-functions/evidence.json')
    parser.add_argument('--report', type=Path, default=ROOT / 'build-runs/abi-functions/validation.json')
    parser.add_argument('--sample', type=Path, default=ROOT / 'build-runs/abi-functions/sample.json')
    args = parser.parse_args()
    raise SystemExit(validate(args.export, args.table, args.evidence, args.report, args.sample))


if __name__ == '__main__':
    main()
