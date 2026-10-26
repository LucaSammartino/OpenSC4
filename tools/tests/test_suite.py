"""Queue round trips, recorded-log accounting and shipped scenario contracts."""
import contextlib
import csv
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


def test_findings_set_can_be_staged_from_cli():
    with patch.object(suite, 'read_scenarios', return_value=[dict(name='finding')]) as read, \
            patch.object(suite, 'run_scenarios', return_value=(Path('unused'), {})) as run:
        assert suite.main(['run', '--set', 'findings', '--dry-run']) == 0
        assert read.call_args.args[1] == 'findings'
        assert run.call_args.kwargs['dry_run']


def test_result_exports_read_each_completed_trace_once_per_harvest(tmp_path):
    import suite_select
    directory = tmp_path / 'results'
    directory.mkdir()
    scenarios = [dict(name=n, end='2000-04-01', region='Test', city=n, notes='')
                 for n in ('one', 'two')]
    manifest = dict(profile='shadows', dll_sha256='fixture', shadow_keys=[],
                    requests=[dict(scenario=row) for row in scenarios])
    cache, results = {}, {}
    with patch.object(suite_select, 'reach', return_value=({'00401000'}, {})) as read:
        for row in scenarios:
            results[row['name']] = suite.parse_log('', row['end'])
            suite.write_results(directory, manifest, results, reach_cache=cache)
        suite.write_results(directory, manifest, results, reach_cache=cache)
        assert read.call_count == 2
        # A new operation must read again, including a newly replaced trace.
        suite.write_results(directory, manifest, results)
        assert read.call_count == 4


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
    def test_city_limits_scale_long_large_runs_and_keep_operator_overrides(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            (root / 'actions.script').write_text('2000-01-01 mark start\n2001-12-31 save full\n')
            row.update(end='2002-01-01', notes='large 256-cell city')
            limits, ini = suite.city_limits(row, root / 'actions.script')
            self.assertGreater(int(limits['seconds']), 4000)
            self.assertGreater(int(limits['timeout']), int(limits['seconds']) + 600)
            self.assertEqual(ini['autotest_stall_grace_ms'], '120000')
            _, sized = suite.make_batch(root, [row], 'plain', str(root), 'large-save')
            self.assertIn('script_save_grace_ms = 120000\n', sized['requests'][0]['request'])
            _, manifest = suite.make_batch(root, [row], 'plain', str(root), 'sized',
                                          ['seconds = 123', 'script_save_grace_ms = 0'])
            request = manifest['requests'][0]['request']
            self.assertIn('seconds = 123\n', request)
            self.assertIn('script_save_grace_ms = 0\n', request)
            self.assertIn('autotest_stall_grace_ms = 120000\n', request)

    def test_resume_preserves_claimed_labels_and_only_bounds_unsubmitted_requests(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = dict(checkout(root), script='-')
            other = dict(row, name='other')
            directory, manifest = suite.make_batch(root, [row, other], 'plain', str(root), 'resume')
            results = root / 'build-runs/game-results'
            results.mkdir(parents=True)
            request = manifest['requests'][0]
            source = results / request['label']
            Path(str(source) + '.log').write_text(
                'AUTOTEST VERDICT: clean (date=2000-04-01, days_per_second=8.0)\n', encoding='utf-8')
            Path(str(source) + '.verdict').write_text('verdict: clean\n', encoding='utf-8')
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                status = suite.main(['resume', 'resume', '--wait-seconds', '0',
                                     '--seconds', '240', '--timeout', '300'], root=root)
            updated = json.loads((directory / 'manifest.json').read_text(encoding='utf-8'))
            self.assertEqual(status, 1)
            self.assertIn('seconds = 750\n', updated['requests'][0]['request'])
            self.assertIn('seconds = 240\n', updated['requests'][1]['request'])
            self.assertIn('timeout = 300\n', updated['requests'][1]['request'])
            self.assertFalse(list((root / 'build-runs/game-queue').glob('*.req')))
            with (directory / 'summary.tsv').open(encoding='utf-8', newline='') as stream:
                rows = list(csv.DictReader(stream, delimiter='\t'))
            self.assertEqual(rows[0]['verdict'], 'clean')
            self.assertEqual(rows[1]['verdict'], 'not-submitted')

    def test_reach_refuses_a_stale_dll_before_submitting(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            with self.assertRaisesRegex(ValueError, 'DLL lacks reach tracer'):
                suite.make_batch(root, [row], 'plain', str(root), 'old', ['reach = 1'])
            self.assertFalse((root / 'build-runs/suite/old').exists())

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

    def test_known_original_quit_fault_retains_evidence_and_passes(self):
        text = (FIXTURES / 'suite-exit-crash.log').read_text(encoding='utf-8')
        result = suite.parse_log(text, '2028-01-28', 'verdict: corrupt')
        self.assertEqual((result['verdict'], result['end_reached'], result['raw_verdict']),
                         ('clean', '1', 'corrupt'))
        self.assertEqual(result['known_quit_faults'], 1)
        self.assertEqual(result['exception_address'], '0x005D0F94')
        self.assertEqual(result['exceptions'][0]['location'], 'SimCity 4.exe+0x1D0F94')
        self.assertEqual(result['exceptions'][0]['line'], 3)
        self.assertTrue(suite.successful({'quit': result}))
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            manifest = dict(requests=[dict(scenario=dict(name='quit', end='2028-01-28'))],
                            profile='shadows', dll_sha256='fixture', shadow_keys=[])
            suite.write_results(directory, manifest, {'quit': result})
            summary = (directory / 'summary.tsv').read_text(encoding='utf-8')
            self.assertIn('exception_address', summary.splitlines()[0])
            self.assertIn('clean', summary)
            self.assertIn('0x005D0F94', summary)

    def test_posted_quit_crash_requires_an_accepted_command_after_target(self):
        lines = (FIXTURES / 'suite-exit-crash.log').read_text(encoding='utf-8').splitlines(True)
        lines[1] = lines[1].replace('cISC4App::RequestQuit(false, false)',
                                  'native quit command 0xC7AA61C6 queued')
        result = suite.parse_log(''.join(lines), '2028-01-28', 'verdict: corrupt')
        self.assertEqual((result['verdict'], result['exception_address']),
                         ('clean', '0x005D0F94'))
        self.assertTrue(suite.successful({'posted': result}))
        for text in (''.join(lines).replace('queued -> 1', 'queued -> 0'),
                     lines[1] + lines[0] + ''.join(lines[2:]),
                     lines[0] + lines[2] + lines[1] + lines[3]):
            with self.subTest(text=text):
                self.assertEqual(suite.parse_log(text, '2028-01-28')['verdict'], 'corrupt')

    def test_unknown_quit_faults_and_watchdogs_remain_failures(self):
        text = (FIXTURES / 'suite-exit-crash.log').read_text(encoding='utf-8')
        for changed in (
                text.replace('SimCity 4.exe+0x1D0F94', 'SimCity 4.exe+0x123456'),
                text.replace('SimCity 4.exe+0x1D0F94', 'FreeCity4.dll+0x1D0F94'),
                text.replace('reading address 0x00000000', 'reading address 0x00000004'),
                'WATCHDOG: main thread stalled - no OnTick for 10000 ms\n' + text):
            with self.subTest(text=changed):
                result = suite.parse_log(changed, '2028-01-28', 'verdict: corrupt')
                self.assertEqual(result['verdict'], 'clean-exit-crash')
                self.assertEqual(result['known_quit_faults'], 0)
                self.assertFalse(suite.successful({'unknown': result}))

    def test_end_date_alone_cannot_reclassify_an_exception_as_a_quit_crash(self):
        lines = (FIXTURES / 'suite-exit-crash.log').read_text(encoding='utf-8').splitlines(True)
        cases = [
            lines[2] + lines[0] + lines[1] + lines[3],  # exception before target
            lines[0] + lines[2] + lines[1] + lines[3],  # exception before quit
            lines[0] + lines[2] + lines[3],  # target, no accepted quit
            lines[1] + lines[2] + lines[3],  # final verdict date alone
            lines[0] + lines[1].replace('-> 1', '-> 0') + lines[2] + lines[3],
            ''.join(lines[:2] + lines[3:]),  # no exception evidence
            lines[2] + ''.join(lines),  # earlier exception plus exit exception
        ]
        for text in cases:
            with self.subTest(text=text):
                self.assertEqual(suite.parse_log(text, '2028-01-28')['verdict'], 'corrupt')
        for verdict in ('harness-error', 'stalled', 'aborted'):
            with self.subTest(verdict=verdict):
                result = suite.parse_log(''.join(lines), '2028-01-28', 'verdict: ' + verdict)
                self.assertEqual(result['verdict'], verdict)
        loaded = suite.parse_log(''.join(lines), '2028-01-28', only_load=True)
        self.assertEqual((loaded['verdict'], loaded['end_reached']), ('corrupt', '0'))

    def test_failed_native_quit_cannot_pass_by_terminating_before_shutdown(self):
        text = ('autotest: end date 2028-01-28 reached\n'
                'autotest: native quit command 0xC7AA61C6 queued -> 0 (autotest_quit_post)\n'
                'AUTOTEST VERDICT: clean (date=2028-01-28, days_per_second=3.2)\n')
        result = suite.parse_log(text, '2028-01-28')
        self.assertEqual((result['verdict'], result['raw_verdict']), ('harness-error', 'clean'))
        self.assertIn('shutdown untested', result['invalid_reason'])
        self.assertFalse(suite.successful({'failed-dispatch': result}))

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
        self.assertEqual(suite.profile_ini(ROOT, 'plain'),
                         {'disable': 'simulator,budget', 'enable': 'none'})
        for value in ('x = 1\ncity = Other', 'region = Other', 'dll = evil.dll', '[ini]', 'script = other'):
            with self.subTest(value=value), self.assertRaises(ValueError):
                suite.extras([value])

    def test_starter_scenarios_cover_regions_sizes_and_actions(self):
        rows = suite.read_scenarios(ROOT, 'full')
        # Trigger/Lua lanes append focused scenarios; the original city census
        # keeps its stronger all-actions contract independently of those rows.
        cities = [r for r in rows if r['name'].startswith(
            ('berlin-', 'london-', 'timbuktu-', 'maxisland-', 'paradis-', 'empty-'))]
        self.assertEqual(len(cities), 108)
        self.assertGreaterEqual(len(rows), len(cities))
        for row in rows:
            if row['script'] != '-':
                self.assertTrue(suite.script_file(ROOT, row['script']).is_file(), row['name'])
        self.assertEqual(len(suite.read_scenarios(ROOT, 'smoke')), 6)
        self.assertEqual({r['region'] for r in cities}, {'Berlin', 'London', 'Timbuktu', 'Maxisland', 'Paradis'})
        self.assertEqual(sum(r['region'] == 'Maxisland' for r in cities), 44)
        self.assertEqual(sum(r['region'] == 'Paradis' for r in cities), 47)
        self.assertTrue({'maxisland-west-weburg', 'maxisland-san-indersty', 'maxisland-simicon-valley', 'paradis-mitras'}
                        <= {r['name'] for r in suite.read_scenarios(ROOT, 'smoke')})
        self.assertTrue({'empty-small', 'empty-medium', 'empty-large'} <= {r['name'] for r in rows})
        for row in cities:
            if suite.load_only(row):
                self.assertIn(row['name'], ('berlin-tempelhof', 'berlin-schonerlinde'))
                self.assertEqual(row['script'], '-')
                continue
            script = ROOT / row['script']
            if script.suffix == '.lua':
                script = script.with_suffix('.script')  # Lua establishes mayor mode first.
            lines = [line.split() for line in script.read_text().splitlines()
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
            (root / 'probe.lua').write_text('print("fixture")\n', encoding='utf-8')
            row.update(script='probe.lua')
            loaded = dict(row, name='load', script='-', notes='load-only; unfounded city')
            with patch.object(suite, 'city_path', return_value=root / 'fixture.sc4'):
                directory, manifest = suite.make_batch(root, [row, loaded], 'plain', str(root), 'lua')
            lua, load = [r['request'] for r in manifest['requests']]
            self.assertIn('lua_script = build-runs/suite/lua/scripts/case.one.lua', lua)
            self.assertNotIn('\nscript =', lua)
            self.assertTrue(all('close_dialogs = 1' in r['request'] for r in manifest['requests']))
            self.assertIn('seconds = 60\n', load)
            self.assertIn('timeout = 240\n', load)
            self.assertNotIn('end_date =', load)

    def test_load_only_does_not_wait_for_a_whole_install_archive_scan(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            loaded = dict(row, name='load', script='-', notes='load-only; medium city')
            _, manifest = suite.make_batch(root, [row, loaded], 'shadows', str(root), 'archives')
            advancing, load = [r['request'] for r in manifest['requests']]
            for key in ('dbpf_shadow', 'dbpf_bounds_shadow'):
                self.assertIn(key + ' = 0\n', load)
                self.assertIn(key + ' = 1\n', advancing)


class SuiteQueueTests(unittest.TestCase):
    def test_dated_saves_opt_into_bounded_grace_with_explicit_override(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            script = root / row['script']
            for tag, action, expected in (
                    ('full', '2000-01-01 save # full save', True),
                    ('fast', '2000-01-01 save fast', True),
                    ('none', '# 2000-01-01 save\n2000-01-01 mark save', False)):
                script.write_text(action + '\n', encoding='utf-8')
                _, manifest = suite.make_batch(root, [row], 'plain', str(root), tag)
                body = manifest['requests'][0]['request']
                self.assertEqual('\nscript_save_grace_ms = 60000\n' in body, expected)
                self.assertIn('[ini]\n', body.split('script_save_grace_ms')[0])
            script.write_text('2000-01-01 save\n', encoding='utf-8')
            _, manifest = suite.make_batch(root, [row], 'plain', str(root), 'override',
                                           ['script_save_grace_ms=0'])
            self.assertIn('\nscript_save_grace_ms = 0\n', manifest['requests'][0]['request'])

    def test_lua_can_stage_a_dated_terrain_companion_without_replacing_lua_entry(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            row['script'] = 'probe.lua'
            (root / 'probe.lua').write_text('-- Lua fixture\n')
            setup = '2000-01-01 terrain 0 0 256 256 280 1\n2000-01-01 save\n'
            (root / 'probe.script').write_text(setup)
            directory, manifest = suite.make_batch(root, [row], 'plain', str(root), 'companion')
            body = manifest['requests'][0]['request']
            self.assertIn('lua_script = build-runs/suite/companion/scripts/case.one.lua', body)
            self.assertIn('\nscript = build-runs/suite/companion/scripts/case.one.script', body)
            self.assertIn('\nscript_save_grace_ms = 60000\n', body)
            self.assertEqual((directory / 'scripts/case.one.script').read_text(), setup)

    def test_lua_request_and_harvested_command_evidence_round_trip(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            row['script'] = 'probe.lua'
            (root / row['script']).write_text('-- Lua fixture\n')

            def queue_tick(_):
                request, = (root / 'build-runs/game-queue').glob('*.req')
                body = request.read_text()
                self.assertIn('lua_script = build-runs/suite/lua/scripts/case.one.lua', body)
                self.assertIn('lua_extensions = 1', body)
                self.assertIn('autotest_unpause = 0', body)
                self.assertNotIn('\nscript = ', body)
                label = next(line.split(' = ')[1] for line in body.splitlines() if line.startswith('label = '))
                prefix = root / 'build-runs/game-results' / label
                prefix.parent.mkdir(exist_ok=True)
                Path(str(prefix) + '.verdict').write_text('verdict: clean\n')
                Path(str(prefix) + '.log').write_text('AUTOTEST VERDICT: clean (date=2000-04-01)\n')
                Path(str(prefix) + '.lua.log').write_text(
                    'FC4_LUA 1 probe Before PlaceLot 42 8 8 0 true false\n'
                    'FC4_LUA 2 probe Return PlaceLot status=0 true nil nil nil nil\n'
                    'FC4_LUA 3 probe scenario-complete\n')

            with patch.object(suite.time, 'sleep', side_effect=queue_tick), contextlib.redirect_stdout(io.StringIO()):
                directory, results = suite.run_scenarios(root, [row], profile='plain', dll=str(root), tag='lua')
            self.assertTrue(suite.successful(results))
            self.assertEqual(results[row['name']]['actions_ok'], 1)
            self.assertTrue((directory / 'case.one.lua.log').is_file())

    def test_lua_missing_incomplete_and_failed_returns_cannot_pass(self):
        cases = [('', 0, 1), ('FC4_LUA 1 test Before SaveCityQuick true', 0, 2),
                 ('FC4_LUA 1 test Before PlaceLot 42\nFC4_LUA 2 test Return PlaceLot status=2\n'
                  'FC4_LUA 3 test scenario-complete', 1, 0)]
        for text, failed, pending in cases:
            result = suite.add_lua_evidence(suite.parse_log('AUTOTEST VERDICT: clean (date=2000-04-01)', '2000-04-01'), text)
            self.assertEqual((result['actions_failed'], result['actions_pending']), (failed, pending))
            self.assertFalse(suite.successful({'test': result}))

    def test_real_files_round_trip_all_requests_and_evidence(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            second = dict(row, name='other', script='-')
            calls = []

            def queue_tick(_):
                requests = list((root / 'build-runs/game-queue').glob('*.req'))
                self.assertEqual(len(requests), 2)
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

            with patch.object(suite.time, 'sleep', side_effect=queue_tick), contextlib.redirect_stdout(io.StringIO()), \
                    patch.object(suite, 'write_results', wraps=suite.write_results) as export:
                directory, results = suite.run_scenarios(root, [row, second], profile='replace', dll=str(root), tag='v1.0')
            self.assertEqual(export.call_count, 2)  # one changed harvest, one final flush
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

    def test_large_batch_refills_only_after_a_completed_verdict(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            rows = [dict(row, name=f'case{i}', script='-') for i in range(9)]
            completed, peak = [], []

            def queue_tick(_):
                requests = sorted((root / 'build-runs/game-queue').glob('*.req'))
                peak.append(len(requests))
                self.assertLessEqual(len(requests), 6)
                self.assertTrue(requests)
                request = requests[0]
                result = root / 'build-runs/game-results' / request.stem
                result.parent.mkdir(exist_ok=True)
                Path(str(result) + '.log').write_text(
                    'AUTOTEST VERDICT: clean (date=2000-04-01, days_per_second=8.0)\n', encoding='utf-8')
                Path(str(result) + '.verdict').write_text('verdict: clean\n', encoding='utf-8')
                completed.append(request.stem)
                request.unlink()  # the shared poller has moved it to done/

            with patch.object(suite.time, 'sleep', side_effect=queue_tick), contextlib.redirect_stdout(io.StringIO()):
                _, results = suite.run_scenarios(root, rows, profile='plain', dll=str(root), tag='bounded', slots=12)
            self.assertEqual(len(set(completed)), 9)
            self.assertEqual(max(peak), 6)
            self.assertTrue(suite.successful(results))

    def test_expired_batch_distinguishes_unsubmitted_requests(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = checkout(root)
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                _, results = suite.run_scenarios(root, [row], profile='plain', dll=str(root),
                                                 tag='not-sent', wait_seconds=0)
            self.assertEqual(results[row['name']]['verdict'], 'not-submitted')
            self.assertFalse(list((root / 'build-runs/game-queue').glob('*.req')))

    def test_stop_file_drains_claimed_requests_without_refilling(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            row = dict(checkout(root), script='-')
            directory, manifest = suite.make_batch(root, [row, dict(row, name='later')],
                                                    'plain', str(root), 'drain')
            queue = root / 'build-runs/game-queue'
            queue.mkdir(parents=True)
            claimed = queue / (manifest['requests'][0]['label'] + '.req')
            claimed.write_text(manifest['requests'][0]['request'])
            stop = directory / 'STOP'
            stop.touch()

            def finish_claimed(_):
                self.assertEqual(list(queue.glob('*.req')), [claimed])
                output = root / 'build-runs/game-results' / claimed.stem
                output.parent.mkdir(parents=True)
                Path(str(output) + '.verdict').write_text('verdict: clean\n')
                Path(str(output) + '.log').write_text(
                    'AUTOTEST VERDICT: clean (date=2000-04-01, days_per_second=8)\n')

            with patch.object(suite.time, 'sleep', side_effect=finish_claimed), contextlib.redirect_stdout(io.StringIO()):
                results = suite.wait_for_batch(root, directory, manifest, 60, submit=True, stop_file=stop)
            self.assertEqual(results[row['name']]['verdict'], 'clean')
            self.assertEqual(results['later']['verdict'], 'not-submitted')
