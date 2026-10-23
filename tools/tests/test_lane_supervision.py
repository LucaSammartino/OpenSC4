"""Isolated actual CLI/dispatcher flow, fake workers/time and owned filesystem only.

Run without pytest: python -m unittest discover -s tools/tests -p test_lane_supervision.py -v
The --fixture entry is an owned subprocess, never a production dispatcher.
"""
from contextlib import ExitStack, nullcontext
import copy
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import lane_heartbeat as heartbeat
import lane_refill as refill
import lane_supervision as supervision


def put(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding='utf-8')


def fixture(root, at, arguments):
    # Exercise main() and snapshot(), including real dependency/spec reads and atomic files.
    original = supervision.capacity
    real_sleep = time.sleep
    def bounded_sleep(seconds):
        if seconds > 0.05:
            raise AssertionError('underfill must never poll/sleep')
        real_sleep(seconds)
    with ExitStack() as stack:
        stack.enter_context(patch.object(heartbeat.lane_launch, 'alive', lambda pid: type(pid) is int and 1 <= pid <= 9))
        stack.enter_context(patch.object(heartbeat, 'last_progress', return_value=None))
        stack.enter_context(patch.object(refill, 'unmet_deps', lambda root, deps: [d for d in deps if d == 'waiting']))
        stack.enter_context(patch.object(supervision, 'capacity', lambda root, observe: original(root, observe, now=lambda: at)))
        stack.enter_context(patch.object(heartbeat.time, 'sleep', bounded_sleep))
        stack.enter_context(patch.object(sys, 'argv', ['heartbeat', '--root', str(root), *arguments]))
        heartbeat.main()


class Sandbox(unittest.TestCase):
    def setUp(self):
        parent = Path(__file__).resolve().parents[2] / 'build-runs'
        parent.mkdir(exist_ok=True)
        temporary = tempfile.TemporaryDirectory(prefix='supervision-test-', dir=parent)
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.runs = self.root / 'build-runs'
        self.runs.mkdir()
        self.state = dict(concurrency=9, provider='claude', rounds=9, lanes={})
        self.journal = self.runs / 'capacity-supervision/incident-v1.json'
        self.configure(8, launchable=1)

    def configure(self, live, launchable=0, waiting=0):
        self.state['lanes'] = {f'worker{i}': dict(pid=i, role='verify') for i in range(1, live + 1)}
        self.state['helpers'] = {'chat': dict(pid=9)}  # not a registered worker
        self.state['drafts'] = {f'draft{i}': dict(pid=9) for i in range(20)}
        put(self.runs / 'run4-coordinator-state.json', self.state)
        queue = []
        for i in range(launchable + waiting):
            spec = self.runs / f'brief{i}.json'
            put(spec, dict(args=dict(name=f'brief{i}', deps=[] if i < launchable else ['waiting'])))
            queue.append(dict(name=f'brief{i}', spec=str(spec), status='pending'))
        put(self.runs / 'run4-refill-queue.json', queue)
        put(self.runs / 'merge-train/state.json', dict(current=dict(id='train9', gate=dict(ok=True),
                                                               regress=dict(ok=True, verdict='PASS'))))

    def cli(self, at=1000, *arguments):
        result = subprocess.run([sys.executable, str(Path(__file__).resolve()), '--fixture', str(self.root),
                                 str(at), *arguments], capture_output=True, text=True, timeout=10)
        self.assertNotIn('Traceback', result.stderr, result.stderr)
        return result

    def observed(self):
        return json.loads(self.journal.read_text(encoding='utf-8'))

    def snapshot(self, at):
        original = supervision.capacity
        with patch.object(heartbeat.lane_launch, 'alive', lambda pid: type(pid) is int and 1 <= pid <= 9), \
                patch.object(heartbeat, 'last_progress', return_value=None), \
                patch.object(refill, 'unmet_deps', lambda root, deps: [d for d in deps if d == 'waiting']), \
                patch.object(supervision, 'capacity', lambda root, observe: original(root, observe, now=lambda: at)):
            return heartbeat.snapshot(self.root)


