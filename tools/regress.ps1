<#
In-game regression suite (coordinator tooling): runs the staged FreeCity4.dll through a fixed set of unattended
runs on the test city and prints one table. Use it after staging a new DLL.

Runs (each through tools\ingame_run.ps1 with popup_dialogs = 0 and autotest_end = quit, so the full Shutdown path
is part of every verdict):
  shadows  every same-process shadow at once, on the original subsystems: budget (needs disable = budget), demand,
           pollution, protection, crime, aura, flora, landfill, power, plumbing, land value, flammability, residential, neighbours, zones. A shadow line is clean when every mismatch count
           on it is 0.
  replace  every merged replacement at once (-Replace overrides the list).

Usage:
  powershell -File tools\regress.ps1 -Tag r1 [-Seconds 150] [-Only shadows|replace] [-Replace "landvalue, weather"]
           [-City <path to .sc4>] [-Dll <path to FreeCity4.dll>]

-Dll stages that DLL (and the .pdb next to it, if any) as Plugins\FreeCity4.dll first; it refuses while the game
runs. Every run prints the staged DLL's SHA-256 prefix, so the record names the build it tested.

Logs: build-runs\regress-<Tag>-<run>.log. A summary per run is printed by tools\run_summary.ps1; the table at the
end reads: verdict, problem count (foreign exceptions, stalls, reference warnings, panics, structured exceptions,
INCOMPLETE shadow lines), and per shadow "clean/lines".
#>
param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [int]$Seconds = 150,
    [ValidateSet("", "shadows", "replace")][string]$Only = "",
    [string]$Replace = "landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution, protection, crime, aura, flora, landfill, residential, neighbors, zones",
    [string]$City = "C:\Users\tlori\Documents\SimCity 4\Regions\FC4 Test\City - Konradshohe.sc4",
    [string]$Dll = ""
)
$ErrorActionPreference = "Continue"
$Plugins = Join-Path $env:USERPROFILE "Documents\SimCity 4\Plugins"
if ($Dll) {
    if (Get-Process -Name "SimCity 4" -ErrorAction SilentlyContinue) { Write-Output "REGRESS: the game is running; not staging $Dll"; exit 2 }
    if (-not (Test-Path -LiteralPath $Dll)) { Write-Output "REGRESS: no such DLL: $Dll"; exit 2 }
    Copy-Item -LiteralPath $Dll -Destination (Join-Path $Plugins "FreeCity4.dll") -Force
    $pdb = [IO.Path]::ChangeExtension($Dll, ".pdb")
    if (Test-Path -LiteralPath $pdb) { Copy-Item -LiteralPath $pdb -Destination (Join-Path $Plugins "FreeCity4.pdb") -Force }
}
$staged = Join-Path $Plugins "FreeCity4.dll"
$hash = if (Test-Path -LiteralPath $staged) { (Get-FileHash -Algorithm SHA256 -LiteralPath $staged).Hash.Substring(0, 8).ToLower() } else { "none" }
Write-Output "staged DLL: $staged (sha256 $hash)"
$Run = Join-Path $PSScriptRoot "ingame_run.ps1"
$Sum = Join-Path $PSScriptRoot "run_summary.ps1"
$Out = Join-Path $PSScriptRoot "..\build-runs"

$runs = [ordered]@{
    shadows = "disable = budget`nbudget_shadow = 1`ndemand_shadow = 1`npollution_shadow = 1`nprotection_shadow = 1`ncrime_shadow = 1`naura_shadow = 1`nflora_shadow = 1`nlandfill_shadow = 1`npower_shadow = 1`nplumbing_shadow = 1`nlandvalue_shadow = 1`nflammability_shadow = 1`nresidential_shadow = 1`nneighbours_shadow = 1`nzones_shadow = 1"
    replace = "enable = $Replace"
}
$rows = @()
foreach ($name in $runs.Keys) {
    if ($Only -and $Only -ne $name) { continue }
    $label = "regress-$Tag-$name"
    $ini = $runs[$name] + "`nautotest_unpause = 2`nautotest_end = quit`npopup_dialogs = 0`nautoload_city = $City"
    Write-Output "=== $label ($Seconds s after load) ==="
    $r = & powershell -ExecutionPolicy Bypass -File $Run -Ini $ini -AfterLoadSeconds $Seconds -Seconds ($Seconds + 150) -Label $label 2>&1
    $r | Select-Object -First 3 | ForEach-Object { Write-Output "  $_" }
    $log = Join-Path $Out "$label.log"
    $s = & powershell -ExecutionPolicy Bypass -File $Sum -Log $log -Width 160 2>&1
    $s | ForEach-Object { Write-Output "  $_" }

    $verdict = ($s | Select-String "^verdict:\s+AUTOTEST VERDICT: ([a-z]+)" | Select-Object -First 1)
    $v = if ($verdict) { $verdict.Matches[0].Groups[1].Value } else { "NONE" }
    $problems = 0
    $inProblems = $false
    foreach ($line in $s) {
        if ($line -match "^problems") { $inProblems = $true; continue }
        if ($inProblems -and $line -match "^\s+(.+?)\s{2,}(\d+)$") { if ($Matches[1] -ne "other WARNING lines") { $problems += [int]$Matches[2] } }
        elseif ($inProblems -and $line -notmatch "^\s") { $inProblems = $false }
    }
    $shadow = ($s | Select-String "^\s+([a-z]+)\s+(\d+) lines, (\d+) with every" | ForEach-Object {
            "{0} {1}/{2}" -f $_.Matches[0].Groups[1].Value, $_.Matches[0].Groups[3].Value, $_.Matches[0].Groups[2].Value }) -join ", "
    $months = ($s | Select-String "^\s+([a-z ()]+?)\s+(\d+) lines" | Where-Object { $_.Line -notmatch "with every" } | Measure-Object).Count
    $rows += [pscustomobject]@{ run = $name; verdict = $v; problems = $problems; monthGroups = $months; shadows = $shadow }
}
Write-Output ""
Write-Output "=== regress $Tag ==="
$rows | Format-Table -AutoSize | Out-String -Width 200 | Write-Output
$bad = @($rows | Where-Object { $_.verdict -ne "clean" -or $_.problems -ne 0 }).Count
Write-Output ("REGRESS {0}: {1}" -f $Tag, $(if ($bad -eq 0) { "PASS (verdicts clean, 0 problems; read the shadow column)" } else { "FAIL ($bad run(s))" }))
