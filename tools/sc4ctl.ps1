<#
Drive a running SimCity 4 (started by tools\ingame_run.ps1) without screen control: post clicks and keys to its window,
save a PNG of the client area, and read the simulator state back from FreeCity4.log.

Usage:
  powershell -File tools\sc4ctl.ps1 state                    # last heartbeat: date, speed, pause, hidden, emergency
  powershell -File tools\sc4ctl.ps1 snap  build-runs\x.png   # PNG of the 800x600 client area
  powershell -File tools\sc4ctl.ps1 click 526 410            # posted left click at client (x, y)
  powershell -File tools\sc4ctl.ps1 rclick 400 300           # posted right click
  powershell -File tools\sc4ctl.ps1 drag 300 300 "360,340"    # posted left-button drag from (x1, y1) to (x2, y2), e.g. a zone tool drag
  powershell -File tools\sc4ctl.ps1 key  S ctrl              # posted key (WM_KEYDOWN/UP), optional modifiers ctrl,shift,alt
  powershell -File tools\sc4ctl.ps1 realkey S ctrl           # real input (SendInput); brings the window to the front
  powershell -File tools\sc4ctl.ps1 realclick 526 410        # real input click; brings the window to the front
  powershell -File tools\sc4ctl.ps1 realdrag 300 300 "360,340" # real input drag (the 3D view ignores posted input)
  powershell -File tools\sc4ctl.ps1 ui <name>                # click a named control (see $Ui below)
  powershell -File tools\sc4ctl.ps1 clear [pngPrefix]        # click advisor popups away until hidden=0
  powershell -File tools\sc4ctl.ps1 findclose                # which known dialog is up, and where its button is
  powershell -File tools\sc4ctl.ps1 save [cityFile]          # real Ctrl+S, then close the "City Saved" box
  powershell -File tools\sc4ctl.ps1 close                    # post WM_CLOSE to the game window (quit request)

Client coordinates are for the 800x600 windowed game (-w). Posted input needs no focus and leaves the user's
cursor alone; real input moves the cursor and needs the window in front.
#>
param([Parameter(Mandatory = $true, Position = 0)][string]$Cmd,
      [Parameter(Position = 1)][string]$A,
      [Parameter(Position = 2)][string]$B,
      [Parameter(Position = 3)][string]$C)

$ErrorActionPreference = "Stop"
$Plugins = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "SimCity 4\Plugins"
$Log = Join-Path $Plugins "FreeCity4.log"

# Named controls of the city view, client coordinates measured on run 49 (800x600, windowed).
$Ui = @{
    "popup-close" = @(526, 410)   # the Close button of a centred advisor popup
    "pause"       = @(107, 557)   # toggles the user pause: Pause() / Resume()
    "turtle"      = @(126, 557)   # SetSimSpeed(0)
    "llama"       = @(148, 557)   # SetSimSpeed(1)
    "cheetah"     = @(175, 557)   # SetSimSpeed(2)
    "more"        = @(214, 557)   # the "..." button at the bottom right of the control panel
    "god"         = @(61, 404)    # God mode (the sun); posted clicks open it
    "disasters"   = @(49, 223)    # God mode -> Disasters (orange tornado); posted clicks open it
    "fire"        = @(125, 233)   # Disasters -> fire. Selecting it and clicking the city need REAL input
                                  # (realclick): the 3D view ignores posted clicks (run 54)
}

Add-Type -ErrorAction SilentlyContinue @"
using System; using System.Runtime.InteropServices;
public static class FC4Ctl {
    [StructLayout(LayoutKind.Sequential)] public struct PT { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] public struct RC { public int L, T, R, B; }
    [StructLayout(LayoutKind.Sequential)] public struct KI { public ushort vk, scan; public uint flags, time; public IntPtr extra; }
    [StructLayout(LayoutKind.Sequential)] public struct INP { public uint type; public KI ki; public uint pad1, pad2; }
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref PT p);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RC r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, IntPtr e);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, IntPtr extra);
    [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint type);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