class CapacityCLI(Sandbox):
    def test_restarted_instances_keep_first_observation_until_actual_recovery(self):
        for at in (1000, 1040):
            result = self.cli(at, '--once')
            self.assertEqual(result.returncode, 4, result.stdout + result.stderr)
        record = self.observed()
        self.assertEqual((record['first_observed'], record['age_seconds']), (1000, 40))
        self.assertTrue(record['active'])  # a covering launchable backlog is not a ninth worker
        self.assertEqual((record['live'], record['open_slots'], record['launchable_deficit']), (8, 1, 0))
        self.assertIn('1 actual workers missing; launchable deficit 0; observed age 40s', result.stdout)
        self.configure(9)
        recovered = self.cli(1050, '--once')
        self.assertEqual(recovered.returncode, 0, recovered.stdout + recovered.stderr)
        self.assertFalse(self.observed()['active'])
        self.assertEqual(self.observed()['resolved_at'], 1050)
        self.assertEqual(self.cli(1060, '--once').returncode, 0)
        self.assertEqual(self.observed()['first_observed'], 1000)
        self.assertEqual(self.observed()['age_seconds'], 50)
        self.configure(8)
        self.assertEqual(self.cli(1100, '--once').returncode, 4)
        self.assertEqual(self.observed()['first_observed'], 1100)

    def test_underfill_before_train_verdict_is_non_destructive_and_never_sleeps(self):
        train = self.runs / 'merge-train/state.json'
        before = train.read_bytes(), train.stat().st_mtime_ns
        for arguments in (('--wait-until', 'train'), (), ('--once',)):
            result = self.cli(1000, *arguments)
            self.assertEqual(result.returncode, 4, result.stdout + result.stderr)
        self.assertEqual((train.read_bytes(), train.stat().st_mtime_ns), before)
        self.configure(9)
        result = self.cli(1010, '--wait-until', 'train')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('TRAIN train9: gate ok, regress PASS', result.stdout)

    def test_dependency_waiting_queue_helpers_drafts_are_not_live(self):
        self.configure(8, launchable=0, waiting=12)
        result = self.cli(1000, '--wait-until', 'train')
        self.assertEqual(result.returncode, 4, result.stdout)
        record = self.observed()
        self.assertEqual((record['live'], record['launchable'], record['waiting']), (8, 0, 12))
        self.assertTrue(record['launchable_shortfall'])
        self.assertEqual(record['launchable_deficit'], 1)
        self.assertIn('brief0: waiting', result.stdout)

    def test_stop_and_provider_pause_priority_and_observed_span_includes_pause(self):
        self.assertEqual(self.cli(1000, '--once').returncode, 4)
        pause = self.runs / 'provider-pauses/claude.json'
        put(pause, dict(reason='fixture'))
        for arguments in (('--wait-until', 'train'), ('--once',), ()):
            result = self.cli(1040, *arguments)
            self.assertEqual(result.returncode, 5, result.stdout)
            self.assertIn('no spawning', result.stdout)
        record = self.observed()
        self.assertTrue(record['active'] and record['suppressed'] and record['provider_paused'])
        self.assertEqual(record['age_seconds'], 40)
        pause.unlink()
        self.assertEqual(self.cli(1080, '--once').returncode, 4)
        self.assertEqual(self.observed()['age_seconds'], 80)
        (self.runs / 'STOP').touch()
        result = self.cli(1090, '--wait-until', 'train')
        self.assertEqual(result.returncode, 5, result.stdout)
        self.assertIn('STOP set', result.stdout)
        self.assertTrue(self.observed()['stop'])
        self.assertEqual(self.observed()['first_observed'], 1000)

    def test_malformed_partial_old_schema_and_null_are_preserved_not_healthy(self):
        self.journal.parent.mkdir()
        for content in ('{', '{}', '{"version":0}', 'null', '{"version":1}'):
            with self.subTest(content=content):
                self.journal.write_text(content, encoding='utf-8')
                result = self.cli(1000, '--once')
                self.assertEqual(result.returncode, 1, result.stdout)
                self.assertIn('evidence retained', result.stdout)
                self.assertEqual(self.journal.read_text(), content)
                self.assertFalse((self.runs / 'run4-heartbeat.json').exists())

    def test_rollback_keeps_first_and_last_verified_age_but_marks_duration_unknown(self):
        self.assertEqual(self.cli(1000, '--once').returncode, 4)
        self.assertEqual(self.cli(1050, '--once').returncode, 4)
        result = self.cli(900, '--once')
        record = self.observed()
        self.assertEqual(record['first_observed'], 1000)
        self.assertIsNone(record['age_seconds'])
        self.assertEqual(record['last_verified_age_seconds'], 50)
        self.assertTrue(record['clock_uncertain'])
        self.assertIn('UNKNOWN', result.stdout)
        self.assertEqual(self.cli(1100, '--once').returncode, 4)
        self.assertIsNone(self.observed()['age_seconds'])
        self.configure(9)
        self.assertEqual(self.cli(1110, '--once').returncode, 0)
        self.assertFalse(self.observed()['active'])

    def test_read_permission_failure_and_replace_failure_retain_original_evidence(self):
        self.snapshot(1000)
        before = self.journal.read_bytes()
        original = Path.read_text
        def read(path, *args, **kwargs):
            if path == self.journal:
                raise PermissionError('fixture access denied')
            return original(path, *args, **kwargs)
        with patch.object(Path, 'read_text', read):
            with self.assertRaisesRegex(supervision.SupervisionError, 'evidence retained'):
                self.snapshot(1010)
        self.assertEqual(self.journal.read_bytes(), before)
        with patch.object(Path, 'replace', side_effect=PermissionError('fixture replace denied')):
            with self.assertRaisesRegex(supervision.SupervisionError, 'evidence retained'):
                self.snapshot(1020)
        self.assertEqual(self.journal.read_bytes(), before)
        self.assertEqual(list(self.journal.parent.glob('*.tmp')), [])

    def test_concurrent_writer_is_bounded_and_does_not_observe_or_overwrite(self):
        self.snapshot(1000)
        before = self.journal.read_bytes()
        with supervision.ownership(self.journal.parent / 'writer.lock'):
            result = self.cli(1020, '--once')
            self.assertEqual(result.returncode, 1, result.stdout)
            self.assertIn('ownership lock unavailable', result.stdout)
            self.assertEqual(self.journal.read_bytes(), before)
        self.assertEqual(self.cli(1030, '--once').returncode, 4)
        self.assertEqual(self.observed()['age_seconds'], 30)

    def test_atomic_readers_never_see_partial_journal_during_updates(self):
        self.snapshot(1000)
        errors, done = [], threading.Event()
        def read():
            while not done.is_set():
                try:
                    supervision._validate_incident(supervision.read_artifact(self.journal))
                except BaseException as exc:
                    errors.append(exc)
                    break
        thread = threading.Thread(target=read, daemon=True)
        thread.start()
        try:
            for at in range(1001, 1015):
                self.snapshot(at)
        finally:
            done.set()
            thread.join(timeout=3)
        self.assertFalse(thread.is_alive())
        self.assertEqual(errors, [])
        self.assertEqual(self.observed()['age_seconds'], 14)


