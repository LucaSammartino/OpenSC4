"""Audit a census without treating a clean verdict or an estimated tier as proof.

python tools/suite_health.py TAG [--census CENSUS_TAG]
python tools/suite_health.py CURRENT --reconcile ORIGINAL CORRECTED [CORRECTED...]
Writes health.json, evidence_gaps.tsv and one finding candidate per failure
signature alongside suite_triage's causes.tsv. Never changes scenario inputs.
Reconciliation preserves failed attempts and their union reach; only the last
listed attempt controls each scenario's health. It never supplies measured wall time.
"""
import argparse
from collections import Counter
import json
from pathlib import Path
import shutil

import suite
import suite_coverage
import suite_triage


def reconcile(root, tag, tags):
    """Explicitly supersede rows in source order, retaining every raw attempt.

    A failed or absent later result supersedes an earlier success too. This is
    not best-result selection. Sources must have drained before snapshotting;
    the reconciled timing is deliberately not a measured parallel run.
    """
    if not tags or len(set(tags)) != len(tags) or tag in tags:
        raise ValueError('reconcile needs distinct sources different from destination')
    directory = suite_coverage.folder(root, tag)
    if directory.exists():
        raise ValueError('destination tag already exists')
    sources, selected, keys, attempts = [], {}, set(), []
    for source in tags:
        folder = suite_coverage.folder(root, source)
        manifest = json.loads((folder / 'manifest.json').read_text(encoding='utf-8'))
        if sources and manifest['profile'] != sources[0][1]['profile']:
            raise ValueError('reconcile requires one profile')
        for request in manifest['requests']:
            name = request['scenario']['name']
            if sources and name not in selected:
                raise ValueError(f'correction scenario absent from baseline: {name}')
            label = request.get('label', '')
            if label and not (root / 'build-runs/game-results' / (label + '.verdict')).exists():
                if any((root / rel / (label + suffix)).exists() for rel, suffix in (
                        ('build-runs/game-queue', '.req'), ('build-runs/game-results', '.running'))):
                    raise ValueError(f'source still queued/running: {source}/{name}')
            selected[name] = (folder, dict(request, evidence_source=source,
                                         dll_sha256=manifest['dll_sha256']))
            attempts.append(dict(source=source, scenario=name,
                                 verdict_file=str(folder / (name + '.verdict'))))
        keys.update(manifest.get('shadow_keys', []))
        sources.append((folder, manifest))
    directory.mkdir(parents=True)
    for source, _ in sources:
        shutil.copytree(source, directory / source.name)
    manifest = dict(tag=tag, profile=sources[0][1]['profile'], dll_sha256='mixed',
                    shadow_keys=sorted(keys), requests=[r for _, r in selected.values()],
                    reconciliation=dict(sources=tags, policy='last-listed-attempt'), timing={})
    results = {}
    for name, (source, request) in selected.items():
        for suffix in ('.log', '.verdict', '.reach.tsv', '.lua.log', '.req', '.json', '.shadows.txt'):
            path = source / (name + suffix)
            if path.is_file():
                shutil.copy2(path, directory / path.name)
        scenario = request['scenario']
        log, verdict = directory / (name + '.log'), directory / (name + '.verdict')
        parsed = suite.parse_log(log.read_text(encoding='utf-8-sig', errors='replace') if log.exists() else '',
                                 scenario['end'], verdict.read_text(encoding='utf-8-sig') if verdict.exists() else '',
                                 only_load=suite.load_only(scenario))
        if Path(scenario['script']).suffix.lower() == '.lua':
            lua = directory / (name + '.lua.log')
            suite.add_lua_evidence(parsed, lua.read_text(encoding='utf-8-sig', errors='replace') if lua.exists() else '')
        elif scenario['script'] != '-' and not parsed['actions_expected'] and not parsed['action_lines']:
            parsed['actions_pending'] = 1
        parsed['log'] = str(log) if log.exists() else '-'
        results[name] = parsed
    (directory / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    (directory / 'sources.json').write_text(json.dumps(tags, indent=2) + '\n', encoding='utf-8')
    suite.write_results(directory, manifest, results)
    suite.write_tsv(directory / 'attempts.tsv', ['source', 'scenario', 'verdict_file'], attempts)
    return directory


def audit(root, tag):
    directory = suite_coverage.folder(root, tag)
    manifest = json.loads((directory / 'manifest.json').read_text(encoding='utf-8'))
    triage, _ = suite_triage.triage(directory)
    gaps, candidates, seen, union = [], [], set(), set()
    counts = Counter()
    shadows = Counter()
    for request, diagnostic in zip(manifest['requests'], triage):
        scenario = request['scenario']
        name = scenario['name']
        log = directory / (name + '.log')
        verdict = directory / (name + '.verdict')
        parsed = suite.parse_log(log.read_text(encoding='utf-8-sig', errors='replace') if log.exists() else '',
                                 scenario['end'], verdict.read_text(encoding='utf-8-sig') if verdict.exists() else '',
                                 only_load=suite.load_only(scenario))
        if Path(scenario['script']).suffix.lower() == '.lua':
            lua = directory / (name + '.lua.log')
            suite.add_lua_evidence(parsed, lua.read_text(encoding='utf-8-sig', errors='replace') if lua.exists() else '')
        elif scenario['script'] != '-' and not parsed['actions_expected'] and not parsed['action_lines']:
            parsed['actions_pending'] = 1
        addresses, _ = suite_coverage.reached(directory / (name + '.reach.tsv'))
        union.update(addresses)
        clean = diagnostic['verdict'] == 'clean' and diagnostic['end_reached'] == '1'
        missing = []
        if not clean:
            missing.append('clean-end')
        if not addresses or not parsed.get('reach_lines'):
            missing.append('reach')
        if not parsed.get('shadow_lines'):
            missing.append('shadows')
        if not log.exists() or parsed['actions_pending']:
            missing.append('pending-actions')
        # Native refusals are evidence, but do not pass the strict action gate.
        counts['clean'] += clean
        counts['complete'] += not missing
        counts['strict'] += not missing and not parsed.get('actions_failed', 0)
        counts['returned'] += diagnostic['verdict'] not in ('missing', 'not-submitted', 'queue-timeout')
        counts['traces'] += bool(addresses)
        for key in ('shadow_lines', 'shadow_exact_lines', 'actions_ok', 'actions_failed'):
            shadows[key] += parsed.get(key, 0)
        if missing or parsed.get('actions_failed', 0):
            gaps.append(dict(scenario=name, missing=','.join(missing) or '-',
                             actions_failed=parsed.get('actions_failed', '-'), log=diagnostic['log']))
        if diagnostic['owner'] == 'game-or-engine':
            signature = (diagnostic['cause'], diagnostic['location'] if diagnostic['location'] != '-'
                         else diagnostic['address'])
            if signature not in seen:
                seen.add(signature)
                candidates.append(dict(scenario=name, cause=signature[0], address=diagnostic['address'],
                                       location=diagnostic['location'],
                                       evidence=diagnostic['first_exception_or_stall'], log=diagnostic['log']))
    # Earlier failed attempts remain in the reach denominator after a repair.
    for source in manifest.get('reconciliation', {}).get('sources', []):
        source_manifest = json.loads((directory / source / 'manifest.json').read_text(encoding='utf-8'))
        for request in source_manifest['requests']:
            addresses, _ = suite_coverage.reached(directory / source / (request['scenario']['name'] + '.reach.tsv'))
            union.update(addresses)
    total = len(manifest['requests'])
    health = dict(tag=tag, requested=total, **counts, **shadows,
                  clean_complete_share=counts['complete'] / total,
                  strict_share=counts['strict'] / total,
                  verdicts=dict(Counter(r['verdict'] for r in triage)),
                  causes=dict(Counter(r['cause'] for r in triage if r['verdict'] != 'clean')),
                  timing=manifest.get('timing', {}),
                  census_passed=counts['complete'] >= total * .8,
                  strict_passed=counts['strict'] >= total * .8)
    suite.write_tsv(directory / 'evidence_gaps.tsv', ['scenario', 'missing', 'actions_failed', 'log'], gaps)
    suite.write_tsv(directory / 'finding_candidates.tsv', ['scenario', 'cause', 'address', 'location', 'evidence', 'log'], candidates)
    return health, union


def compare_tier(root, health, addresses, census):
    reference, original = audit(root, census)
    coverage = suite_coverage.sibling('coverage')
    data = coverage.load_report_data()
    weights = {a: size for a, size, i, _ in data['game']
               if i is not None and data['rows'][i]['implemented']}
    denominator = sum(weights[a] for a in original & weights.keys())
    numerator = sum(weights[a] for a in addresses & original & weights.keys())
    timing = health['timing']
    share = numerator / denominator if denominator else 0
    measured = (timing.get('complete') is True and timing.get('slots') == 6
                and 0 < timing.get('wall_seconds', 0) <= 900)
    health['tier1'] = dict(census=census, census_union_bytes=denominator,
                          reached_union_bytes=numerator, union_share=share,
                          measured_six_slot_wall=timing.get('wall_seconds'),
                          passed=(reference['census_passed'] and measured and share >= .95
                                  and health['complete'] == health['requested']))


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('tag')
    parser.add_argument('--census', help='compare a real six-slot tier run with this census union')
    parser.add_argument('--reconcile', nargs='+', metavar='TAG', help='baseline then explicit correction batches; retain all attempts')
    args = parser.parse_args(argv)
    try:
        if args.reconcile:
            reconcile(suite.ROOT, args.tag, args.reconcile)
        health, addresses = audit(suite.ROOT, args.tag)
        if args.census:
            compare_tier(suite.ROOT, health, addresses, args.census)
        output = suite_coverage.folder(suite.ROOT, args.tag) / 'health.json'
        output.write_text(json.dumps(health, indent=2) + '\n', encoding='utf-8')
    except (OSError, ValueError, KeyError) as exc:
        parser.error(str(exc))
    print(json.dumps(health, sort_keys=True))
    return 0 if (health.get('tier1', {}).get('passed') if args.census else health['census_passed']) else 1


if __name__ == '__main__':
    raise SystemExit(main())
