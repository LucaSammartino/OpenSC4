"""Exercise the read-only native query without starting a game."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(os.name == "nt" and shutil.which("powershell"), "Windows process query")
class ProcessQueryTests(unittest.TestCase):
    def test_cim_denied_fallback_identifies_only_the_actual_userdir(self):
        source = (ROOT / "tools/ingame_run.ps1").read_text(encoding="utf-8")
        function = source[source.index("function GamesUsing("):source.index("function HasKey(")]
        with tempfile.TemporaryDirectory() as tmp:
            script = Path(tmp) / "query.ps1"
            script.write_text(
                "param([string]$UserDir)\n"
                "$ErrorActionPreference = 'Stop'\n"
                ". $env:FC4_QUERY_HELPER\n"
                "function Get-CimInstance { throw 'deliberate WMI denial' }\n"
                "function Get-Process { [pscustomobject]@{ Id = $PID } }\n"
                "$DocsUser = 'C:\\Documents-test'\n" + function + "\n"
                "@{ same = @(GamesUsing $UserDir); other = @(GamesUsing 'C:\\other-slot'); "
                "docs = @(GamesUsing $DocsUser); pid = $PID; "
                "native = [FC4ProcessQuery]::CommandLine($PID); "
                "missing = [FC4ProcessQuery]::CommandLine(2147483647) } | ConvertTo-Json -Compress\n",
                encoding="utf-8")
            env = dict(os.environ, FC4_QUERY_HELPER=str(ROOT / "tools/process_cmdline.ps1"))
            result = subprocess.run(["powershell", "-NoProfile", "-File", str(script), "-UserDir:C:\\query-slot"],
                                    env=env, capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            data = json.loads(result.stdout)
            self.assertEqual(data["same"], [data["pid"]])
            self.assertEqual(data["other"], [])
            self.assertEqual(data["docs"], [])
            self.assertIn("query-slot", data["native"])
            self.assertIsNone(data["missing"])


if __name__ == "__main__":
    unittest.main()
