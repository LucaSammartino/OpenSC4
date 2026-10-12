<#
Run SimCity 4 once with FreeCity4 and a given FreeCity4.ini, unattended, and report a verdict.

The game is started from this shell (normal integrity), so it can be stopped here. Starting it from the
Start-menu shortcut via the desktop app made it elevated and unkillable.

Usage:
  powershell -File tools\ingame_run.ps1 -Ini "budget_default = 0`nlot_scan = 1" [-Seconds 240] [-Label run8]

Verdict sources, in order:
  1. "AUTOTEST VERDICT:" line written by the plugin (ini autotest_seconds), which also ends the process.
  2. Fallback: a foreign first-chance AV or a WATCHDOG stall in the log => corrupt/stalled; otherwise "clean" once
     -Seconds have passed after the city loaded.
#>
param(
    [Parameter(Mandatory = $true)][string]$Ini,
    [int]$Seconds = 240,
    [int]$AfterLoadSeconds = 75,
    [string]$Label = (Get-Date -Format "HHmmss")
)

$ErrorActionPreference = "Stop"
$GameDir = "C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition\Apps"
$Exe = Join-Path $GameDir "SimCity 4.exe"
$Plugins = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "SimCity 4\Plugins"
$Log = Join-Path $Plugins "FreeCity4.log"
$OutDir = Join-Path $PSScriptRoot "..\build-runs"
New-Item -ItemType Directory -Force $OutDir | Out-Null

if (Get-Process -Name "SimCity 4" -ErrorAction SilentlyContinue) {
    Write-Output "BLOCKED: SimCity 4 is already running (close it first)"
    exit 2
}

Set-Content -Path (Join-Path $Plugins "FreeCity4.ini") -Value ("# ingame_run $Label`n" + $Ini) -Encoding ascii
Set-Content -Path $Log -Value "" -Encoding ascii

$p = Start-Process -FilePath $Exe -ArgumentList "-CPUCount:1", "-w" -WorkingDirectory $GameDir -PassThru
$start = Get-Date
$loadedAt = $null
$verdict = $null

while ((Get-Date) - $start -lt [TimeSpan]::FromSeconds($Seconds)) {
    Start-Sleep -Seconds 3
    $text = ""
    try { $text = [IO.File]::ReadAllText($Log) } catch { }
    if ($text -match "AUTOTEST VERDICT: ([a-z]+)[^\r\n]*") { $verdict = $Matches[0]; break }
    if ($text -match "first-chance exception OUTSIDE FreeCity4\.dll[^\r\n]*") { $verdict = "corrupt (fallback): " + $Matches[0] }
    if ($text -match "WATCHDOG: main thread[^\r\n]*stalled[^\r\n]*") { $verdict = "stalled (fallback): " + $Matches[0]; Start-Sleep -Seconds 5; break }
    if (-not $loadedAt -and $text -match "config \(city load: budget Init\)") { $loadedAt = Get-Date }
    if ($loadedAt -and ((Get-Date) - $loadedAt).TotalSeconds -ge $AfterLoadSeconds) {
        if (-not $verdict) { $verdict = "clean (fallback): no foreign exception or stall $AfterLoadSeconds s after load" }
        break
    }
    if ($p.HasExited) { break }
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
Write-Output "[$Label] $verdict"
$t = [IO.File]::ReadAllText($Log)
foreach ($pat in "budget switches OFF[^\r\n]*", "heartbeat \(periodic\)[^\r\n]*") {
    $m = [regex]::Matches($t, $pat)
    if ($m.Count -gt 0) { Write-Output ("  " + $m[$m.Count - 1].Value) }
}
