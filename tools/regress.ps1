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
           [-City <path to .sc4>] [-Dll <path to FreeCity4.dll>] [-Parallel] [-Turbo [-EndDate 2032-01-01]]
           [-Baseline <tag or .out path>]

-Dll stages that DLL (and the .pdb next to it, if any) as Plugins\FreeCity4.dll first; it refuses while the game
runs. Every run prints the staged DLL's SHA-256 prefix, so the record names the build it tested.
-Parallel runs the two runs at the same time in game slots 1 and 2 (ingame_run -Slot; each slot gets the DLL, -Dll
or the one staged in Documents; Documents is not touched). Same table and verdict line.
-Turbo: the turbo profile. Instead of -Seconds of real time after the load, each run goes to the game date -EndDate
(default 2032-01-01, six game years on Konradshohe) with our simulator's turbo clock (sim_turbo = 1; a plugin from
0786064 on starts it after the city's first ConnectionsReady, so popups are already off). -TurboFrom <date> starts it
with a script action at that game date instead (for older DLLs). -TurboTimeout (default 600 s) is the timeout.

Logs: build-runs\regress-<Tag>-<run>.log (ini: .ini). A summary per run is printed by tools\run_summary.ps1; the
table at the end reads: verdict, problem count (foreign exceptions, stalls, reference warnings, panics, structured
exceptions, INCOMPLETE shadow lines), and per shadow "clean/lines".

