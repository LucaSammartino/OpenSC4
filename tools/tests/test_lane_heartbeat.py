import importlib.util
import json
from pathlib import Path
import sys
import time

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
spec = importlib.util.spec_from_file_location('lane_heartbeat', TOOLS / 'lane_heartbeat.py')
heartbeat = importlib.util.module_from_spec(spec)
spec.loader.exec_module(heartbeat)


def test_counts_processes_instead_of_recent_activity():
    state = {'concurrency': 9, 'rounds': 10, 'lanes': {
        'running': {'pid': 11, 'role': 'glue'},
        'finished': {'pid': 12, 'role': 'verify', 'recent_activity': True},
        'never_started': {'role': 'verify'}}}
    result = heartbeat.summarize(state, [], {}, is_alive=lambda pid: pid == 11,
                                 refill_pid=12)
    assert result['lanes'] == ['running']
    assert result['roles'] == {'glue': 1}
    assert result['live'] == 1 and result['target'] == 9
    assert any(alert.startswith('REFILLER:') for alert in result['alerts'])
    assert any(alert.startswith('REVIEW QUEUE:') for alert in result['alerts'])
    assert state['lanes']['finished']['pid'] == 12  # Read-only, including stale entries.


def test_provider_pause_and_stop_do_not_request_spawning():
    for flags in ({'paused': True}, {'stopped': True}):
        result = heartbeat.summarize({'lanes': {}}, [], {},
                                     is_alive=lambda pid: False, **flags)
        assert not any(alert.startswith('REFILL:') for alert in result['alerts'])
        assert not any(alert.startswith('REFILLER:') for alert in result['alerts'])


def test_pending_queue_and_train_ready_are_separate():
    result = heartbeat.summarize({'lanes': {}}, [
        {'name': 'next', 'status': 'pending'}, {'name': 'denied', 'status': 'denied'}],
        {'current': {'id': 'train7', 'gate': {'ok': True},
                     'regress': {'ok': True, 'verdict': 'PASS'}}},
        is_alive=lambda pid: pid == 7, refill_pid=7)
    assert result['pending'] == ['next']
    assert result['queue_states'] == {'pending': 1, 'denied': 1}
    assert result['refiller_alive']
    assert any(alert.startswith('LAND:') for alert in result['alerts'])
    assert not any(alert.startswith('REVIEW QUEUE:') for alert in result['alerts'])


def test_train_with_unstarted_nullable_stages():
    result = heartbeat.summarize({'lanes': {}}, [], {
        'current': {'id': 'train8', 'gate': None, 'regress': None}},
        is_alive=lambda pid: False)
    assert result['train'] == 'train8'
    assert result['gate_ok'] is None and result['regress'] is None
    assert not any(alert.startswith('LAND:') for alert in result['alerts'])


def test_live_but_idle_lane_is_stalled():
    state = {'concurrency': 2, 'lanes': {'busy': {'pid': 1}, 'idle': {'pid': 2}, 'done': {'pid': 3}}}
    now = 100 * 3600.0
    result = heartbeat.summarize(state, [{'name': 'x', 'status': 'pending'}], {},
                                 is_alive=lambda pid: pid in (1, 2, 9), refill_pid=9, now=now,
                                 progress={'busy': now - 600, 'idle': now - 72 * 3600, 'done': now - 99 * 3600})
    assert result['stalled'] == ['idle']  # 'done' is not live, so it is not a stalled worker
    assert any(a.startswith('STALLED:') and 'idle (72.0h)' in a for a in result['alerts'])


def test_ready_queue_entry_waiting_is_land_lag():
    now = 50 * 3600.0
    result = heartbeat.summarize({'lanes': {}}, [], {}, is_alive=lambda pid: False, now=now,
                                 ready={'fresh': now - 60, 'old': now - 5 * 3600})
    assert result['land_lag'] == ['old']
    assert any(a.startswith('LAND LAG: 1 ready queue entry') for a in result['alerts'])


