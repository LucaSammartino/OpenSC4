"""Candidate shadow checks preserve the house profile and require the gated DLL."""
import importlib.util
import io
import json
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / ".claude/lane-bin"))
spec = importlib.util.spec_from_file_location("shadow_profile_train", REPO / "tools/merge-train.py")
mt = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mt)


class CandidateShadowTests(unittest.TestCase):
    def test_only_shadow_keys_are_accepted(self):
        self.assertIsNone(mt.candidate_shadows(None))
        self.assertEqual(mt.candidate_shadows(" Formats_Shadow, aura_shadow, formats_shadow "),
                         "formats_shadow,aura_shadow")
        for value in ("", ",", "enable", "aura_shadow; exit", "formats_shadow\nenable=terrain",
                      "occmgr_shadow_follow=0", "occmgr_shadow_follow\nenable=terrain", "aura_shadow_follow"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                mt.candidate_shadows(value)

    def test_followed_occupant_candidate_remains_gate_checked_and_audited(self):
        record, run = self.invoke(extra="occmgr_shadow,occmgr_shadow_follow,occmgr_shadow_follow")
        self.assertTrue(record["ok"])
        self.assertEqual(record["extra_shadows"], "occmgr_shadow,occmgr_shadow_follow")
        run.assert_called_once_with(Path("wt"), "trial", Path("gated.dll"), "aura,occmgr",
                                    "occmgr_shadow,occmgr_shadow_follow", shadow_split=1)

    def invoke(self, extra="formats_shadow", gate_error=""):
        state = {"current": {"id": "trial", "worktree": "wt", "regress": {"ok": True, "verdict": "PASS"}}}
        with patch.object(mt, "load_state", return_value=state), patch.object(mt, "save_state"), \
             patch.object(mt, "integration_head", return_value="head"), \
             patch.object(mt, "batch_touches_engine", return_value=True), \
             patch.object(mt, "gated_dll", return_value=(None, gate_error) if gate_error else (Path("gated.dll"), "")), \
             patch.object(mt, "replacement_union", return_value="aura,occmgr"), \
             patch.object(mt, "run_regress", return_value=("PASS", "")) as run:
            mt.cmd_regress(SimpleNamespace(no_regress=False, add_replace="occmgr", add_shadow=extra))
        return state["current"]["regress"], run

    def test_replacement_and_shadow_candidates_are_audited_together(self):
        record, run = self.invoke()
        self.assertTrue(record["ok"])
        self.assertEqual(record["replace"], "aura,occmgr")
        self.assertEqual(record["extra_shadows"], "formats_shadow")
        run.assert_called_once_with(Path("wt"), "trial", Path("gated.dll"), "aura,occmgr", "formats_shadow", shadow_split=1)

    def test_candidate_cannot_bypass_gate_or_retain_invalid_retry_pass(self):
        for kwargs in ({"gate_error": "wrong DLL hash"}, {"extra": "enable=terrain"}):
            with self.subTest(kwargs=kwargs):
                record, run = self.invoke(**kwargs)
                self.assertFalse(record["ok"])
                self.assertEqual(record["verdict"], "REFUSED")
                run.assert_not_called()

    def test_runner_passes_both_lists_as_separate_single_arguments(self):
        with tempfile.TemporaryDirectory() as td:
            result = subprocess.CompletedProcess([], 0, "REGRESS trial: PASS\n", "")
            with patch.object(mt, "LOG_DIR", Path(td)), patch.object(mt, "sh", return_value=result) as sh:
                verdict, _ = mt.run_regress(Path(td), "trial", Path("gated.dll"),
                                           "aura,occmgr", "aura_shadow,formats_shadow")
            argv = sh.call_args.args[0]
            self.assertEqual(verdict, "PASS")
            self.assertEqual(argv[argv.index("-Replace") + 1], "aura,occmgr")
            self.assertEqual(argv[argv.index("-AddShadows") + 1], "aura_shadow,formats_shadow")

    def test_cli_rejects_skip_and_shadow_candidate(self):
        with patch.object(sys, "argv", ["merge-train", "regress", "--no-regress", "--add-shadow", "formats_shadow"]), \
             redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as error:
            mt.main()
        self.assertEqual(error.exception.code, 2)

    @unittest.skipUnless(sys.platform == "win32", "executes the actual PowerShell profile builder")
    def test_powershell_preserves_defaults_deduplicates_and_rejects_ini_injection(self):
        # Load only the function AST, so this test cannot stage a DLL or start a game.
        source = str(REPO / "tools/regress.ps1").replace("'", "''")
        harness = r"""
$tokens = $null; $errors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile('SOURCE', [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw "regress.ps1 does not parse" }
$node = $ast.FindAll({ param($n) $n -is [Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'Add-RegressShadowKeys' }, $true)[0]
. ([scriptblock]::Create($node.Extent.Text))
$base = @('disable = budget', 'aura_shadow = 1', 'traffic_shadow_hypotheses = 7') -join [Environment]::NewLine
$profile = Add-RegressShadowKeys $base 'aura_shadow,formats_shadow,formats_shadow,occmgr_shadow,occmgr_shadow_follow,occmgr_shadow_follow'
$rejected = 0
foreach ($bad in @('enable', 'aura_shadow; exit', ('formats_shadow' + [Environment]::NewLine + 'enable = terrain'), 'occmgr_shadow_follow=0', 'aura_shadow_follow')) {
    try { $null = Add-RegressShadowKeys $base $bad } catch { $rejected++ }
}
@{ profile = $profile; rejected = $rejected } | ConvertTo-Json -Compress
""".replace("SOURCE", source)
        with tempfile.TemporaryDirectory() as td:
            p = Path(td) / "profile.ps1"
            p.write_text(harness, encoding="utf-8")
            result = subprocess.run(["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(p)],
                                    capture_output=True, text=True, check=True)
        actual = json.loads(result.stdout)
        self.assertEqual(actual["profile"].splitlines(), [
            "disable = budget", "aura_shadow = 1", "traffic_shadow_hypotheses = 7", "formats_shadow = 1",
            "occmgr_shadow = 1", "occmgr_shadow_follow = 1"])
        self.assertEqual(actual["rejected"], 5)
