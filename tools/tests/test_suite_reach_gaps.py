"""The fixed attribution must account for gains and regressions conservatively."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import suite_reach_gaps as gaps
import suite_coverage


def evidence(reached=(), exact=('foo',), bad=()):
    return dict(reached=set(reached), exact=set(exact), replaced=set(),
                scenarios=[dict(ran=set(exact) | set(bad), exact=set(exact))])


class ReachGapTests(unittest.TestCase):
    def setUp(self):
        self.data = dict(game=[(1, 100, 0, ''), (2, 200, 0, ''), (3, 300, 1, ''),
                               (4, 400, None, '')],
                         rows=[dict(implemented=True, shadow='foo_shadow'),
                               dict(implemented=False, shadow='foo_shadow')],
                         owner={1: 'Owner', 2: 'Owner', 3: 'Other'})

    def test_unexecuted_bytes_and_gain_share_one_denominator(self):
        owners, functions, metrics = gaps.measure(self.data, evidence([1]), evidence([2, 3, 4]))
        self.assertEqual(owners[0]['gap_bytes'], 200)
        self.assertEqual(owners[0]['newly_executed_bytes'], 200)
        self.assertEqual(owners[0]['newly_verified_bytes'], 200)
        self.assertEqual(functions, [dict(address='00000002', bytes=200, owner='Owner', key='foo', newly_executed=1)])
        self.assertEqual(metrics['game_bytes'], 1000)
        self.assertEqual(metrics['before_percent'], 10)
        self.assertEqual(metrics['after_percent'], 30)
        self.assertEqual(metrics['gain_points'], 20)

    def test_new_mismatch_removes_old_credit_too(self):
        owners, _, metrics = gaps.measure(self.data, evidence([1]), evidence([2], exact=(), bad=['foo']))
        self.assertEqual(owners[0]['verdict'], 'different')
        self.assertEqual(owners[0]['newly_executed_bytes'], 200)
        self.assertEqual(owners[0]['newly_verified_bytes'], 0)
        self.assertEqual(metrics['gain_points'], -10)

    def test_alias_and_explicit_owner_suffix_match_official_states(self):
        for label in ('clock24_shadow', 'ignored_shadow:clock'):
            self.data['rows'][0]['shadow'] = label
            baseline, after = evidence([1], ['clock']), evidence([2], ['clock'])
            _, _, metrics = gaps.measure(self.data, baseline, after, {'clock24': 'clock'})
            union = evidence([1, 2], ['clock'])
            states = suite_coverage.states(union, self.data['game'], self.data['rows'], {'clock24': 'clock'})
            expected = sum(size for address, size, _, _ in self.data['game']
                           if states[address]['suite-verified'])
            self.assertEqual(metrics['after_verified_bytes'], expected)

    def test_new_key_is_credited_only_if_baseline_did_not_fail_it(self):
        _, _, metrics = gaps.measure(self.data, evidence(exact=()), evidence([1]))
        self.assertEqual(metrics['after_verified_bytes'], 100)
        _, _, metrics = gaps.measure(self.data, evidence(exact=(), bad=['foo']), evidence([1]))
        self.assertEqual(metrics['after_verified_bytes'], 0)

    def test_no_after_is_unchanged_and_nonexact_owners_are_not_ranked(self):
        owners, _, metrics = gaps.measure(self.data, evidence([1]))
        self.assertEqual(metrics['gain_points'], 0)
        self.assertEqual(len(owners), 1)
        owners, functions, _ = gaps.measure(self.data, evidence(exact=(), bad=['foo']))
        self.assertEqual((owners, functions), ([], []))


if __name__ == '__main__':
    unittest.main()
