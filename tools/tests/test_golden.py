"""Golden cache invariants, full-TGI/QFS comparison and monthly diagnostics."""
import hashlib
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import golden
import golden_dbpf as records


def package(path, entries, *, width=20, time=0):
    head = bytearray(96)
    head[:4] = b'DBPF'
    struct.pack_into('<II', head, 4, 1, 0)
    struct.pack_into('<I', head, 24, time)
    index, payload = bytearray(), bytearray()
    for (t, g, i), data in entries:
        words = [t, g, i & 0xFFFFFFFF]
        if width == 24:
            words.append(i >> 32)
        words += [96 + len(payload), len(data)]
        index.extend(struct.pack('<' + 'I' * len(words), *words))
        payload.extend(data)
    struct.pack_into('<IIII', head, 0x20, 7, len(entries), 96 + len(payload), len(index))
    struct.pack_into('<I', head, 0x3C, int(width == 24))
    path.write_bytes(head + payload + index)


class GoldenTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)

    def test_compression_layout_and_save_timestamp_are_not_gameplay(self):
        key = (1, 2, 3)
        original, changed = self.root / 'a.sc4', self.root / 'b.sc4'
        package(original, [(key, b'abcd')])
        qfs = b'\x10\xfb\x00\x00\x04\xe0abcd\xfc'
        package(changed, [((records.DIR, 0, 0), struct.pack('<IIII', *key, 4)),
                          (key, qfs)], time=1234)
        diff = records.compare_saves(original, changed)
        self.assertEqual(diff['exact_records'], 1)
        self.assertEqual(diff['different_records'], 0)
        self.assertEqual(diff['ignored_gameplay_fields'], [])

    def test_qfs_backreferences_and_size_bounds(self):
        # Four literals, then distance 1 count 3, then a terminal marker.
        qfs = b'\x10\xfb\x00\x00\x07\xe0aaaa\x00\x00\xfc'
        self.assertEqual(records._qfs(qfs, 7), b'a' * 7)
        for data, size in ((qfs, 8), (qfs[:-1], 7),
                           (b'\x10\xfb\x00\x00\x03\x00\x00\xfc', 3)):
            with self.assertRaises(ValueError):
                records._qfs(data, size)

    def test_64_bit_instances_are_not_aliased(self):
        path = self.root / 'high.sc4'
        entries = [((1, 2, 3), b'one'), ((1, 2, 0x100000003), b'two')]
        package(path, entries, width=24)
        self.assertEqual(records.read_records(path), dict(entries))

    def test_added_removed_byte_and_decoded_fields(self):
        a, b = self.root / 'a.sc4', self.root / 'b.sc4'
        text_key = (0x2026960B, 0, 0)
        package(a, [(text_key, struct.pack('<I', 0x10000001) + b'A\0'), ((1, 0, 0), b'gone')])
        package(b, [(text_key, struct.pack('<I', 0x10000001) + b'B\0'), ((2, 0, 0), b'new')])
        diff = records.compare_saves(a, b)
        self.assertEqual(diff['different_records'], 3)
        self.assertEqual(diff['records'][-1]['fields']['text'], {'expected': 'A', 'actual': 'B'})
        self.assertEqual(diff['records'][-1]['first_byte'], 4)

    def test_bad_index_duplicate_and_truncation_fail(self):
        path = self.root / 'broken.sc4'
        for entries in ([((1, 0, 0), b'a'), ((1, 0, 0), b'b')],):
            package(path, entries)
            with self.assertRaises(ValueError):
                records.read_records(path)
        package(path, [((1, 0, 0), b'abc')])
        path.write_bytes(path.read_bytes()[:-1])
        with self.assertRaises(ValueError):
            records.read_records(path)

    def test_digest_first_month_fields_missing_and_prefix(self):
        a = golden.parse_digests('2026 x DIGEST julian=1 (2026-02-01) funds=10 R/C/I=2/3/4\n'
                                 '2026 x DIGEST julian=2 (2026-03-01) funds=20')
        b = golden.parse_digests('other pid DIGEST julian=1 (2026-02-01) funds=10 R/C/I=2/3/4\n'
                                 'other pid DIGEST julian=2 (2026-03-01) funds=19')
        self.assertIsNone(golden.compare_digests(a[:1], b[:1]))
        diff = golden.compare_digests(a, b)
        self.assertEqual(diff['date'], '2026-03-01')
        self.assertEqual(diff['fields']['funds'], {'expected': '20', 'actual': '19'})
        self.assertEqual(golden.compare_digests(a, b[:1])['date'], '2026-03-01')
        self.assertIsNone(golden.compare_digests([], []))

    def test_fingerprint_uses_bytes_and_all_settings(self):
        path = self.root / 'input'
        path.write_bytes(b'one')
        first, manifest = golden.fingerprint({'save': path}, {'until': '2026-03-01'})
        self.assertEqual(first, golden.fingerprint({'save': path}, {'until': '2026-03-01'})[0])
        self.assertNotEqual(first, golden.fingerprint({'save': path}, {'until': '2026-04-01'})[0])
        path.write_bytes(b'two')
        self.assertNotEqual(first, golden.fingerprint({'save': path}, manifest['settings'])[0])

    def make_run(self, name, payload=b'original'):
        run = self.root / name
        run.mkdir()
        package(run / 'end.sc4', [((1, 0, 0), payload)])
        golden.write_json(run / 'digests.json', [{'date': '2026-03-01', 'text': payload.decode(), 'fields': {}}])
        for file in ('verdict.json', 'summary.json'):
            golden.write_json(run / file, {})
        for file in ('game.log', 'harness.txt'):
            (run / file).write_text('evidence', encoding='utf-8')
        return run

    def test_cache_never_overwrites_a_different_result(self):
        run = self.make_run('first')
        key = 'f' * 64
        manifest = {'settings': {'until': '2026-03-01'}}
        cache = self.root / 'cache'
        self.assertEqual(golden.cache_record(cache, key, manifest, run)[0], 'recorded')
        self.assertEqual(golden.qualification(cache / key), 'unverified')
        before = golden.sha256(cache / key / 'end.sc4')
        self.assertEqual(golden.cache_record(cache, key, manifest, run)[0], 'reproduced')
        self.assertEqual(golden.qualification(cache / key), 'reproduced')
        other = self.make_run('second', b'different')
        status, report = golden.cache_record(cache, key, manifest, other)
        self.assertEqual(status, 'nondeterministic')
        self.assertEqual(report['save']['different_records'], 1)
        self.assertEqual(golden.sha256(cache / key / 'end.sc4'), before)
        self.assertEqual(golden.qualification(cache / key), 'nondeterministic')
        golden.cache_record(cache, key, manifest, run)
        self.assertEqual(golden.qualification(cache / key), 'nondeterministic')

    def test_corrupt_cache_is_not_a_comparison_or_cache_hit(self):
        run = self.make_run('run')
        cache, key = self.root / 'cache', 'a' * 64
        golden.cache_record(cache, key, {}, run)
        (cache / key / 'end.sc4').write_bytes(b'corrupt')
        with self.assertRaisesRegex(ValueError, 'corrupt golden artifact'):
            golden.cache_record(cache, key, {}, run)

    def test_script_has_a_full_save_and_replay_preserves_history(self):
        generated = golden.input_script(None, '2026-04-01')
        self.assertIn('2026-04-01 pause\n2026-04-01 digest\n2026-04-01 save\n', generated)
        command = golden.shadow_command('FC4 Test', 'Konradshohe', '2026-03-01', Path('candidate'))
        self.assertIn('--profile shadows', command)
        self.assertIn('--until 2026-03-01', command)
        with self.assertRaises(ValueError):
            golden.safe_name('../Documents')


if __name__ == '__main__':
    unittest.main()
