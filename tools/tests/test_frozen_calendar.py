"""ingame_run.ps1 ends a run whose calendar stays on one date while the simulator ticks (112 minutes in slot 4)."""
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


def beat(date, pause, emergency, ticks):
    return (f'2026-10-26 11:41:57.204 [pid 1 tid 2] heartbeat (periodic): {date} (julian 2457308) speed 2 pause {pause} '
            f'hidden 0 emergency {emergency} city running | +{ticks} OnTick, +58949 deliveries, +0 interface calls\n')


@unittest.skipUnless(shutil.which('powershell'), 'requires PowerShell')
class FrozenCalendarTests(unittest.TestCase):
    def probe(self, log, limit=600, seconds=700):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'game.log'
            path.write_text(log, encoding='utf-8')
            out = subprocess.run(['powershell', '-NoProfile', '-File', str(ROOT / 'tools/tests/fixtures/frozen-calendar-probe.ps1'),
                                  '-Source', str(ROOT / 'tools/ingame_run.ps1'), '-LogFile', str(path),
                                  '-Limit', str(limit), '-Seconds', str(seconds)], capture_output=True, text=True, check=True)
            return json.loads(out.stdout)

    def test_a_date_that_never_moves_while_the_simulator_ticks_is_a_stall(self):
        got = self.probe(beat('2015-10-12', 1, 1, 526))
        self.assertIsNone(got['first'])  # the first sight only starts the clock
        self.assertTrue(got['later'].startswith('stalled (fallback): frozen calendar at 2015-10-12 for 700 s'))
        self.assertIn('pause 1 hidden 0 emergency 1', got['later'])

    def test_a_short_freeze_a_moving_date_and_a_silent_simulator_are_not(self):
        self.assertIsNone(self.probe(beat('2015-10-12', 1, 1, 526), seconds=300)['later'])
        self.assertIsNone(self.probe(beat('2015-10-12', 0, 0, 0))['later'])
        self.assertIsNone(self.probe('no heartbeat yet\n')['later'])
        moving = beat('2015-10-12', 0, 0, 500)
        self.assertEqual(self.probe(moving)['date'], '2015-10-12')


if __name__ == '__main__':
    unittest.main()
