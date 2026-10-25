"""Byte-weighted scenario reach, conservative suite credit, and greedy tier selection.

python tools/suite_coverage.py TAG [--select --slots 6 --budget-seconds 900]
python tools/suite_coverage.py TAG --combine SHADOW_TAG REPLACE_TAG
All evidence stays in build-runs/suite; no game or Documents files are modified.
"""
import argparse
from collections import defaultdict
import csv
import importlib.util
import json
from pathlib import Path
import re
import shutil

STAT = ('suite-exercised', 'suite-verified', 'suite-replaced')
NAME = re.compile(r'^[A-Za-z0-9][A-Za-z0-9._-]{0,39}$')


def sibling(name):
    path = Path(__file__).with_name(name + '.py')
    module = importlib.util.module_from_spec(importlib.util.spec_from_file_location(name, path))
    exec(compile(path.read_text(encoding='utf-8'), str(path), 'exec'), module.__dict__)
    return module


def tsv(path):
    with path.open(encoding='utf-8-sig', newline='') as stream:
        return list(csv.DictReader((s for s in stream if not s.startswith('#')), delimiter='\t'))


def folder(root, tag):
    if not NAME.fullmatch(tag):
        raise ValueError('suite tag: expected 1..40 letters, digits, dots, underscores or dashes')
    return root / 'build-runs/suite' / tag


def reached(path):
    """Original entry counters only; skips and zero entries never earn bytes."""
    if not path.is_file():
        return set(), {}
    addresses, first = set(), {}
    for row in tsv(path):
        if int(row['entries']) > 0 and row.get('skipped', '') in ('', '-', '0'):
            address = int(row['address'], 16)
            addresses.add(address)
            first[address] = int(row['first_window'])
    return addresses, first


def clean(row):
    return row['verdict'] == 'clean' and str(row['end_reached']) == '1'


def bad_shadow(body, parser):
    # A trailing aggregate zero must never hide an earlier nonzero mismatch.
    counts = re.findall(r'\bmismatch(?:es|ed)?[= ](\d+)', body)
    return (any(int(n) for n in counts) or parser.mismatch(body)
            or parser.resynced(body))


def read_batch(directory):
    """Read summaries, counters, requests and raw logs; fail closed on absent evidence."""
    parser = sibling('suite')
    manifest = json.loads((directory / 'manifest.json').read_text(encoding='utf-8'))
    requests = {r['scenario']['name']: r for r in manifest['requests']}
    aggregate = {r['key']: r for r in tsv(directory / 'per_key.tsv')}
    scenarios, files = [], {directory / 'manifest.json', directory / 'summary.tsv', directory / 'per_key.tsv'}
    seen = set()
    for row in tsv(directory / 'summary.tsv'):
        name = row['scenario']
        if not NAME.fullmatch(name) or name in seen or name not in requests:
            raise ValueError(f'{directory}: invalid, duplicate or unrequested scenario {name!r}')
        seen.add(name)
        request = requests[name]
        if row['profile'] != manifest['profile']:
            raise ValueError(f'{directory}: profile differs from manifest')
        log = directory / (name + '.log')
        trace = directory / (name + '.reach.tsv')
        addresses, first = reached(trace)
        if trace.is_file():
            files.add(trace)
        exact, ran = set(), set()
        actual_clean = False
        if log.is_file():
            files.add(log)
            verdict = directory / (name + '.verdict')
            verdict_text = verdict.read_text(encoding='utf-8-sig') if verdict.is_file() else ''
            if verdict.is_file():
                files.add(verdict)
            parsed = parser.parse_log(log.read_text(encoding='utf-8-sig', errors='replace'),
                                      request['scenario']['end'], verdict_text,
                                      only_load=parser.load_only(request['scenario']))
            actual_clean = clean(parsed)
            for key, counts in parsed['per_key'].items():
                if not counts['lines']:
                    continue
                ran.add(key)
                meta = aggregate.get(key, {})
                if (counts['exact'] == counts['lines'] and not counts['first_mismatch']
                        and int(meta.get('scenarios_run', 0)) > 0
                        and meta.get('scenarios_run') == meta.get('scenarios_exact')
                        and meta.get('first_mismatch') == '-' and clean(row) and actual_clean):
                    exact.add(key)
            # Include summaries too: monthly matching after resync is not proof.
            for line in parsed['shadow_payloads']:
                body = line['text']
                match = parser.SHADOW.match(body)
                if match and bad_shadow(body, parser):
                    key = match[1].replace(' ', '_')
                    ran.add(key)
                    exact.discard(key)
        else:
            # Missing log from a requested shadow can never prove exactness.
            aliases = {'clock24': 'clock', 'automata_owners': 'automataowners'}
            ran.update(aliases.get(k, k) for k in manifest.get('shadow_keys', []))
            ran.update(aggregate)
        enabled = set()
        if row['profile'] == 'replace':
            ini = request['request'].split('[ini]', 1)[-1]
            settings = dict(re.findall(r'^\s*([a-z][a-z0-9_]*)\s*=\s*([^\r\n]+)', ini, re.M))
            enabled = {k.strip() for k in settings.get('enable', '').split(',') if k.strip()}
            enabled -= {k.strip() for k in settings.get('disable', '').split(',')}
        parsed_path = directory / (name + '.json')
        parsed_metrics = json.loads(parsed_path.read_text(encoding='utf-8')) if parsed_path.is_file() else {}
        scenarios.append(dict(name=name, row=row, scenario=request['scenario'], reached=addresses,
                              first=first, ran=ran, exact=exact, enabled=enabled,
                              replace_clean=clean(row) and actual_clean,
                              wall_seconds=parsed_metrics.get('wall_seconds'),
                              windows=parsed_metrics.get('reach_windows', []),
                              trace=trace, log=log, source=directory.name))
    if seen != set(requests):
        raise ValueError(f'{directory}: summary does not contain every requested scenario')
    return scenarios, files