def test_no_progress_data_no_new_alerts():
    result = heartbeat.summarize({'lanes': {'a': {'pid': 1}}}, [], {}, is_alive=lambda pid: True, refill_pid=1)
    assert result['stalled'] == [] and result['land_lag'] == []


def test_pending_split_into_launchable_and_waiting():
    queue = [{'name': 'ready', 'status': 'pending'}, {'name': 'blocked', 'status': 'pending'},
             {'name': 'gone', 'status': 'launched'}]
    deps = {'ready': [], 'blocked': ['train99-lane', 'other']}
    result = heartbeat.summarize({'concurrency': 9, 'lanes': {'a': {'pid': 1}}}, queue, {},
                                 is_alive=lambda pid: pid == 1, refill_pid=1,
                                 waiting_on=lambda entry: deps[entry['name']])
    assert result['pending'] == ['ready', 'blocked']
    assert result['launchable'] == ['ready']
    assert result['waiting'] == {'blocked': ['train99-lane', 'other']}
    assert result['open_slots'] == 8
    assert 'BACKLOG LOW: 1 launchable < 8 open slots' in result['alerts']
    assert not any(alert.startswith('REVIEW QUEUE:') for alert in result['alerts'])
    assert heartbeat.backlog(result) == 'pending 1 launchable / 1 waiting [blocked: train99-lane,other]'


def test_backlog_low_only_when_launchable_below_open_slots():
    lanes = {'a': {'pid': 1}, 'b': {'pid': 2}}
    queue = [{'name': 'one', 'status': 'pending'}, {'name': 'two', 'status': 'pending'}]
    full = heartbeat.summarize({'concurrency': 4, 'lanes': lanes}, queue, {},
                               is_alive=lambda pid: pid in (1, 2), refill_pid=1,
                               waiting_on=lambda entry: [])
    assert full['launchable'] == ['one', 'two'] and full['open_slots'] == 2
    assert not any(alert.startswith('BACKLOG LOW') for alert in full['alerts'])
    for flags in ({'paused': True}, {'stopped': True}):
        quiet = heartbeat.summarize({'concurrency': 4, 'lanes': {}}, [], {},
                                    is_alive=lambda pid: False, **flags)
        assert not any(alert.startswith('BACKLOG LOW') for alert in quiet['alerts'])


def test_deps_checker_uses_refill_rule_once_per_dep(tmp_path, monkeypatch):
    calls = []

    def unmet(root, deps):
        calls.append(tuple(deps))
        return [dep for dep in deps if dep == 'unlanded']
    monkeypatch.setattr(heartbeat.lane_refill, 'unmet_deps', unmet)
    one, two = tmp_path / 'one.json', tmp_path / 'two.json'
    one.write_text(json.dumps({'args': {'deps': 'landed,unlanded'}}), encoding='utf-8')
    two.write_text(json.dumps({'args': {'deps': ['unlanded']}}), encoding='utf-8')
    check = heartbeat.deps_checker(tmp_path)
    assert check({'spec': str(one)}) == ['unlanded']
    assert check({'spec': str(two)}) == ['unlanded']
    assert check({'spec': str(tmp_path / 'missing.json')}) == ['(unreadable spec)']
    assert calls == [('landed',), ('unlanded',)]


class Clock:
    def __init__(self, t=1000.0):
        self.t = t

    def now(self):
        return self.t

    def sleep(self, seconds):
        self.t += seconds


def write_train(root, current):
    path = root / 'build-runs' / 'merge-train' / 'state.json'
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({'current': current}), encoding='utf-8')


def quiet_snapshot(**changes):
    value = heartbeat.summarize({'concurrency': 2, 'lanes': {'a': {'pid': 1}, 'b': {'pid': 2}}}, [], {},
                                is_alive=lambda pid: pid is not None, refill_pid=3)
    value.update(changes)
    return lambda root: value


def wait(root, event, snap=None, clock=None, **kw):
    clock = clock or Clock()
    return heartbeat.wait_until(root, event, snap=snap or quiet_snapshot(), sleep=clock.sleep,
                                now=clock.now, **kw)


