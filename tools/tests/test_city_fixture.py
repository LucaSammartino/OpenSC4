"""Run the actual PowerShell fixture helpers without starting a game or touching Documents."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]
POWERSHELL = shutil.which("powershell")


@unittest.skipUnless(os.name == "nt" and POWERSHELL, "Windows PowerShell fixture staging")
class CityFixtureTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name).resolve()
        self.fixtures = self.root / "fixtures"
        self.fixtures.mkdir()
        self.source = self.fixtures / "tutorial city.sc4"
        self.data = b"DBPF" + bytes(range(128))
        self.source.write_bytes(self.data)
        self.docs = self.root / "Documents" / "SimCity 4"
        self.docs.mkdir(parents=True)
        (self.docs / "sentinel").write_bytes(b"unchanged")
        self.slot = self.root / "slots" / "slot1"

    def run_ps(self, code, *, ini=None, slot=1, user_dir=None, success=True):
        env = os.environ.copy()
        env.update(FC4_FIXTURE_HELPER=str(REPO / "tools/city_fixture.ps1"),
                   FC4_STATE_MAIN=str(self.root),
                   FC4_FIXTURE_ROOT=str(self.fixtures), FC4_FIXTURE_SOURCE=str(self.source),
                   FC4_FIXTURE_DOCS=str(self.docs), FC4_FIXTURE_SLOT=str(user_dir or self.slot),
                   FC4_FIXTURE_INI=ini if ini is not None else f"fixture_city = {self.source}\nenable=tutorial\n",
                   FC4_FIXTURE_SLOT_NUMBER=str(slot))
        p = subprocess.run([POWERSHELL, "-NoProfile", "-NonInteractive", "-Command",
                            "$ErrorActionPreference='Stop'; . $env:FC4_FIXTURE_HELPER; " + code],
                           env=env, capture_output=True, text=True, timeout=60)
        if success:
            self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        else:
            self.assertNotEqual(p.returncode, 0, p.stdout)
        self.assertEqual((self.docs / "sentinel").read_bytes(), b"unchanged")
        self.assertEqual(self.source.read_bytes(), self.data)
        return p

    RESOLVE = ("$f=Resolve-HarnessCityFixture $env:FC4_FIXTURE_INI "
               "([int]$env:FC4_FIXTURE_SLOT_NUMBER) 'Konradshohe' $env:FC4_FIXTURE_ROOT; ")

    def test_legacy_ini_unchanged_without_fixture(self):
        original = "enable=weather\n[debug]\nflag=1\n"
        p = self.run_ps(self.RESOLVE + "$f | ConvertTo-Json -Compress", ini=original, slot=0)
        self.assertEqual(json.loads(p.stdout), {"Ini": original, "Source": ""})

    def test_suite_state_input_and_export_are_confined_and_harness_only(self):
        states = self.root / 'build-runs/suite/states/warm'
        states.mkdir(parents=True)
        source = states / 'city.sc4'
        source.write_bytes(self.data)
        output = states / 'export.sc4'
        resolve = '$s=Resolve-HarnessSuiteState $env:FC4_FIXTURE_INI ([int]$env:FC4_FIXTURE_SLOT_NUMBER) $env:FC4_STATE_MAIN; '
        ini = f'prepared_city = {source}\nexport_city = {output}\nenable=weather\n'
        result = json.loads(self.run_ps(resolve + '$s | ConvertTo-Json -Compress', ini=ini).stdout)
        self.assertNotIn('prepared_city', result['Ini'])
        self.assertNotIn('export_city', result['Ini'])
        self.assertIn('enable=weather', result['Ini'])
        self.assertEqual(Path(result['Source']), source)
        for bad in (f'prepared_city={self.source}', f'export_city={self.docs / "bad.sc4"}',
                    f'prepared_city={states / "missing.sc4"}', f'export_city={source}',
                    ini + f'prepared_city={source}\n'):
            self.run_ps(resolve, ini=bad, success=False)
        self.run_ps(resolve, ini=ini, slot=0, success=False)
        self.assertFalse(output.exists())

    def test_suite_export_requires_changed_save_and_preserves_loaded_input(self):
        states = self.root / 'build-runs/suite/states/export'
        states.mkdir(parents=True)
        output = states / 'city.sc4'
        source = states / 'slot-copy.sc4'
        source.write_bytes(self.data + b'saved')
        ini = f'prepared_city={source}\nexport_city={output}\n'
        resolve = '$s=Resolve-HarnessSuiteState $env:FC4_FIXTURE_INI 1 $env:FC4_STATE_MAIN; '
        self.run_ps(resolve + 'Export-HarnessSuiteState $s.Source $s.Export ((Get-FileHash $s.Source).Hash)', ini=ini, success=False)
        self.assertFalse(output.exists())
        p = self.run_ps(resolve + "Export-HarnessSuiteState $s.Source $s.Export 'original-hash'", ini=ini)
        self.assertEqual(output.read_bytes(), source.read_bytes())
        self.assertEqual(p.stdout.strip(), hashlib.sha256(source.read_bytes()).hexdigest())

    def test_real_copy_preserves_source_documents_and_strips_harness_key(self):
        self.source.chmod(0o444)
        self.addCleanup(self.source.chmod, 0o666)
        p = self.run_ps(self.RESOLVE +
                        "$c=Copy-HarnessCityFixture $f.Source $env:FC4_FIXTURE_SLOT $env:FC4_FIXTURE_DOCS 'Konradshohe'; "
                        "@{ini=$f.Ini; copy=$c; readonly=(Get-Item -LiteralPath $c.Destination).IsReadOnly; "
                        "source_readonly=(Get-Item -LiteralPath $f.Source).IsReadOnly} | ConvertTo-Json -Compress")
        r = json.loads(p.stdout)
        self.assertNotIn("fixture_city", r["ini"])
        self.assertIn("enable=tutorial", r["ini"])
        self.assertFalse(r["readonly"])
        self.assertTrue(r["source_readonly"])
        destination = self.slot / "Regions/FC4 Test/City - Konradshohe.sc4"
        self.assertEqual(destination.read_bytes(), self.data)
        self.assertEqual(r["copy"]["SHA256"], hashlib.sha256(self.data).hexdigest())

    def test_non_slot_fixture_rejected_before_staging(self):
        self.run_ps(self.RESOLVE, slot=0, success=False)
        self.assertFalse(self.slot.exists())

    def test_bad_fixture_declarations_fail_closed(self):
        for ini in ["fixture_city=\n", "fixture_city=relative.sc4\n",
                    f"fixture_city={self.source}\nfixture_city={self.source}\n",
                    f"fixture_city={self.root / 'outside.sc4'}\n",
                    f"fixture_city={self.fixtures / 'missing.sc4'}\n"]:
            with self.subTest(ini=ini):
                self.run_ps(self.RESOLVE, ini=ini, success=False)
        self.assertFalse(self.slot.exists())

    def test_wrong_dbpf_magic_rejected(self):
        self.data = b"not a save"
        self.source.write_bytes(self.data)
        self.run_ps(self.RESOLVE, success=False)
        self.assertFalse(self.slot.exists())

    def test_documents_destination_rejected(self):
        self.run_ps(self.RESOLVE +
                    "Copy-HarnessCityFixture $f.Source $env:FC4_FIXTURE_SLOT $env:FC4_FIXTURE_DOCS 'Konradshohe'",
                    user_dir=self.docs, success=False)
        self.assertFalse((self.docs / "Regions").exists())

    def test_destination_junction_rejected(self):
        self.slot.parent.mkdir(parents=True)
        self.run_ps("New-Item -ItemType Junction -Path $env:FC4_FIXTURE_SLOT -Target $env:FC4_FIXTURE_DOCS | Out-Null; " +
                    self.RESOLVE + "Copy-HarnessCityFixture $f.Source $env:FC4_FIXTURE_SLOT $env:FC4_FIXTURE_DOCS 'Konradshohe'",
                    success=False)
        self.assertFalse((self.docs / "Regions").exists())

    def test_existing_hardlink_is_unlinked_without_writing_original(self):
        original = self.docs / "original.sc4"
        original.write_bytes(b"original city must remain unchanged")
        destination = self.slot / "Regions/FC4 Test/City - Konradshohe.sc4"
        destination.parent.mkdir(parents=True)
        os.link(original, destination)
        self.run_ps(self.RESOLVE +
                    "Copy-HarnessCityFixture $f.Source $env:FC4_FIXTURE_SLOT $env:FC4_FIXTURE_DOCS 'Konradshohe' | Out-Null")
        self.assertEqual(original.read_bytes(), b"original city must remain unchanged")
        self.assertEqual(destination.read_bytes(), self.data)

    def test_runner_parses_and_validates_fixture_before_region_mirror(self):
        runner = REPO / "tools/ingame_run.ps1"
        env_path = str(runner).replace("'", "''")
        self.run_ps("$tokens=$null; $errors=$null; "
                    f"[System.Management.Automation.Language.Parser]::ParseFile('{env_path}', [ref]$tokens, [ref]$errors) | Out-Null; "
                    "if ($errors.Count) { throw ($errors | Out-String) }")
        source = runner.read_text(encoding="utf-8")
        self.assertLess(source.index("$null = Resolve-HarnessFixtureDestination"), source.index("& robocopy"))
        self.assertGreater(source.index("$fixtureCopy = Copy-HarnessCityFixture"), source.index("& robocopy"))
        self.assertLess(source.index("$Ini = $cityFixture.Ini"), source.index("$text = $Ini"))


if __name__ == "__main__":
    unittest.main()