def load(root, tag):
    directory = folder(root, tag)
    index = directory / 'sources.json'
    if index.is_file():
        sources = json.loads(index.read_text(encoding='utf-8'))
        scenarios, files = [], {index, directory / 'summary.tsv', directory / 'per_key.tsv'}
        for source in sources:
            if not NAME.fullmatch(source):
                raise ValueError('invalid bundled source')
            batch, inputs = read_batch(directory / source)
            scenarios.extend(batch)
            files.update(inputs)
    else:
        scenarios, files = read_batch(directory)
    if not scenarios:
        raise ValueError('suite has no scenario evidence')
    union, ran, bad, enabled, failed = set(), set(), set(), set(), set()
    for item in scenarios:
        union.update(item['reached'])
        ran.update(item['ran'])
        bad.update(item['ran'] - item['exact'])
        enabled.update(item['enabled'])
        if not item['replace_clean']:
            failed.update(item['enabled'])
    return dict(tag=tag, scenarios=scenarios, reached=union, exact=ran - bad,
                replaced=enabled - failed, inputs=files)


def states(evidence, game, rows, aliases):
    result = {}
    for address, _, index, _ in game:
        row = rows[index] if index is not None else {}
        implemented = row.get('implemented', False)
        shadow, _, own = row.get('shadow', '-').partition(':')
        label = shadow.removesuffix('_shadow')
        label = own or aliases.get(label, label)
        exercised = implemented and address in evidence['reached']
        result[address] = {'suite-exercised': exercised,
                           'suite-verified': exercised and label in evidence['exact'],
                           'suite-replaced': implemented and row.get('replace') in evidence['replaced']}
    return result


def greedy(items, weights, fraction=0.98, slots=6, budget=900):
    """Deterministic weighted set cover, then delete redundant selected rows.

    This is a greedy heuristic, not a claim of globally optimal set cover. The
    target includes every measured row, even a failed row; candidates must have
    a clean completed game. Time is conservative LPT packing of observed walls.
    """
    if not 0 < fraction <= 1 or slots < 1 or budget <= 0:
        raise ValueError('fraction, slots and budget must be positive (fraction <= 1)')
    select = sibling('suite_select')
    full = set().union(*(r['reached'] for r in items)) & weights.keys()
    total = sum(weights[a] for a in full)
    if not total:
        raise ValueError('no implemented reach evidence')
    threshold = total * fraction
    candidates = [r for r in items if clean(r['row']) and r['wall_seconds'] and r['wall_seconds'] > 0]
    chosen, covered = [], set()
    while sum(weights[a] for a in covered) < threshold:
        options = []
        for item in candidates:
            gain = sum(weights[a] for a in (item['reached'] & full) - covered)
            if gain and select.wall_time(chosen + [item], slots) <= budget:
                options.append((gain, -item['wall_seconds'], item['name'], item))
        if not options:
            break
        best = sorted(options, key=lambda t: (-t[0], -t[1], t[2]))[0][-1]
        chosen.append(best)
        covered.update(best['reached'] & full)
        candidates.remove(best)
    for item in list(reversed(chosen)):
        remainder = [r for r in chosen if r is not item]
        union = set().union(*(r['reached'] for r in remainder)) & full
        if sum(weights[a] for a in union) >= threshold:
            chosen.remove(item)
            covered = union
    reached_bytes = sum(weights[a] for a in covered)
    return dict(selected=[r['name'] for r in chosen], full_bytes=total, reached_bytes=reached_bytes,
                fraction=reached_bytes / total, target=fraction, achieved=reached_bytes >= threshold,
                slots=slots, wall_seconds=select.wall_time(chosen, slots), budget_seconds=budget)


