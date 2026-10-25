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
        export_failure = suite.parse_log('AUTOTEST VERDICT: clean (date=2000-04-01, days_per_second=8.0)',
                                         '2000-04-01', 'verdict: harness-error\nline: state-export-failed: no save')
        self.assertEqual(export_failure['verdict'], 'harness-error')
        self.assertFalse(suite.successful({'export': export_failure}))
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

    def test_frozen_clock_and_short_windows_fail_but_explicit_load_only_passes(self):
        text = ('memory: address space 968/2047 MiB; largest free 624 MiB\n'
                'AUTOTEST VERDICT: clean (date=2000-01-01, real_seconds=1800, days_per_second=0.00)\n')
        frozen = suite.parse_log(text, '2000-01-01')
        self.assertEqual(frozen['verdict'], 'invalid')
        self.assertEqual(frozen['end_reached'], '0')
        self.assertIn('did not advance', frozen['invalid_reason'])
        self.assertEqual(len(frozen['memory_lines']), 1)
        self.assertFalse(suite.successful({'frozen': frozen}))
        loaded = suite.parse_log(text, '2000-01-01', only_load=True)
        self.assertTrue(suite.successful({'load': loaded}))
        short = suite.parse_log(text.replace('0.00', '2.00'), '2000-04-01')
        self.assertEqual(short['verdict'], 'invalid')

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
        self.assertEqual(len(rows), 108)
        self.assertEqual(len(suite.read_scenarios(ROOT, 'smoke')), 6)
        self.assertEqual({r['region'] for r in rows}, {'Berlin', 'London', 'Timbuktu', 'Maxisland', 'Paradis'})
        self.assertEqual(sum(r['region'] == 'Maxisland' for r in rows), 44)
        self.assertEqual(sum(r['region'] == 'Paradis' for r in rows), 47)
        self.assertTrue({'maxisland-west-weburg', 'maxisland-san-indersty', 'maxisland-simicon-valley', 'paradis-mitras'}
                        <= {r['name'] for r in suite.read_scenarios(ROOT, 'smoke')})
        self.assertTrue({'empty-small', 'empty-medium', 'empty-large'} <= {r['name'] for r in rows})
        for row in rows:
            if suite.load_only(row):
                self.assertIn(row['name'], ('berlin-tempelhof', 'berlin-schonerlinde'))
                self.assertEqual(row['script'], '-')
                continue
            lines = [line.split() for line in (ROOT / row['script']).read_text().splitlines()
                     if line and not line.startswith('#')]
            actions = {line[1] for line in lines}
            self.assertTrue({'zone', 'network', 'terrain', 'disaster', 'save', 'speed', 'clockmode'} <= actions)
            self.assertEqual({int(line[-1]) for line in lines if line[1] == 'network'}, {0, 1, 3, 4, 5, 6})
            self.assertEqual({int(line[-1]) for line in lines if line[1] == 'zone'}, set(range(1, 10)))
            self.assertTrue({'fire', 'riot'} <= {line[2] for line in lines if line[1] == 'disaster'})
            start, end = date.fromisoformat(lines[0][0]), date.fromisoformat(row['end'])
            self.assertLessEqual((end - start).days, 92)
            self.assertTrue(all(date.fromisoformat(line[0]) <= end for line in lines))
            self.assertTrue(all(date.fromisoformat(line[0]) == end for line in lines if line[1] == 'disaster'))
            # Emergency probes can pause the clock; no later game day may be
            # needed to execute the save/quit protocol.
            self.assertEqual([line[1] for line in lines[-5:]], ['disaster', 'disaster', 'save', 'digest', 'mark'])

    def test_lua_script_and_load_only_requests(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            (root / 'actions.lua').write_text('print("fixture")\n', encoding='utf-8')
            row.update(script='actions.lua')
            loaded = dict(row, name='load', script='-', notes='load-only; unfounded city')
            directory, manifest = suite.make_batch(root, [row, loaded], 'plain', str(root), 'lua')
            lua, load = [r['request'] for r in manifest['requests']]
            self.assertIn('lua_script = build-runs/suite/lua/scripts/case.one.lua', lua)
            self.assertNotIn('\nscript =', lua)
            self.assertTrue(all('close_dialogs = 1' in r['request'] for r in manifest['requests']))
            self.assertIn('seconds = 12', load)
            self.assertNotIn('end_date =', load)


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
            self.assertEqual(table.count(manifest['dll_sha256']), 2)
            self.assertIn('terrain\t2\t0\tcase.one', (directory / 'per_key.tsv').read_text())
            findings = (directory / 'findings.tsv').read_text()
            self.assertIn('terrain\tcase.one\t', findings)
            self.assertIn('terrain\tother\t', findings)
            self.assertTrue((directory / 'case.one.shadows.txt').exists())
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                suite.report(root, 'v1.0', 'v1.0')
            self.assertIn('case.one\texact\t-', out.getvalue())
            self.assertIn('case.one\texact\t-', (directory / 'determinism.tsv').read_text())
            baseline = directory.parent / 'different-dll'
            shutil.copytree(directory, baseline)
            changed = dict(manifest, dll_sha256='0' * 64)
            (baseline / 'manifest.json').write_text(json.dumps(changed))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                suite.report(root, 'v1.0', 'different-dll')
            self.assertIn('case.one\tinput-different\t-', out.getvalue())
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
