"""Strict replay credit must agree with census and never borrow replay reach."""
import json
from pathlib import Path
import shutil
import sys

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import suite_coverage as metric
from test_suite_coverage import batch

FIXTURE = Path(__file__).parent / 'fixtures/replay'


def recording(root):
    body = 'foo shadow 2000-02-01: mismatches=0\nbar shadow 2000-02-01: mismatches=0\nreach summary reached=1 saved=1\n'
    census = batch(root, bodies={'one': body, 'two': body})
    directory = root / 'build-runs/traces/pilot'
    shutil.copytree(FIXTURE, directory)
    (directory / 'recordings.json').write_text(json.dumps(dict(
        census='test', recordings={n: dict(suite='test') for n in ('one', 'two')})))
    return census, directory


def load(root, directory):
    return metric.load(root, f'replay:{directory / "manifest.tsv"}')


def states(evidence):
    rows = [dict(implemented=True, shadow='foo_shadow'), dict(implemented=True, shadow='bar_shadow')]
    game = [(0x401000, 100, 0, ''), (0x402000, 900, 0, ''), (0x401001, 200, 1, '')]
    return metric.states(evidence, game, rows, {})


def test_fixture_matches_census_and_missing_trace_uses_census(tmp_path):
    _, directory = recording(tmp_path)
    replay, census = load(tmp_path, directory), metric.load(tmp_path, 'test')
    assert states(replay) == states(census)
    assert replay['exact'] == census['exact'] == {'foo', 'bar'}
    assert replay['key_sources']['foo']['source'].startswith('replay:')
    assert replay['key_sources']['bar']['source'] == 'census:test'


@pytest.mark.parametrize('old,new', [('exact', 'inexact'), ('\t0\t0\tfalse', '\t0\t1\tfalse')])
def test_one_bad_trace_loses_strict_without_census_rescue(tmp_path, old, new):
    _, directory = recording(tmp_path)
    path = directory / 'replay-verdicts.tsv'
    path.write_text(path.read_text().replace(old, new, 1))
    evidence = load(tmp_path, directory)
    assert 'foo' not in evidence['exact']
    assert 'bar' in evidence['exact']
    assert states(evidence)[0x401000]['suite-exercised']
    assert not states(evidence)[0x401000]['suite-verified']
    assert evidence['key_sources']['foo']['source'].startswith('replay:')


def test_stale_manifest_counters_do_not_inherit_cached_exactness(tmp_path):
    _, directory = recording(tmp_path)
    path = directory / 'manifest.tsv'
    path.write_text(path.read_text().replace('\t1\t0\t0', '\t1\t0\t3', 1))
    assert 'foo' not in load(tmp_path, directory)['exact']


def test_missing_replay_row_or_mixed_sources_are_rejected(tmp_path):
    _, directory = recording(tmp_path)
    path = directory / 'replay-verdicts.tsv'
    original = path.read_text()
    path.write_text('\n'.join(original.splitlines()[:-1]) + '\n')
    with pytest.raises(ValueError, match='cover the manifest'):
        load(tmp_path, directory)
    path.write_text(original.replace('c' * 64, 'd' * 64, 1))
    with pytest.raises(ValueError, match='source fingerprints'):
        load(tmp_path, directory)


@pytest.mark.parametrize('damage', ['reach', 'failed', 'unfinished', 'provenance'])
def test_recording_must_be_complete_with_original_entry_evidence(tmp_path, damage):
    census, directory = recording(tmp_path)
    if damage == 'reach':
        (census / 'one.reach.tsv').unlink()
    elif damage == 'failed':
        (census / 'one.verdict').write_text('verdict: corrupt\n')
    elif damage == 'unfinished':
        path = census / 'one.log'
        path.write_text(path.read_text().replace('date=2000-04-01', 'date=2000-03-01'))
    else:
        (directory / 'recordings.json').unlink()
    evidence = load(tmp_path, directory)
    assert 'foo' not in evidence['exact']
    assert not states(evidence)[0x401000]['suite-verified']


def test_replay_reach_cannot_leak_from_census_fallback(tmp_path):
    census, directory = recording(tmp_path)
    for name in ('one', 'two'):
        (census / (name + '.reach.tsv')).write_text(
            'address\tentries\tfirst_window\tskipped\n00402000\t1\t1\t\n')
    body = 'bar shadow 2000-02-01: mismatches=0\n'
    batch(tmp_path, tag='fallback', bodies={'other': body})
    meta = directory / 'recordings.json'
    data = json.loads(meta.read_text())
    data['census'] = 'fallback'
    meta.write_text(json.dumps(data))
    evidence = load(tmp_path, directory)
    assert 0x401000 in evidence['reached']
    assert not states(evidence)[0x401000]['suite-exercised']
    assert states(evidence)[0x402000]['suite-verified']


def test_newest_census_per_untraced_key(tmp_path):
    _, directory = recording(tmp_path)
    meta = directory / 'recordings.json'
    data = json.loads(meta.read_text())
    del data['census']
    meta.write_text(json.dumps(data))
    batch(tmp_path, tag='new', bodies={'other': 'bar shadow 2000-02-01: mismatches=1\n'})
    evidence = load(tmp_path, directory)
    assert evidence['key_sources']['bar']['source'] == 'census:new'
    assert 'bar' not in evidence['exact']


def test_path_loaded_coverage_does_not_require_tools_on_sys_path(tmp_path, monkeypatch):
    _, directory = recording(tmp_path)
    module = metric.sibling('suite_coverage')
    monkeypatch.setattr(sys, 'path', [p for p in sys.path if Path(p).resolve() != ROOT / 'tools'])
    assert load(tmp_path, directory)['exact'] == module.load(tmp_path, f'replay:{directory / "manifest.tsv"}')['exact']
