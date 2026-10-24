"""Exercise real PS5 grouping/aggregation and the merge-train cadence."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / '.claude/lane-bin'))
spec = importlib.util.spec_from_file_location('split_train', REPO / 'tools/merge-train.py')
mt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mt)


class SplitTests(unittest.TestCase):
    def test_cadence(self):
        self.assertEqual([mt.regress_shadow_split(f'train{n}') for n in range(195, 201)],
                         [3, 1, 3, 3, 3, 1])
        self.assertEqual(mt.regress_shadow_split('train197', True), 1)

    @unittest.skipUnless(sys.platform == 'win32', 'PowerShell 5.1')
    def test_groups_preserve_keys_and_helpers_and_merge_worst_verdict(self):
        harness = r'''
param([string]$Repo, [string]$Out)
$ErrorActionPreference = 'Stop'
. (Join-Path $Repo 'tools/regress_shadows.ps1')
$tokens=$null; $errors=$null
$ast=[Management.Automation.Language.Parser]::ParseFile((Join-Path $Repo 'tools/regress.ps1'),[ref]$tokens,[ref]$errors)
if ($errors.Count) { throw "regress.ps1 parse errors: $errors" }
$node=$ast.FindAll({param($n) $n -is [Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Add-RegressShadowKeys'},$true)[0]
. ([scriptblock]::Create($node.Extent.Text))
$source=[IO.File]::ReadAllText((Join-Path $Repo 'tools/regress.ps1'))
$match=[regex]::Match($source,'(?m)^    shadows = "(.+)"\r?$')
if (-not $match.Success) { throw 'missing single house profile' }
$ini=$match.Groups[1].Value.Replace('`n',"`n")
$ini=Add-RegressShadowKeys $ini 'formats_shadow,occmgr_shadow_follow'
$groups=@(Split-RegressShadows $ini 3)
$logs=@{
 shadows1=@('2026-01-01 [pid 1] aura shadow month 2026-01-01: mismatches=0', 'AUTOTEST VERDICT: problems');
 shadows2=@('2026-01-01 [pid 2] aura shadow month 2026-01-01: mismatches=2', 'AUTOTEST VERDICT: clean');
 shadows3=@('2026-01-01 [pid 3] power shadow month 2026-01-01: mismatches=0', 'AUTOTEST VERDICT: clean')
}
foreach ($name in $logs.Keys) { [IO.File]::WriteAllLines((Join-Path $Out "regress-test-$name.log"),$logs[$name]) }
$v=Join-RegressShadowLogs $Out 'test' @('shadows1','shadows2','shadows3') @{}
$sum=@(& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $Repo 'tools/run_summary.ps1') -Log (Join-Path $Out 'regress-test-shadows.log'))
Remove-Item -LiteralPath (Join-Path $Out 'regress-test-shadows2.log')
$missing=Join-RegressShadowLogs $Out 'missing' @('shadows1','shadows2','shadows3') @{}
@{ groups=$groups; ini=$ini; verdict=$v; missing=$missing; summary=$sum } | ConvertTo-Json -Depth 5 -Compress
'''
        with tempfile.TemporaryDirectory(dir=REPO / 'build-runs') as td:
            script = Path(td) / 'test.ps1'
            script.write_text(harness, encoding='utf-8')
            result = subprocess.run(['powershell', '-NoProfile', '-ExecutionPolicy', 'Bypass',
                                     '-File', str(script), '-Repo', str(REPO), '-Out', td],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            actual = json.loads(result.stdout)
        original = actual['ini'].splitlines()
        self.assertTrue(all(g['Cost'] > 100 for g in actual['groups']), actual['groups'])
        self.assertLess(max(g['Cost'] for g in actual['groups']), 250)
        grouped = [line for group in actual['groups'] for line in group['Ini'].splitlines()]
        self.assertEqual(sorted(x for x in original if not x.startswith('disable')),
                         sorted(x for x in grouped if not x.startswith('disable')))
        for family in (('occmgr_shadow', 'occmgr_shadow_follow', 'propmgr_shadow', 'citydetail_shadow'),
                       ('automata_shadow', 'automata_owners_shadow'),
                       ('weather_shadow', 'scres_file_shadow', 'dbpf_shadow')):
            indices = {i for i, g in enumerate(actual['groups'])
                       for key in family if f'{key} = ' in g['Ini']}
            self.assertEqual(len(indices), 1, family)
        self.assertEqual(actual['verdict'], 'problems')
        self.assertEqual(actual['missing'], 'NONE')
        self.assertRegex('\n'.join(actual['summary']), r'aura\s+2 lines, 1 with every mismatch count 0')


if __name__ == '__main__':
    unittest.main()