def combine(root, tag, tags):
    """Snapshot batches under one tag so both profiles share a reproducible report."""
    if len(set(tags)) != len(tags) or tag in tags:
        raise ValueError('combine needs distinct source tags, different from destination')
    batches = [(folder(root, source), read_batch(folder(root, source))) for source in tags]
    directory = folder(root, tag)
    if directory.exists():
        raise ValueError('destination tag already exists')
    directory.mkdir(parents=True)
    summary, keys = [], {}
    for source, (items, _) in batches:
        # Keep staged DLLs, scripts, counters and logs together; all are ignored artifacts.
        shutil.copytree(source, directory / source.name)
        summary.extend(tsv(source / 'summary.tsv'))
        for row in tsv(source / 'per_key.tsv'):
            current = keys.setdefault(row['key'], dict(key=row['key'], scenarios_run=0, scenarios_exact=0, first_mismatch='-'))
            for key in ('scenarios_run', 'scenarios_exact'):
                current[key] += int(row[key])
            if row['first_mismatch'] != '-' and current['first_mismatch'] == '-':
                current['first_mismatch'] = row['first_mismatch']
    parser = sibling('suite')
    parser.write_tsv(directory / 'summary.tsv', list(summary[0]), summary)
    parser.write_tsv(directory / 'per_key.tsv', parser.KEY_COLUMNS, [keys[k] for k in sorted(keys)])
    (directory / 'sources.json').write_text(json.dumps(tags, indent=2) + '\n', encoding='utf-8')


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('tag')
    ap.add_argument('--combine', nargs='+', metavar='TAG')
    ap.add_argument('--select', action='store_true')
    ap.add_argument('--fraction', type=float, default=0.98)
    ap.add_argument('--slots', type=int, default=6)
    ap.add_argument('--budget-seconds', type=float, default=900)
    args = ap.parse_args()
    root = Path(__file__).resolve().parents[1]
    try:
        if args.combine:
            combine(root, args.tag, args.combine)
        evidence = load(root, args.tag)
        coverage = sibling('coverage')
        data = coverage.load_report_data()
        weights = {a: size for a, size, i, _ in data['game'] if i is not None and data['rows'][i]['implemented']}
        implemented = sum(weights.values())
        unions = defaultdict(set)
        print('scenario\tprofile\timplemented_bytes\tpercent_implemented\twall_seconds')
        for item in evidence['scenarios']:
            addresses = item['reached'] & weights.keys()
            count = sum(weights[a] for a in addresses)
            print(f'{item["name"]}\t{item["row"]["profile"]}\t{count}\t{count / implemented:.4%}\t{item["wall_seconds"]}')
            for name in item['scenario']['sets'].split(',') + ['all']:
                unions[name].update(addresses)
        lines = []
        for name, addresses in sorted(unions.items()):
            count = sum(weights[a] for a in addresses)
            lines.append(dict(set=name, implemented_bytes=count, implemented_total=implemented,
                              percent=f'{count / implemented:.6%}', functions=len(addresses)))
            print(f'union {name}: {count}/{implemented} implemented bytes ({count / implemented:.4%}); {len(addresses)} functions')
        parser = sibling('suite')
        directory = folder(root, args.tag)
        parser.write_tsv(directory / 'reach_sets.tsv', list(lines[0]), lines)
        if args.select:
            plan = greedy(evidence['scenarios'], weights, args.fraction, args.slots, args.budget_seconds)
            (directory / 'tier1-selection.json').write_text(json.dumps(plan, indent=2) + '\n', encoding='utf-8')
            print(json.dumps(plan, sort_keys=True))
            return 0 if plan['achieved'] else 1
        return 0
    except (ValueError, OSError, KeyError) as exc:
        ap.error(str(exc))


if __name__ == '__main__':
    raise SystemExit(main())
