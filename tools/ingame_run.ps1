<#
Run SimCity 4 once with FreeCity4 and a given FreeCity4.ini, unattended, and report a verdict.

The game is started from this shell (normal integrity), so it can be stopped here. Starting it from the
Start-menu shortcut via the desktop app made it elevated and unkillable.

Usage:
  powershell -File tools\ingame_run.ps1 -Ini "budget_default = 0`nlot_scan = 1" [-Seconds 240] [-Label run8]
                                        [-Actions "20=ui cheetah; 40=ui pause; 60=save <city file>"] [-NoDismiss]
                                        [-KeepSaves]

Verdict sources, in order:
  1. "AUTOTEST VERDICT:" line written by the plugin (ini autotest_seconds), which also ends the process.
  2. Fallback: a foreign first-chance AV or a WATCHDOG stall in the log => corrupt/stalled; otherwise "clean" once
     -Seconds have passed after the city loaded.

Modal dialogs. Advisor popups (and the save confirmation) call HiddenPause() on our simulator and hold the city until
a button is clicked; Esc and Enter do not close them (runs 48-50). Unless -NoDismiss is given, the harness runs
`sc4ctl.ps1 clear` whenever a new HiddenPause shows up in the log after the load, or the date stands still across two
periodic heartbeats with OnTick counting and nothing delivered. `clear` only clicks dialogs it recognises, saves a PNG
of each to build-runs\<label>-popupN.png, and stops once the log shows hidden=0.

Actions. -Actions takes "seconds-after-load=sc4ctl command" pairs separated by ';'. Each runs once, in order, through
tools\sc4ctl.ps1 (see that file: ui <control>, click x y, key, realkey, save, snap, state, clear). Use
autotest_unpause = 0 in the ini when an action pauses the game, or the autotest will undo it.
#>
param(
    [Parameter(Mandatory = $true)][string]$Ini,
    [int]$Seconds = 240,
    [int]$AfterLoadSeconds = 75,
    [string]$Label = (Get-Date -Format "HHmmss"),
    [string]$Actions = "",
    [switch]$NoDismiss,
    # The autoload city is backed up before the run and restored afterwards if the run changed it (a save action,
    # or a save by hand). -KeepSaves keeps the changed file.
    [switch]$KeepSaves
)

$ErrorActionPreference = "Stop"
$GameDir = "C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition\Apps"
$Exe = Join-Path $GameDir "SimCity 4.exe"
$Plugins = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "SimCity 4\Plugins"
$Log = Join-Path $Plugins "FreeCity4.log"
$OutDir = Join-Path $PSScriptRoot "..\build-runs"
$Ctl = Join-Path $PSScriptRoot "sc4ctl.ps1"

# The game keeps the log open for writing, so it must be read with FileShare.ReadWrite (ReadAllText fails).
function ReadShared([string]$path) {
    try {
        $fs = [IO.File]::Open($path, 'Open', 'Read', 'ReadWrite,Delete')
        try { return (New-Object IO.StreamReader($fs)).ReadToEnd() } finally { $fs.Close() }
    } catch { return "" }
}
function Ctl([string]$cmdline) {
    # "save", "snap" and "clear" take one path, which may contain spaces; the others take whitespace-separated words.
    $head = $cmdline.Trim() -split '\s+', 2
    if ($head[0] -in "save", "snap", "clear") { $parts = @($head | Where-Object { $_ }) }
    else { $parts = @($cmdline.Trim() -split '\s+' | Where-Object { $_ }) }
    # PowerShell 5.1 turns a native command's stderr into terminating errors under "Stop"; a failed action must
    # not end the run.
    $old = $ErrorActionPreference; $ErrorActionPreference = "Continue"
    try { $out = & powershell -NoProfile -ExecutionPolicy Bypass -File $Ctl @parts 2>&1 } finally { $ErrorActionPreference = $old }
    return ($out | ForEach-Object { "$_" }) -join " | "
}
New-Item -ItemType Directory -Force $OutDir | Out-Null

