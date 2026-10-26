"""Audit a census without treating a clean verdict or an estimated tier as proof.

python tools/suite_health.py TAG [--census CENSUS_TAG]
Writes health.json, evidence_gaps.tsv and one finding candidate per failure
signature alongside suite_triage's causes.tsv. Never changes scenario inputs.
"""
import argparse
from collections import Counter
import json
from pathlib import Path

import suite
import suite_coverage
import suite_triage


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
        counts['returned'] += diagnostic['verdict'] != 'missing'
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
    args = parser.parse_args(argv)
    try:
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
