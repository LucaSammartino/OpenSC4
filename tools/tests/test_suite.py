"""Queue round trips, recorded-log accounting and shipped scenario contracts."""
import contextlib
from datetime import date
import io
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import suite

FIXTURES = Path(__file__).parent / 'fixtures'


def checkout(root):
    (root / 'tools').mkdir()
    shutil.copy2(ROOT / 'tools/regress.ps1', root / 'tools/regress.ps1')
    dll = root / 'engine/build-x86/FreeCity4.dll'
    dll.parent.mkdir(parents=True)
    dll.write_bytes(b'fixture DLL')
    script = root / 'actions.script'
    script.write_text('2000-01-01 mark fixture\n', encoding='utf-8')
    return dict(name='case.one', region='San Francisco', city='New City (55)',
                script='actions.script', end='2000-04-01', sets='full', notes='fixture')


class SuiteParsingTests(unittest.TestCase):
    def test_actions_summaries_and_compare_counts(self):
        text = (FIXTURES / 'suite-evidence.log').read_text(encoding='utf-8')
        text += (FIXTURES / 'shadowlabels-clean.log').read_text(encoding='utf-8')
        text += (FIXTURES / 'shadowlabels-empty.log').read_text(encoding='utf-8')
        result = suite.parse_log(text, '2000-04-01')
        self.assertEqual((result['verdict'], result['end_reached'], result['days_per_second']),
                         ('clean', '1', '9.10'))
        self.assertEqual((result['actions_ok'], result['actions_failed'], result['actions_pending']), (2, 1, 0))
        self.assertEqual((result['compare_exact'], result['compare_different'], result['reach_functions']), (18, 1, '9'))
        self.assertEqual(result['per_key']['terrain']['lines'], 2)
        self.assertEqual(result['per_key']['terrain']['exact'], 1)
        self.assertIn('line 8:', result['per_key']['terrain']['first_mismatch'])
        self.assertEqual(result['per_key']['land_value']['exact'], 1)
        self.assertEqual(result['per_key']['flora-occupant']['exact'], 1)
        self.assertEqual(result['per_key']['app']['exact'], 0)
        self.assertEqual(result['per_key']['weather']['lines'], 3)
        self.assertEqual(result['per_key']['weather']['exact'], 2)
        self.assertEqual(result['per_key']['formats']['lines'], 1)
        self.assertEqual(result['per_key']['nettool_plan']['lines'], 2)
        self.assertGreaterEqual(len(result['shadow_summaries']), 4)
        self.assertTrue(any('weather shadow: city shutdown:' in line['text'] for line in result['shadow_payloads']))
        self.assertGreater(len(result['shadow_payloads']), len(result['shadow_records']))
        self.assertEqual(len(result['inplace_lines']), 1)

    def test_missing_rejected_and_refused_runs_cannot_pass(self):
        missing = suite.parse_log('', '2000-04-01', 'verdict: rejected\nline: unknown region')
        self.assertEqual(missing['verdict'], 'rejected')
        self.assertFalse(suite.successful({'missing': missing}))
        refused = suite.parse_log('script: ERROR line 2: unknown action\nscript: REFUSED script\n', '2000-04-01')
        self.assertEqual(refused['actions_failed'], 1)
        partial = suite.parse_log("script: loaded 'x': 7 action(s), today\n", '2000-04-01')
        self.assertEqual(partial['actions_pending'], 7)

    def test_shadow_differences_are_findings_not_run_failures(self):
        result = suite.parse_log('terrain shadow month 2000-04-01: mismatches=9\n'
                                 'AUTOTEST VERDICT: clean (date=2000-04-01)\n', '2000-04-01')
        self.assertTrue(suite.successful({'case': result}))
        self.assertEqual(result['worst_shadow'], 'terrain:1/1')

    def test_profiles_follow_regress_defaults(self):
        shadows = suite.profile_ini(ROOT, 'shadows')
        self.assertEqual(shadows['traffic_shadow_hypotheses'], '7')
        self.assertEqual(shadows['disable'], 'budget,ordinance')
        self.assertEqual(shadows['dbpf_bounds_shadow'], '1')
        replace = suite.profile_ini(ROOT, 'replace')['enable']
        self.assertIn('terrain', replace)
        self.assertIn('netrules', replace)
        self.assertNotIn('enable', shadows)
        self.assertEqual(suite.profile_ini(ROOT, 'plain'), {})
        for value in ('x = 1\ncity = Other', 'region = Other', 'dll = evil.dll', '[ini]', 'script = other'):
            with self.subTest(value=value), self.assertRaises(ValueError):
                suite.extras([value])

    def test_shipped_scenarios_cover_regions_sizes_and_actions(self):
        rows = suite.read_scenarios(ROOT, 'full')
        self.assertGreaterEqual(len(rows), 19)
        self.assertEqual(len(suite.read_scenarios(ROOT, 'smoke')), 6)
        self.assertEqual({r['region'] for r in rows}, {'Berlin', 'London', 'Timbuktu', 'New York', 'San Francisco'})
        self.assertTrue({'empty-small', 'empty-medium', 'empty-large'} <= {r['name'] for r in rows})
        for row in rows:
            lines = [line.split() for line in (ROOT / row['script']).read_text().splitlines()
                     if line and not line.startswith('#')]
            actions = {line[1] for line in lines}
            self.assertTrue({'zone', 'network', 'terrain', 'disaster', 'save', 'speed', 'clockmode'} <= actions)
            self.assertEqual({int(line[-1]) for line in lines if line[1] == 'network'}, {0, 1, 3, 4, 5, 6})
            self.assertEqual({int(line[-1]) for line in lines if line[1] == 'zone'}, set(range(1, 10)))
            self.assertTrue({'fire', 'riot'} <= {line[2] for line in lines if line[1] == 'disaster'})
            start, end = date.fromisoformat(lines[0][0]), date.fromisoformat(row['end'])
            self.assertLessEqual((end - start).days, 92)
            self.assertTrue(all(date.fromisoformat(line[0]) < end for line in lines))