$plan = @()
foreach ($a in ($Actions -split ';')) {
    if ($a.Trim() -match '^(\d+)\s*=\s*(.+)$') { $plan += , @([int]$Matches[1], $Matches[2].Trim()) }
    elseif ($a.Trim()) { Write-Output "BAD ACTION: '$a' (want seconds=command)"; exit 4 }
}
$nextAction = 0

# A previous run's game can still be exiting (a quit ending unloads for a few seconds): wait up to 60 s.
for ($i = 0; (Get-Process -Name "SimCity 4" -ErrorAction SilentlyContinue) -and $i -lt 30; $i++) { Start-Sleep -Seconds 2 }
if (Get-Process -Name "SimCity 4" -ErrorAction SilentlyContinue) {
    Write-Output "BLOCKED: SimCity 4 is already running (close it first)"
    exit 2
}

$auto = ""
if ($Ini -notmatch "autotest_seconds") { $auto += "autotest_seconds = $AfterLoadSeconds`n" }
if ($Ini -notmatch "autotest_unpause") { $auto += "autotest_unpause = 1`n" }
Set-Content -Path (Join-Path $Plugins "FreeCity4.ini") -Value ("# ingame_run $Label`n" + $auto + $Ini) -Encoding ascii
# The log can stay locked for a few seconds while a previous game process finishes exiting.
for ($i = 0; ; $i++) {
    try { Set-Content -Path $Log -Value "" -Encoding ascii -ErrorAction Stop; break }
    catch { if ($i -ge 20) { Write-Output "BLOCKED: $Log is locked: $($_.Exception.Message)"; exit 3 }; Start-Sleep -Seconds 3 }
}

$cityPath = if ($Ini -match "autoload_city\s*=\s*([^
]+)") { $Matches[1].Trim() } else { $null }
$cityBak = $null
if ($cityPath -and (Test-Path $cityPath) -and -not $KeepSaves) {
    $cityHash = (Get-FileHash $cityPath).Hash
    $cityBak = Join-Path $OutDir ("city-backups\{0}.{1}.bak" -f [IO.Path]::GetFileNameWithoutExtension($cityPath), $cityHash.Substring(0, 8))
    if (-not (Test-Path $cityBak)) { New-Item -ItemType Directory -Force (Split-Path $cityBak) | Out-Null; Copy-Item $cityPath $cityBak }
}

$p = Start-Process -FilePath $Exe -ArgumentList "-CPUCount:1", "-w" -WorkingDirectory $GameDir -PassThru
$start = Get-Date
$loadedAt = $null
$verdict = $null
$hpSeen = $null; $frozenDate = $null; $frozenCount = 0; $lastBeatCount = 0; $rounds = 0; $clicked = 0