def test_wait_train_verdicts(tmp_path):
    write_train(tmp_path, {'id': 'train9', 'gate': {'ok': False}, 'regress': None})
    assert wait(tmp_path, 'train') == (0, 'TRAIN train9: gate FAIL')
    write_train(tmp_path, {'id': 'train9', 'gate': {'ok': True}, 'regress': {'ok': True, 'verdict': 'PASS'}})
    assert wait(tmp_path, 'train') == (0, 'TRAIN train9: gate ok, regress PASS')
    write_train(tmp_path, {'id': 'train9', 'gate': {'ok': True},
                           'regress': {'ok': True, 'skipped': 'docs/tools-only'}})
    assert wait(tmp_path, 'train') == (0, 'TRAIN train9: gate ok, regress PASS (skipped: docs/tools-only)')
    write_train(tmp_path, None)
    assert wait(tmp_path, 'train')[0] == heartbeat.WAIT_USAGE


def test_wait_train_lands_while_waiting(tmp_path):
    write_train(tmp_path, {'id': 'train9', 'gate': {'ok': True}, 'regress': {'ok': False, 'verdict': 'RUNNING'}})
    snaps = []

    def snap(root):
        snaps.append(1)
        if len(snaps) == 2:
            write_train(root, None)
        return quiet_snapshot()(root)
    assert wait(tmp_path, 'train', snap=snap) == (0, 'TRAIN train9: no longer current (landed or cleared)')
    assert len(snaps) == 2


def test_wait_fresh_ignores_verdict_already_there(tmp_path):
    write_train(tmp_path, {'id': 'train9', 'gate': {'ok': False}, 'regress': None})
    (tmp_path / 'done.txt').write_text('x', encoding='utf-8')
    for event in ('train', 'done.txt'):
        code, line = wait(tmp_path, event, clock=Clock(time.time() + 60), timeout=30, fresh=True)
        assert code == heartbeat.WAIT_TIMEOUT and line.startswith(f'TIMEOUT after 30s waiting for {event}')


def test_wait_file_appears(tmp_path):
    def snap(root):
        (root / 'gate.out').write_text('done', encoding='utf-8')
        return quiet_snapshot()(root)
    code, line = wait(tmp_path, 'gate.out', snap=snap)
    assert code == heartbeat.WAIT_EVENT and line.startswith('FILE ') and line.endswith('gate.out: present')


def test_wait_backlog_low_stop_and_timeout(tmp_path):
    low = heartbeat.summarize({'concurrency': 9, 'lanes': {}}, [{'name': 'x', 'status': 'pending'}], {},
                              is_alive=lambda pid: True, refill_pid=1, waiting_on=lambda entry: ['train99-lane'])
    code, line = wait(tmp_path, 'backlog', snap=lambda root: low)
    assert code == heartbeat.WAIT_BACKLOG
    assert 'CAPACITY: 9 actual workers missing; launchable deficit 9;' in line
    assert 'pending 0 launchable / 1 waiting [x: train99-lane]' in line
    assert wait(tmp_path, 'backlog', snap=quiet_snapshot(stop=True))[0] == heartbeat.WAIT_HALT
    assert wait(tmp_path, 'backlog', snap=quiet_snapshot(provider_paused=True))[0] == heartbeat.WAIT_HALT
    code, line = wait(tmp_path, 'missing.out', timeout=25, poll=10)
    assert code == heartbeat.WAIT_TIMEOUT and line.startswith('TIMEOUT after 25s waiting for missing.out')


def test_wait_capacity_wins_over_completed_train(tmp_path):
    write_train(tmp_path, {'id': 'train9', 'gate': {'ok': False}, 'regress': None})
    low = heartbeat.summarize({'concurrency': 9, 'lanes': {}}, [], {}, is_alive=lambda pid: True, refill_pid=1)
    before = (tmp_path / 'build-runs/merge-train/state.json').read_bytes()
    assert wait(tmp_path, 'train', snap=lambda root: low)[0] == heartbeat.WAIT_BACKLOG
    assert (tmp_path / 'build-runs/merge-train/state.json').read_bytes() == before
    assert wait(tmp_path, 'train')[0] == heartbeat.WAIT_EVENT