class SuiteQueueTests(unittest.TestCase):
    def test_real_files_round_trip_all_requests_and_evidence(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            second = dict(row, name='other', script='-')
            calls = []

            def queue_tick(_):
                requests = list((root / 'build-runs/game-queue').glob('*.req'))
                self.assertEqual(len(requests), 2)  # submit the whole batch before waiting
                for request in requests:
                    body = request.read_text(encoding='utf-8')
                    self.assertIn('region = San Francisco', body)
                    self.assertIn('parallel = 1', body)
                    calls.append(body)
                    label = next(line.split(' = ')[1] for line in body.splitlines() if line.startswith('label = '))
                    result = root / 'build-runs/game-results' / label
                    result.parent.mkdir(exist_ok=True)
                    Path(str(result) + '.log').write_text(
                        "script: loaded 'x': 1 action(s), today\nscript: line 1 2000-01-01 mark fixture -> marked\n"
                        'terrain shadow month 2000-04-01: mismatches=1\n'
                        'AUTOTEST VERDICT: clean (date=2000-04-01, days_per_second=8.00)\n', encoding='utf-8')
                    Path(str(result) + '.verdict').write_text('verdict: clean\n', encoding='utf-8')

            with patch.object(suite.time, 'sleep', side_effect=queue_tick), contextlib.redirect_stdout(io.StringIO()):
                directory, results = suite.run_scenarios(root, [row, second], profile='replace', dll=str(root), tag='v1.0')
            self.assertEqual(len(calls), 2)
            self.assertTrue(suite.successful(results))
            manifest = json.loads((directory / 'manifest.json').read_text())
            self.assertEqual(len(manifest['dll_sha256']), 64)
            self.assertEqual((directory / 'scripts/case.one.script').read_text(), (root / 'actions.script').read_text())
            table = (directory / 'summary.tsv').read_text()
            self.assertIn('case.one\treplace\tclean\t1\t8.00', table)
            self.assertIn('terrain\t2\t0\tcase.one', (directory / 'per_key.tsv').read_text())
            self.assertTrue((directory / 'case.one.shadows.txt').exists())
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                suite.report(root, 'v1.0', 'v1.0')
            self.assertIn('case.one\texact\t-', out.getvalue())
            with self.assertRaises(ValueError):
                suite.make_batch(root, [row], 'plain', str(root), 'v1.0')

    def test_wait_expiry_writes_rows_without_claiming_a_game_ran(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            directory, manifest = suite.make_batch(root, [row], 'plain', str(root), 'timeout')
            with contextlib.redirect_stderr(io.StringIO()):
                results = suite.wait_for_batch(root, directory, manifest, 0)
            self.assertEqual(results[row['name']]['verdict'], 'queue-timeout')
            self.assertIn('queue-timeout\t0', (directory / 'summary.tsv').read_text())