class DispatcherOwnership(Sandbox):
    def identity(self, pid):
        return dict(created='fixture-created-28668', executable=sys.executable,
                    argv=[sys.executable, str(self.root / 'tools/lane_refill.py'), '--root', str(self.root)])

    def marker(self):
        return json.loads((self.runs / 'run4-refill-owner-v1.json').read_text())

    def test_actual_daemon_flow_publishes_self_after_lock_and_retains_stale_on_exit(self):
        def launch(root, state_path):
            record = self.marker()
            self.assertEqual(record['pid'], os.getpid())
            self.assertEqual(record['created'], 'fixture-created-28668')
            self.assertEqual(record['provider'], 'claude')
            self.assertEqual(record['root'], str(self.root))
            self.assertEqual(record['argv'], self.identity(os.getpid())['argv'])
            self.assertEqual((self.runs / 'run4-refill.pid').read_text().strip(), str(os.getpid()))
            status = supervision.dispatcher(root, 'claude')
            self.assertTrue(status['verified'], status)
        self.configure(9)
        with patch.object(supervision, 'process_identity', self.identity), \
                patch.object(refill.launcher, 'launch', launch), \
                patch.object(refill.launcher, 'alive', return_value=True), \
                patch.object(sys, 'argv', ['refill', '--root', str(self.root), '--once']):
            self.assertIsNone(refill.main())
            status = supervision.dispatcher(self.root, 'claude')
            self.assertFalse(status['verified'])
            self.assertIn('lock is not held', status['reason'])
        self.assertTrue((self.runs / 'run4-refill-owner-v1.json').exists())

    def test_stale23912_reused_pid_wrong_root_role_missing_identity_and_access_denied(self):
        with supervision.ownership(self.runs / 'run4-refill-owner.lock'):
            supervision.publish_dispatcher(self.root, 'claude', identity=self.identity)
            good = self.marker()
            marker_path = self.runs / 'run4-refill-owner-v1.json'
            cases = [
                ('stale23912', lambda m: m.update(pid=23912), self.identity),
                ('reused', lambda m: m.update(created='old-creation'), self.identity),
                ('wrongroot', lambda m: m.update(root=str(self.root / 'other')), self.identity),
                ('wrongrole', lambda m: m.update(role='queue-helper'), self.identity),
                ('missingidentity', lambda m: m.pop('created'), self.identity),
                ('AccessDenied', lambda m: None, lambda pid: (_ for _ in ()).throw(PermissionError('AccessDenied'))),
                ('unsupported', lambda m: None, lambda pid: (_ for _ in ()).throw(OSError('unsupported OS'))),
            ]
            for name, mutate, identity in cases:
                with self.subTest(name=name):
                    marker = copy.deepcopy(good)
                    mutate(marker)
                    put(marker_path, marker)
                    status = supervision.dispatcher(self.root, 'claude', identity=identity)
                    self.assertFalse(status['verified'], status)
                    self.assertEqual(status['status'], 'UNKNOWN')
                    self.assertTrue(status['reason'])
            put(marker_path, good)
            for argv in ([sys.executable, str(self.root / 'other.py')],
                         [sys.executable, str(self.root / 'tools/lane_refill.py'), '--root', str(self.root / 'other')],
                         [*good['argv'][:2], '--root=' + str(self.root / 'other')],
                         [*good['argv'], '--provider=codex'],
                         [*good['argv'], '--queue', 'brief.json']):
                fake = dict(self.identity(0), argv=argv)
                marker = dict(good, **fake)
                put(marker_path, marker)
                status = supervision.dispatcher(self.root, 'claude', identity=lambda pid: fake)
                self.assertFalse(status['verified'], status)

    def test_legacy_bare_pid_alone_is_never_exact_owner(self):
        (self.runs / 'run4-refill.pid').write_text(str(os.getpid()))
        status = supervision.dispatcher(self.root, 'claude', identity=self.identity)
        self.assertFalse(status['verified'])
        self.assertIn('FileNotFoundError', status['reason'])

    def test_refused_second_and_queue_only_cannot_overwrite_markers(self):
        self.configure(9)
        with supervision.ownership(self.runs / 'run4-refill-owner.lock'):
            supervision.publish_dispatcher(self.root, 'claude', identity=self.identity)
            paths = [self.runs / 'run4-refill-owner-v1.json', self.runs / 'run4-refill.pid']
            before = [p.read_bytes() for p in paths]
            with patch.object(sys, 'argv', ['refill', '--root', str(self.root), '--once']), \
                    patch.object(supervision, 'process_identity', side_effect=AssertionError('second must not publish')), \
                    patch('sys.stdout', new_callable=io.StringIO) as output:
                self.assertEqual(refill.main(), 1)
                self.assertIn('ownership lock unavailable', output.getvalue())
            spec = self.runs / 'queue-only.json'
            put(spec, dict(args=dict(name='queued', maxRounds=1)))
            with patch.object(sys, 'argv', ['refill', '--root', str(self.root), '--queue', str(spec)]), \
                    patch.object(refill.lane_evidence, 'preflight'), \
                    patch.object(supervision, 'process_identity', side_effect=AssertionError('queue must not publish')), \
                    patch('sys.stdout', new_callable=io.StringIO) as output:
                self.assertIsNone(refill.main())
                self.assertEqual(output.getvalue().strip(), 'queued 1')
            self.assertEqual([p.read_bytes() for p in paths], before)

    def test_old_exit_cannot_delete_newer_marker(self):
        self.configure(9)
        def replace_with_newer(*args):
            newer = dict(self.marker(), pid=777, created='newer-creation')
            put(self.runs / 'run4-refill-owner-v1.json', newer)
            (self.runs / 'run4-refill.pid').write_text('777\n')
        with patch.object(supervision, 'process_identity', self.identity), \
                patch.object(refill.launcher, 'launch', replace_with_newer), \
                patch.object(refill.launcher, 'alive', return_value=True), \
                patch.object(sys, 'argv', ['refill', '--root', str(self.root), '--once']):
            refill.main()
        self.assertEqual(self.marker()['created'], 'newer-creation')
        self.assertEqual((self.runs / 'run4-refill.pid').read_text(), '777\n')

    def test_marker_failure_fails_before_launch_and_stop_pause_never_launch(self):
        self.configure(9)
        with patch.object(supervision, 'process_identity', self.identity), \
                patch.object(refill.launcher, 'launch', side_effect=AssertionError('must not launch')), \
                patch.object(sys, 'argv', ['refill', '--root', str(self.root), '--once']), \
                patch('sys.stdout', new_callable=io.StringIO) as output:
            with patch.object(supervision, 'atomic_text', side_effect=PermissionError('fixture marker denied')):
                self.assertEqual(refill.main(), 1)
                self.assertIn('marker denied', output.getvalue())
            (self.runs / 'STOP').touch()
            self.assertIsNone(refill.main())
            (self.runs / 'STOP').unlink()
            put(self.runs / 'provider-pauses/claude.json', {})
            self.assertIsNone(refill.main())
        self.assertFalse(supervision.dispatcher(self.root, 'claude', identity=self.identity)['verified'])

    def test_supported_os_identity_of_owned_test_process(self):
        actual = supervision.process_identity(os.getpid())
        self.assertTrue(actual['created'])
        self.assertTrue(actual['argv'])
        self.assertTrue(actual['executable'])
        self.assertEqual(actual['created'], supervision.process_identity(os.getpid())['created'])


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == '--fixture':
        fixture(Path(sys.argv[2]), float(sys.argv[3]), sys.argv[4:])
    else:
        unittest.main()