Delta: after the table, one line per run verdict, problem count and shadow against an earlier regress, read from
its build-runs\regress-<tag>.out (the coordinator's redirected output): -Baseline <tag> or a path; default the newest
earlier regress-*.out with a PASS line and the same profile (normal or turbo). "better" / "worse" compare the share
of lines with a mismatch. Any worse line adds "REGRESS <tag>: N worse (vs <baseline>)" after the verdict line.
#>
param(
    [Parameter(Mandatory = $true)][string]$Tag,
    [int]$Seconds = 150,
    [ValidateSet("", "shadows", "replace")][string]$Only = "",
    [string]$Replace = "landvalue, weather, ordinance, demand, power, plumbing, flammability, pollution, protection, crime, aura, flora, landfill, residential, neighbors, zones",
    [string]$City = "C:\Users\tlori\Documents\SimCity 4\Regions\FC4 Test\City - Konradshohe.sc4",
    [string]$Dll = "",
    [switch]$Parallel,
    [switch]$Turbo,
    [string]$EndDate = "2032-01-01",
    [string]$TurboFrom = "",
    [int]$TurboTimeout = 600,
    [string]$Baseline = ""
)
$ErrorActionPreference = "Continue"
$wallStart = Get-Date
$Plugins = Join-Path $env:USERPROFILE "Documents\SimCity 4\Plugins"
if ($Dll -and -not (Test-Path -LiteralPath $Dll)) { Write-Output "REGRESS: no such DLL: $Dll"; exit 2 }
if ($Dll -and -not $Parallel) {
    # Only a game on the Documents user dir blocks staging (slot games use their own Plugins).
    $docsGames = @(Get-CimInstance Win32_Process -Filter "Name='SimCity 4.exe'" -ErrorAction SilentlyContinue | Where-Object { "$($_.CommandLine)" -notmatch '-UserDir:' })
    if ($docsGames.Count -gt 0) { Write-Output "REGRESS: the game is running; not staging $Dll"; exit 2 }
    Copy-Item -LiteralPath $Dll -Destination (Join-Path $Plugins "FreeCity4.dll") -Force
    $pdb = [IO.Path]::ChangeExtension($Dll, ".pdb")
    if (Test-Path -LiteralPath $pdb) { Copy-Item -LiteralPath $pdb -Destination (Join-Path $Plugins "FreeCity4.pdb") -Force }
}
$staged = if ($Dll -and $Parallel) { (Resolve-Path -LiteralPath $Dll).Path } else { Join-Path $Plugins "FreeCity4.dll" }
$hash = if (Test-Path -LiteralPath $staged) { (Get-FileHash -Algorithm SHA256 -LiteralPath $staged).Hash.Substring(0, 8).ToLower() } else { "none" }
Write-Output "staged DLL: $staged (sha256 $hash)"
$runProfile = if ($Turbo) { "turbo to $EndDate$(if ($TurboFrom) { " from $TurboFrom" }), timeout $TurboTimeout s" } else { "normal, $Seconds s after load" }
Write-Output ("profile: {0}; {1}" -f $runProfile, $(if ($Parallel) { "parallel (slots 1 and 2)" } else { "serial" }))
$Run = Join-Path $PSScriptRoot "ingame_run.ps1"
$Sum = Join-Path $PSScriptRoot "run_summary.ps1"
$Out = Join-Path $PSScriptRoot "..\build-runs"
New-Item -ItemType Directory -Force $Out | Out-Null

$runs = [ordered]@{
    shadows = "disable = budget`nbudget_shadow = 1`ndemand_shadow = 1`npollution_shadow = 1`nprotection_shadow = 1`ncrime_shadow = 1`naura_shadow = 1`nflora_shadow = 1`nlandfill_shadow = 1`npower_shadow = 1`nplumbing_shadow = 1`nlandvalue_shadow = 1`nflammability_shadow = 1`nresidential_shadow = 1`nneighbours_shadow = 1`nzones_shadow = 1"
    replace = "enable = $Replace"
}
$after = if ($Turbo) { $TurboTimeout } else { $Seconds }

# Start the runs (all at once with -Parallel, else one after the other) and collect each one's output.
$results = [ordered]@{}
$jobs = [ordered]@{}
$slot = 0
foreach ($name in $runs.Keys) {
    if ($Only -and $Only -ne $name) { continue }
    $slot++
    $label = "regress-$Tag-$name"
    $ini = $runs[$name] + "`nautotest_unpause = 2`nautotest_end = quit`npopup_dialogs = 0`nautoload_city = $City"
    $iniPath = Join-Path $Out "$label.ini"
    Set-Content -Path $iniPath -Value $ini -Encoding ascii
    $a = @("-IniFile", $iniPath, "-AfterLoadSeconds", "$after", "-Seconds", "$($after + 150)", "-Label", $label)
    if ($Turbo) { $a += @("-EndDate", $EndDate); if ($TurboFrom) { $a += @("-TurboFrom", $TurboFrom) } else { $a += "-Turbo" } }
    if ($Parallel) {
        $a += @("-Slot", "$slot"); if ($Dll) { $a += @("-Dll", $staged) }
        $jobs[$name] = Start-Job -ScriptBlock { param($script, $argv) & powershell -NoProfile -ExecutionPolicy Bypass -File $script @argv 2>&1 | ForEach-Object { "$_" } } -ArgumentList $Run, $a
        Start-Sleep -Seconds 2
    } else {
        $results[$name] = @(& powershell -NoProfile -ExecutionPolicy Bypass -File $Run @a 2>&1 | ForEach-Object { "$_" })
    }
}
foreach ($name in $jobs.Keys) {
    $null = Wait-Job $jobs[$name]
    $results[$name] = @(Receive-Job $jobs[$name])
    Remove-Job $jobs[$name]
}
$wall = [int]((Get-Date) - $wallStart).TotalSeconds

$rows = @()
$cur = @{}
foreach ($name in $results.Keys) {
    $label = "regress-$Tag-$name"
    Write-Output "=== $label ($(if ($Turbo) { "to $EndDate, timeout $after s" } else { "$Seconds s after load" })) ==="
    $r = $results[$name]
    $r | Select-Object -First 3 | ForEach-Object { Write-Output "  $_" }
    $r | Select-Object -Skip 3 | Where-Object { $_ -match "^\s+(game:|sc4mutex:|HELD:|simulator: turbo month)" } | ForEach-Object { Write-Output "  $_" }
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
    $sh = [ordered]@{}
    foreach ($m in ($s | Select-String "^\s+([a-z]+)\s+(\d+) lines, (\d+) with every")) { $sh[$m.Matches[0].Groups[1].Value] = @([int]$m.Matches[0].Groups[3].Value, [int]$m.Matches[0].Groups[2].Value) }
    $shadow = ($sh.Keys | ForEach-Object { "{0} {1}/{2}" -f $_, $sh[$_][0], $sh[$_][1] }) -join ", "
    $months = ($s | Select-String "^\s+([a-z ()]+?)\s+(\d+) lines" | Where-Object { $_.Line -notmatch "with every" } | Measure-Object).Count
    $rows += [pscustomobject]@{ run = $name; verdict = $v; problems = $problems; monthGroups = $months; shadows = $shadow }
    $cur[$name] = @{ verdict = $v; problems = $problems; shadows = $sh }
}
Write-Output ""
Write-Output ("regress wall time: {0} s ({1}; {2})" -f $wall, $(if ($Parallel) { "parallel" } else { "serial" }), $runProfile)
Write-Output "=== regress $Tag ==="
$rows | Format-Table -AutoSize | Out-String -Width 200 | Write-Output

# Delta against an earlier regress (its redirected .out).
function ReadOut([string]$path) {
    $runsOut = @{}; $runName = $null; $inTable = $false
    foreach ($line in (Get-Content -LiteralPath $path)) {
        if ($line -match "^=== regress-.+-(shadows|replace) \(") { $runName = $Matches[1]; $runsOut[$runName] = @{ verdict = "NONE"; problems = $null; shadows = [ordered]@{} }; continue }
        if ($line -match "^=== regress \S+ ===") { $inTable = $true; $runName = $null; continue }
        if ($inTable -and $line -match "^(shadows|replace)\s+(\S+)\s+(\d+)\s+\d+") { if ($runsOut[$Matches[1]]) { $runsOut[$Matches[1]].problems = [int]$Matches[3] }; continue }
        if (-not $runName) { continue }
        if ($line -match "^\s+verdict:\s+AUTOTEST VERDICT: ([a-z]+)") { $runsOut[$runName].verdict = $Matches[1] }
        elseif ($line -match "^\s+([a-z]+)\s+(\d+) lines, (\d+) with every") { $runsOut[$runName].shadows[$Matches[1]] = @([int]$Matches[3], [int]$Matches[2]) }
    }
    return $runsOut
}
$basePath = $null
if ($Baseline) {
    $basePath = if (Test-Path -LiteralPath $Baseline) { $Baseline } else { Join-Path $Out "regress-$Baseline.out" }
    if (-not (Test-Path -LiteralPath $basePath)) { Write-Output "delta: no baseline $basePath"; $basePath = $null }
} else {
    $wantTurbo = [bool]$Turbo
    $cands = @(Get-ChildItem -Path (Join-Path $Out "regress-*.out") -ErrorAction SilentlyContinue | Where-Object { $_.Name -ne "regress-$Tag.out" } | Sort-Object LastWriteTime -Descending)
    foreach ($pass in 1, 2) {
        foreach ($c in $cands) {
            $txt = (Get-Content -LiteralPath $c.FullName) -join "`n"
            if ($txt -notmatch "(?m)^REGRESS \S+: PASS") { continue }
            if ($pass -eq 1 -and (($txt -match "(?m)^profile: turbo") -ne $wantTurbo)) { continue }
            $basePath = $c.FullName; break
        }
        if ($basePath) { break }
    }
    if (-not $basePath) { Write-Output "delta: no earlier regress-*.out with a PASS line in $Out" }
}
$worse = 0
if ($basePath) {
    $baseTag = [IO.Path]::GetFileNameWithoutExtension($basePath) -replace '^regress-', ''
    $base = ReadOut $basePath
    $pm = Get-Content -LiteralPath $basePath | Select-String "^profile: (.*)" | Select-Object -First 1
    $baseProfile = if ($pm) { $pm.Matches[0].Groups[1].Value } else { "" }
    Write-Output ("delta vs {0} ({1}; {2}):" -f $baseTag, $basePath, $(if ($baseProfile) { $baseProfile } else { "normal" }))
    function Rate($pair) { if ($pair[1] -le 0) { return 1.0 } else { return ($pair[1] - $pair[0]) / [double]$pair[1] } }
    foreach ($name in $cur.Keys) {
        $b = $base[$name]
        if (-not $b) { Write-Output "  ${name}: not in the baseline"; continue }
        $c = $cur[$name]
        $how = if ($c.verdict -eq $b.verdict) { "same" } elseif ($c.verdict -eq "clean") { "better" } else { "worse" }
        if ($how -eq "worse") { $worse++ }
        Write-Output "  ${name}: verdict $($b.verdict) -> $($c.verdict) ($how)"
        if ($null -ne $b.problems) {
            $how = if ($c.problems -eq $b.problems) { "same" } elseif ($c.problems -lt $b.problems) { "better" } else { "worse" }
            if ($how -eq "worse") { $worse++ }
            Write-Output "  ${name}: problems $($b.problems) -> $($c.problems) ($how)"
        }
        foreach ($k in $b.shadows.Keys) {
            $o = $b.shadows[$k]
            if (-not $c.shadows.Contains($k)) { $worse++; Write-Output ("  {0}: {1} {2}/{3} -> missing (worse)" -f $name, $k, $o[0], $o[1]); continue }
            $n = $c.shadows[$k]
            $ro = Rate $o; $rn = Rate $n
            $how = if ($rn -lt $ro) { "better" } elseif ($rn -gt $ro) { "worse" } else { "same" }
            if ($how -eq "worse") { $worse++ }
            Write-Output ("  {0}: {1} {2}/{3} -> {4}/{5} ({6})" -f $name, $k, $o[0], $o[1], $n[0], $n[1], $how)
        }
        foreach ($k in $c.shadows.Keys) { if (-not $b.shadows.Contains($k)) { $n = $c.shadows[$k]; Write-Output ("  {0}: {1} new {2}/{3}" -f $name, $k, $n[0], $n[1]) } }
    }
}
$bad = @($rows | Where-Object { $_.verdict -ne "clean" -or $_.problems -ne 0 }).Count
Write-Output ("REGRESS {0}: {1}" -f $Tag, $(if ($bad -eq 0) { "PASS (verdicts clean, 0 problems; read the shadow column)" } else { "FAIL ($bad run(s))" }))
if ($worse -gt 0) { Write-Output ("REGRESS {0}: {1} worse (vs {2})" -f $Tag, $worse, $baseTag) }
