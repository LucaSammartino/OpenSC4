"""Real enqueue/filesystem coverage for the launcher's optional evidence list."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))
import lane_evidence
import lane_launch


class Evidence(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.parent = Path(temporary.name)
        self.root = self.parent / 'repo'
        self.runs = self.root / 'build-runs'
        self.runs.mkdir(parents=True)
        (self.runs / 'run4-refill-queue.json').write_bytes(b'[ {"name":"old", "status":"pending"} ]\n')
        (self.runs / 'run4-coordinator-state.json').write_bytes(b'{"rounds": 7, "lanes": {}}\n')

    def brief(self, name='new', **fields):
        path = self.root / 'briefs' / f'{name}.json'
        path.parent.mkdir(exist_ok=True)
        path.write_text(json.dumps(dict(args=dict(name=name, kind='verify', maxRounds=1),
                                        touches=['tools'], **fields)), encoding='utf-8')
        return path

    def enqueue(self, *paths, cwd=None):
        return subprocess.run([sys.executable, str(TOOLS / 'lane_refill.py'), '--root', str(self.root),
                               '--queue', *map(str, paths)], cwd=cwd or self.root,
                              capture_output=True, text=True, encoding='utf-8', timeout=30)

    def ledger(self):
        return [(self.runs / name).read_bytes() for name in
                ('run4-refill-queue.json', 'run4-coordinator-state.json')]

    def test_enqueue_without_evidence(self):
        state = self.ledger()[1]
        for index, fields in enumerate(({}, {'evidence': []})):
            with self.subTest(fields=fields):
                result = self.enqueue(self.brief(f'new{index}', **fields))
                self.assertEqual(result.stdout.strip(), 'queued 1', result.stderr)
                self.assertEqual(json.loads(self.ledger()[0])[-1]['name'], f'new{index}')
                self.assertEqual(self.ledger()[1], state)

    def test_paths_resolve_against_root_not_cwd_or_spec_parent(self):
        relative = self.root / 'valid space café [1];.txt'
        absolute = self.parent / 'outside root λ.txt'
        relative.write_bytes(b'relative')
        absolute.write_bytes(b'absolute')
        path = self.brief(evidence=[relative.name, str(absolute)])
        self.assertEqual(lane_evidence.preflight(self.root, json.loads(path.read_text()), path),
                         [relative, absolute])
        result = self.enqueue(path, cwd=self.parent)
        self.assertEqual(result.stdout.strip(), 'queued 1', result.stderr)

    def test_bad_items_rejected_without_mutation(self):
        values = [
            'same-name guardGO 2026-10-22: python tools/lanes.py check verify-flora-records4 --resume --touches flora',
            '', '   ', 7, None, {}, [], 'missing.txt', '.', 'bad\0path',
        ]
        for value in values:
            with self.subTest(value=value):
                path = self.brief(evidence=[value])
                before = self.ledger()
                result = self.enqueue(path)
                self.assertNotIn('queued 1', result.stdout)
                self.assertIn(str(path), result.stderr)
                self.assertIn('evidence[0]=', result.stderr)
                self.assertIn(repr(value), result.stderr)
                self.assertEqual(self.ledger(), before)

    def test_container_contract(self):
        for value in ('file.txt', {}, 1, None):
            with self.subTest(value=value):
                path = self.brief(evidence=value)
                before = self.ledger()
                result = self.enqueue(path)
                self.assertIn('evidence=', result.stderr)
                self.assertIn('expected a list', result.stderr)
                self.assertEqual(self.ledger(), before)

    def test_multispec_is_all_or_nothing(self):
        first = self.brief('first', evidence=[])
        second = self.brief('second', evidence=['missing'])
        before = self.ledger()
        result = self.enqueue(first, second)
        self.assertIn('evidence[0]', result.stderr)
        self.assertEqual(self.ledger(), before)

    @unittest.skipUnless(os.name == 'nt', 'Windows filename syntax')
    def test_windows_invalid_path(self):
        for value in ('bad:name', 'bad<name', 'bad|name'):
            with self.subTest(value=value), self.assertRaisesRegex(ValueError, r'evidence\[0\]'):
                lane_evidence.preflight(self.root, {'evidence': [value]}, 'spec.json')

    @unittest.skipIf(os.name == 'nt', 'POSIX filename syntax')
    def test_legal_posix_punctuation_and_spaces(self):
        for name in ('guardGO: punctuation? is legal', '   '):
            source = self.root / name
            source.write_bytes(b'ok')
            self.assertEqual(lane_evidence.preflight(self.root, {'evidence': [name]}, 'spec.json'), [source])

    def test_read_errors_are_actionable(self):
        source = self.root / 'source.txt'
        source.write_bytes(b'evidence')
        for error in (PermissionError('sharing denied'), OSError('read interrupted'), ValueError('invalid path')):
            with self.subTest(error=error), patch.object(Path, 'read_bytes', side_effect=error):
                with self.assertRaisesRegex(ValueError, r'spec.json: evidence\[0\].*source.txt.*cannot read'):
                    lane_evidence.preflight(self.root, {'evidence': [source.name]}, 'spec.json')

    def test_direct_launch_rechecks_deleted_source_before_any_side_effect(self):
        source = self.root / 'source.txt'
        source.write_bytes(b'evidence')
        path = self.brief(evidence=[source.name])
        self.assertEqual(self.enqueue(path).stdout.strip(), 'queued 1')
        source.unlink()
        self.assert_launch_rejects_without_side_effects(path)

    def test_direct_launch_rejects_bad_container_before_any_side_effect(self):
        self.assert_launch_rejects_without_side_effects(self.brief(evidence='not a list'))

    def assert_launch_rejects_without_side_effects(self, path):
        before = self.ledger()
        files = sorted(p.relative_to(self.root) for p in self.root.rglob('*'))
        effects = Mock(side_effect=AssertionError('launch side effect before validation'))
        with patch.object(lane_launch.backend.codexguard, 'locked', effects), \
                patch.object(lane_launch.backend, 'write_json', effects), \
                patch.object(lane_launch, 'checked', effects), \
                patch.object(lane_launch.subprocess, 'Popen', effects):
            with self.assertRaisesRegex(ValueError, 'evidence'):
                lane_launch.launch(self.root, self.runs / 'run4-coordinator-state.json', path)
        effects.assert_not_called()
        self.assertEqual(self.ledger(), before)
        self.assertEqual(sorted(p.relative_to(self.root) for p in self.root.rglob('*')), files)
