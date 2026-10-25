"""Exercise the real PowerShell request parser with isolated region/save fixtures."""
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(shutil.which('powershell'), 'requires PowerShell')
class QueueRegionTests(unittest.TestCase):
    def parse(self, settings):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for region in ('FC4 Test', 'San Francisco'):
                directory = root / 'regions' / region
                directory.mkdir(parents=True)
                (directory / 'City - New City (55).sc4').touch()
                (directory / 'City - Konradshohe.sc4').touch()
            dll = root / 'engine/build-x86/FreeCity4.dll'
            dll.parent.mkdir(parents=True)
            dll.touch()
            req = root / 'trial.req'
            req.write_text(settings + '\n[ini]\npopup_dialogs = 0\n', encoding='utf-8')
            result = subprocess.run([
                'powershell', '-NoProfile', '-File',
                str(ROOT / 'tools/tests/fixtures/queue-probe.ps1'),
                '-Source', str(ROOT / 'tools/game_queue.ps1'), '-Root', str(root),
                '-Regions', str(root / 'regions'), '-Request', str(req)],
                capture_output=True, text=True, check=True)
            return json.loads(result.stdout)

    def test_default_region_and_explicit_region(self):
        default = self.parse('label = default')
        self.assertIsNone(default['error'])
        self.assertEqual(default['region'], 'FC4 Test')
        other = self.parse('region = San Francisco\ncity = New City (55)\nparallel = 1')
        self.assertIsNone(other['error'])
        self.assertEqual(other['region'], 'San Francisco')
        self.assertTrue(other['peekParallel'])
        self.assertEqual(other['parallel'], '1')

    def test_regions_cannot_escape_and_errors_name_the_region(self):
        for region in ('../elsewhere', 'C:/elsewhere', '.', 'San Francisco.', 'Missing'):
            with self.subTest(region=region):
                self.assertIn('region', self.parse('region = ' + region)['error'])
        self.assertIn("region 'San Francisco'", self.parse(
            'region = San Francisco\ncity = Missing')['error'])

    def test_reach_output_defaults_to_the_labelled_results_file(self):
        bare = self.parse('label = trial\n[ini]\nreach = 1')
        self.assertIsNone(bare['error'])
        lines = bare['iniText'].split('\n')
        defaults = [line for line in lines if line.startswith('reach_output = ')]
        self.assertEqual(len(defaults), 1)
        self.assertTrue(defaults[0].endswith('build-runs\\game-results\\trial.reach.tsv'), defaults[0])
        explicit = self.parse('label = trial\n[ini]\nreach = on\nreach_output = C:\\x\\y.tsv')
        self.assertEqual([line for line in explicit['iniText'].split('\n') if line.startswith('reach_output')],
                         ['reach_output = C:\\x\\y.tsv'])
        for off in ('reach = 0', 'popup_dialogs = 0'):
            self.assertNotIn('reach_output', self.parse('label = trial\n[ini]\n' + off)['iniText'])

    def test_parallel_is_opt_in_and_ini_cannot_enable_it(self):
        q = self.parse('label = trial\n[ini]\nparallel = 1')
        self.assertFalse(q['peekParallel'])
        self.assertEqual(q['parallel'], '0')
        self.assertIn('parallel must be', self.parse('parallel = yes')['error'])
