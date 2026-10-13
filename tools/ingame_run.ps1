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
    [string]$Label = (Get-Date -Format "HHmmss"),
    # Advisor popups are modal: they freeze the date until their Close button is clicked (Esc and Enter do not
    # close them; checked on run 48). By default the harness clicks them away; -NoDismiss leaves them up.
    [switch]$NoDismiss
)

$ErrorActionPreference = "Stop"
$GameDir = "C:\Program Files (x86)\GOG Galaxy\Games\SimCity 4 Deluxe Edition\Apps"
$Exe = Join-Path $GameDir "SimCity 4.exe"
$Plugins = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "SimCity 4\Plugins"
$Log = Join-Path $Plugins "FreeCity4.log"
$OutDir = Join-Path $PSScriptRoot "..\build-runs"

# The game keeps the log open for writing, so it must be read with FileShare.ReadWrite (ReadAllText fails).
function ReadShared([string]$path) {
    try {
        $fs = [IO.File]::Open($path, 'Open', 'Read', 'ReadWrite,Delete')
        try { return (New-Object IO.StreamReader($fs)).ReadToEnd() } finally { $fs.Close() }
    } catch { return "" }
}
New-Item -ItemType Directory -Force $OutDir | Out-Null

# Advisor popup dismissal. The popup is centred in the 800x600 client area and its Close button sits at about
# client (526, 410) (measured on run 48). The signal is the date standing still across two periodic heartbeats while
# OnTick keeps counting and nothing is delivered; the plugin's pause flags stay 0, so they cannot be used.
Add-Type -ErrorAction SilentlyContinue @"
using System; using System.Runtime.InteropServices;
public static class FC4Win {
    [StructLayout(LayoutKind.Sequential)] public struct PT { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct RC { public int L, T, R, B; }
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref PT p);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RC r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
[FC4Win]::SetProcessDPIAware() | Out-Null
$CloseX = 526; $CloseY = 410
function SnapWindow([IntPtr]$h, [string]$path) {
    try {
        Add-Type -AssemblyName System.Drawing
        $o = New-Object FC4Win+PT; [FC4Win]::ClientToScreen($h, [ref]$o) | Out-Null
        $r = New-Object FC4Win+RC; [FC4Win]::GetClientRect($h, [ref]$r) | Out-Null
        $bmp = New-Object Drawing.Bitmap $r.R, $r.B
        $g = [Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($o.X, $o.Y, 0, 0, $bmp.Size)
        $bmp.Save($path, [Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
    } catch { }
}
function PostClick([IntPtr]$h, [int]$x, [int]$y) {
    $lp = [IntPtr](($y -shl 16) -bor ($x -band 0xFFFF))
    [FC4Win]::PostMessage($h, 0x0200, [IntPtr]0, $lp) | Out-Null   # WM_MOUSEMOVE
    Start-Sleep -Milliseconds 100
    [FC4Win]::PostMessage($h, 0x0201, [IntPtr]1, $lp) | Out-Null   # WM_LBUTTONDOWN, MK_LBUTTON
    Start-Sleep -Milliseconds 100
    [FC4Win]::PostMessage($h, 0x0202, [IntPtr]0, $lp) | Out-Null   # WM_LBUTTONUP
}
function RealClick([IntPtr]$h, [int]$x, [int]$y) {
    $o = New-Object FC4Win+PT; $o.X = $x; $o.Y = $y; [FC4Win]::ClientToScreen($h, [ref]$o) | Out-Null
    [FC4Win]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 300
    [FC4Win]::SetCursorPos($o.X, $o.Y) | Out-Null; Start-Sleep -Milliseconds 100
    [FC4Win]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 80   # LEFTDOWN
    [FC4Win]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)                                 # LEFTUP
}
$frozenDate = $null; $frozenCount = 0; $lastBeatCount = 0; $dismissals = 0; $postFailed = 0

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

$p = Start-Process -FilePath $Exe -ArgumentList "-CPUCount:1", "-w" -WorkingDirectory $GameDir -PassThru
$start = Get-Date
$loadedAt = $null
$verdict = $null

while ((Get-Date) - $start -lt [TimeSpan]::FromSeconds($Seconds)) {
    Start-Sleep -Seconds 3
    $text = ""
    $text = ReadShared $Log
    if ($text -match "AUTOTEST VERDICT: ([a-z]+)[^\r\n]*") { $verdict = $Matches[0]; break }
    if ($text -match "first-chance exception OUTSIDE FreeCity4\.dll[^\r\n]*") { $verdict = "corrupt (fallback): " + $Matches[0] }
    if ($text -match "WATCHDOG: main thread[^\r\n]*stalled[^\r\n]*") { $verdict = "stalled (fallback): " + $Matches[0]; Start-Sleep -Seconds 5; break }
    if (-not $loadedAt -and $text -match "config \(city load: budget Init\)") { $loadedAt = Get-Date }
    if (-not $NoDismiss -and $loadedAt) {
        $beats = [regex]::Matches($text, "heartbeat \(periodic\): (\d{4}-\d\d-\d\d)[^\r\n]*?\+(\d+) OnTick, \+(\d+) deliveries")
        if ($beats.Count -gt $lastBeatCount) {
            $lastBeatCount = $beats.Count
            $b = $beats[$beats.Count - 1]
            $date = $b.Groups[1].Value
            $stuck = ([int64]$b.Groups[2].Value -gt 0) -and ([int64]$b.Groups[3].Value -eq 0)
            if ($stuck -and $date -eq $frozenDate) { $frozenCount++ } elseif ($stuck) { $frozenDate = $date; $frozenCount = 1 } else { $frozenDate = $null; $frozenCount = 0 }
            if ($frozenCount -ge 2) {
                $p.Refresh(); $h = $p.MainWindowHandle
                if ($h -ne [IntPtr]::Zero) {
                    if ($frozenCount -ge 3) { $postFailed++ }   # still frozen after a posted round
                    $dismissals++
                    SnapWindow $h (Join-Path $OutDir ("{0}-popup{1}.png" -f $Label, $dismissals))
                    # Posted clicks first (no focus or cursor needed); real input once posted clicks have failed.
                    for ($k = 0; $k -lt 3; $k++) {
                        if ($postFailed -ge 1) { RealClick $h $CloseX $CloseY } else { PostClick $h $CloseX $CloseY }
                        Start-Sleep -Seconds 2
                    }
                    $how = if ($postFailed -ge 1) { "real input" } else { "posted" }
                    Write-Output ("  [{0}] date frozen at {1} with OnTick counting: advisor popup suspected, clicked Close ({2}), dismissal #{3}" -f (Get-Date -Format "HH:mm:ss"), $date, $how, $dismissals)
                }
            }
        }
    }
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
if ($dismissals -gt 0) { Write-Output "  advisor popup dismissal rounds: $dismissals (screenshots: build-runs\$Label-popup*.png)" }
$t = ReadShared $Log
foreach ($pat in "budget switches OFF[^\r\n]*", "heartbeat \(periodic\)[^\r\n]*") {
    $m = [regex]::Matches($t, $pat)
    if ($m.Count -gt 0) { Write-Output ("  " + $m[$m.Count - 1].Value) }
}
