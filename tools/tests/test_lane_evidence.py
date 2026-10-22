"""Real enqueue/filesystem coverage for the launcher's optional evidence list."""
import json
import os
from pathlib import Path
import subprocess
import sys
from unittest.mock import Mock

import pytest

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
import lane_evidence
import lane_launch


@pytest.fixture
def repo(tmp_path):
    root = tmp_path / 'repo'
    runs = root / 'build-runs'
    runs.mkdir(parents=True)
    (runs / 'run4-refill-queue.json').write_bytes(b'[ {"name":"old", "status":"pending"} ]\n')
    (runs / 'run4-coordinator-state.json').write_bytes(b'{"rounds": 7, "lanes": {}}\n')
    return root


def brief(root, name='new', **fields):
    path = root / 'briefs' / f'{name}.json'
    path.parent.mkdir(exist_ok=True)
    path.write_text(json.dumps(dict(args=dict(name=name, kind='verify', maxRounds=1),
                                    touches=['tools'], **fields)), encoding='utf-8')
    return path


def enqueue(root, *paths, cwd=None):
    return subprocess.run([sys.executable, str(TOOLS / 'lane_refill.py'), '--root', str(root),
                           '--queue', *map(str, paths)], cwd=cwd or root,
                          capture_output=True, text=True, encoding='utf-8')


def ledger(root):
    return [(root / 'build-runs' / name).read_bytes() for name in
            ('run4-refill-queue.json', 'run4-coordinator-state.json')]


@pytest.mark.parametrize('fields', [{}, {'evidence': []}])
def test_enqueue_without_evidence(repo, fields):
    state = ledger(repo)[1]
    result = enqueue(repo, brief(repo, **fields))
    assert result.stdout.strip() == 'queued 1', result.stderr
    assert json.loads(ledger(repo)[0])[-1]['name'] == 'new'
    assert ledger(repo)[1] == state


def test_paths_resolve_against_root_not_cwd_or_spec_parent(repo, tmp_path):
    relative = repo / 'valid space café [1];.txt'
    absolute = tmp_path / 'outside root λ.txt'
    relative.write_bytes(b'relative')
    absolute.write_bytes(b'absolute')
    values = [relative.name, str(absolute)]
    path = brief(repo, evidence=values)
    assert lane_evidence.preflight(repo, json.loads(path.read_text()), path) == [relative, absolute]
    result = enqueue(repo, path, cwd=tmp_path)
    assert result.stdout.strip() == 'queued 1', result.stderr


@pytest.mark.parametrize('value', [
    'same-name guardGO 2026-10-22: python tools/lanes.py check verify-flora-records4 --resume --touches flora',
    '', '   ', 7, None, {}, [], 'missing.txt', '.', 'bad\0path',
])
def test_bad_items_rejected_without_mutation(repo, value):
    path = brief(repo, evidence=[value])
    before = ledger(repo)
    result = enqueue(repo, path)
    assert 'queued 1' not in result.stdout
    assert str(path) in result.stderr and 'evidence[0]=' in result.stderr
    assert repr(value) in result.stderr
    assert ledger(repo) == before


@pytest.mark.parametrize('value', ['file.txt', {}, 1, None])
def test_container_contract(repo, value):
    path = brief(repo, evidence=value)
    before = ledger(repo)
    result = enqueue(repo, path)
    assert 'evidence=' in result.stderr and 'expected a list' in result.stderr
    assert ledger(repo) == before


def test_multispec_is_all_or_nothing(repo):
    first = brief(repo, 'first', evidence=[])
    second = brief(repo, 'second', evidence=['missing'])
    before = ledger(repo)
    result = enqueue(repo, first, second)
    assert 'evidence[0]' in result.stderr
    assert ledger(repo) == before


@pytest.mark.skipif(os.name != 'nt', reason='Windows filename syntax')
@pytest.mark.parametrize('value', ['bad:name', 'bad<name', 'bad|name'])
def test_windows_invalid_path(repo, value):
    with pytest.raises(ValueError, match=r'evidence\[0\]'):
        lane_evidence.preflight(repo, {'evidence': [value]}, 'spec.json')


def test_legal_posix_punctuation(repo):
    if os.name == 'nt':
        pytest.skip('colon is not a Windows filename character')
    path = repo / 'guardGO: punctuation? is legal'
    path.write_bytes(b'ok')
    assert lane_evidence.preflight(repo, {'evidence': [path.name]}, 'spec.json') == [path]


@pytest.mark.parametrize('error', [PermissionError('sharing denied'), OSError('read interrupted'),
                                 ValueError('invalid path')])
def test_read_errors_are_actionable(repo, monkeypatch, error):
    source = repo / 'source.txt'
    source.write_bytes(b'evidence')
    monkeypatch.setattr(Path, 'read_bytes', Mock(side_effect=error))
    with pytest.raises(ValueError, match=r'spec.json: evidence\[0\].*source.txt.*cannot read'):
        lane_evidence.preflight(repo, {'evidence': [source.name]}, 'spec.json')


def test_direct_launch_rechecks_deleted_source_before_any_side_effect(repo, monkeypatch):
    source = repo / 'source.txt'
    source.write_bytes(b'evidence')
    path = brief(repo, evidence=[source.name])
    assert enqueue(repo, path).stdout.strip() == 'queued 1'
    source.unlink()
    before = ledger(repo)
    before_files = sorted(p.relative_to(repo) for p in repo.rglob('*'))
    side_effects = Mock(side_effect=AssertionError('launch side effect before validation'))
    monkeypatch.setattr(lane_launch.backend.codexguard, 'locked', side_effects)
    monkeypatch.setattr(lane_launch.backend, 'write_json', side_effects)
    monkeypatch.setattr(lane_launch, 'checked', side_effects)
    monkeypatch.setattr(lane_launch.subprocess, 'Popen', side_effects)
    with pytest.raises(ValueError, match=r'evidence\[0\].*source.txt'):
        lane_launch.launch(repo, repo / 'build-runs/run4-coordinator-state.json', path)
    side_effects.assert_not_called()
    assert ledger(repo) == before
    assert sorted(p.relative_to(repo) for p in repo.rglob('*')) == before_files
