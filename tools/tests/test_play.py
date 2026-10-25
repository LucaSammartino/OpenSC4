"""One-off commands share the suite queue and preserve/validate dated actions."""
import contextlib
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import play
import suite


class PlayTests(unittest.TestCase):
    def test_combines_actions_and_prints_real_parse_results(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'base.script').write_text('2000-01-01 speed 0\n')
            result = suite.parse_log('script: line 1 2000-01-01 speed 0 -> speed=0\n'
                                     'AUTOTEST VERDICT: clean (date=2000-04-01)\n', '2000-04-01')
            out = io.StringIO()
            with patch.object(suite, 'run_scenarios', return_value=(root, {'play': result})) as run, contextlib.redirect_stdout(out):
                code = play.main(['--region', 'Berlin', '--city', 'Konradshohe', '--script', 'base.script',
                                  '--do', '2000-02-01 zone 10 10 20 20 1', '--do', '2000-03-01 save',
                                  '--until', '2000-04-01', '--profile', 'shadows', '--dll', str(root), '--tag', 'one'], root)
            self.assertEqual(code, 0)
            row = run.call_args.args[1][0]
            self.assertEqual(row['region'], 'Berlin')
            self.assertEqual(row['end'], '2000-04-01')
            self.assertEqual((root / row['script']).read_text().splitlines(),
                             ['2000-01-01 speed 0', '2000-02-01 zone 10 10 20 20 1', '2000-03-01 save'])
            self.assertEqual(run.call_args.kwargs['profile'], 'shadows')
            self.assertIn('verdict: clean; end_reached=1', out.getvalue())
            self.assertIn('actions: 1 ok, 0 failed, 0 pending', out.getvalue())

    def test_invalid_inputs_do_not_submit(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'commands.lua').write_text('return true\n')
            cases = [['--region', '../Berlin'], ['--until', '2000-02-30'],
                     ['--do', '2000-01-01 mark x\n2000-01-02 save'],
                     ['--script', 'commands.lua', '--do', '2000-01-01 save']]
            for args in cases:
                with self.subTest(args=args), patch.object(suite, 'run_scenarios') as run, contextlib.redirect_stderr(io.StringIO()):
                    with self.assertRaises(SystemExit):
                        play.main(['--city', 'Konradshohe', '--until', '2000-04-01'] + args, root)
                    run.assert_not_called()
