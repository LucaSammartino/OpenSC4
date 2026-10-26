"""The conductor's round classification and resume-spec writing, without launching a lane."""
import argparse
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import conductor  # noqa: E402


class ClassifyTests(unittest.TestCase):
    def test_continue_transient_review(self):
        self.assertEqual(conductor.classify({'state': 'continue'}), 'continue')
        self.assertEqual(conductor.classify({'state': 'ready'}), 'review')
        self.assertEqual(conductor.classify('error: stream disconnected before completion'), 'transient')
        self.assertEqual(conductor.classify('lane refused: touches overlap'), 'review')
        self.assertEqual(conductor.classify(None), 'review')


class ResumeTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        (self.root / 'specs').mkdir()
        self.state = self.root / 'state.json'
        self.state.write_text(json.dumps({'lanes': {}}), encoding='utf-8')
        spec = {'args': {'name': 'x27', 'kind': 'tool', 'task': 'Do the thing.'}}
        (self.root / 'specs' / 'x27.spec.json').write_text(json.dumps(spec), encoding='utf-8')
        a = argparse.Namespace(run=str(self.root), state=str(self.state))
        self.run = conductor.Run(a)
        self.launched = []
        self.run.launch = lambda p: self.launched.append(Path(p)) or 'LAUNCHED x27'
        self.old_sh = conductor.sh
        conductor.sh = lambda args, **kw: '1'  # one commit ahead of main: no empty commit needed
        self.addCleanup(setattr, conductor, 'sh', self.old_sh)

    def written(self):
        return json.loads(self.launched[-1].read_text(encoding='utf-8'))

    def test_next_round_spec(self):
        self.assertEqual(self.run.resume('x27', {'rounds': 1, 'round_limit': 3}, 'Carry on.'), 'LAUNCHED x27')
        d = self.written()
        self.assertEqual(self.launched[-1].name, 'x27-r2.spec.json')
        self.assertTrue(d['resume'])
        self.assertTrue(d['args']['task'].startswith('Resume: continue from docs/lanes/x27.md'))
        self.assertNotIn('FINAL ROUND', d['args']['task'])
        self.assertEqual(self.run.kind('x27'), 'tool')

    def test_last_round_says_final(self):
        self.run.resume('x27', {'rounds': 2, 'round_limit': 3}, 'Carry on.')
        self.assertIn('FINAL ROUND', self.written()['args']['task'])

    def test_no_round_left(self):
        self.assertIsNone(self.run.resume('x27', {'rounds': 3, 'round_limit': 3}, 'Carry on.'))
        self.assertEqual(self.launched, [])

    def test_note_used_once(self):
        (self.root / 'notes').mkdir()
        (self.root / 'notes' / 'x27-next.txt').write_text('Fix A.\nFix B.\n', encoding='utf-8')
        self.assertEqual(self.run.take_note('x27'), 'Fix A. Fix B. ')
        self.assertEqual(self.run.take_note('x27'), 'Fix A. Fix B. ')  # a refused launch keeps the note
        self.run.use_note('x27')  # what resume does once the round launched
        self.assertTrue((self.root / 'notes' / 'x27-next.used.txt').exists())
        self.assertEqual(self.run.take_note('x27'), '')


if __name__ == '__main__':
    unittest.main()


class FillFromPendingTests(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        (self.root / 'specs').mkdir()
        self.state = self.root / 'state.json'
        self.state.write_text(json.dumps({'lanes': {}}), encoding='utf-8')
        self.specs = []
        for name in ('a29', 'b29', 'c29'):
            path = self.root / 'specs' / f'{name}.spec.json'
            path.write_text('{}', encoding='utf-8')
            self.specs.append(path)
        (self.root / 'pending.txt').write_text(''.join(str(p) + chr(10) for p in self.specs), encoding='utf-8')
        self.run = conductor.Run(argparse.Namespace(run=str(self.root), state=str(self.state)))
        self.run.log = lambda line: None
        self.tried = []

    def pending(self):
        return [Path(line).stem.split('.')[0] for line in
                (self.root / 'pending.txt').read_text(encoding='utf-8').splitlines() if line.strip()]

    def test_a_refusal_for_one_spec_does_not_block_the_next(self):
        replies = {'a29': 'RuntimeError: touches x: possible duplicate, mentioned by unmerged lane(s) y',
                   'b29': 'LAUNCHED run4 round 1', 'c29': 'LAUNCHED run4 round 2'}
        self.run.launch = lambda p: self.tried.append(Path(p).stem.split('.')[0]) or replies[Path(p).stem.split('.')[0]]
        self.assertTrue(self.run.fill_from_pending())
        self.assertEqual(self.tried, ['a29', 'b29'])
        self.assertEqual(self.pending(), ['a29', 'c29'])  # the blocked spec keeps its place

    def test_a_run_wide_refusal_stops_the_pass(self):
        self.run.launch = lambda p: self.tried.append(Path(p).stem) or 'ValueError: Live lane cap reached'
        self.assertFalse(self.run.fill_from_pending())
        self.assertEqual(len(self.tried), 1)
        self.assertEqual(self.pending(), ['a29', 'b29', 'c29'])


class TargetTests(unittest.TestCase):
    def test_target_file_written_with_a_byte_order_mark_still_reads(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            state = root / 'state.json'
            state.write_text(json.dumps({'lanes': {}}), encoding='utf-8')
            run = conductor.Run(argparse.Namespace(run=str(root), state=str(state)))
            self.assertEqual(run.target(), 13)  # no file: the default
            (root / 'target.txt').write_bytes(b'\xef\xbb\xbf24\r\n')
            self.assertEqual(run.target(), 24)
