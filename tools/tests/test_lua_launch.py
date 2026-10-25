"""Exercise the actual optional launch block, without starting a game."""
import json
import os
from pathlib import Path
import shutil
import subprocess

import pytest


REPO = Path(__file__).resolve().parents[2]
PS = shutil.which("powershell")
pytestmark = pytest.mark.skipif(os.name != "nt" or not PS, reason="PowerShell 5.1 launch staging")
LAUNCH = (REPO / "tools/ingame_run.ps1").read_text(encoding="utf-8")
BLOCK = LAUNCH.split("        # Optional Maxis command script;", 1)[1].split("        $docLog =", 1)[0]


def stage(tmp_path, ini, slot=11):
    repo = tmp_path / "repo with spaces"
    (repo / "tools").mkdir(parents=True)
    source = repo / "scripts" / "example's test.lua"
    source.parent.mkdir()
    source.write_text("sc4app.GetAppState()\n", encoding="utf-8")
    helper = repo / "engine/tests/scenarios/lua/fc4.lua"
    helper.parent.mkdir(parents=True)
    helper.write_text("fc4 = {}\n", encoding="utf-8")
    extensions = repo / "build-runs/thirdparty/sc4-lua-extensions"
    extensions.mkdir(parents=True)
    for name in ("SC4LuaExtensions.dll", "SC4LuaExtensions.dat"):
        (extensions / name).write_bytes(name.encode())
    user = tmp_path / "slot with spaces"
    env = dict(os.environ, FC4_LUA_TEST_ROOT=str(repo / "tools"), FC4_LUA_TEST_SLOT=str(user),
               FC4_LUA_TEST_INI=ini, FC4_LUA_TEST_NUMBER=str(slot))
    code = ("$ErrorActionPreference='Stop'; $PSScriptRoot=$env:FC4_LUA_TEST_ROOT; "
            "$UserDir=$env:FC4_LUA_TEST_SLOT; $Ini=$env:FC4_LUA_TEST_INI; "
            "$Slot=[int]$env:FC4_LUA_TEST_NUMBER; $gameArgs=@('-CPUCount:1'); "
            "$Plugins=Join-Path $UserDir 'Plugins'; "
            "function MainCheckout { Split-Path $env:FC4_LUA_TEST_ROOT }; "
            "if ($Ini -match 'lua_extensions.*1') { $null=New-Item -ItemType Directory -Force $Plugins }; "
            "# Optional Maxis command script;" + BLOCK +
            "ConvertTo-Json -Compress -InputObject @($gameArgs)")
    result = subprocess.run([PS, "-NoProfile", "-NonInteractive", "-Command", code], env=env,
                            capture_output=True, text=True, timeout=20)
    return result, source, user


def test_off_by_default(tmp_path):
    result, _, user = stage(tmp_path, "disable = budget\n", slot=0)
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout) == ["-CPUCount:1"]
    assert not user.exists()


def test_stages_private_script_and_library_with_quoted_absolute_argument(tmp_path):
    result, source, user = stage(tmp_path, "lua_script = scripts/example's test.lua\n")
    assert result.returncode == 0, result.stderr
    args = json.loads(result.stdout.splitlines()[-1])
    copy = user / "LuaScripts/scripts/example's test.lua"
    assert args[-1] == f'-LuaScript:"{copy}"'
    assert copy.read_text(encoding="utf-8").endswith(source.read_text(encoding="utf-8"))
    assert copy.read_bytes().startswith(b"FC4_ROOT = '")
    assert not copy.read_bytes().startswith(b"\xef\xbb\xbf")
    assert (user / "LuaScripts/engine/tests/scenarios/lua/fc4.lua").read_text() == "fc4 = {}\n"
    assert source.read_text() == "sc4app.GetAppState()\n"


@pytest.mark.parametrize("path", ["../outside.lua", "C:/outside.lua", "scripts/missing.lua", "tools/test.ps1"])
def test_rejects_paths_outside_repo_or_not_lua(tmp_path, path):
    result, _, user = stage(tmp_path, "lua_script = " + path)
    assert result.returncode != 0
    assert "existing repo-relative .lua file" in result.stderr
    assert not user.exists()


def test_rejects_documents_run(tmp_path):
    result, _, user = stage(tmp_path, "lua_script = scripts/example's test.lua", slot=0)
    assert result.returncode != 0
    assert "requires -Slot" in result.stderr
    assert not user.exists()


def test_extensions_are_opt_in_and_staged_only_in_slot(tmp_path):
    result, _, user = stage(tmp_path, "lua_extensions = 1\n")
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout.splitlines()[-1]) == ["-CPUCount:1"]
    for name in ("SC4LuaExtensions.dll", "SC4LuaExtensions.dat"):
        assert (user / "Plugins" / name).read_bytes() == name.encode()


def test_extensions_refuse_documents(tmp_path):
    result, _, _ = stage(tmp_path, "lua_extensions = 1\n", slot=0)
    assert result.returncode != 0
    assert "requires -Slot" in result.stderr


def test_harvests_extension_print_and_game_lua_errors(tmp_path):
    user = tmp_path / "private slot"
    plugins = user / "Plugins"
    plugins.mkdir(parents=True)
    (plugins / "SC4LuaExtensions.log").write_text("FC4_LUA entry\nLua error: bad command\n")
    (plugins / "FreeCity4.log").write_text("unrelated heartbeat\nLua error from game\n")
    env = dict(os.environ, FC4_TEST_SLOT=str(user), FC4_TEST_MAIN=str(tmp_path))
    block = LAUNCH.split("# Optional Lua evidence;", 1)[1].split("# End optional Lua evidence.", 1)[0]
    code = ("$ErrorActionPreference='Stop'; $luaExtensions=$true; "
            "$UserDir=$env:FC4_TEST_SLOT; $OutDir=Join-Path $env:FC4_TEST_MAIN 'out'; "
            "$null=New-Item -ItemType Directory -Force $OutDir; $Label='probe'; "
            "function MainCheckout { $env:FC4_TEST_MAIN }; "
            "function ReadShared($path) { [IO.File]::ReadAllText($path) }; "
            "# Optional Lua evidence;" + block)
    result = subprocess.run([PS, "-NoProfile", "-NonInteractive", "-Command", code], env=env,
                            capture_output=True, text=True, timeout=20)
    assert result.returncode == 0, result.stderr
    evidence = (tmp_path / "build-runs/game-results/probe.lua.log").read_text()
    assert "FC4_LUA entry" in evidence
    assert "Lua error: bad command" in evidence
    assert "Lua error from game" in evidence
    assert "unrelated heartbeat" not in evidence
    assert (tmp_path / "out/probe.lua.log").read_text() == evidence
