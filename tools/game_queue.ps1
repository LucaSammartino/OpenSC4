<#
Run queue for in-game test requests (coordinator tooling). Lanes drop small request files into their own worktree;
this poller runs each request in a free game slot (tools\ingame_run.ps1 -Slot N) with that worktree's DLL and writes
the results back into the same worktree. The user doc for lanes is docs\harness.md.

Usage (the coordinator starts it detached and leaves it running):
  powershell -File tools\game_queue.ps1 [-Slots 3 | -SlotList 4,5] [-Instance <name>] [-PollSeconds 5] [-Once]
                                        [-Roots <dir>,<dir>] [-CoordinatorRoot <dir>]
  powershell -File tools\game_queue.ps1 -Status [-Last 10] [-Roots ...]
  powershell -File tools\game_queue.ps1 -Check <worktree>\build-runs\game-queue\<file>.req

  -Slots N      games at once, slots 1..N (default 3; each slot is one game, build-runs\sc4slots\slotN of the main
                checkout)
  -SlotList     the slot numbers to use instead, e.g. 4,5 (1..16; slots 1-7 each get their own P-core, see
                docs\harness.md); give each queue instance its own slots
  -Instance     a second queue next to the default one: its own single-instance mutex (FC4GameQueue-<name>), log
                (build-runs\game-queue-<name>.log) and work folder. Instances claim each request and each slot with a
                named mutex, so two instances of this version can scan the same roots; a queue older than this
                version does not claim, so do not run one next to it on the same roots
  -Once         run what is queued now (and what arrives meanwhile), then exit when nothing is waiting or running
  -Roots        the checkouts to scan (default: the main checkout and every worktree in .claude\worktrees\)
  -CoordinatorRoot  the checkout whose requests have priority (default: the main checkout)
  -Status       print the queue's state and exit (does not disturb a running queue): each slot and what it runs
                (lane, label, elapsed), the waiting requests in the order the queue will take them, and the last
                -Last results with their verdict words. Reads the queue log(s) of this checkout, the .running markers
                and the request folders.
  -Check        parse one request file and print what the queue would run, or the rejection it would write (the
                worktree is the folder above build-runs\game-queue\, else the first -Roots); runs nothing

Per root: <root>\build-runs\game-queue\*.req, oldest first, one at a time (submission order); different roots run
in parallel. Priority: the coordinator's requests (the main checkout's build-runs\game-queue\) are taken before any
waiting lane request (a running game is never stopped); a coordinator request with "priority = high" also goes
before the coordinator's other requests and does not wait for them (several run at once, one slot each; regress.ps1
-Queue uses it). Results: <root>\build-runs\game-results\<label>.log (the plugin log), <label>.verdict (first line
"verdict: <word>", then the harness lines and the run summary), <label>.running while it runs, pictures
<label>-*.png when a run ended badly; the request moves to build-runs\game-queue\done\. A request that cannot be run
(bad label, a DLL path outside the worktree, unknown key, a control character in a value, ...) gets
"verdict: rejected" with the line number, the reason and the request text, and moves to done\ too.

