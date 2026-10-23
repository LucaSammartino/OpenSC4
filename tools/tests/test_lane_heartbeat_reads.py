"""Critical-state recovery with real files and deterministic retry time."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import lane_heartbeat as heartbeat


class Clock:
    def __init__(self):
        self.t = 1000.0

    def now(self):
        return self.t

    def sleep(self, seconds):
        self.t += seconds


class CriticalReads(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.runs = self.root / 'build-runs'
        self.runs.mkdir()
        (self.runs / 'merge-train').mkdir()
        self.paths = [self.runs / 'run4-coordinator-state.json', self.runs / 'run4-refill-queue.json',
                      self.runs / 'merge-train/state.json']
        self.reset()

    def reset(self):
        self.paths[0].write_text(json.dumps(dict(provider='claude', concurrency=0, lanes={}, rounds=9)),
                                 encoding='utf-8')
        self.paths[1].write_text('[]', encoding='utf-8')
        self.train({'id': 'train9', 'gate': None, 'regress': None})
        self.clock = Clock()

    def train(self, current):
        self.paths[2].write_text(json.dumps({'current': current}), encoding='utf-8')

    def reader(self, **kwargs):
        return heartbeat.CriticalReader(sleep=self.clock.sleep, now=self.clock.now, **kwargs)

    def wait(self, event='train', **kwargs):
        return heartbeat.wait_until(self.root, event, now=self.clock.now,
                                    sleep=self.clock.sleep, **kwargs)

    def ledger(self):
        return [p.read_bytes() for p in self.paths]

    def error(self, mode):
        return PermissionError(13, 'sharing violation') if mode == 'permission' else \
            json.JSONDecodeError('partial write', '{', 1)

    def test_snapshot_transient_critical_reads_recover(self):
        original = heartbeat.read_json
        for target in self.paths:
            for mode in ('permission', 'malformed'):
                with self.subTest(path=target, mode=mode):
                    self.reset()
                    before, calls = self.ledger(), []

                    def read(path):
                        if path == target:
                            calls.append(path)
                            if len(calls) == 1:
                                raise self.error(mode)
                        return original(path)
                    with patch.object(heartbeat, 'read_json', read):
                        value = heartbeat.snapshot(self.root, reader=self.reader())
                    self.assertEqual((value['rounds'], value['train']), (9, 'train9'))
                    self.assertEqual(value['alerts'], ['REFILLER: process is not running'])
                    self.assertEqual(len(calls), 2)
                    self.assertEqual(self.clock.t, 1000 + heartbeat.READ_DELAY)
                    self.assertEqual(self.ledger(), before)

    def test_real_partial_json_completes_during_retry(self):
        self.paths[2].write_text('{"current":', encoding='utf-8')

        def sleep(seconds):
            self.clock.sleep(seconds)
            self.train({'id': 'train10', 'gate': {'ok': False}})
        reader = heartbeat.CriticalReader(sleep=sleep, now=self.clock.now)
        self.assertEqual(heartbeat.snapshot(self.root, reader=reader)['train'], 'train10')
        self.assertEqual(self.clock.t, 1000 + heartbeat.READ_DELAY)

    def test_failed_report_preserves_artifacts_and_ledger(self):
        output, log = self.runs / 'run4-heartbeat.json', self.runs / 'run4-heartbeat.log'
        original = heartbeat.read_json
        for mode in ('permission', 'malformed'):
            for existing in (False, True):
                with self.subTest(mode=mode, existing=existing):
                    self.reset()
                    output.unlink(missing_ok=True)
                    log.unlink(missing_ok=True)
                    if existing:
                        output.write_bytes(b'previous report\n')
                        log.write_bytes(b'previous log\n')
                    if mode == 'malformed':
                        self.paths[0].write_text('{', encoding='utf-8')
                    before, calls = self.ledger(), []

                    def read(path):
                        if path == self.paths[0]:
                            calls.append(path)
                            if mode == 'permission':
                                raise self.error(mode)
                        return original(path)
                    reader = self.reader()
                    with patch.object(heartbeat, 'read_json', read), \
                            patch.object(heartbeat, 'CriticalReader', return_value=reader):
                        with self.assertRaisesRegex(heartbeat.CriticalReadError, 'coordinator-state.json'):
                            heartbeat.report(self.root)
                    self.assertEqual(len(calls), heartbeat.READ_ATTEMPTS)
                    self.assertEqual(self.clock.t, 1000 + (heartbeat.READ_ATTEMPTS - 1) * heartbeat.READ_DELAY)
                    if existing:
                        self.assertEqual(output.read_bytes(), b'previous report\n')
                        self.assertEqual(log.read_bytes(), b'previous log\n')
                    else:
                        self.assertFalse(output.exists() or log.exists())
                    self.assertFalse(list(self.runs.glob('run4-heartbeat.json.*.tmp')))
                    self.assertEqual(self.ledger(), before)

    def test_wait_persistent_failures_are_explicit(self):
        original = heartbeat.read_json
        for phase in ('discovery', 'event', 'snapshot'):
            for mode in ('permission', 'malformed'):
                with self.subTest(phase=phase, mode=mode):
                    self.reset()
                    before, calls = self.ledger(), []
                    target = self.paths[0] if phase == 'snapshot' else self.paths[2]

                    def read(path):
                        if path == target:
                            calls.append(path)
                            if phase != 'event' or len(calls) > 1:
                                raise self.error(mode)
                        return original(path)
                    with patch.object(heartbeat, 'read_json', read):
                        code, line = self.wait()
                    self.assertEqual(code, heartbeat.WAIT_ERROR)
                    self.assertIn('ERROR: cannot read critical JSON', line)
                    self.assertIn(str(target), line)
                    self.assertEqual(self.clock.t, 1000 + (heartbeat.READ_ATTEMPTS - 1) * heartbeat.READ_DELAY)
                    self.assertFalse((self.runs / 'run4-heartbeat.json').exists())
                    self.assertEqual(self.ledger(), before)

    def test_retry_sleep_respects_caller_deadline(self):
        original = heartbeat.read_json

        def read(path):
            if path == self.paths[0]:
                raise self.error('permission')
            return original(path)
        with patch.object(heartbeat, 'read_json', read):
            code, line = self.wait('backlog', timeout=0.1)
        self.assertEqual(code, heartbeat.WAIT_TIMEOUT)
        self.assertIn('deadline exhausted', line)
        self.assertIn('after 1 attempts', line)
        self.assertEqual(self.clock.t, 1000.1)

    def test_total_read_budget_counts_time_inside_failed_reads(self):
        calls = []

        def read(path):
            calls.append(path)
            self.clock.sleep(0.75)
            raise self.error('permission')
        with patch.object(heartbeat, 'read_json', read):
            with self.assertRaises(heartbeat.CriticalReadError):
                self.reader()(self.paths[0])
        self.assertEqual(self.clock.t, 1000 + heartbeat.READ_SECONDS)
        self.assertEqual(len(calls), 2)

    def test_wait_checks_fresh_stop_and_provider_while_recovering(self):
        original = heartbeat.read_json
        for phase in ('discovery', 'snapshot'):
            for halt in ('STOP', 'provider'):
                with self.subTest(phase=phase, halt=halt):
                    self.reset()
                    (self.runs / 'STOP').unlink(missing_ok=True)
                    pause = self.runs / 'provider-pauses/codex.json'
                    pause.unlink(missing_ok=True)
                    target = self.paths[2] if phase == 'discovery' else self.paths[1]

                    def read(path):
                        if path == target:
                            raise self.error('permission')
                        return original(path)

                    def sleep(seconds):
                        self.clock.sleep(seconds)
                        if halt == 'STOP':
                            (self.runs / 'STOP').touch()
                        else:
                            state = json.loads(self.paths[0].read_text())
                            state['provider'] = 'codex'
                            self.paths[0].write_text(json.dumps(state), encoding='utf-8')
                            pause.parent.mkdir(exist_ok=True)
                            pause.write_text('{}', encoding='utf-8')
                    before = self.ledger()
                    with patch.object(heartbeat, 'read_json', read):
                        code, line = heartbeat.wait_until(self.root, 'train', now=self.clock.now, sleep=sleep)
                    self.assertEqual(code, heartbeat.WAIT_HALT)
                    self.assertIn('STOP set' if halt == 'STOP' else 'provider codex paused', line)
                    self.assertEqual(self.clock.t, 1000 + heartbeat.READ_DELAY)
                    self.assertEqual(self.paths[1].read_bytes(), before[1])
                    self.assertEqual(self.paths[2].read_bytes(), before[2])

    def test_train_discovery_recovers_and_event_still_wins_over_backlog(self):
        original, calls = heartbeat.read_json, []
        self.train({'id': 'train9', 'gate': {'ok': False}})

        def read(path):
            if path == self.paths[2]:
                calls.append(path)
                if len(calls) == 1:
                    raise self.error('permission')
            return original(path)
        with patch.object(heartbeat, 'read_json', read):
            self.assertEqual(self.wait(), (heartbeat.WAIT_EVENT, 'TRAIN train9: gate FAIL'))
        self.assertEqual(len(calls), 3)

    def test_valid_no_current_train_remains_usage(self):
        self.train(None)
        code, line = self.wait()
        self.assertEqual(code, heartbeat.WAIT_USAGE)
        self.assertIn('no current train', line)

    def test_once_cli_reports_failed_snapshot_nonzero(self):
        self.paths[0].write_text('{', encoding='utf-8')
        reader = self.reader()
        with patch.object(heartbeat, 'CriticalReader', return_value=reader), \
                patch.object(sys, 'argv', ['heartbeat', '--root', str(self.root), '--once']), \
                patch('builtins.print') as output:
            with self.assertRaises(SystemExit) as result:
                heartbeat.main()
        self.assertEqual(result.exception.code, heartbeat.WAIT_ERROR)
        self.assertIn('ERROR: cannot read critical JSON', output.call_args.args[0])
        self.assertFalse((self.runs / 'run4-heartbeat.json').exists())
