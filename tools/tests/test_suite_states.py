"""Prepared states exclude setup evidence and reject stale/failed inputs."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import suite
import suite_states
from test_suite import checkout


class PreparedStatesTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.row = checkout(self.root)
        self.row.update(end='2000-04-01', notes='medium; saved 2000-01-31')
        (self.root / 'actions.script').write_text(
            '2000-02-01 zone 1 1 2 2 1\n2000-02-02 speed 2\n'
            '2000-04-01 disaster fire auto\n2000-04-01 save\n', encoding='utf-8')
        self.city = self.root / 'original.sc4'
        self.city.write_bytes(b'DBPF original')
        self.source = patch.object(suite_states, 'city_source', return_value=self.city)
        self.source.start()
        self.addCleanup(self.source.stop)
        self.dll = self.root / 'engine/build-x86/FreeCity4.dll'

    def prepare(self):
        rows = suite_states.prepare(self.root, [self.row], 'warm', 1)
        directory, manifest = suite.make_batch(self.root, rows, 'plain', str(self.root), 'setup-warm')
        self.states = self.root / 'build-runs/suite/states/warm'
        self.save = self.states / (self.row['name'] + '.sc4')
        return rows, directory, manifest

    def finish(self):
        rows, directory, manifest = self.prepare()
        self.save.write_bytes(b'DBPF prepared')
        parsed = suite.parse_log('script: line 1 2000-02-29 save -> ok=1\n'
                                'AUTOTEST VERDICT: clean (date=2000-02-29, days_per_second=4.0)', '2000-02-29')
        self.assertTrue(suite_states.finalize(self.root, 'warm', manifest, {self.row['name']: parsed}))
        return rows, directory, manifest

    def test_setup_export_and_reuse_keep_actions_and_provenance_separate(self):
        rows, directory, manifest = self.finish()
        setup = (self.root / rows[0]['script']).read_text()
        self.assertIn('2000-02-29 save', setup)  # month-end/leap-year clamping
        self.assertNotIn('disaster', setup)
        self.assertIn('export_city = ', manifest['requests'][0]['request'])
        self.assertNotIn('reach = 1', manifest['requests'][0]['request'])
        reused = suite_states.reuse(self.root, [self.row], 'warm', self.dll)
        measured = (self.root / reused[0]['script']).read_text()
        self.assertNotIn('zone 1', measured)
        self.assertIn('2000-04-01 disaster', measured)
        self.assertEqual(reused[0]['_setup_actions'], 5)
        _, window = suite.make_batch(self.root, reused, 'shadows', str(self.root), 'measured')
        self.assertIn('prepared_city = ', window['requests'][0]['request'])
        self.assertNotIn('export_city = ', window['requests'][0]['request'])
        self.assertEqual(self.city.read_bytes(), b'DBPF original')

    def test_reuse_rejects_tampered_save_city_script_and_dll(self):
        self.finish()
        for path in (self.save, self.city, self.root / 'actions.script', self.dll):
            with self.subTest(path=path):
                original = path.read_bytes()
                path.write_bytes(original + b'changed')
                with self.assertRaisesRegex(ValueError, 'hash changed'):
                    suite_states.reuse(self.root, [self.row], 'warm', self.dll)
                path.write_bytes(original)

    def test_failed_setup_cannot_create_completed_metadata(self):
        _, _, manifest = self.prepare()
        self.save.write_bytes(b'DBPF prepared')
        failed = suite.parse_log('script: line 1 2000-02-01 zone -> ok=0 error=1\n'
                                'AUTOTEST VERDICT: clean (date=2000-02-29, days_per_second=4.0)', '2000-02-29')
        self.assertFalse(suite_states.finalize(self.root, 'warm', manifest, {self.row['name']: failed}))
        with self.assertRaisesRegex(ValueError, 'no completed prepared state'):
            suite_states.reuse(self.root, [self.row], 'warm', self.dll)

    def test_setup_stops_before_emergency_and_does_not_overwrite(self):
        rows = suite_states.prepare(self.root, [self.row], 'long', 12)
        self.assertEqual(rows[0]['end'], '2000-03-31')
        with self.assertRaisesRegex(ValueError, 'already exists'):
            suite_states.prepare(self.root, [self.row], 'long', 1)


if __name__ == '__main__':
    unittest.main()
