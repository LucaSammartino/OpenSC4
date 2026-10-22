import importlib.util
from pathlib import Path
import sys

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