Request format (key = value lines, '#' comments; every line after "[ini]" goes into FreeCity4.ini as is):
  label    = power-check-1          letters, digits, '.', '_', '-' (default: the file name)
  dll      = engine\build-x86\FreeCity4.dll   relative to the worktree (default); the .pdb next to it is copied too
  region   = FC4 Test               folder under Documents\SimCity 4\Regions, mirrored into the slot
  city     = Konradshohe            a city of that region (default Konradshohe)
  seconds  = 150                    game time after the city load, in real seconds (the plugin's autotest_seconds);
                                    with end_date it is the timeout for reaching the date
  timeout  = 300                    hard limit for the whole run in real seconds (default seconds + 150)
  end_date = 2028-01-01             the run ends at this game date (autotest_end_date)
  turbo    = 1                      our simulator's turbo clock from the start (sim_turbo = 1)
  turbo_from = 2026-04-01           the turbo clock from this game date (a script action; keeps the first months,
                                    where the plugin switches popups off, at normal speed)
  minimize = 0                       keep the game window on screen after the city load (default 1: minimized);
  parallel = 1                       allow this request alongside other requests of its root (default 0)
                                    for runs that need the game to draw frames
  priority = high                   coordinator requests only (ignored, with a note, for a lane's request)
  [ini]
  enable = power
  ...
In the [ini] part, autoload_city is ignored (city = sets it) and script = <path> is taken relative to the worktree.
The queue adds popup_dialogs = 0, autotest_unpause = 2 and autotest_end = quit unless the [ini] part sets them.
#>
param(
    [ValidateRange(1, 9)][int]$Slots = 3,
    [string[]]$SlotList = @(),
    [string]$Instance = "",
    [int]$PollSeconds = 5,
    [switch]$Once,
    [string[]]$Roots = @(),
    [string]$CoordinatorRoot = "",
    [switch]$Status,
    [int]$Last = 10,
    [string]$Check = ""
)
$ErrorActionPreference = "Stop"
$Run = Join-Path $PSScriptRoot "ingame_run.ps1"
$Sum = Join-Path $PSScriptRoot "run_summary.ps1"
$Here = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$i = $Here.IndexOf("\.claude\worktrees\", [StringComparison]::OrdinalIgnoreCase)
$Main = if ($i -gt 0) { $Here.Substring(0, $i) } else { $Here }
if ($Instance -and $Instance -notmatch '^[A-Za-z0-9_-]{1,32}$') { Write-Output "game_queue: -Instance: letters, digits, '_' and '-', at most 32"; exit 4 }
$Suffix = if ($Instance) { "-$Instance" } else { "" }
$Out = Join-Path $Here "build-runs"
$Work = Join-Path $Out "game-queue-work$Suffix"
$QueueLog = Join-Path $Out "game-queue$Suffix.log"
$DocsRegions = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "SimCity 4\Regions"
$Utf8 = New-Object Text.UTF8Encoding($false)
# "-Roots a,b" and "-SlotList 4,5" arrive as one string under powershell -File: split them here.
$Roots = @($Roots | ForEach-Object { "$_" -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$SlotSet = @()
if ($SlotList.Count -gt 0) {
    foreach ($x in @($SlotList | ForEach-Object { "$_" -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })) {
        if ($x -notmatch '^\d+$' -or [int]$x -lt 1 -or [int]$x -gt 16) { Write-Output "game_queue: -SlotList: '$x' is not a slot number 1..16"; exit 4 }
        $SlotSet += [int]$x
    }
    $SlotSet = @($SlotSet | Sort-Object -Unique)
} else { $SlotSet = @(1..$Slots) }
$Coord = [IO.Path]::GetFullPath($(if ($CoordinatorRoot) { $CoordinatorRoot } else { $Main })).TrimEnd('\')
$script:HoldsDefault = $false   # true once this process is the default queue instance

function Say([string]$msg) {
    $line = "{0} {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $msg
    Write-Host $line   # not Write-Output: Say runs inside functions whose output is their return value
    try { [IO.File]::AppendAllText($QueueLog, $line + "`r`n", $Utf8) } catch { }
}
function WriteText([string]$path, [string]$text) { [IO.File]::WriteAllText($path, $text, $Utf8) }
function QuoteArg([string]$a) { if ($a -eq "") { return '""' }; if ($a -notmatch '[\s"]') { return $a }; return '"' + ($a -replace '(\\*)"', '$1$1\"' -replace '(\\+)$', '$1$1') + '"' }
function RootsNow {
    if ($Roots.Count -gt 0) { return @($Roots | ForEach-Object { [IO.Path]::GetFullPath($_).TrimEnd('\') }) }
    $r = @($Main.TrimEnd('\'))
    $wt = Join-Path $Main ".claude\worktrees"
    if (Test-Path $wt) { $r += @(Get-ChildItem -LiteralPath $wt -Directory | ForEach-Object { $_.FullName }) }
    return $r
}
function SamePath([string]$a, [string]$b) { return [string]::Equals($a.TrimEnd('\'), $b.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase) }
function IsCoord([string]$root) { return SamePath $root $Coord }
# Named mutexes: a slot another harness run holds (ingame_run's user-dir mutex), and the claims of queue instances.
function TryClaim([string]$name) {
    $m = New-Object System.Threading.Mutex($false, $name)
    try { if ($m.WaitOne(0)) { return $m } } catch [System.Threading.AbandonedMutexException] { return $m }
    $m.Dispose(); return $null
}
function Unclaim($m) { if ($m) { try { $m.ReleaseMutex() } catch { }; try { $m.Dispose() } catch { } } }
# Held by another process: take it for an instant and give it back (our own claims are checked before this).
function HeldElsewhere([string]$name) { $m = TryClaim $name; if ($m) { Unclaim $m; return $false }; return $true }
function MutexExists([string]$name) {
    $m = $null
    try { if ([System.Threading.Mutex]::TryOpenExisting($name, [ref]$m)) { $m.Dispose(); return $true } } catch [System.UnauthorizedAccessException] { return $true } catch { }
    return $false
}
function SlotDir([int]$n) {
    $sr = if ($env:FC4_SLOT_ROOT) { $env:FC4_SLOT_ROOT } else { Join-Path $Main "build-runs\sc4slots" }
    return [IO.Path]::GetFullPath((Join-Path $sr "slot$n"))
}
function SlotBusy([int]$n) { return HeldElsewhere ("FC4UserDir-" + ((SlotDir $n).TrimEnd('\').ToLower() -replace '[^a-z0-9]', '_')) }
function Sha([string]$s) {
    $sha = [Security.Cryptography.SHA1]::Create()
    try { return (($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($s.ToLower())) | ForEach-Object { $_.ToString("x2") }) -join "").Substring(0, 24) } finally { $sha.Dispose() }
}
function ReqClaimName([string]$path) { return "FC4QueueReq-" + (Sha ([IO.Path]::GetFullPath($path))) }
function SlotClaimName([int]$n) { return "FC4QueueSlot-$n" }
function RootName([string]$root) { if (SamePath $root $Main) { return "main" } else { return (Split-Path $root -Leaf) } }
function Inside([string]$root, [string]$path) { return $path.StartsWith($root.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase) }

# The .running markers of a root: "running in slot N since <time> (request X.req; queue pid P, instance I)". Markers
# of a queue older than this version have no queue pid.
function Markers([string]$root) {
    $r = @()
    foreach ($f in @(Get-ChildItem -Path (Join-Path $root "build-runs\game-results\*.running") -File -ErrorAction SilentlyContinue)) {
        $t = ""; try { $t = [IO.File]::ReadAllText($f.FullName) } catch { }
        $m = @{ file = $f.FullName; label = $f.BaseName; slot = 0; since = $null; req = ""; qpid = 0 }
        if ($t -match 'slot (\d+)') { $m.slot = [int]$Matches[1] }
        if ($t -match 'since (\d{4}-\d\d-\d\d \d\d:\d\d:\d\d)') { $m.since = [datetime]::ParseExact($Matches[1], 'yyyy-MM-dd HH:mm:ss', $null) }
        if ($t -match '\(request ([^;)]+)') { $m.req = $Matches[1].Trim() }
        if ($t -match 'queue pid (\d+)') { $m.qpid = [int]$Matches[1] }
        $r += $m
    }
    return $r
}
# A marker of a run another queue process is running now.
function MarkerLive($m) {
    if ($m.qpid -gt 0) {
        if ($m.qpid -eq $PID) { return $false }
        $pp = Get-Process -Id $m.qpid -ErrorAction SilentlyContinue
        return [bool]($pp -and $pp.ProcessName -match '^(powershell|pwsh)$')
    }
    # Old format: only the default instance of an older queue writes it (live while that queue runs).
    return (-not $script:HoldsDefault) -and (MutexExists "FC4GameQueue")
}

# ---- Request parsing ---------------------------------------------------------------------------------------------
$CtlNames = @{ 0 = "NUL"; 7 = "bell"; 8 = "backspace"; 9 = "tab"; 10 = "line feed"; 11 = "vertical tab"; 12 = "form feed"; 13 = "carriage return"; 27 = "escape"; 127 = "delete" }
$CtlEscapes = @{ 0 = '\0'; 7 = '\a'; 8 = '\b'; 9 = '\t'; 10 = '\n'; 11 = '\v'; 12 = '\f'; 13 = '\r'; 27 = '\e' }
# Control characters shown as <0xNN>.
function Visible([string]$s) {
    $sb = New-Object Text.StringBuilder
    foreach ($ch in $s.ToCharArray()) { $c = [int]$ch; if ($c -lt 32 -or $c -eq 127) { [void]$sb.Append(("<0x{0:X2}>" -f $c)) } else { [void]$sb.Append($ch) } }
    return $sb.ToString()
}
# The first control character of a value, described (null if none).
function ControlChar([string]$v, [switch]$AllowTab) {
    for ($k = 0; $k -lt $v.Length; $k++) {
        $c = [int]$v[$k]
        if (($c -lt 32 -or $c -eq 127) -and -not ($AllowTab -and $c -eq 9)) {
            $name = if ($CtlNames.ContainsKey($c)) { $CtlNames[$c] } else { "control character" }
            $msg = "a {0} (0x{1:X2}) at character {2}: '{3}'" -f $name, $c, ($k + 1), (Visible $v)
            if ($CtlEscapes.ContainsKey($c)) { $msg += "; the writer probably turned '$($CtlEscapes[$c])' into this character (an escape sequence in a string): write the path with plain single backslashes and no escapes, or with '/'" }
            return $msg
        }
    }
    return $null
}
# A path of the request, made absolute against the worktree. Returns @(full path, error).
function RequestPath([string]$root, [string]$p) {
    try { $full = [IO.Path]::GetFullPath($(if ([IO.Path]::IsPathRooted($p)) { $p } else { Join-Path $root $p })); return @($full, $null) }
    catch { $e = $_.Exception; while ($e.InnerException) { $e = $e.InnerException }; return @($null, "not a valid path ($($e.Message))") }
}
function PeekPriority([string]$file) {
    try { foreach ($line in [IO.File]::ReadAllLines($file)) { $t = $line.Trim(); if ($t -eq "[ini]") { break }; if ($t -match '^priority\s*=\s*(\S+)') { return $Matches[1].ToLower() } } } catch { }
    return ""
}
function PeekParallel([string]$file) {
    try { foreach ($line in [IO.File]::ReadAllLines($file)) { $t = $line.Trim(); if ($t -eq "[ini]") { break }; if ($t -match '^parallel\s*=\s*1\s*(#.*)?$') { return $true } } } catch { }
    return $false
}
function PeekLabel([string]$file) {
    try { foreach ($line in [IO.File]::ReadAllLines($file)) { $t = $line.Trim(); if ($t -eq "[ini]") { break }; if ($t -match '^label\s*=\s*(.+)$') { return (Visible $Matches[1].Trim()) } } } catch { }
    return [IO.Path]::GetFileNameWithoutExtension($file)
}
# Parse and check one request. Returns a hashtable; .error is set when the request must be rejected (.errLine: the
# line number, .lines: the request's lines for the rejection).
function ParseRequest([string]$root, [string]$file) {
    $q = @{ label = [IO.Path]::GetFileNameWithoutExtension($file); dll = "engine\build-x86\FreeCity4.dll"; city = "Konradshohe"; region = "FC4 Test";
            seconds = 150; timeout = $null; end_date = ""; turbo = ""; turbo_from = ""; minimize = ""; priority = ""; parallel = "0"; ini = @(); notes = @();
            error = $null; errLine = 0; lines = @(); at = @{} }
    $bytes = [IO.File]::ReadAllBytes($file)
    $bom16 = $bytes.Length -ge 2 -and (($bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) -or ($bytes[0] -eq 0xFE -and $bytes[1] -eq 0xFF))
    if (-not $bom16 -and [Array]::IndexOf($bytes, [byte]0) -ge 0) {
        $q.error = "the file has NUL bytes (UTF-16 without a byte order mark?): write it as UTF-8 or ASCII"; return $q
    }
    $text = [IO.File]::ReadAllText($file)   # UTF-8 unless a byte order mark says otherwise
    $lines = [regex]::Split($text, "\r\n|\n|\r")
    if ($lines.Count -gt 0 -and $lines[$lines.Count - 1] -eq "") { $lines = @($lines | Select-Object -First ($lines.Count - 1)) }
    $q.lines = @($lines)
    $inIni = $false; $n = 0; $prevPathLine = 0
    foreach ($line in $lines) {
        $n++
        if ($inIni) {
            $cc = ControlChar $line -AllowTab
            if ($cc) { $q.error = "line ${n} ([ini] part) has $cc"; $q.errLine = $n; return $q }
            $q.ini += , @($n, $line); continue
        }
        $t = $line.Trim()
        if ($t -eq "" -or $t.StartsWith("#")) { continue }
        if ($t -eq "[ini]") { $inIni = $true; continue }
        if ($t -notmatch '^([a-z_]+)\s*=\s*(.*)$') {
            $q.error = "line ${n}: not 'key = value': '$(Visible $t)'"
            if ($prevPathLine -eq $n - 1 -and $t -notmatch '=') { $q.error += " (line $($n - 1) holds a path: if it had '\n' or '\r' in it, the writer turned that into a line break; write single backslashes and no escapes, or '/')" }
            $q.errLine = $n; return $q
        }
        $k = $Matches[1]; $v = $Matches[2].Trim()
        $cc = ControlChar $v
        if ($cc) { $q.error = "line ${n}: the value of '$k' has $cc"; $q.errLine = $n; return $q }
        $q.at[$k] = $n
        switch ($k) {
            "label" { $q.label = $v }
            "dll" { $q.dll = $v; $prevPathLine = $n }
            "city" { $q.city = $v }
            "region" { $q.region = $v }
            "parallel" { if ($v -notin "0", "1") { $q.error = "line ${n}: parallel must be 0 or 1"; $q.errLine = $n; return $q }; $q.parallel = $v }
            "seconds" { if ($v -notmatch '^\d+$' -or [int64]$v -lt 10 -or [int64]$v -gt 7200) { $q.error = "line ${n}: seconds must be 10..7200 (got '$v')"; $q.errLine = $n; return $q }; $q.seconds = [int]$v }
            "timeout" { if ($v -notmatch '^\d+$' -or [int64]$v -lt 30 -or [int64]$v -gt 10800) { $q.error = "line ${n}: timeout must be 30..10800 (got '$v')"; $q.errLine = $n; return $q }; $q.timeout = [int]$v }
            "end_date" { if ($v -notmatch '^\d{4}-\d\d-\d\d$') { $q.error = "line ${n}: end_date must be YYYY-MM-DD (got '$v')"; $q.errLine = $n; return $q }; $q.end_date = $v }
            "turbo" { if ($v -notin "0", "1") { $q.error = "line ${n}: turbo must be 0 or 1 (got '$v')"; $q.errLine = $n; return $q }; $q.turbo = $v }
            "turbo_from" { if ($v -notmatch '^\d{4}-\d\d-\d\d$') { $q.error = "line ${n}: turbo_from must be YYYY-MM-DD (got '$v')"; $q.errLine = $n; return $q }; $q.turbo_from = $v }
            "minimize" { if ($v -notin "0", "1") { $q.error = "line ${n}: minimize must be 0 or 1 (got '$v')"; $q.errLine = $n; return $q }; $q.minimize = $v }
            "priority" {
                if ($v -notin "high", "normal") { $q.error = "line ${n}: priority must be high or normal (got '$v')"; $q.errLine = $n; return $q }
                $q.priority = $v
                if ($v -eq "high" -and -not (IsCoord $root)) { $q.notes += "priority = high is honoured only for the coordinator's requests (the main checkout); this ran as a normal request" }
            }
            default { $q.error = "line ${n}: unknown key '$k' (label, dll, region, city, parallel, seconds, timeout, end_date, turbo, turbo_from, minimize, priority, [ini])"; $q.errLine = $n; return $q }
        }
    }
    $ln = { param($key) if ($q.at.ContainsKey($key)) { "line $($q.at[$key]): " } else { "" } }
    if ($q.label -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$') { $q.error = (& $ln "label") + "label '$($q.label)': use letters, digits, '.', '_', '-' (no path characters, at most 64; the first a letter or digit)"; $q.errLine = [int]$q.at["label"]; return $q }
    if ($q.region -notmatch '^[A-Za-z0-9][A-Za-z0-9 ._()-]*$' -or $q.region.EndsWith('.') -or $q.region.EndsWith(' ')) { $q.error = (& $ln "region") + "region '$($q.region)': use one folder name"; $q.errLine = [int]$q.at["region"]; return $q }
    $docsRegion = Join-Path $DocsRegions $q.region
    if (-not (Test-Path -LiteralPath $docsRegion -PathType Container)) { $q.error = (& $ln "region") + "region '$($q.region)': no such region"; $q.errLine = [int]$q.at["region"]; return $q }
    if ($q.city -notmatch '^[A-Za-z0-9 ._()''-]+$' -or -not (Test-Path -LiteralPath (Join-Path $docsRegion "City - $($q.city).sc4"))) { $q.error = (& $ln "city") + "city '$($q.city)': no such city in region '$($q.region)'"; $q.errLine = [int]$q.at["city"]; return $q }
    $rp = RequestPath $root $q.dll
    if ($rp[1]) { $q.error = (& $ln "dll") + "dll '$($q.dll)': $($rp[1])"; $q.errLine = [int]$q.at["dll"]; return $q }
    $dllFull = $rp[0]
    if (-not (Inside $root $dllFull)) { $q.error = (& $ln "dll") + "dll '$($q.dll)' is outside the worktree ($dllFull; the worktree is $root)"; $q.errLine = [int]$q.at["dll"]; return $q }
    if ($dllFull -notmatch '\.dll$' -or -not (Test-Path -LiteralPath $dllFull -PathType Leaf)) { $q.error = (& $ln "dll") + "dll '$($q.dll)': no such file ($dllFull)"; $q.errLine = [int]$q.at["dll"]; return $q }
    $q.dllFull = $dllFull
    if (-not $q.timeout) { $q.timeout = $q.seconds + 150 }
    # The [ini] part: autoload_city comes from city =, a script path is relative to the worktree.
    $ini = @()
    foreach ($pair in $q.ini) {
        $n = $pair[0]; $line = $pair[1]
        if ($line -match '^\s*autoload_city\s*=') { $q.notes += "ignored the [ini] line $n '$($line.Trim())' (city = sets autoload_city)"; continue }
        if ($line -match '^(\s*script\s*=\s*)(.+?)\s*$') {
            $sv = $Matches[2]
            $cc = ControlChar $sv
            if ($cc) { $q.error = "line ${n}: the script path has $cc"; $q.errLine = $n; return $q }
            $rp = RequestPath $root $sv
            if ($rp[1]) { $q.error = "line ${n}: script '$sv': $($rp[1])"; $q.errLine = $n; return $q }
            $sp = $rp[0]
            if (-not (Inside $root $sp) -or -not (Test-Path -LiteralPath $sp -PathType Leaf)) { $q.error = "line ${n}: script '$sv': no such file in the worktree ($sp)"; $q.errLine = $n; return $q }
            if ($q.turbo_from) { $q.error = "line ${n}: turbo_from writes its own script; use 'turbo on' in your script instead"; $q.errLine = $n; return $q }
            $ini += "script = $sp"; continue
        }
        $ini += $line
    }
    $text = $ini -join "`n"
    foreach ($kv in @(@("popup_dialogs", "0"), @("autotest_unpause", "2"), @("autotest_end", "quit"))) {
        if ($text -notmatch "(?m)^\s*$($kv[0])\s*=") { $text += "`n$($kv[0]) = $($kv[1])" }
    }
    # reach = 1 without reach_output: the tracer writes <slot>\Plugins\FreeCity4.log.reach.tsv, which Finish never copies, so
    # the run has no labelled reach file. Default it to <label>.reach.tsv next to the results (an explicit reach_output stays).
    if ($text -match '(?im)^\s*reach\s*=\s*(1|on|true|yes)\s*(#.*)?$' -and $text -notmatch '(?m)^\s*reach_output\s*=') {
        $text += "`nreach_output = " + (Join-Path (Join-Path $root "build-runs\game-results") "$($q.label).reach.tsv")
    }
    $q.iniText = $text
    return $q
}
function VerdictWord([string]$v) {
    if ($v -match '^AUTOTEST VERDICT: ([a-z]+)') { return $Matches[1] }
    if ($v -match '^(clean|corrupt|stalled) \(fallback\)') { return $Matches[1] }
    if ($v -match '^failed start') { return "failed-start" }
    if ($v -match '^exited') { return "exited" }
    if ($v -match '^no city load') { return "no-load" }
    if ($v -match '^timeout') { return "timeout" }
    if ($v -match '^state-export-failed') { return "harness-error" }
    if ($v -match '^BLOCKED|^BAD') { return "harness-error" }
    if ($v -match '^REJECTED') { return "rejected" }
    return "unknown"
}

# ---- Running a request -------------------------------------------------------------------------------------------
function Finish($job) {
    $res = Join-Path $job.root "build-runs\game-results"
    New-Item -ItemType Directory -Force $res | Out-Null
    $lines = @(); if (Test-Path $job.outFile) { $lines = @(Get-Content -LiteralPath $job.outFile) }
    if (Test-Path $job.errFile) { $lines += @(Get-Content -LiteralPath $job.errFile | Where-Object { $_ }) }
    $vline = $lines | Where-Object { $_ -match "^\[$([regex]::Escape($job.runLabel))\] " } | Select-Object -First 1
    $v = if ($vline) { $vline -replace "^\[[^\]]+\] ", "" } else { ($lines | Where-Object { $_ -match '^(BLOCKED|BAD)' } | Select-Object -First 1) }
    if (-not $v) { $v = "no verdict line from the harness (exit code $($job.proc.ExitCode))" }
    $word = VerdictWord $v
    $held = $lines | Where-Object { $_ -match "^\s+HELD: " } | Select-Object -First 1
    $wall = [int]((Get-Date) - $job.started).TotalSeconds
    $waited = [int][Math]::Max(0, ($job.started - $job.queued).TotalSeconds)
    $log = Join-Path $Out "$($job.runLabel).log"
    $body = @("verdict: $word", "line: $v")
    if ($held) { $body += "held: " + ($held.Trim() -replace '^HELD: ', '') }
    foreach ($dl in @($lines | Where-Object { $_ -match '^\s+DIALOG \(' })) { $body += "dialog: " + ($dl.Trim() -replace '^DIALOG ', '') }
    $body += @("request: $($job.reqName) (queued $($job.queued.ToString('yyyy-MM-dd HH:mm:ss')), started $($job.started.ToString('HH:mm:ss')) in slot $($job.slot) after $waited s in the queue, wall $wall s$(if ($job.prio -eq 0) { '; priority high' } elseif ($job.prio -eq 1) { '; coordinator' }))",
        "dll: $($job.q.dllFull)", "region: $($job.q.region)", "city: $($job.q.city); seconds $($job.q.seconds); timeout $($job.q.timeout)$(if ($job.q.end_date) { "; end_date $($job.q.end_date)" })$(if ($job.q.turbo -eq '1') { '; turbo 1' })$(if ($job.q.turbo_from) { "; turbo_from $($job.q.turbo_from)" })")
    foreach ($nt in $job.q.notes) { $body += "note: $nt" }
    $body += @("", "harness:") + @($lines | ForEach-Object { "  $_" })
    if (Test-Path $log) {
        Copy-Item -LiteralPath $log -Destination (Join-Path $res "$($job.label).log") -Force
        $s = & powershell -NoProfile -ExecutionPolicy Bypass -File $Sum -Log $log -Width 200 2>&1 | ForEach-Object { "$_" }
        $body += @("", "summary:") + @($s | ForEach-Object { "  $_" })
    } else { $body += @("", "(no plugin log)") }
    # Optional Lua transcript follows the plugin log into the request checkout.
    $luaLog = Join-Path $Out "$($job.runLabel).lua.log"
    if (Test-Path -LiteralPath $luaLog) {
        Copy-Item -LiteralPath $luaLog -Destination (Join-Path $res "$($job.label).lua.log") -Force
    }
    # End optional Lua transcript.
    foreach ($png in @(Get-ChildItem -Path (Join-Path $Out "$($job.runLabel)-*.png") -ErrorAction SilentlyContinue)) {
        Copy-Item -LiteralPath $png.FullName -Destination (Join-Path $res ($png.Name -replace "^$([regex]::Escape($job.runLabel))", $job.label)) -Force
    }
    WriteText (Join-Path $res "$($job.label).verdict") (($body -join "`r`n") + "`r`n")
    Remove-Item -LiteralPath (Join-Path $res "$($job.label).running") -Force -ErrorAction SilentlyContinue
    MoveDone $job.root $job.reqFile
    Say ("done  {0}/{1} in slot {2}: {3} ({4} s)" -f (RootName $job.root), $job.label, $job.slot, $v, $wall)
}
function MoveDone([string]$root, [string]$reqFile) {
    $done = Join-Path $root "build-runs\game-queue\done"
    New-Item -ItemType Directory -Force $done | Out-Null
    $dest = Join-Path $done (Split-Path $reqFile -Leaf)
    if (Test-Path -LiteralPath $dest) { $dest = Join-Path $done ("{0}.{1}.req" -f [IO.Path]::GetFileNameWithoutExtension($reqFile), (Get-Date -Format "yyyyMMdd-HHmmss")) }
    if (Test-Path -LiteralPath $reqFile) { Move-Item -LiteralPath $reqFile -Destination $dest -Force }
}
function RejectBody([string]$root, [string]$reqFile, $q) {
    $body = @("verdict: rejected", "line: REJECTED: $($q.error)", "request: $(Split-Path $reqFile -Leaf) (in $root)")
    $lines = @($q.lines)
    if ($q.errLine -gt 0 -and $q.errLine -le $lines.Count) { $body += "the line: {0}: {1}" -f $q.errLine, (Visible $lines[$q.errLine - 1]) }
    if ($lines.Count -gt 0) {
        $body += @("", "request text (control characters shown as <0xNN>):")
        for ($k = 0; $k -lt [Math]::Min($lines.Count, 80); $k++) { $body += "{0}{1,3}: {2}" -f $(if ($k + 1 -eq $q.errLine) { ">" } else { " " }), ($k + 1), (Visible $lines[$k]) }
        if ($lines.Count -gt 80) { $body += "  ... ($($lines.Count - 80) more lines)" }
    }
    return $body
}
function Reject([string]$root, [string]$reqFile, $q) {
    $res = Join-Path $root "build-runs\game-results"
    New-Item -ItemType Directory -Force $res | Out-Null
    $label = if ($q.label -match '^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$') { $q.label } else { [IO.Path]::GetFileNameWithoutExtension($reqFile) -replace '[^A-Za-z0-9._-]', '_' }
    $body = RejectBody $root $reqFile $q
    WriteText (Join-Path $res "$label.verdict") (($body -join "`r`n") + "`r`n")
    MoveDone $root $reqFile
    Say ("reject {0}/{1}: {2}" -f (RootName $root), (Split-Path $reqFile -Leaf), $q.error)
}
function StartJob([string]$root, [IO.FileInfo]$req, [int]$slot, [int]$prio) {
    $q = ParseRequest $root $req.FullName
    if ($q.error) { Reject $root $req.FullName $q; return $null }
    $runLabel = "q-{0}-{1}" -f (RootName $root), $q.label
    $iniFile = Join-Path $Work "$runLabel.ini"
    WriteText $iniFile $q.iniText
    $outFile = Join-Path $Work "$runLabel.out"; $errFile = Join-Path $Work "$runLabel.err"
    # A run of a queue that was stopped can still hold these open (its game runs on to its timeout): use new names.
    foreach ($f in @($outFile, $errFile)) { Remove-Item -LiteralPath $f -Force -ErrorAction SilentlyContinue }
    if ((Test-Path -LiteralPath $outFile) -or (Test-Path -LiteralPath $errFile)) {
        $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
        $outFile = Join-Path $Work "$runLabel.$stamp.out"; $errFile = Join-Path $Work "$runLabel.$stamp.err"
    }
    $argv = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $Run, "-Slot", "$slot", "-Dll", $q.dllFull, "-Region", $q.region, "-City", $q.city,
              "-IniFile", $iniFile, "-AfterLoadSeconds", "$($q.seconds)", "-Seconds", "$($q.timeout)", "-Label", $runLabel)
    if ($q.end_date) { $argv += @("-EndDate", $q.end_date) }
    if ($q.turbo -eq "1") { $argv += "-Turbo" }
    if ($q.turbo_from) { $argv += @("-TurboFrom", $q.turbo_from) }
    if ($q.minimize -eq "0") { $argv += "-NoMinimize" }
    $res = Join-Path $root "build-runs\game-results"
    New-Item -ItemType Directory -Force $res | Out-Null
    Remove-Item -LiteralPath (Join-Path $res "$($q.label).verdict") -Force -ErrorAction SilentlyContinue
    WriteText (Join-Path $res "$($q.label).running") ("running in slot $slot since $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') (request $($req.Name); queue pid $PID, instance $(if ($Instance) { $Instance } else { 'default' }))`r`n")
    $proc = Start-Process -FilePath "powershell" -ArgumentList (($argv | ForEach-Object { QuoteArg $_ }) -join " ") -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $outFile -RedirectStandardError $errFile
    $null = $proc.Handle   # keeps the exit code readable
    Say ("start {0}/{1} in slot {2} (dll {3}){4}" -f (RootName $root), $q.label, $slot, $q.dllFull, $(if ($prio -eq 0) { " [priority high]" } elseif ($prio -eq 1) { " [coordinator]" } else { "" }))
    return @{ root = $root; reqFile = $req.FullName; reqName = $req.Name; queued = $req.LastWriteTime; started = (Get-Date); slot = $slot; prio = $prio;
              label = $q.label; runLabel = $runLabel; q = $q; proc = $proc; outFile = $outFile; errFile = $errFile; claims = @() }
}

# ---- Waiting requests (the scheduler and -Status share this) ----------------------------------------------------
# Every waiting request of every root with its class (0 coordinator priority high, 1 coordinator, 2 lane), whether it
# may start now (its root runs nothing, or it is priority high), and the busy roots. $mine: requests this process runs.
function Waiting($mine) {
    $all = @(); $order = 0
    foreach ($root in RootsNow) {
        $qdir = Join-Path $root "build-runs\game-queue"
        if (-not (Test-Path -LiteralPath $qdir)) { continue }
        $reqs = @(Get-ChildItem -LiteralPath $qdir -Filter "*.req" -File -ErrorAction SilentlyContinue | Where-Object { $_.Extension -eq ".req" } | Sort-Object LastWriteTime, Name)
        if ($reqs.Count -eq 0) { continue }
        $coord = IsCoord $root
        $live = @(Markers $root | Where-Object { $_.req -and (MarkerLive $_) } | ForEach-Object { $_.req.ToLower() })
        $busy = $false; $rows = @()
        foreach ($r in $reqs) {
            if ($mine -contains $r.FullName.ToLower()) { $busy = $true; continue }
            if ($live -contains $r.Name.ToLower() -or (HeldElsewhere (ReqClaimName $r.FullName))) { $busy = $true; continue }
            $rows += $r
        }
        $headSeen = $false
        foreach ($r in $rows) {
            $prio = if ($coord -and (PeekPriority $r.FullName) -eq "high") { 0 } elseif ($coord) { 1 } else { 2 }
            $young = ((Get-Date) - $r.LastWriteTime).TotalSeconds -lt 2   # still being written
            $head = $false
            $parallel = PeekParallel $r.FullName
            if ($prio -ne 0 -and -not $parallel) { $head = -not $headSeen; $headSeen = $true }
            $all += @{ root = $root; req = $r; prio = $prio; young = $young; parallel = $parallel; head = ($prio -eq 0 -or $parallel -or $head); rootBusy = $busy; i = $order++ }
        }
    }
    return @($all | Sort-Object { $_.prio }, { $_.req.LastWriteTime }, { $_.i })
}

# ---- -Status ----------------------------------------------------------------------------------------------------
function ReadShared([string]$path) {
    try { $fs = [IO.File]::Open($path, 'Open', 'Read', 'ReadWrite,Delete'); try { return (New-Object IO.StreamReader($fs)).ReadToEnd() } finally { $fs.Close() } } catch { return "" }
}
function Age([TimeSpan]$t) { $s = [int]$t.TotalSeconds; if ($s -lt 0) { $s = 0 }; if ($s -lt 120) { return "$s s" }; if ($s -lt 7200) { return "{0} min {1:00} s" -f [int][Math]::Floor($s / 60), ($s % 60) }; return "{0:0.0} h" -f ($s / 3600.0) }
function ShowStatus {
    $now = Get-Date
    $games = @{}   # slot -> game pid (read-only: process command lines)
    try {
        foreach ($g in @(Get-CimInstance Win32_Process -Filter "Name='SimCity 4.exe'" -ErrorAction Stop)) {
            if ("$($g.CommandLine)" -match '-UserDir:"?([^"]+)') { $ud = $Matches[1].TrimEnd('\'); if ($ud -match '\\slot(\d+)$') { $games[[int]$Matches[1]] = [int]$g.ProcessId } }
        }
    } catch { }
    # The logs of this checkout's queue instances, and of the main checkout's when this is a worktree.
    $dirs = @($Out); $mo = Join-Path $Main "build-runs"; if (-not (SamePath $mo $Out)) { $dirs = @($mo, $Out) }
    $logs = @(foreach ($d in $dirs) { Get-ChildItem -Path (Join-Path $d "game-queue*.log") -File -ErrorAction SilentlyContinue | Where-Object { $_.Name -match '^game-queue(-[A-Za-z0-9_-]+)?\.log$' } | Sort-Object Name })
    $results = @(); $shown = @{}
    if ($logs.Count -eq 0) { Write-Output "no queue log in $Out" }
    foreach ($lf in $logs) {
        $inst = if ($lf.Name -match '^game-queue-(.+)\.log$') { $Matches[1] } else { "" }
        $alive = MutexExists ("FC4GameQueue" + $(if ($inst) { "-$inst" } else { "" }))
        $lines = @((ReadShared $lf.FullName) -split "\r?\n" | Where-Object { $_ })
        $si = -1
        for ($k = $lines.Count - 1; $k -ge 0; $k--) { if ($lines[$k] -match '^\S+ \S+ game_queue: (\d+ slot|slots )') { $si = $k; break } }
        if (-not $alive -and $lf.LastWriteTime -lt $now.AddDays(-1)) { continue }   # an old instance's log
        $slots = @(); $startT = $null; $head = ""
        if ($si -ge 0) {
            $head = $lines[$si]
            $startT = [datetime]::ParseExact($head.Substring(0, 19), 'yyyy-MM-dd HH:mm:ss', $null)
            if ($head -match 'game_queue: slots ([\d,]+)') { $slots = @($Matches[1] -split ',' | ForEach-Object { [int]$_ }) }
            elseif ($head -match 'game_queue: (\d+) slot') { $slots = @(1..[int]$Matches[1]) }
        }
        $jobs = [ordered]@{}
        for ($k = [Math]::Max(0, $si); $k -lt $lines.Count; $k++) {
            $l = $lines[$k]
            if ($l -match '^(\S+ \S+) start (\S+?)/(\S+) in slot (\d+) \(dll [^)]*\)(.*)$') { $jobs["$($Matches[2])/$($Matches[3])|$($Matches[4])"] = @{ t = [datetime]::ParseExact($Matches[1], 'yyyy-MM-dd HH:mm:ss', $null); root = $Matches[2]; label = $Matches[3]; slot = [int]$Matches[4]; tag = $Matches[5].Trim() } }
            elseif ($l -match '^\S+ \S+ done  (\S+?)/(\S+) in slot (\d+):') { $jobs.Remove("$($Matches[1])/$($Matches[2])|$($Matches[3])") }
        }
        foreach ($l in $lines) {
            if ($l -match '^(\S+ \S+) done  (\S+?)/(\S+) in slot (\d+): (.*) \((\d+) s\)$') { $results += @{ t = $Matches[1]; who = "$($Matches[2])/$($Matches[3])"; slot = $Matches[4]; v = $Matches[5]; wall = $Matches[6]; inst = $inst } }
            elseif ($l -match '^(\S+ \S+) reject (\S+?)/(.+?\.req): (.*)$') { $results += @{ t = $Matches[1]; who = "$($Matches[2])/$($Matches[3])"; slot = "-"; v = "REJECTED: $($Matches[4])"; wall = ""; inst = $inst } }
        }
        $name = if ($inst) { "queue instance '$inst'" } else { "queue" }
        Write-Output ("{0}: {1}{2}; slots {3}; log {4}" -f $name, $(if ($alive) { "running" } else { "NOT running" }),
            $(if ($startT) { " (started $($startT.ToString('yyyy-MM-dd HH:mm')))" } else { "" }), $(if ($slots.Count) { $slots -join ',' } else { "?" }), $lf.FullName)
        if (-not $alive) { continue }
        foreach ($s in $slots) {
            $shown[$s] = $true
            $j = @($jobs.Values | Where-Object { $_.slot -eq $s }) | Select-Object -Last 1
            $gp = if ($games.ContainsKey($s)) { "game pid $($games[$s])" } else { "no game" }
            if ($j) { Write-Output ("  slot {0}: {1}/{2}, {3} elapsed{4} ({5})" -f $s, $j.root, $j.label, (Age ($now - $j.t)), $(if ($j.tag) { " $($j.tag)" } else { "" }), $gp) }
            elseif ($games.ContainsKey($s)) { Write-Output ("  slot {0}: held by a harness run outside this queue ({1})" -f $s, $gp) }
            else { Write-Output ("  slot {0}: free" -f $s) }
        }
    }
    foreach ($s in @($games.Keys | Sort-Object)) { if (-not $shown.ContainsKey($s)) { Write-Output ("  slot {0}: not a queue slot; game pid {1} (regress -Parallel or a manual ingame_run -Slot)" -f $s, $games[$s]) } }

    $w = @(Waiting @())
    Write-Output ""
    Write-Output ("waiting: {0} request(s), in the order the queue takes them{1}" -f $w.Count, $(if ($w.Count) { ":" } else { "" }))
    $k = 0
    foreach ($x in $w) {
        $k++
        $cls = switch ($x.prio) { 0 { "coordinator, priority high" } 1 { "coordinator" } default { "lane" } }
        $note = @()
        if (-not $x.head) { $note += "after its root's earlier request" } elseif ($x.prio -ne 0 -and -not $x.parallel -and $x.rootBusy) { $note += "waits for its root's running request" }
        if ($x.young) { $note += "being written" }
        Write-Output ("  {0,2}. {1}/{2}  ({3}; {4})  waiting {5}{6}" -f $k, (RootName $x.root), (PeekLabel $x.req.FullName), $x.req.Name, $cls, (Age ($now - $x.req.LastWriteTime)), $(if ($note.Count) { "  [" + ($note -join "; ") + "]" } else { "" }))
    }
    Write-Output ""
    $lastN = @($results | Sort-Object { $_.t } | Select-Object -Last $Last)
    Write-Output ("last {0} result(s):" -f $lastN.Count)
    foreach ($r in $lastN) {
        $word = VerdictWord $r.v
        $extra = @()
        foreach ($f in "date", "days_per_second", "ref_warnings", "foreign_exceptions", "dialogs_closed") { if ($r.v -match "\b$f=([^,)\s]+)") { if ($f -in "date", "days_per_second" -or $Matches[1] -ne "0") { $extra += "$f=$($Matches[1])" } } }
        if ($word -in "rejected", "unknown", "harness-error", "failed-start", "exited", "timeout", "no-load") { $extra = @($(if ($r.v.Length -gt 140) { $r.v.Substring(0, 140) + " ..." } else { $r.v })) }
        Write-Output ("  {0}  {1,-14} {2}{3}  slot {4}{5}  {6}" -f $r.t.Substring(11), $word, $r.who, $(if ($r.inst) { " [$($r.inst)]" } else { "" }), $r.slot, $(if ($r.wall) { ", $($r.wall) s" } else { "" }), ($extra -join " "))
    }
}
if ($Status) { ShowStatus; exit 0 }
# -Check <file>: what the queue would do with a request, without running it.
if ($Check) {
    if (-not (Test-Path -LiteralPath $Check -PathType Leaf)) { Write-Output "game_queue: -Check: no such file: $Check"; exit 4 }
    $cf = (Resolve-Path -LiteralPath $Check).Path
    $root = if ($Roots.Count -gt 0) { @(RootsNow)[0] } elseif ($cf -match '^(.*)\\build-runs\\game-queue\\[^\\]+$') { $Matches[1] } else { $Here }
    $q = ParseRequest $root $cf
    if ($q.error) { RejectBody $root $cf $q | ForEach-Object { Write-Output $_ }; exit 1 }
    $prio = if ((IsCoord $root) -and $q.priority -eq "high") { "coordinator, priority high" } elseif (IsCoord $root) { "coordinator" } else { "lane" }
    Write-Output "ok: label $($q.label) ($prio; root $root)"
    Write-Output "dll: $($q.dllFull)"
    Write-Output "region: $($q.region)"
    Write-Output "city: $($q.city); seconds $($q.seconds); timeout $($q.timeout)$(if ($q.end_date) { "; end_date $($q.end_date)" })$(if ($q.turbo -eq '1') { '; turbo 1' })$(if ($q.turbo_from) { "; turbo_from $($q.turbo_from)" })"
    foreach ($nt in $q.notes) { Write-Output "note: $nt" }
    Write-Output "ini (before the city and the harness keys):"
    $q.iniText -split "`n" | ForEach-Object { Write-Output "  $_" }
    exit 0
}

# ---- The queue --------------------------------------------------------------------------------------------------
New-Item -ItemType Directory -Force $Work | Out-Null
# One queue per instance name.
$single = New-Object System.Threading.Mutex($false, "FC4GameQueue$Suffix")
$mine = $false
try { $mine = $single.WaitOne(0) } catch [System.Threading.AbandonedMutexException] { $mine = $true }
if (-not $mine) { Write-Output "game_queue: another game_queue$(if ($Instance) { " instance '$Instance'" }) is running"; exit 2 }
$script:HoldsDefault = ($Suffix -eq "")

$rootsText = if ($Roots.Count -gt 0) { (RootsNow) -join ", " } else { "main + worktrees under " + (Join-Path $Main ".claude\worktrees") }
Say ("game_queue: slots {0}; roots: {1}; priority: coordinator {2}; instance {3}, pid {4}{5}" -f ($SlotSet -join ','), $rootsText, $Coord,
    $(if ($Instance) { $Instance } else { "default" }), $PID, $(if ($Once) { "; -Once" } else { "" }))
# Stale markers from a queue that stopped mid-run: those requests are still queued and run again. Markers of a run
# another live queue instance is running stay.
foreach ($root in RootsNow) { foreach ($m in @(Markers $root)) { if (-not (MarkerLive $m)) { Remove-Item -LiteralPath $m.file -Force -ErrorAction SilentlyContinue } } }
$running = @{}   # slot -> job
while ($true) {
    try {
    foreach ($s in @($running.Keys)) {
        if ($running[$s].proc.HasExited) {
            try { Finish $running[$s] } catch { Say "finish error: $($_.Exception.Message)" }
            foreach ($c in $running[$s].claims) { Unclaim $c }   # after the request moved to done\
            $running.Remove($s)
        }
    }
    $mineReqs = @($running.Values | ForEach-Object { $_.reqFile.ToLower() })
    $w = @(Waiting $mineReqs)
    $pending = $w.Count
    foreach ($c in $w) {
        if ($c.young -or -not $c.head) { continue }
        # A normal request waits until its root runs nothing (here or in another queue instance).
        if ($c.prio -ne 0 -and -not $c.parallel -and ($c.rootBusy -or @($running.Values | Where-Object { SamePath $_.root $c.root }).Count -gt 0)) { continue }
        # Two runs of one root with the same label would share their result files.
        $lab = PeekLabel $c.req.FullName
        if (@($running.Values | Where-Object { (SamePath $_.root $c.root) -and $_.label -eq $lab }).Count -gt 0) { continue }
        $free = @($SlotSet | Where-Object { -not $running.ContainsKey($_) -and -not (SlotBusy $_) -and -not (HeldElsewhere (SlotClaimName $_)) })
        if ($free.Count -eq 0) { break }
        $reqClaim = TryClaim (ReqClaimName $c.req.FullName)
        if (-not $reqClaim) { continue }   # another queue instance took it
        $slotClaim = $null; $slot = 0
        foreach ($s in $free) { $slotClaim = TryClaim (SlotClaimName $s); if ($slotClaim) { $slot = $s; break } }
        if (-not $slotClaim) { Unclaim $reqClaim; break }
        if (-not (Test-Path -LiteralPath $c.req.FullName)) { Unclaim $slotClaim; Unclaim $reqClaim; continue }   # finished elsewhere meanwhile
        try {
            $job = @(StartJob $c.root $c.req $slot $c.prio) | Where-Object { $_ -is [hashtable] } | Select-Object -First 1
            if ($job) { $job.claims = @($slotClaim, $reqClaim); $running[$slot] = $job } else { Unclaim $slotClaim; Unclaim $reqClaim }
        }
        catch {
            Say "start error for $($c.req.FullName): $($_.Exception.Message)"
            try { Reject $c.root $c.req.FullName @{ label = [IO.Path]::GetFileNameWithoutExtension($c.req.Name); error = "the queue could not start it: $($_.Exception.Message)"; lines = @(); errLine = 0 } } catch { }
            Unclaim $slotClaim; Unclaim $reqClaim
        }
    }
    if ($Once -and $pending -eq 0 -and $running.Count -eq 0) { break }
    } catch { Say "queue error (continuing): $($_.Exception.Message)" }
    Start-Sleep -Seconds $PollSeconds
}
Say "game_queue: -Once: nothing left, exiting"
$single.ReleaseMutex()
