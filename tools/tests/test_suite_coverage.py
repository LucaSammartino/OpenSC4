"""Suite credit must follow actual entered bytes and every observed key/run."""
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import suite
import suite_coverage as metric


def batch(root, tag='test', profile='shadows', bodies=None, enable='foo'):
    directory = metric.folder(root, tag)
    directory.mkdir(parents=True)
    bodies = bodies or {'one': 'foo shadow 2000-02-01: mismatches=0; resyncs=0\n'}
    requests, results = [], {}
    for name, body in bodies.items():
        scenario = dict(name=name, region='Berlin', city='Tegel', script='-', end='2000-04-01',
                        sets='smoke,full', notes='fixture')
        requests.append(dict(scenario=scenario, request=f'[ini]\nenable = {enable}\n'))
        text = body + 'AUTOTEST VERDICT: clean (date=2000-04-01, days_per_second=8.0)\n'
        (directory / (name + '.log')).write_text(text, encoding='utf-8')
        (directory / (name + '.verdict')).write_text('verdict: clean\n', encoding='utf-8')
        (directory / (name + '.reach.tsv')).write_text(
            '# original counters\naddress\tentries\tfirst_window\tskipped\n'
            '00401000\t2\t1\t\n00402000\t0\t0\t\n00403000\t3\t1\tpreexisting-hook\n', encoding='utf-8')
        results[name] = dict(suite.parse_log(text, scenario['end']), log=str(directory / (name + '.log')))
        (directory / (name + '.json')).write_text(json.dumps(dict(wall_seconds=120)), encoding='utf-8')
    manifest = dict(profile=profile, dll_sha256='fixture', shadow_keys=['foo'], requests=requests)
    (directory / 'manifest.json').write_text(json.dumps(manifest), encoding='utf-8')
    suite.write_results(directory, manifest, results)
    return directory


