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