[FC4Ctl]::SetProcessDPIAware() | Out-Null

function GameWindow {
    $p = Get-Process -Name "SimCity 4" -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $p -or $p.MainWindowHandle -eq [IntPtr]::Zero) { Write-Output "NOT RUNNING"; exit 2 }
    return $p.MainWindowHandle
}
function ReadShared([string]$path) {
    try {
        $fs = [IO.File]::Open($path, 'Open', 'Read', 'ReadWrite,Delete')
        try { return (New-Object IO.StreamReader($fs)).ReadToEnd() } finally { $fs.Close() }
    } catch { return "" }
}
function MakeLParam([int]$x, [int]$y) { return [IntPtr](($y -shl 16) -bor ($x -band 0xFFFF)) }
function CaptureClient([IntPtr]$h) {
    Add-Type -AssemblyName System.Drawing
    $o = New-Object FC4Ctl+PT; [FC4Ctl]::ClientToScreen($h, [ref]$o) | Out-Null
    $r = New-Object FC4Ctl+RC; [FC4Ctl]::GetClientRect($h, [ref]$r) | Out-Null
    $bmp = New-Object Drawing.Bitmap $r.R, $r.B
    $g = [Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($o.X, $o.Y, 0, 0, $bmp.Size); $g.Dispose()
    return $bmp
}
# The Close button of an advisor popup is a flat light band 15-40 px tall that spans client x 437..614 (orange news
# popups: 255,214,204; blue offer popups: 225,244,255). Its height on screen depends on the popup's text, so scan
# column x=600 from the bottom for the lowest such band and require the same colour at x=470. Returns the band's
# middle y, or $null.
function FindCloseY($bmp) {
    $run = 0; $c0 = $null
    for ($y = [Math]::Min($bmp.Height - 1, 590); $y -ge 120; $y--) {
        $c = $bmp.GetPixel(600, $y)
        $light = (($c.R + $c.G + $c.B) / 3) -ge 200
        if ($light -and $null -ne $c0 -and $c.ToArgb() -eq $c0.ToArgb()) { $run++ }
        else {
            if ($run -ge 15 -and $run -le 40) {
                $mid = $y + 1 + [int]($run / 2)
                if ($bmp.GetPixel(470, $mid).ToArgb() -eq $c0.ToArgb()) { return $mid }
            }
            $run = if ($light) { 1 } else { 0 }; $c0 = if ($light) { $c } else { $null }
        }
    }
    return $null
}
# Every modal we have met, as (name, click x, click y): the advisor popup's Close (found by FindCloseY) and the
# "City Saved" confirmation's OK (fixed: a flat light button around client 465,286, measured on run 50). Returns
# $null when no known dialog is on screen, so callers never click blind.
function FindDialogButton($bmp) {
    $y = FindCloseY $bmp
    if ($null -ne $y) { return @("popup-close", 526, $y) }
    $ref = $bmp.GetPixel(420, 286); $ok = $ref.ToArgb()   # left of the "OK" text
    $okLight = (($ref.R + $ref.G + $ref.B) / 3) -ge 200
    if ($okLight -and $bmp.GetPixel(400, 286).ToArgb() -eq $ok -and $bmp.GetPixel(530, 286).ToArgb() -eq $ok -and
        $bmp.GetPixel(420, 276).ToArgb() -eq $ok -and $bmp.GetPixel(420, 296).ToArgb() -eq $ok -and
        $bmp.GetPixel(420, 304).ToArgb() -ne $ok) { return @("save-ok", 465, 286) }
    return $null
}
function PostLeftClick([IntPtr]$h, [int]$x, [int]$y) {
    $l = MakeLParam $x $y
    [FC4Ctl]::PostMessage($h, 0x0200, [IntPtr]0, $l) | Out-Null; Start-Sleep -Milliseconds 100
    [FC4Ctl]::PostMessage($h, 0x0201, [IntPtr]1, $l) | Out-Null; Start-Sleep -Milliseconds 100
    [FC4Ctl]::PostMessage($h, 0x0202, [IntPtr]0, $l) | Out-Null
}
# Close every known modal (advisor popups, the save confirmation) until the log shows hidden=0. Never clicks blind:
# a hidden pause with no known dialog on screen (a save in progress, say) is waited out, up to ~$waitTries x 0.8 s.
# $prefix: optional PNG path prefix; each dialog is saved as <prefix>N.png before it is closed. Returns @(ok, text).
function ClearDialogs([IntPtr]$h, [string]$prefix, [int]$waitTries = 16) {
    $clicks = 0; $msgs = @(); $hidden = "?"
    for ($i = 0; $i -lt $waitTries; $i++) {
        $t = ReadShared $Log
        $m = [regex]::Matches($t, "Hidden(Pause|Resume)\(\) -> 1 \(pause=\d+ hidden=(\d+)")
        $hidden = if ($m.Count -eq 0) { "0" } else { $m[$m.Count - 1].Groups[2].Value }
        if ($hidden -eq "0") { return @($true, (($msgs + "clear: hidden=0 after $clicks click(s)") -join " | ")) }
        $bmp = CaptureClient $h
        $btn = FindDialogButton $bmp
        if ($null -ne $btn) {
            $clicks++
            if ($prefix) { $bmp.Save(("{0}{1}.png" -f $prefix, $clicks), [Drawing.Imaging.ImageFormat]::Png) }
            $bmp.Dispose()
            PostLeftClick $h $btn[1] $btn[2]
            $msgs += ("clear: clicked {0} at client ({1}, {2})" -f $btn[0], $btn[1], $btn[2])
            Start-Sleep -Milliseconds 1500
        } else { $bmp.Dispose(); Start-Sleep -Milliseconds 800 }
    }
    return @($false, (($msgs + "clear: still hidden=$hidden and no known dialog on screen (after $clicks click(s))") -join " | "))
}
function VkOf([string]$k) {
    $named = @{ esc = 0x1B; escape = 0x1B; enter = 0x0D; return = 0x0D; space = 0x20; tab = 0x09; ctrl = 0x11;
                shift = 0x10; alt = 0x12; f1 = 0x70; f2 = 0x71; f3 = 0x72; f4 = 0x73; f5 = 0x74; f6 = 0x75; f7 = 0x76;
                f8 = 0x77; f9 = 0x78; f10 = 0x79; f11 = 0x7A; f12 = 0x7B; pageup = 0x21; pagedown = 0x22;
                left = 0x25; up = 0x26; right = 0x27; down = 0x28; plus = 0xBB; minus = 0xBD; backspace = 0x08 }
    if ($named.ContainsKey($k.ToLower())) { return [byte]$named[$k.ToLower()] }
    if ($k.Length -eq 1) { return [byte][char]$k.ToUpper() }
    throw "unknown key '$k'"
}
function Mods([string]$m) {
    if (-not $m) { return @() }
    return @($m.Split(",+") | Where-Object { $_ } | ForEach-Object { VkOf $_ })
}
function KeyLp([byte]$vk, [bool]$up) {
    $scan = [FC4Ctl]::MapVirtualKey($vk, 0)
    $v = 1 -bor ($scan -shl 16)
    if ($up) { $v = $v -bor (1 -shl 30) -bor (1 -shl 31) }
    return [IntPtr]([int64]$v -band 0xFFFFFFFF -as [int64])
}

switch ($Cmd.ToLower()) {
    "state" {
        $t = ReadShared $Log
        $m = [regex]::Matches($t, "heartbeat \((periodic|month start)\): (\d{4}-\d\d-\d\d)[^\r\n]*?speed (-?\d+) pause (\d+) hidden (\d+) emergency (\d+)")
        if ($m.Count -eq 0) { Write-Output "no heartbeat yet"; exit 1 }
        $x = $m[$m.Count - 1]
        $hp = [regex]::Matches($t, "(HiddenPause|HiddenResume|Pause|Resume|EmergencyPause|EmergencyResume)\(\) -> \d[^\r\n]*")
        Write-Output ("date {0} speed {1} pause {2} hidden {3} emergency {4}" -f $x.Groups[2].Value, $x.Groups[3].Value, $x.Groups[4].Value, $x.Groups[5].Value, $x.Groups[6].Value)
        if ($hp.Count -gt 0) { Write-Output ("last pause call: " + $hp[$hp.Count - 1].Value) }
        $lv = [regex]::Matches($t, "\(pause=(\d+) hidden=(\d+) emergency=(\d+)")
        if ($lv.Count -gt 0) { $z = $lv[$lv.Count - 1]; Write-Output ("now (from the last pause/speed call): pause {0} hidden {1} emergency {2}" -f $z.Groups[1].Value, $z.Groups[2].Value, $z.Groups[3].Value) }
        $sp = [regex]::Matches($t, "SetSimSpeed[^\r\n]*")
        if ($sp.Count -gt 0) { Write-Output ("last speed call: " + $sp[$sp.Count - 1].Value) }
    }
    "clear" {
        $h = GameWindow
        $r = ClearDialogs $h $A
        Write-Output $r[1]
        if (-not $r[0]) { exit 1 }
    }
    "save" {
        # Save the city with a real Ctrl+S (posted keys do not save; the game reads the real Ctrl state), wait for the
        # save confirmation and close it. $A = the city file to watch (optional).
        $h = GameWindow
        $before = if ($A -and (Test-Path $A)) { (Get-Item $A).LastWriteTimeUtc } else { $null }
        [FC4Ctl]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 300
        [FC4Ctl]::keybd_event(0x11, 0, 0, [IntPtr]::Zero); [FC4Ctl]::keybd_event(0x53, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 80
        [FC4Ctl]::keybd_event(0x53, 0, 2, [IntPtr]::Zero); [FC4Ctl]::keybd_event(0x11, 0, 2, [IntPtr]::Zero)
        for ($i = 0; $i -lt 40; $i++) {
            Start-Sleep -Milliseconds 500
            $bmp = CaptureClient $h; $btn = FindDialogButton $bmp; $bmp.Dispose()
            if ($null -ne $btn -and $btn[0] -eq "save-ok") { PostLeftClick $h $btn[1] $btn[2]; break }
        }
        $after = if ($A -and (Test-Path $A)) { (Get-Item $A).LastWriteTimeUtc } else { $null }
        $saved = ($null -ne $btn -and $btn[0] -eq "save-ok")
        Write-Output ("save: confirmation {0}{1}" -f $(if ($saved) { "seen and closed" } else { "NOT seen" }), $(if ($A) { "; file " + $(if ($after -ne $before) { "written" } else { "NOT written" }) } else { "" }))
        if (-not $saved) { exit 1 }
    }
    "findclose" {
        $h = GameWindow; $bmp = CaptureClient $h; $btn = FindDialogButton $bmp; $bmp.Dispose()
        if ($null -eq $btn) { Write-Output "no known dialog on screen" } else { Write-Output ("{0} at client ({1}, {2})" -f $btn[0], $btn[1], $btn[2]) }
    }
    "snap" {
        $h = GameWindow
        Add-Type -AssemblyName System.Drawing
        $o = New-Object FC4Ctl+PT; [FC4Ctl]::ClientToScreen($h, [ref]$o) | Out-Null
        $r = New-Object FC4Ctl+RC; [FC4Ctl]::GetClientRect($h, [ref]$r) | Out-Null
        $bmp = New-Object Drawing.Bitmap $r.R, $r.B
        $g = [Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($o.X, $o.Y, 0, 0, $bmp.Size)
        $path = if ($A) { $A } else { Join-Path $PSScriptRoot ("..\build-runs\snap-{0}.png" -f (Get-Date -Format "HHmmss")) }
        $bmp.Save($path, [Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
        Write-Output "saved $path"
    }
    { $_ -in "click", "rclick", "ui" } {
        $h = GameWindow
        $pre = ClearDialogs $h "" 3
        if ($pre[1] -notmatch "^clear: hidden=0 after 0 click") {
            Write-Output ("(first " + $pre[1] + ")")
            Start-Sleep -Milliseconds 1000   # a click right after closing a popup is lost (run 53)
        }
        if ($Cmd -eq "ui") { if (-not $Ui.ContainsKey($A)) { throw "unknown control '$A' (known: $($Ui.Keys -join ', '))" }; $x = $Ui[$A][0]; $y = $Ui[$A][1] }
        else { $x = [int]$A; $y = [int]$B }
        $l = MakeLParam $x $y
        $down = if ($Cmd -eq "rclick") { 0x0204 } else { 0x0201 }
        $upm = if ($Cmd -eq "rclick") { 0x0205 } else { 0x0202 }
        $btn = if ($Cmd -eq "rclick") { 2 } else { 1 }
        [FC4Ctl]::PostMessage($h, 0x0200, [IntPtr]0, $l) | Out-Null; Start-Sleep -Milliseconds 100
        [FC4Ctl]::PostMessage($h, $down, [IntPtr]$btn, $l) | Out-Null; Start-Sleep -Milliseconds 100
        [FC4Ctl]::PostMessage($h, $upm, [IntPtr]0, $l) | Out-Null
        Write-Output "posted $Cmd at client ($x, $y)"
    }
    "drag" {
        # Left button down at (A, B), moves with MK_LBUTTON in 8 steps, up at the end point given as one argument,
        # "x2,y2" (the script takes three positional values). Posted, like click, so it needs no focus.
        $h = GameWindow
        $pre = ClearDialogs $h "" 3
        if ($pre[1] -notmatch "^clear: hidden=0 after 0 click") { Write-Output ("(first " + $pre[1] + ")"); Start-Sleep -Milliseconds 1000 }
        $to = ($C -split "[ ,]+") | Where-Object { $_ -ne "" }
        if ($to.Count -ne 2) { throw "drag needs: drag x1 y1 'x2,y2'" }
        $x1 = [int]$A; $y1 = [int]$B; $x2 = [int]$to[0]; $y2 = [int]$to[1]
        [FC4Ctl]::PostMessage($h, 0x0200, [IntPtr]0, (MakeLParam $x1 $y1)) | Out-Null; Start-Sleep -Milliseconds 150
        [FC4Ctl]::PostMessage($h, 0x0201, [IntPtr]1, (MakeLParam $x1 $y1)) | Out-Null; Start-Sleep -Milliseconds 150
        for ($i = 1; $i -le 8; $i++) {
            $x = [int]($x1 + ($x2 - $x1) * $i / 8); $y = [int]($y1 + ($y2 - $y1) * $i / 8)
            [FC4Ctl]::PostMessage($h, 0x0200, [IntPtr]1, (MakeLParam $x $y)) | Out-Null; Start-Sleep -Milliseconds 120
        }
        [FC4Ctl]::PostMessage($h, 0x0202, [IntPtr]0, (MakeLParam $x2 $y2)) | Out-Null
        Write-Output "posted drag from client ($x1, $y1) to ($x2, $y2)"
    }
    "key" {
        $h = GameWindow
        $vk = VkOf $A; $mods = Mods $B
        foreach ($m in $mods) { [FC4Ctl]::PostMessage($h, 0x0100, [IntPtr]$m, (KeyLp $m $false)) | Out-Null }
        [FC4Ctl]::PostMessage($h, 0x0100, [IntPtr]$vk, (KeyLp $vk $false)) | Out-Null
        if ($vk -ge 0x30 -and $vk -le 0x5A -and $mods.Count -eq 0) { [FC4Ctl]::PostMessage($h, 0x0102, [IntPtr][int][char]$A.ToLower(), (KeyLp $vk $false)) | Out-Null }
        Start-Sleep -Milliseconds 80
        [FC4Ctl]::PostMessage($h, 0x0101, [IntPtr]$vk, (KeyLp $vk $true)) | Out-Null
        foreach ($m in $mods) { [FC4Ctl]::PostMessage($h, 0x0101, [IntPtr]$m, (KeyLp $m $true)) | Out-Null }
        Write-Output "posted key $A $(if ($B) { "with $B" })"
    }
    "realkey" {
        $h = GameWindow
        [FC4Ctl]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 300
        $vk = VkOf $A; $mods = Mods $B
        foreach ($m in $mods) { [FC4Ctl]::keybd_event($m, 0, 0, [IntPtr]::Zero) }
        [FC4Ctl]::keybd_event($vk, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 80
        [FC4Ctl]::keybd_event($vk, 0, 2, [IntPtr]::Zero)
        foreach ($m in $mods) { [FC4Ctl]::keybd_event($m, 0, 2, [IntPtr]::Zero) }
        Write-Output "real key $A $(if ($B) { "with $B" })"
    }
    "realclick" {
        $h = GameWindow
        $o = New-Object FC4Ctl+PT; $o.X = [int]$A; $o.Y = [int]$B; [FC4Ctl]::ClientToScreen($h, [ref]$o) | Out-Null
        [FC4Ctl]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 300
        [FC4Ctl]::SetCursorPos($o.X, $o.Y) | Out-Null; Start-Sleep -Milliseconds 100
        [FC4Ctl]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 80
        [FC4Ctl]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)
        Write-Output "real click at client ($A, $B) = screen ($($o.X), $($o.Y))"
    }
    "realdrag" {
        # Real input: the 3D view ignores posted mouse messages (run 54), so a zone or road tool drag needs this.
        $h = GameWindow
        $to = ($C -split "[ ,]+") | Where-Object { $_ -ne "" }
        if ($to.Count -ne 2) { throw "realdrag needs: realdrag x1 y1 'x2,y2'" }
        $a = New-Object FC4Ctl+PT; $a.X = [int]$A; $a.Y = [int]$B; [FC4Ctl]::ClientToScreen($h, [ref]$a) | Out-Null
        $b = New-Object FC4Ctl+PT; $b.X = [int]$to[0]; $b.Y = [int]$to[1]; [FC4Ctl]::ClientToScreen($h, [ref]$b) | Out-Null
        [FC4Ctl]::SetForegroundWindow($h) | Out-Null; Start-Sleep -Milliseconds 300
        [FC4Ctl]::SetCursorPos($a.X, $a.Y) | Out-Null; Start-Sleep -Milliseconds 200
        [FC4Ctl]::mouse_event(0x0002, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 200
        for ($i = 1; $i -le 12; $i++) {
            [FC4Ctl]::SetCursorPos([int]($a.X + ($b.X - $a.X) * $i / 12), [int]($a.Y + ($b.Y - $a.Y) * $i / 12)) | Out-Null
            Start-Sleep -Milliseconds 100
        }
        Start-Sleep -Milliseconds 200
        [FC4Ctl]::mouse_event(0x0004, 0, 0, 0, [IntPtr]::Zero)
        Write-Output "real drag from client ($A, $B) to ($($to[0]), $($to[1])) = screen ($($a.X), $($a.Y)) to ($($b.X), $($b.Y))"
    }
    "close" {
        $h = GameWindow
        [FC4Ctl]::PostMessage($h, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
        Write-Output "posted WM_CLOSE to the game window"
    }
    default { throw "unknown command '$Cmd'" }
}
