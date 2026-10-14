<#
In-game regression suite (coordinator tooling): runs the staged FreeCity4.dll through a fixed set of unattended
runs on the test city and prints one table. Use it after staging a new DLL.

Runs (each through tools\ingame_run.ps1 with popup_dialogs = 0 and autotest_end = quit, so the full Shutdown path
is part of every verdict):
  shadows  every same-process shadow at once, on the original subsystems: budget (needs disable = budget), demand,
           pollution, protection, crime, aura, flora, landfill. A shadow line is clean when every mismatch count on it is
           0.
  replace  every merged replacement at once (-Replace overrides the list).

Usage:
  powershell -File tools\regress.ps1 -Tag r1 [-Seconds 150] [-Only shadows|replace] [-Replace "landvalue, weather"]
           [-City <path to .sc4>]

Logs: build-runs\regress-<Tag>-<run>.log. A summary per run is printed by tools\run_summary.ps1; the table at the
end reads: verdict, problem count (foreign exceptions, stalls, reference warnings, panics, structured exceptions,
INCOMPLETE shadow lines), and per shadow "clean/lines".
#>
param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [int]$Seconds = 150,
    [ValidateSet("", "shadows", "replace")][string]$Only = "",
    [string]$Replace = "landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution, protection, crime, aura, flora, landfill",
    [string]$City = "C:\Users\tlori\Documents\SimCity 4\Regions\FC4 Test\City - Konradshohe.sc4"
)
$ErrorActionPreference = "Continue"
$Run = Join-Path $PSScriptRoot "ingame_run.ps1"
$Sum = Join-Path $PSScriptRoot "run_summary.ps1"
$Out = Join-Path $PSScriptRoot "..\build-runs"

$runs = [ordered]@{
    shadows = "disable = budget`nbudget_shadow = 1`ndemand_shadow = 1`npollution_shadow = 1`nprotection_shadow = 1`ncrime_shadow = 1`naura_shadow = 1`nflora_shadow = 1`nlandfill_shadow = 1"
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
