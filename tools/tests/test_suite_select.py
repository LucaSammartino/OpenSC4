"""Coverage selection must buy distinct reach with measured time, never counts alone."""
import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import suite
import suite_select as select


def item(name, addresses, wall, size=100, keys=None):
    return dict(name=name, addresses=set(addresses), wall_seconds=wall, city_bytes=size,
                per_key=keys or {}, windows=[], source='fixture')


class SelectionTests(unittest.TestCase):
    def test_greedy_uses_marginal_reach_time_and_culls_subsumed_rows(self):
        items = [item('fast', 'ab', 1), item('broad', 'abc', 2), item('slow', 'd', 100)]
        plan = select.choose(items, slots=1, budget=5)
        self.assertEqual([r['name'] for r in plan['selected']], ['broad'])
        self.assertEqual((plan['union_reach'], plan['full_union_reach']), (3, 4))
        self.assertEqual(plan['wall_seconds'], 2)
        self.assertEqual(plan['selected'][0]['marginal_functions'], 3)

    def test_culling_releases_budget_for_a_later_distinct_window(self):
        plan = select.choose([item('fast', 'ab', 1), item('broad', 'abc', 2), item('extra', 'd', 4)],
                             slots=1, budget=6)
        self.assertEqual({r['name'] for r in plan['selected']}, {'broad', 'extra'})
        self.assertEqual(plan['union_reach'], 4)

    def test_ties_choose_smaller_city_and_pack_all_six_slots(self):
        items = [item('big', 'a', 10, 1000), item('small', 'a', 10, 10)]
        items += [item(str(i), {str(i)}, 10) for i in range(5)]
        plan = select.choose(items, slots=6, budget=10)
        self.assertEqual(len(plan['selected']), 6)
        self.assertNotIn('big', [r['name'] for r in plan['selected']])
        self.assertEqual(plan['wall_seconds'], 10)
        self.assertEqual(select.wall_time(plan['selected'], 1), 60)

    def test_target_keys_prefer_most_activity_per_second_without_claiming_functions(self):
        def key(count):
            return {'traffic': dict(lines=count, exact=count, first_mismatch='')}
        items = [item('slow', [], 20, keys=key(50)), item('fast', [], 5, keys=key(100))]
        with self.assertRaisesRegex(ValueError, 'requires per-scenario reach'):
            select.choose(items)
        plan = select.choose(items, target_keys={'traffic'})
        self.assertEqual(plan['objective'], 'observed-keys')
        self.assertEqual([r['name'] for r in plan['selected']], ['fast'])
        self.assertIsNone(plan['selected'][0]['marginal_functions'])

    def test_key_fallback_buys_activity_per_second_not_just_one_sample(self):
        items = [item('one', [], 1, keys={'traffic': dict(lines=1)}),
                 item('many', [], 2, keys={'traffic': dict(lines=1000)})]
        plan = select.choose(items, target_keys={'traffic'})
        self.assertEqual([r['name'] for r in plan['selected']], ['many'])

    def test_target_functions_do_not_select_unrelated_reach(self):
        items = [item('unrelated', 'abcdef', 1), item('owner', 'xy', 2)]
        plan = select.choose(items, target_addresses={'x', 'y'}, target_keys={'owner'})
        self.assertEqual([r['name'] for r in plan['selected']], ['owner'])
        self.assertEqual(plan['union_reach'], 2)

    def test_reach_tracer_table_skips_unentered_and_unarmed_rows(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'reach.tsv'
            path.write_text('# bounded counts\naddress\tentries\tfirst_window\tskipped\n'
                            '00401000\t3\t1\t\n00402000\t0\t0\t\n'
                            '00403000\t2\t2\tlate-entry-conflict\n')
            found, first = select.reach(path)
            self.assertEqual(found, {'00401000'})
            self.assertEqual(first, {'00401000': 1})

    def test_saturation_needs_more_than_the_load_window(self):
        self.assertIsNone(select.knee([dict(new=500, total=500)]))
        self.assertEqual(select.knee([dict(new=500, total=500), dict(new=20, total=520),
                                      dict(new=2, total=522), dict(new=0, total=522)]), 2)

    def test_real_owner_and_shadow_key_mapping(self):
        addresses, labels = select.targets(ROOT, 'traffic_shadow,cSC4ZoneManager')
        self.assertIn('traffic', labels)
        self.assertIn('004dd780', addresses)  # published PlaceZone entry

    def test_log_timing_reach_windows_and_memory_are_preserved(self):
        text = ('2026-10-25 12:00:00.000 [pid 1 tid 2] DLL_PROCESS_ATTACH: dll\n'
                '2026-10-25 12:00:02.000 [pid 1 tid 2] autoload: calling LoadCity(x)\n'
                '2026-10-25 12:00:05.000 [pid 1 tid 2] autoload: LoadCity(x) -> true\n'
                'reach window 1 date=2000-01-01 new=500 total=500\n'
                'reach window 2 date=2000-02-01 new=2 total=502\n'
                '2026-10-25 12:00:15.000 [pid 1 tid 2] AUTOTEST VERDICT: clean '
                '(date=2000-03-01, real_seconds=10.0, days_per_second=6.00)\n')
        result = suite.parse_log(text, '2000-03-01', 'request: wall 22 s')
        self.assertEqual(result['load_seconds'], 3)
        self.assertEqual(result['wall_seconds'], 22)
        self.assertEqual(result['seconds_per_month'], 5.073)
        self.assertEqual(result['reach_windows'][1]['new'], 2)

    def test_default_refuses_to_invent_tier1_before_calibration(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'engine/tests/scenarios').mkdir(parents=True)
            (root / 'engine/tests/scenarios/suite.tsv').write_text(
                '\t'.join(suite.SCENARIO_COLUMNS) + '\n'
                'city\tBerlin\tTegel\t-\t2000-04-01\tfull\tfixture\n')
            with contextlib.redirect_stderr(io.StringIO()) as errors, self.assertRaises(SystemExit):
                suite.main(['run'], root=root)
            self.assertIn('requires per-scenario reach', errors.getvalue())

    def test_marked_tier_runs_without_calibration_and_all_includes_gen(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'engine/tests/scenarios').mkdir(parents=True)
            (root / 'engine/tests/scenarios/suite.tsv').write_text(
                '\t'.join(suite.SCENARIO_COLUMNS) + '\n'
                'city\tBerlin\tTegel\t-\t2000-04-01\tfull,tier1\tfixture\n'
                'generated\tBerlin\tTegel\t-\t2000-04-01\tgen\tfixture\n')
            self.assertEqual(len(suite.read_scenarios(root, 'all')), 2)
            with patch.object(suite, 'run_scenarios', return_value=(root, {})) as run:
                self.assertEqual(suite.main(['run', '--set', 'tier1', '--dry-run', '--slots', '12'], root=root), 0)
            self.assertEqual([r['name'] for r in run.call_args.args[1]], ['city'])
            self.assertEqual(run.call_args.kwargs['slots'], 12)


if __name__ == '__main__':
    unittest.main()
