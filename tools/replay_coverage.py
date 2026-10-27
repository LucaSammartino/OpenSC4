"""Replay verdict ingestion. Reach always comes from the original recording game.

recordings.json beside manifest.tsv maps scenario names to {"suite": <tag or
absolute suite directory>}. Optional "census" pins fallback evidence; otherwise
the newest census containing each untraced key is used. Legacy manifests without
recording provenance remain readable, but cannot earn replay reach or strict bytes.
"""
from collections import Counter, defaultdict
import csv
import json
from pathlib import Path
import re
import subprocess

SHA = re.compile(r'^[0-9a-f]{64}$')
MANIFEST_COLUMNS = ['scenario', 'key', 'trace_sha', 'comparisons', 'mismatches', 'incomplete']


def key(value):
    return value.removesuffix('_shadow')


def number(row, field):
    value = int(row[field])
    if value < 0:
        raise ValueError(f'negative replay {field}')
    return value


def manifest_rows(path):
    with path.open(encoding='utf-8-sig', newline='') as stream:
        cells = list(csv.reader((s for s in stream if s.strip() and not s.startswith('#')), delimiter='\t'))
    if not cells:
        raise ValueError('replay manifest has no traces')
    rows = []
    for values in cells:
        if len(values) != len(MANIFEST_COLUMNS):
            raise ValueError('replay manifest requires six columns (without a header)')
        row = dict(zip(MANIFEST_COLUMNS, values))
        if (not row['scenario'] or not re.fullmatch(r'[a-z0-9_]+', row['key'])
                or not SHA.fullmatch(row['trace_sha'])):
            raise ValueError('invalid replay scenario, key or trace SHA-256')
        for field in ('comparisons', 'mismatches', 'incomplete'):
            number(row, field)
        rows.append(row)
    return rows


def exact(row):
    return (row['verdict'] == 'exact' and row['shadow_equal'] == 'true'
            and number(row, 'comparisons') > 0 and number(row, 'mismatches') == 0
            and number(row, 'incomplete') == 0 and not row.get('errors', ''))


def pairs(path, sc):
    """Cross-check both replay outputs against every manifest row, including duplicates."""
    rows = manifest_rows(path)
    verdict_path, suite_path = path.parent / 'replay-verdicts.tsv', path.parent / 'replay-suite.tsv'
    verdicts, suites = sc.tsv(verdict_path), sc.tsv(suite_path)
    identity = lambda r: (r['scenario'], r['key'], r['trace_sha'])
    if Counter(map(identity, rows)) != Counter(map(identity, verdicts)):
        raise ValueError('replay verdicts do not cover the manifest exactly')
    sources = {r['source_sha'] for r in verdicts + suites}
    if len(sources) != 1 or not all(SHA.fullmatch(s) for s in sources):
        raise ValueError('replay outputs mix source fingerprints')
    expected, observed = defaultdict(list), defaultdict(list)
    for row in rows:
        expected[identity(row)].append(row)
    for row in verdicts:
        original = expected[identity(row)].pop()
        counters_match = all(row[f] == original[f] for f in ('comparisons', 'mismatches', 'incomplete'))
        observed[(row['scenario'], row['key'])].append((row, counters_match and exact(row)))
    result, seen = {}, set()
    for aggregate in suites:
        pair = aggregate['scenario'], aggregate['key']
        if pair in seen or pair not in observed:
            raise ValueError('duplicate or unrecorded replay suite pair')
        seen.add(pair)
        traces = observed[pair]
        consistent = number(aggregate, 'traces') == len(traces)
        for field in ('comparisons', 'mismatches', 'incomplete'):
            consistent &= number(aggregate, field) == sum(number(r, field) for r, _ in traces)
        result[pair] = consistent and exact(aggregate) and all(ok for _, ok in traces)
    if seen != set(observed):
        raise ValueError('replay suite does not cover every scenario/key')
    return result, {path, verdict_path, suite_path}


def census(root, ref, sc, enabled):
    directory = Path(ref)
    if directory.is_absolute():
        return sc.load(directory.parents[2], directory.name, enabled)
    return sc.load(root, ref, enabled)


def fallbacks(root, metadata, sc, enabled):
    if metadata.get('census'):
        evidence = census(root, metadata['census'], sc, enabled)
        return {k: evidence for item in evidence['scenarios'] for k in item['ran']}
    roots = [root]
    try:
        main = sc.sibling('suite').main_checkout(root)
        if main != root:
            roots.append(main)
    except (OSError, subprocess.SubprocessError):
        pass
    candidates = [p for base in roots for p in (base / 'build-runs/suite').glob('*/per_key.tsv')
                  if (p.parent / 'summary.tsv').is_file()
                  and any((p.parent / f).is_file() for f in ('manifest.json', 'sources.json'))]
    result = {}
    for path in sorted(candidates, key=lambda p: (p.stat().st_mtime_ns, str(p)), reverse=True):
        keys = {r['key'] for r in sc.tsv(path) if int(r['scenarios_run']) > 0} - result.keys()
        if keys:
            evidence = census(root, str(path.parent.resolve()), sc, enabled)
            result.update({k: evidence for k in keys})
    return result


def load(root, value, sc, enabled=()):
    path = Path(value)
    path = (root / path).resolve() if not path.is_absolute() else path.resolve()
    replay_pairs, files = pairs(path, sc)
    metadata_path = path.parent / 'recordings.json'
    metadata = json.loads(metadata_path.read_text(encoding='utf-8')) if metadata_path.is_file() else {}
    if metadata_path.is_file():
        files.add(metadata_path)
    recordings = metadata.get('recordings', {})
    loaded, scenarios, by_name = {}, [], {}
    for name in {name for name, _ in replay_pairs}:
        ref = recordings.get(name, {}).get('suite')
        if ref:
            if ref not in loaded:
                loaded[ref] = census(root, ref, sc, enabled)
                files.update(loaded[ref]['inputs'])
                scenarios.extend(loaded[ref]['scenarios'])
            matches = [s for s in loaded[ref]['scenarios'] if s['name'] == name]
            if len(matches) != 1:
                raise ValueError(f'recording census must contain scenario exactly once: {name}')
            by_name[name] = matches[0]
    per_key = defaultdict(list)
    for (name, owner), ok in replay_pairs.items():
        per_key[key(owner)].append((name, ok))
    key_reached, sources, verified = {}, {}, set()
    for owner, observations in per_key.items():
        reached = set()
        ready = True
        for name, ok in observations:
            recording = by_name.get(name)
            if recording:
                reached.update(recording['reached'])
            ready &= bool(ok and recording and recording['selection_ready'])
        key_reached[owner] = reached
        if ready:
            verified.add(owner)
        sources[owner] = dict(source=f'replay:{path.as_posix()}', exact=ready,
                              scenarios=len(observations))
    fallback = fallbacks(root, metadata, sc, enabled)
    for owner, evidence in fallback.items():
        if owner in per_key:
            continue  # An inexact/incomplete replay must never inherit census exactness.
        key_reached[owner] = evidence['reached']
        ok = owner in evidence['exact']
        if ok:
            verified.add(owner)
        sources[owner] = dict(source=f'census:{evidence["tag"]}', exact=ok,
                              scenarios=sum(owner in s['ran'] for s in evidence['scenarios']))
        files.update(evidence['inputs'])
        scenarios.extend(evidence['scenarios'])
    unique = {(s['source'], s['name']): s for s in scenarios}
    return dict(tag=f'replay:{path}', scenarios=list(unique.values()), exact=verified,
                reached=set().union(*key_reached.values()), key_reached=key_reached,
                replaced=set(), inputs=files, key_sources=sources)