while ((Get-Date) - $start -lt [TimeSpan]::FromSeconds($Seconds)) {
    Start-Sleep -Seconds 3
    $text = ReadShared $Log
    if ($text -match "AUTOTEST VERDICT: ([a-z]+)[^\r\n]*") { $verdict = $Matches[0]; break }
    if ($text -match "first-chance exception OUTSIDE FreeCity4\.dll[^\r\n]*") { $verdict = "corrupt (fallback): " + $Matches[0] }
    if ($text -match "WATCHDOG: main thread[^\r\n]*stalled[^\r\n]*") { $verdict = "stalled (fallback): " + $Matches[0]; Start-Sleep -Seconds 5; break }
    if (-not $loadedAt -and $text -match "config \(city load: budget Init\)") { $loadedAt = Get-Date }
    if (-not $loadedAt) { if ($p.HasExited) { break }; continue }

    # Scheduled actions.
    while ($nextAction -lt $plan.Count -and ((Get-Date) - $loadedAt).TotalSeconds -ge $plan[$nextAction][0]) {
        $cmd = $plan[$nextAction][1]
        $res = Ctl $cmd
        Write-Output ("  [{0}] action @{1}s '{2}': {3}" -f (Get-Date -Format "HH:mm:ss"), $plan[$nextAction][0], $cmd, $res)
        $nextAction++
        $text = ReadShared $Log
    }

    # Modal dialogs: a new HiddenPause after the load, or a frozen date with OnTick counting and nothing delivered.
    if (-not $NoDismiss) {
        $hp = [regex]::Matches($text, "HiddenPause\(\) -> 1 \(pause=\d+ hidden=([1-9]\d*)").Count
        if ($null -eq $hpSeen) { $hpSeen = $hp }
        $why = $null
        if ($hp -gt $hpSeen) { $hpSeen = $hp; $why = "HiddenPause" }
        $beats = [regex]::Matches($text, "heartbeat \(periodic\): (\d{4}-\d\d-\d\d)[^\r\n]*?\+(\d+) OnTick, \+(\d+) deliveries")
        if ($beats.Count -gt $lastBeatCount) {
            $lastBeatCount = $beats.Count
            $b = $beats[$beats.Count - 1]
            $stuck = ([int64]$b.Groups[2].Value -gt 0) -and ([int64]$b.Groups[3].Value -eq 0)
            if ($stuck -and $b.Groups[1].Value -eq $frozenDate) { $frozenCount++ } elseif ($stuck) { $frozenDate = $b.Groups[1].Value; $frozenCount = 1 } else { $frozenDate = $null; $frozenCount = 0 }
            if ($frozenCount -ge 2 -and -not $why) { $why = "date frozen at $frozenDate" }
        }
        if ($why) {
            Start-Sleep -Seconds 1   # let the dialog draw
            $rounds++
            $res = Ctl ("clear " + (Join-Path $OutDir ("{0}-popup{1}-" -f $Label, $rounds)))
            $clicked += ([regex]::Matches($res, "clear: clicked")).Count
            Write-Output ("  [{0}] {1}: {2}" -f (Get-Date -Format "HH:mm:ss"), $why, $res)
            $text = ReadShared $Log
            $hpSeen = [regex]::Matches($text, "HiddenPause\(\) -> 1 \(pause=\d+ hidden=([1-9]\d*)").Count
        }
    }

    if (((Get-Date) - $loadedAt).TotalSeconds -ge $AfterLoadSeconds) {
        if (-not $verdict) { $verdict = "clean (fallback): no foreign exception or stall $AfterLoadSeconds s after load" }
        break
    }
    if ($p.HasExited) { break }
}

# The loop can leave on "process exited" after a dialog round, with the verdict written since its last read
# (run 71: the city ended by the game, then the quit). Read the log once more.
if (-not $verdict -or $verdict -like "clean (fallback)*") {
    $text = ReadShared $Log
    if ($text -match "AUTOTEST VERDICT: ([a-z]+)[^\r\n]*") { $verdict = $Matches[0] }
}
if (-not $verdict) {
    if ($p.HasExited) { $verdict = "exited (code $($p.ExitCode)) without a verdict" }
    elseif (-not $loadedAt) { $verdict = "no city load within $Seconds s (region view? needs a click)" }
    else { $verdict = "timeout" }
}

if (-not $p.HasExited) {
    try { Stop-Process -Id $p.Id -Force -ErrorAction Stop } catch { $verdict += " | WARNING: could not stop the game: $($_.Exception.Message)" }
}
Start-Sleep -Seconds 2
Copy-Item $Log (Join-Path $OutDir "$Label.log") -Force
if ($cityBak -and (Get-FileHash $cityPath).Hash -ne $cityHash) {
    Copy-Item $cityBak $cityPath -Force
    $restored = if ((Get-FileHash $cityPath).Hash -eq $cityHash) { "restored" } else { "RESTORE FAILED" }
    Write-Output "  the city file changed during the run (a save): $restored from $cityBak"
}
Write-Output "[$Label] $verdict"
if ($rounds -gt 0) { Write-Output "  dialog rounds: $rounds, buttons clicked: $clicked (screenshots: build-runs\$Label-popup*.png)" }
if ($nextAction -lt $plan.Count) { Write-Output "  WARNING: $($plan.Count - $nextAction) action(s) never ran" }
$t = ReadShared $Log
foreach ($pat in "budget switches OFF[^\r\n]*", "heartbeat \(periodic\)[^\r\n]*") {
    $m = [regex]::Matches($t, $pat)
    if ($m.Count -gt 0) { Write-Output ("  " + $m[$m.Count - 1].Value) }
}