class SuiteCreditTests(unittest.TestCase):
    def test_logged_native_refusal_does_not_erase_observed_exact_comparisons(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            batch(root, profile='replace', bodies={'one':
                'script: fixture: 1 action(s)\n'
                'script: line 1 2000-01-01 zone -> ok=0 error=5\n'
                'foo shadow 2000-02-01: mismatches=0; resyncs=0\n'})
            evidence = metric.load(root, 'test')
            self.assertEqual(evidence['replaced'], {'foo'})
            self.assertEqual(evidence['scenarios'][0]['row']['actions_failed'], '1')
            self.assertFalse(evidence['scenarios'][0]['selection_ready'])

    def test_only_entered_implemented_bytes_are_exercised_or_verified(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            batch(root)
            evidence = metric.load(root, 'test')
        self.assertEqual(evidence['reached'], {0x401000})
        rows = [dict(implemented=True, shadow='foo_shadow', replace='foo'),
                dict(implemented=False, shadow='foo_shadow', replace='foo')]
        game = [(0x401000, 100, 0, 'simulation'), (0x402000, 900, 0, 'simulation'),
                (0x403000, 700, 1, 'simulation'), (0x404000, 50, None, 'shared')]
        states = metric.states(evidence, game, rows, {})
        self.assertTrue(states[0x401000]['suite-verified'])
        for address in (0x402000, 0x403000, 0x404000):
            self.assertFalse(states[address]['suite-exercised'])
            self.assertFalse(states[address]['suite-verified'])
        self.assertFalse(states[0x401000]['suite-replaced'])

    def test_any_scenario_mismatch_voids_the_key(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            batch(root, bodies={'one': 'foo shadow 2000-02-01: mismatches=0\n',
                                'two': 'foo shadow 2000-02-01: mismatches=1\n'})
            evidence = metric.load(root, 'test')
        self.assertNotIn('foo', evidence['exact'])
        self.assertEqual(evidence['reached'], {0x401000})

    def test_trailing_zero_and_matching_after_resync_do_not_earn_credit(self):
        for body in ('foo shadow 2000-02-01: mismatches=3; mismatches=0\n',
                     'foo shadow 2000-02-01: mismatches=0; resynced: state\n',
                     'foo shadow 2000-02-01: mismatches=0\nfoo shadow shutdown: resyncs=1\n'):
            with self.subTest(body=body), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                batch(root, bodies={'one': body})
                self.assertNotIn('foo', metric.load(root, 'test')['exact'])

    def test_missing_log_or_zero_samples_is_not_exact(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            directory = batch(root)
            (directory / 'one.log').unlink()
            self.assertFalse(metric.load(root, 'test')['exact'])

    def test_missing_log_voids_only_unaccounted_comparisons_not_unsubmitted_keys(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            directory = batch(root, bodies={'one': 'foo shadow 2000-02-01: mismatches=0\n',
                                           'two': 'foo shadow 2000-02-01: mismatches=0\n'})
            (directory / 'two.log').unlink()
            self.assertNotIn('foo', metric.load(root, 'test')['exact'])
            suite.write_tsv(directory / 'per_key.tsv', suite.KEY_COLUMNS,
                            [dict(key='foo', scenarios_run=1, scenarios_exact=1, first_mismatch='-')])
            self.assertIn('foo', metric.load(root, 'test')['exact'])
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            batch(root, bodies={'one': 'foo shadow: ON\n'})
            self.assertFalse(metric.load(root, 'test')['exact'])

    def test_harness_failure_overrides_clean_summary_for_replace(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            directory = batch(root, profile='replace', enable='foo,bar')
            self.assertEqual(metric.load(root, 'test')['replaced'], {'foo', 'bar'})
            (directory / 'one.verdict').write_text('verdict: corrupt\n', encoding='utf-8')
            self.assertFalse(metric.load(root, 'test')['replaced'])

    def test_every_replace_run_enabling_key_must_be_clean(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            batch(root, 'shadows')
            batch(root, 'replace1', 'replace', enable='foo,bar')
            directory = batch(root, 'replace2', 'replace', enable='foo')
            (directory / 'one.verdict').write_text('verdict: crash\n', encoding='utf-8')
            metric.combine(root, 'both', ['shadows', 'replace1', 'replace2'])
            evidence = metric.load(root, 'both')
            self.assertEqual(evidence['replaced'], {'bar'})
            self.assertIn('foo', evidence['exact'])
            self.assertEqual(len(evidence['scenarios']), 3)
            self.assertTrue((metric.folder(root, 'both') / 'shadows/one.reach.tsv').is_file())

    def test_default_replacement_keys_count_and_disable_overrides_them(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            directory = batch(root, profile='replace')
            self.assertIn('default', metric.load(root, 'test', ['default'])['replaced'])
            path = directory / 'manifest.json'
            manifest = json.loads(path.read_text(encoding='utf-8'))
            manifest['requests'][0]['request'] += 'disable = default\n'
            path.write_text(json.dumps(manifest), encoding='utf-8')
            self.assertNotIn('default', metric.load(root, 'test', ['default'])['replaced'])

    def test_later_game_failure_does_not_erase_exact_shadow_evidence(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            directory = batch(root, profile='replace')
            (directory / 'one.verdict').write_text('verdict: corrupt\n', encoding='utf-8')
            evidence = metric.load(root, 'test')
            self.assertIn('foo', evidence['exact'])
            self.assertFalse(evidence['replaced'])

    def test_tag_traversal_and_incomplete_summary_are_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with self.assertRaises(ValueError):
                metric.load(root, '../elsewhere')
            directory = batch(root)
            suite.write_tsv(directory / 'summary.tsv', suite.COLUMNS, [])
            with self.assertRaisesRegex(ValueError, 'every requested'):
                metric.load(root, 'test')

    def test_owner_label_and_clock_alias(self):
        evidence = dict(reached={1, 2}, exact={'clock', 'flora-occupant'}, replaced=set())
        rows = [dict(implemented=True, shadow='clock24_shadow', replace='-'),
                dict(implemented=True, shadow='flora_shadow:flora-occupant', replace='-')]
        states = metric.states(evidence, [(1, 10, 0, ''), (2, 20, 1, '')], rows, {'clock24': 'clock'})
        self.assertTrue(all(row['suite-verified'] for row in states.values()))


def item(name, addresses, wall=100, verdict='clean'):
    return dict(name=name, reached=set(addresses), wall_seconds=wall,
                selection_ready=True,
                row=dict(verdict=verdict, end_reached='1'))


class WeightedTierTests(unittest.TestCase):
    def test_clean_verdict_without_complete_evidence_is_not_a_tier_candidate(self):
        rows = [dict(item('incomplete', [1]), selection_ready=False), item('complete', [2])]
        plan = metric.greedy(rows, {1: 1000, 2: 10}, fraction=.95)
        self.assertEqual(plan['selected'], ['complete'])
        self.assertFalse(plan['achieved'])
        self.assertEqual(plan['full_bytes'], 1010)

    def test_missing_trace_is_unknown_but_an_observed_empty_trace_is_zero(self):
        evidence = dict(item('missing', []), first={}, windows=[], trace_present=False)
        evidence['row']['profile'] = 'shadows'
        row = metric.reach_row(evidence, {1: 100})
        self.assertEqual(row['implemented_bytes'], '-')
        self.assertEqual(row['percent_implemented'], '-')
        evidence['trace_present'] = True
        row = metric.reach_row(evidence, {1: 100})
        self.assertEqual(row['implemented_bytes'], 0)
        self.assertEqual(row['percent_implemented'], '0.000000%')

    def test_greedy_weights_bytes_rather_than_functions_and_keeps_98_percent(self):
        items = [item('many-tiny', range(1, 21)), item('one-big', [21])]
        weights = {**dict.fromkeys(range(1, 21), 1), 21: 1000}
        plan = metric.greedy(items, weights)
        self.assertEqual(plan['selected'], ['one-big'])
        self.assertTrue(plan['achieved'])
        self.assertEqual(plan['reached_bytes'], 1000)

    def test_budget_is_measured_parallel_wall_not_sum(self):
        items = [item(str(i), [i], wall=600) for i in range(6)]
        weights = dict.fromkeys(range(6), 100)
        plan = metric.greedy(items, weights, slots=6, budget=900)
        self.assertTrue(plan['achieved'])
        self.assertEqual(plan['wall_seconds'], 600)
        self.assertFalse(metric.greedy(items, weights, slots=1, budget=900)['achieved'])

    def test_failed_unique_reach_stays_in_denominator_but_cannot_be_selected(self):
        plan = metric.greedy([item('pass', [1]), item('crash', [2], verdict='crash')], {1: 1, 2: 1000})
        self.assertEqual(plan['full_bytes'], 1001)
        self.assertFalse(plan['achieved'])

    def test_ties_are_reproducible_and_instrumentation_only_addresses_ignored(self):
        items = [item('b', [1, 999]), item('a', [1])]
        first = metric.greedy(items, {1: 100})
        self.assertEqual(first, metric.greedy(list(reversed(items)), {1: 100}))
        self.assertEqual(first['selected'], ['a'])


if __name__ == '__main__':
    unittest.main()
