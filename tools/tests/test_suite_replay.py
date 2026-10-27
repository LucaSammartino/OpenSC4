"""Recording identities/reuse, opt-in queue requests and CPU replay failures."""
import hashlib
import json
from pathlib import Path
import shutil
import sys
from types import SimpleNamespace
from unittest.mock import patch

import pytest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import suite
import suite_replay as trace
from test_suite import checkout
from test_suite_coverage import batch


def inputs(root, monkeypatch):
    row = checkout(root)
    exe, city = root / 'game.exe', root / 'source.sc4'
    exe.write_bytes(b'game fixture')
    city.write_bytes(b'city fixture')
    monkeypatch.setattr(suite, 'city_path', lambda row: city)
    return row, exe


def test_identity_tracks_exe_scripts_city_and_settings_not_replay_core(tmp_path, monkeypatch):
    row, exe = inputs(tmp_path, monkeypatch)
    fingerprint = lambda: trace.identity(tmp_path, row, exe, 'shadows', ['reach = 1'])[0]
    original = fingerprint()
    dll = tmp_path / 'engine/build-x86/FreeCity4.dll'
    dll.write_bytes(b'new replay core')
    assert fingerprint() == original
    for path in (exe, tmp_path / row['script'], suite.city_path(row)):
        previous = path.read_bytes()
        path.write_bytes(previous + b'changed')
        assert fingerprint() != original
        path.write_bytes(previous)
    assert trace.identity(tmp_path, row, exe, 'shadows', ['reach = 1', 'power_shadow = 1'])[0] != original


def test_lua_companion_script_invalidates_recording(tmp_path, monkeypatch):
    row, exe = inputs(tmp_path, monkeypatch)
    row['script'] = 'case.lua'
    (tmp_path / 'case.lua').write_text('log("fixture")')
    (tmp_path / 'case.script').write_text('2000-01-01 mark setup')
    first = trace.identity(tmp_path, row, exe, 'shadows', [])[0]
    (tmp_path / 'case.script').write_text('2000-01-01 mark changed')
    assert trace.identity(tmp_path, row, exe, 'shadows', [])[0] != first


def test_record_once_reuses_complete_content_store(tmp_path, monkeypatch, capsys):
    row, exe = inputs(tmp_path, monkeypatch)
    args = ['record', '--set', 'full', '--tag', 'pilot', '--exe', str(exe), '--reach']
    calls = []

    def record_game(root, rows, **kwargs):
        calls.append(kwargs)
        name = rows[0]['name']
        body = 'foo shadow 2000-02-01: mismatches=0\nreach summary reached=1 saved=1\n'
        census = batch(root, kwargs['tag'], bodies={name: body})
        store = Path(suite.extras(kwargs['extra'])['trace_record'])
        store.mkdir(parents=True)
        content = b'completed trace fixture'
        digest = hashlib.sha256(content).hexdigest()
        (store / digest).write_bytes(content)
        (store / 'manifest.tsv').write_text(f'{name}\tfoo_shadow\t{digest}\t1\t0\t0\n')
        result = dict(verdict='clean', end_reached='1', actions_failed=0, actions_pending=0)
        return census, {name: result}

    with patch.object(suite, 'read_scenarios', return_value=[row]), \
            patch.object(suite, 'run_scenarios', side_effect=record_game):
        assert suite.main(args, root=tmp_path) == 0
        assert suite.main(args, root=tmp_path) == 0
    assert len(calls) == 1
    assert calls[0]['slots'] == 1
    settings = suite.extras(calls[0]['extra'])
    assert settings['reach'] == '1'
    assert settings['trace_scenario'] == row['name']
    directory = tmp_path / 'build-runs/traces/pilot'
    metadata = json.loads((directory / 'recordings.json').read_text())
    assert metadata['recordings'][row['name']]['identity']
    assert len(trace.replay_coverage.manifest_rows(directory / 'manifest.tsv')) == 1
    assert 'recorded=0 reused=1' in capsys.readouterr().out


def test_record_dry_run_stages_real_opt_in_request_without_cache_or_game(tmp_path, monkeypatch):
    row, exe = inputs(tmp_path, monkeypatch)
    dll = tmp_path / 'engine/build-x86/FreeCity4.dll'
    dll.write_bytes(b'reach prepared functions= fixture DLL')
    args = ['record', '--tag', 'dry', '--exe', str(exe), '--dll', str(dll), '--reach', '--dry-run']
    with patch.object(suite, 'read_scenarios', return_value=[row]):
        assert suite.main(args, root=tmp_path) == 0
    requests = list((tmp_path / 'build-runs/suite').glob('*/case.one.req'))
    assert len(requests) == 1
    text = requests[0].read_text()
    assert 'parallel = 1\n' in text and 'reach = 1\n' in text
    assert 'trace_scenario = case.one\n' in text
    assert 'trace_record = ' in text
    assert not list((tmp_path / 'build-runs/traces').glob('**/recording.json'))
    assert not (tmp_path / 'build-runs/game-queue').exists()


def test_record_cannot_disable_reach(tmp_path):
    with pytest.raises(SystemExit):
        suite.main(['record', '--tag', 'x', '--reach', '--extra', 'reach = 0'], root=tmp_path)
    with pytest.raises(SystemExit):
        suite.main(['record', '--tag', 'x'], root=tmp_path)


@pytest.mark.parametrize('exit_code', [0, 1, 2])
def test_replay_uses_all_cpus_and_drops_old_verdicts(tmp_path, monkeypatch, exit_code):
    directory = tmp_path / 'build-runs/traces/pilot'
    shutil.copytree(Path(__file__).parent / 'fixtures/replay', directory)
    binary = tmp_path / 'fc4-replay.exe'
    binary.write_bytes(b'fixture')
    monkeypatch.setattr(trace.os, 'cpu_count', lambda: 7)
    commands = []

    def replay_process(command, **kwargs):
        commands.append(command)
        for name in ('replay-verdicts.tsv', 'replay-suite.tsv'):
            assert not (directory / name).exists()
            if exit_code != 2:
                shutil.copy2(Path(__file__).parent / 'fixtures/replay' / name, directory / name)
        return SimpleNamespace(returncode=exit_code)

    with patch.object(trace.subprocess, 'run', side_effect=replay_process):
        if exit_code == 2:
            with pytest.raises(SystemExit):
                suite.main(['replay', 'pilot', '--replayer', str(binary)], root=tmp_path)
        else:
            assert suite.main(['replay', 'pilot', '--replayer', str(binary)], root=tmp_path) == exit_code
    assert commands[0][-2:] == ['--jobs', '7']


def test_replay_cannot_write_another_checkout(tmp_path):
    other = tmp_path / 'another-checkout'
    shutil.copytree(Path(__file__).parent / 'fixtures/replay', other)
    with pytest.raises(SystemExit):
        suite.main(['replay', str(other / 'manifest.tsv')], root=tmp_path)
