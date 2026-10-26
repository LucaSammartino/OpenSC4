"""Run the launcher's city-load decision with representative plugin logs."""
import json
import os
from pathlib import Path
import shutil
import subprocess

import pytest


@pytest.mark.skipif(os.name != 'nt' or not shutil.which('powershell'), reason='PowerShell required')
def test_loaded_city_does_not_require_the_budget_hook():
    source = (Path(__file__).resolve().parents[2] / 'tools/ingame_run.ps1').read_text()
    decision = next(line.strip() for line in source.splitlines()
                    if line.strip().startswith('if (-not $loadedAt -and $text -match'))
    script = ('$ErrorActionPreference="Stop"; $samples = @('
              '"config (city load: budget Init)", '
              '"autotest: clock started; verdict in 240 s", '
              '"config (startup)", "autoload: region loaded"); '
              '$answers = @(foreach ($text in $samples) { $loadedAt=$null; '
              + decision + '; [bool]$loadedAt }); ConvertTo-Json -Compress $answers')
    result = subprocess.run(['powershell', '-NoProfile', '-NonInteractive', '-Command', script],
                            capture_output=True, text=True, timeout=30)
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout) == [True, True, False, False]
