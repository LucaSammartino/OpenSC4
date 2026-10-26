"""A priority = high queue request starts its game at AboveNormal (the train's regress starved at half a core)."""
from pathlib import Path
import re
import shutil
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(shutil.which('powershell'), 'requires PowerShell')
class GamePriorityTests(unittest.TestCase):
    def parse_errors(self, name):
        out = subprocess.run(['powershell', '-NoProfile', '-Command',
                              f"$e=$null;$t=$null;[void][System.Management.Automation.Language.Parser]::ParseFile('{ROOT / name}',[ref]$t,[ref]$e); $e.Count"],
                             capture_output=True, text=True, check=True)
        return int(out.stdout.strip())

    def test_both_scripts_parse(self):
        self.assertEqual(self.parse_errors('tools/ingame_run.ps1'), 0)
        self.assertEqual(self.parse_errors('tools/game_queue.ps1'), 0)

    def test_the_queue_passes_it_only_for_priority_high_and_the_runner_applies_it(self):
        queue = (ROOT / 'tools/game_queue.ps1').read_text(encoding='utf-8')
        runner = (ROOT / 'tools/ingame_run.ps1').read_text(encoding='utf-8')
        self.assertRegex(queue, r'if \(\$prio -eq 0\) \{ \$argv \+= @\("-GamePriority", "AboveNormal"\) \}')
        self.assertIn('[string]$GamePriority', runner)
        self.assertRegex(runner, r'\$p\.PriorityClass = \[Diagnostics\.ProcessPriorityClass\]\$GamePriority')
        self.assertTrue(re.search(r'if \(\$GamePriority -and \$p\)', runner))

    def test_a_normal_request_that_waited_fifteen_minutes_starts_alongside_parallel_ones(self):
        queue = (ROOT / 'tools/game_queue.ps1').read_text(encoding='utf-8')
        self.assertRegex(queue, r'\$StarvedMinutes = 15')
        self.assertRegex(queue, r'\$parallel = \(PeekParallel \$r\.FullName\) -or \(\$prio -ne 0 -and \(\(Get-Date\) - \$r\.LastWriteTime\)\.TotalMinutes -ge \$StarvedMinutes\)')


if __name__ == '__main__':
    unittest.main()
