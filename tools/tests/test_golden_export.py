"""A real PowerShell/Windows mutex test: export wins over immediate slot reuse."""
from pathlib import Path
import queue
import shutil
import subprocess
import tempfile
import threading
import unittest


@unittest.skipUnless(shutil.which('powershell'), 'Windows PowerShell mutex fixture')
class ExportTests(unittest.TestCase):
    def test_active_run_handoff_and_source_region_mirror(self):
        root = Path(__file__).resolve().parents[2]
        def command(script, *args):
            return ['powershell', '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
                    '-File', str(script), *map(str, args)]
        def line(process):
            output = queue.Queue()
            threading.Thread(target=lambda: output.put(process.stdout.readline()), daemon=True).start()
            return output.get(timeout=20).strip()
        with tempfile.TemporaryDirectory() as temporary:
            folder = Path(temporary)
            (folder / 'output').mkdir()
            fixture = subprocess.Popen(command(root / 'tools/tests/fixtures/golden-export-race.ps1',
                '-Root', folder), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
            snapshot = None
            try:
                self.assertEqual(line(fixture), 'fake run ready')
                snapshot = subprocess.Popen(command(root / 'tools/golden_snapshot.ps1',
                    '-HarnessRoot', folder, '-Output', folder / 'output', '-City', 'C', '-Region', 'Paradis',
                    '-Slot', '6', '-ExpectedLog', folder / 'expected.log', '-VerdictFile', folder / 'result.verdict',
                    '-ScriptPath', folder / 'actions.script'), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
                self.assertIn('watching active script', line(snapshot))
                (folder / 'release').write_text('release', encoding='utf-8')
                captured = snapshot.communicate(timeout=20)[0]
                reused = fixture.communicate(timeout=20)[0]
                self.assertEqual(snapshot.returncode, 0, captured)
                self.assertEqual(fixture.returncode, 0, reused)
                self.assertEqual((folder / 'output/end.sc4').read_text(), 'original end save')
                self.assertEqual((folder / 'build-runs/sc4slots/slot6/Regions/FC4 Test/City - C.sc4').read_text(), 'next run save')
            finally:
                for process in (snapshot, fixture):
                    if process is not None and process.poll() is None:
                        process.kill()
                        process.communicate(timeout=10)
