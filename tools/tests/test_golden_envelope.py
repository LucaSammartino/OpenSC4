"""Observed ranges stay narrow at stable fields and never learn candidates."""
import struct
import contextlib
import io
import unittest
from unittest.mock import patch
import test_golden as fixtures
import golden
import golden_envelope as env


package = fixtures.package


class EnvelopeTests(unittest.TestCase):
    setUp = fixtures.GoldenTests.setUp
    make_run = fixtures.GoldenTests.make_run
    def three(self):
        cache = self.root / 'cache'
        key = 'b' * 64
        for i, payload in enumerate((b'a\x10z', b'a\x20z', b'a\x30z')):
            golden.cache_record(cache, key, {}, self.make_run(str(i), payload))
        return cache / key

    def test_unknown_byte_range_stable_bytes_and_new_record(self):
        entry = self.three()
        envelope = env.build(entry)
        self.assertEqual(envelope['stats']['varying_records'], 1)
        self.assertEqual(envelope['stats']['varying_opaque_bytes'], 1)
        self.assertEqual(env.status(entry, envelope), 'enveloped')
        candidate = self.make_run('candidate', b'a\x25z')
        self.assertTrue(golden.artifacts_exact(golden.compare_cached(entry, candidate, rule='envelope')))
        before = sorted(p.name for p in (entry / 'samples').iterdir())
        package(candidate / 'end.sc4', [((1, 0, 0), b'b\x25z'), ((2, 0, 0), b'new')])
        report = golden.compare_cached(entry, candidate, rule='envelope')
        self.assertEqual(report['save']['different_records'], 2)
        self.assertEqual(report['save']['records'][0]['first_byte'], 0)
        self.assertEqual(before, sorted(p.name for p in (entry / 'samples').iterdir()))

    def test_digest_numeric_ranges_name_first_month_and_missing_occurrence(self):
        entry = self.three()
        paths = env.sample_paths(entry)
        # Tests update fixture hashes after deliberately replacing synthetic data.
        for i, path in enumerate(paths):
            golden.write_json(path / 'digests.json', golden.parse_digests(
                f'DIGEST julian=1 (2026-03-01) funds={10 + i * 10} R/C/I=2/3/4'))
            metadata_path = path / ('baseline.json' if path == entry else 'sample.json')
            metadata = golden.load_json(metadata_path)
            metadata['artifacts']['digests.json'] = golden.sha256(path / 'digests.json')
            golden.write_json(metadata_path, metadata)
        candidate = self.make_run('candidate', b'a\x25z')
        golden.write_json(candidate / 'digests.json', golden.parse_digests(
            'DIGEST julian=1 (2026-03-01) funds=25 R/C/I=2/3/4'))
        self.assertTrue(golden.artifacts_exact(golden.compare_cached(entry, candidate, rule='envelope')))
        golden.write_json(candidate / 'digests.json', golden.parse_digests(
            'DIGEST julian=1 (2026-03-01) funds=31 R/C/I=2/3/4'))
        first = golden.compare_cached(entry, candidate, rule='envelope')['first_digest_difference']
        self.assertEqual(first['date'], '2026-03-01')
        self.assertEqual(first['fields']['funds']['expected'], {'min': 10, 'max': 30})
        golden.write_json(candidate / 'digests.json', [])
        self.assertIsNotNone(golden.compare_cached(entry, candidate, rule='envelope')['first_digest_difference'])

    def test_default_accepts_envelope_and_strict_still_requires_exact(self):
        entry = self.three()
        candidate = self.make_run('candidate', b'a\x25z')
        report = golden.compare_cached(entry, candidate)
        self.assertEqual(report['comparison_rule'], 'observed-envelope')
        self.assertEqual(report['verdict'], 'within-envelope')
        self.assertEqual(report['envelope_status'], 'enveloped')
        self.assertEqual(golden.qualification(entry), 'nondeterministic')
        self.assertTrue(golden.artifacts_exact(report))
        strict = golden.compare_cached(entry, candidate, rule='exact')
        self.assertEqual(strict['verdict'], 'outside')
        self.assertEqual(strict['save']['different_records'], 1)
        self.assertEqual(golden.compare_cached(entry, entry)['verdict'], 'exact')

    def test_cli_three_outcomes_and_only_outside_fails(self):
        entry = self.three()
        common = ['--city', 'Test', '--until', '2026-03-01', '--output', str(self.root),
                  '--dll', str(self.root / 'candidate.dll')]
        # Move the fixture under the CLI's cache directory without altering it.
        cache = self.root / 'golden'
        cache.mkdir()
        entry = entry.rename(cache / entry.name)
        for name, payload, code, outcome in [('exact', b'a\x10z', 0, 'exact'),
                                            ('inside', b'a\x25z', 0, 'within-envelope'),
                                            ('outside', b'a\x31z', 1, 'outside')]:
            run = self.make_run(name, payload)
            golden.write_json(run / 'verdict.json', {'window_exact': True, 'save_date': '2026-03-01'})
            with patch.object(golden, 'prepare', return_value=(entry.name, {'settings': {}}, '', {})), \
                    patch.object(golden, 'fingerprint', return_value=(entry.name, {})), \
                    patch.object(golden, 'run_game', return_value=run) as game, \
                    contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(golden.main(['compare'] + common), code)
                self.assertEqual(game.call_count, 1)
                self.assertEqual(golden.load_json(run / 'comparison.json')['verdict'], outcome)

    def test_no_three_originals_is_labelled_unqualified_without_blocking(self):
        cache, key = self.root / 'cache', 'a' * 64
        for i, payload in enumerate((b'a\x10z', b'a\x30z')):
            golden.cache_record(cache, key, {}, self.make_run(str(i), payload))
        report = golden.compare_cached(cache / key, self.make_run('candidate', b'a\x20z'))
        self.assertEqual(report['envelope_status'], 'unqualified')
        self.assertEqual(report['verdict'], 'within-envelope')

    def test_corrupt_repeat_cannot_widen_or_qualify_an_envelope(self):
        entry = self.three()
        (env.sample_paths(entry)[1] / 'end.sc4').write_bytes(b'corrupt')
        with self.assertRaisesRegex(ValueError, 'corrupt golden sample'):
            env.build(entry)

    def test_identical_originals_choose_exact_comparison(self):
        cache, key = self.root / 'cache', 'd' * 64
        for i in range(3):
            golden.cache_record(cache, key, {}, self.make_run(str(i)))
        entry = cache / key
        self.assertEqual(env.status(entry, env.build(entry)), 'qualified')
        report = golden.compare_cached(entry, self.make_run('candidate', b'different'))
        self.assertEqual(report['comparison_rule'], 'exact')
        self.assertFalse(golden.artifacts_exact(report))

    def test_decoded_numeric_span_accepts_carry_and_rejects_outside_range(self):
        cache, key = self.root / 'cache', 'e' * 64
        def run(name, counter):
            directory = self.make_run(name)
            payload = bytearray(91)
            struct.pack_into('<I', payload, 0, 91)
            struct.pack_into('<H', payload, 12, 4)
            struct.pack_into('<I', payload, 32, counter)
            package(directory / 'end.sc4', [((0x2990C1E5, 0, 0), payload)])
            return directory
        for i, value in enumerate((250, 270, 260)):
            golden.cache_record(cache, key, {}, run(str(i), value))
        entry = cache / key
        self.assertTrue(golden.artifacts_exact(golden.compare_cached(entry, run('inside', 256), rule='envelope')))
        report = golden.compare_cached(entry, run('outside', 271), rule='envelope')
        self.assertEqual(report['save']['records'][0]['fields']['level.0.counter']['actual'], 271)
