<#
Run queue for in-game test requests (coordinator tooling). Lanes drop small request files into their own worktree;
this poller runs each request in a free game slot (tools\ingame_run.ps1 -Slot N) with that worktree's DLL and writes
the results back into the same worktree. The user doc for lanes is docs\harness.md.

Usage (the coordinator starts it detached and leaves it running):
  powershell -File tools\game_queue.ps1 [-Slots 3] [-PollSeconds 5] [-Once] [-Roots <dir>,<dir>]

  -Slots N    games at once (default 3; each slot is one game, build-runs\sc4slots\slotN of the main checkout)
  -Once       run what is queued now (and what arrives meanwhile), then exit when nothing is pending or running
  -Roots      the checkouts to scan (default: the main checkout and every worktree in .claude\worktrees\)

Per root: <root>\build-runs\game-queue\*.req, oldest first, one at a time (submission order); different roots run
in parallel. Results: <root>\build-runs\game-results\<label>.log (the plugin log), <label>.verdict (first line
"verdict: <word>", then the harness lines and the run summary), <label>.running while it runs, pictures
<label>-*.png when a run ended badly; the request moves to build-runs\game-queue\done\. A request that cannot be run
(bad label, a DLL path outside the worktree, unknown key, ...) gets "verdict: rejected" and moves to done\ too.

Request format (key = value lines, '#' comments; every line after "[ini]" goes into FreeCity4.ini as is):
  label    = power-check-1          letters, digits, '.', '_', '-' (default: the file name)
  dll      = engine\build-x86\FreeCity4.dll   relative to the worktree (default); the .pdb next to it is copied too
  city     = Konradshohe            a city of the FC4 Test region (default Konradshohe)
  seconds  = 150                    game time after the city load, in real seconds (the plugin's autotest_seconds);
                                    with end_date it is the timeout for reaching the date
  timeout  = 300                    hard limit for the whole run in real seconds (default seconds + 150)
  end_date = 2028-01-01             the run ends at this game date (autotest_end_date)
  turbo    = 1                      our simulator's turbo clock from the start (sim_turbo = 1)
  turbo_from = 2026-04-01           the turbo clock from this game date (a script action; keeps the first months,
                                    where the plugin switches popups off, at normal speed)
  [ini]
  enable = power
  ...
In the [ini] part, autoload_city is ignored (city = sets it) and script = <path> is taken relative to the worktree.
The queue adds popup_dialogs = 0, autotest_unpause = 2 and autotest_end = quit unless the [ini] part sets them.
#>
param(
    [ValidateRange(1, 9)][int]$Slots = 3,
    [int]$PollSeconds = 5,
    [switch]$Once,
    [string[]]$Roots = @()
)
$ErrorActionPreference = "Stop"
$Run = Join-Path $PSScriptRoot "ingame_run.ps1"
$Sum = Join-Path $PSScriptRoot "run_summary.ps1"
$Here = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$i = $Here.IndexOf("\.claude\worktrees\", [StringComparison]::OrdinalIgnoreCase)
$Main = if ($i -gt 0) { $Here.Substring(0, $i) } else { $Here }
$Out = Join-Path $Here "build-runs"
$Work = Join-Path $Out "game-queue-work"
$QueueLog = Join-Path $Out "game-queue.log"
$DocsRegion = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "SimCity 4\Regions\FC4 Test"
$Utf8 = New-Object Text.UTF8Encoding($false)
New-Item -ItemType Directory -Force $Work | Out-Null

function Say([string]$msg) {
    $line = "{0} {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $msg
    Write-Host $line   # not Write-Output: Say runs inside functions whose output is their return value
    try { [IO.File]::AppendAllText($QueueLog, $line + "`r`n", $Utf8) } catch { }
}
function WriteText([string]$path, [string]$text) { [IO.File]::WriteAllText($path, $text, $Utf8) }
function QuoteArg([string]$a) { if ($a -eq "") { return '""' }; if ($a -notmatch '[\s"]') { return $a }; return '"' + ($a -replace '(\\*)"', '$1$1\"' -replace '(\\+)$', '$1$1') + '"' }
function RootsNow {
    if ($Roots.Count -gt 0) { return @($Roots | ForEach-Object { [IO.Path]::GetFullPath($_) }) }
    $r = @($Main)
    $wt = Join-Path $Main ".claude\worktrees"
    if (Test-Path $wt) { $r += @(Get-ChildItem -LiteralPath $wt -Directory | ForEach-Object { $_.FullName }) }
    return $r
}
# A slot another harness run holds (regress.ps1 -Parallel, a manual ingame_run -Slot N): ingame_run's user-dir mutex.
function SlotBusy([int]$n) {
    $sr = if ($env:FC4_SLOT_ROOT) { $env:FC4_SLOT_ROOT } else { Join-Path $Main "build-runs\sc4slots" }
    $dir = [IO.Path]::GetFullPath((Join-Path $sr "slot$n"))
    $m = New-Object System.Threading.Mutex($false, ("FC4UserDir-" + ($dir.TrimEnd('\').ToLower() -replace '[^a-z0-9]', '_')))
    try { $got = $false; try { $got = $m.WaitOne(0) } catch [System.Threading.AbandonedMutexException] { $got = $true }; if ($got) { $m.ReleaseMutex() }; return -not $got }
    finally { $m.Dispose() }
}
function RootName([string]$root) { if ($root.TrimEnd('\') -eq $Main.TrimEnd('\')) { return "main" } else { return (Split-Path $root -Leaf) } }
function Inside([string]$root, [string]$path) { return $path.StartsWith($root.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase) }

# Parse and check one request. Returns a hashtable; .error is set when the request must be rejected.
function ParseRequest([string]$root, [string]$file) {
    $q = @{ label = [IO.Path]::GetFileNameWithoutExtension($file); dll = "engine\build-x86\FreeCity4.dll"; city = "Konradshohe";
            seconds = 150; timeout = $null; end_date = ""; turbo = ""; turbo_from = ""; ini = @(); notes = @(); error = $null }
    $inIni = $false; $n = 0
    foreach ($line in @(Get-Content -LiteralPath $file)) {
        $n++
        if ($inIni) { $q.ini += $line; continue }
        $t = $line.Trim()
        if ($t -eq "" -or $t.StartsWith("#")) { continue }
        if ($t -eq "[ini]") { $inIni = $true; continue }
        if ($t -notmatch '^([a-z_]+)\s*=\s*(.*)$') { $q.error = "line ${n}: not 'key = value': $t"; return $q }
        $k = $Matches[1]; $v = $Matches[2].Trim()
        switch ($k) {
            "label" { $q.label = $v }
            "dll" { $q.dll = $v }
            "city" { $q.city = $v }
            "seconds" { if ($v -notmatch '^\d+$' -or [int]$v -lt 10 -or [int]$v -gt 7200) { $q.error = "line ${n}: seconds must be 10..7200"; return $q }; $q.seconds = [int]$v }
            "timeout" { if ($v -notmatch '^\d+$' -or [int]$v -lt 30 -or [int]$v -gt 10800) { $q.error = "line ${n}: timeout must be 30..10800"; return $q }; $q.timeout = [int]$v }
            "end_date" { if ($v -notmatch '^\d{4}-\d\d-\d\d$') { $q.error = "line ${n}: end_date must be YYYY-MM-DD"; return $q }; $q.end_date = $v }
            "turbo" { if ($v -notin "0", "1") { $q.error = "line ${n}: turbo must be 0 or 1"; return $q }; $q.turbo = $v }
            "turbo_from" { if ($v -notmatch '^\d{4}-\d\d-\d\d$') { $q.error = "line ${n}: turbo_from must be YYYY-MM-DD"; return $q }; $q.turbo_from = $v }
            default { $q.error = "line ${n}: unknown key '$k' (label, dll, city, seconds, timeout, end_date, turbo, turbo_from, [ini])"; return $q }
        }
    }
    if ($q.label -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$') { $q.error = "label '$($q.label)': use letters, digits, '.', '_', '-' (no path characters, at most 64)"; return $q }
    if ($q.city -notmatch '^[A-Za-z0-9 ._()-]+$' -or -not (Test-Path -LiteralPath (Join-Path $DocsRegion "City - $($q.city).sc4"))) { $q.error = "city '$($q.city)': no such city in the FC4 Test region"; return $q }
    $dllFull = [IO.Path]::GetFullPath($(if ([IO.Path]::IsPathRooted($q.dll)) { $q.dll } else { Join-Path $root $q.dll }))
    if (-not (Inside $root $dllFull)) { $q.error = "dll '$($q.dll)' is outside the worktree"; return $q }
    if ($dllFull -notmatch '\.dll$' -or -not (Test-Path -LiteralPath $dllFull -PathType Leaf)) { $q.error = "dll '$($q.dll)': no such file ($dllFull)"; return $q }
    $q.dllFull = $dllFull
    if (-not $q.timeout) { $q.timeout = $q.seconds + 150 }
    # The [ini] part: autoload_city comes from city =, a script path is relative to the worktree.
    $ini = @()
    foreach ($line in $q.ini) {
        if ($line -match '^\s*autoload_city\s*=') { $q.notes += "ignored the [ini] line '$($line.Trim())' (city = sets autoload_city)"; continue }
        if ($line -match '^(\s*script\s*=\s*)(.+?)\s*$') {
            $sp = [IO.Path]::GetFullPath($(if ([IO.Path]::IsPathRooted($Matches[2])) { $Matches[2] } else { Join-Path $root $Matches[2] }))
            if (-not (Inside $root $sp) -or -not (Test-Path -LiteralPath $sp -PathType Leaf)) { $q.error = "script '$($Matches[2])': no such file in the worktree"; return $q }
            if ($q.turbo_from) { $q.error = "turbo_from writes its own script; use 'turbo on' in your script instead"; return $q }
            $ini += "script = $sp"; continue
        }
        $ini += $line
    }
    $text = $ini -join "`n"
    foreach ($kv in @(@("popup_dialogs", "0"), @("autotest_unpause", "2"), @("autotest_end", "quit"))) {
        if ($text -notmatch "(?m)^\s*$($kv[0])\s*=") { $text += "`n$($kv[0]) = $($kv[1])" }
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
    if ($v -match '^BLOCKED|^BAD') { return "harness-error" }
    return "unknown"
}
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
    $log = Join-Path $Out "$($job.runLabel).log"
    $body = @("verdict: $word", "line: $v")
    if ($held) { $body += "held: " + ($held.Trim() -replace '^HELD: ', '') }
    foreach ($dl in @($lines | Where-Object { $_ -match '^\s+DIALOG \(' })) { $body += "dialog: " + ($dl.Trim() -replace '^DIALOG ', '') }
    $body += @("request: $($job.reqName) (queued $($job.queued.ToString('yyyy-MM-dd HH:mm:ss')), started $($job.started.ToString('HH:mm:ss')) in slot $($job.slot), wall $wall s)",
        "dll: $($job.q.dllFull)", "city: $($job.q.city); seconds $($job.q.seconds); timeout $($job.q.timeout)$(if ($job.q.end_date) { "; end_date $($job.q.end_date)" })$(if ($job.q.turbo -eq '1') { '; turbo 1' })$(if ($job.q.turbo_from) { "; turbo_from $($job.q.turbo_from)" })")
    foreach ($nt in $job.q.notes) { $body += "note: $nt" }
    $body += @("", "harness:") + @($lines | ForEach-Object { "  $_" })
    if (Test-Path $log) {
        Copy-Item -LiteralPath $log -Destination (Join-Path $res "$($job.label).log") -Force
        $s = & powershell -NoProfile -ExecutionPolicy Bypass -File $Sum -Log $log -Width 200 2>&1 | ForEach-Object { "$_" }
        $body += @("", "summary:") + @($s | ForEach-Object { "  $_" })
    } else { $body += @("", "(no plugin log)") }
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
function Reject([string]$root, [string]$reqFile, $q) {
    $res = Join-Path $root "build-runs\game-results"
    New-Item -ItemType Directory -Force $res | Out-Null
    $label = if ($q.label -match '^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$') { $q.label } else { [IO.Path]::GetFileNameWithoutExtension($reqFile) -replace '[^A-Za-z0-9._-]', '_' }
    WriteText (Join-Path $res "$label.verdict") ("verdict: rejected`r`nline: REJECTED: $($q.error)`r`nrequest: $(Split-Path $reqFile -Leaf)`r`n")
    MoveDone $root $reqFile
    Say ("reject {0}/{1}: {2}" -f (RootName $root), (Split-Path $reqFile -Leaf), $q.error)
}
function StartJob([string]$root, [IO.FileInfo]$req, [int]$slot) {
    $q = ParseRequest $root $req.FullName
    if ($q.error) { Reject $root $req.FullName $q; return $null }
    $runLabel = "q-{0}-{1}" -f (RootName $root), $q.label
    $iniFile = Join-Path $Work "$runLabel.ini"
    WriteText $iniFile $q.iniText
    $outFile = Join-Path $Work "$runLabel.out"; $errFile = Join-Path $Work "$runLabel.err"
    $argv = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $Run, "-Slot", "$slot", "-Dll", $q.dllFull, "-City", $q.city,
              "-IniFile", $iniFile, "-AfterLoadSeconds", "$($q.seconds)", "-Seconds", "$($q.timeout)", "-Label", $runLabel)
    if ($q.end_date) { $argv += @("-EndDate", $q.end_date) }
    if ($q.turbo -eq "1") { $argv += "-Turbo" }
    if ($q.turbo_from) { $argv += @("-TurboFrom", $q.turbo_from) }
    $res = Join-Path $root "build-runs\game-results"
    New-Item -ItemType Directory -Force $res | Out-Null
    Remove-Item -LiteralPath (Join-Path $res "$($q.label).verdict") -Force -ErrorAction SilentlyContinue
    WriteText (Join-Path $res "$($q.label).running") ("running in slot $slot since $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') (request $($req.Name))`r`n")
    $proc = Start-Process -FilePath "powershell" -ArgumentList (($argv | ForEach-Object { QuoteArg $_ }) -join " ") -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $outFile -RedirectStandardError $errFile
    $null = $proc.Handle   # keeps the exit code readable
    Say ("start {0}/{1} in slot {2} (dll {3})" -f (RootName $root), $q.label, $slot, $q.dllFull)
    return @{ root = $root; reqFile = $req.FullName; reqName = $req.Name; queued = $req.LastWriteTime; started = (Get-Date); slot = $slot;
              label = $q.label; runLabel = $runLabel; q = $q; proc = $proc; outFile = $outFile; errFile = $errFile }
}

# One queue at a time.
$single = New-Object System.Threading.Mutex($false, "FC4GameQueue")
$mine = $false
try { $mine = $single.WaitOne(0) } catch [System.Threading.AbandonedMutexException] { $mine = $true }
if (-not $mine) { Write-Output "game_queue: another game_queue is running"; exit 2 }

$rootsText = if ($Roots.Count -gt 0) { (RootsNow) -join ", " } else { "main + worktrees under " + (Join-Path $Main ".claude\worktrees") }
Say ("game_queue: {0} slot(s), roots: {1}{2}" -f $Slots, $rootsText, $(if ($Once) { "; -Once" } else { "" }))
# Stale markers from a queue that stopped mid-run: those requests are still queued and run again.
foreach ($root in RootsNow) { Remove-Item -Path (Join-Path $root "build-runs\game-results\*.running") -Force -ErrorAction SilentlyContinue }
$running = @{}   # slot -> job
while ($true) {
    try {
    foreach ($s in @($running.Keys)) { if ($running[$s].proc.HasExited) { try { Finish $running[$s] } catch { Say "finish error: $($_.Exception.Message)" }; $running.Remove($s) } }
    # The oldest request of each root that has none running; roots in the order of their oldest request.
    $busy = @($running.Values | ForEach-Object { $_.root.ToLower() })
    $heads = @()
    $pending = 0
    foreach ($root in RootsNow) {
        $qdir = Join-Path $root "build-runs\game-queue"
        if (-not (Test-Path -LiteralPath $qdir)) { continue }
        $reqs = @(Get-ChildItem -LiteralPath $qdir -Filter "*.req" -File -ErrorAction SilentlyContinue | Where-Object { $_.Extension -eq ".req" } | Sort-Object LastWriteTime, Name)
        $pending += $reqs.Count
        if ($reqs.Count -eq 0 -or $busy -contains $root.ToLower()) { continue }
        if (((Get-Date) - $reqs[0].LastWriteTime).TotalSeconds -lt 2) { continue }   # still being written
        $heads += @{ root = $root; req = $reqs[0]; i = $heads.Count }
    }
    foreach ($h in ($heads | Sort-Object { $_.req.LastWriteTime }, { $_.i })) {
        $free = @(1..$Slots | Where-Object { -not $running.ContainsKey($_) -and -not (SlotBusy $_) })
        if ($free.Count -eq 0) { break }
        try { $job = @(StartJob $h.root $h.req $free[0]) | Where-Object { $_ -is [hashtable] } | Select-Object -First 1; if ($job) { $running[$free[0]] = $job } }
        catch {
            Say "start error for $($h.req.FullName): $($_.Exception.Message)"
            try { Reject $h.root $h.req.FullName @{ label = [IO.Path]::GetFileNameWithoutExtension($h.req.Name); error = "the queue could not start it: $($_.Exception.Message)" } } catch { }
        }
    }
    if ($Once -and $pending -eq 0 -and $running.Count -eq 0) { break }
    } catch { Say "queue error (continuing): $($_.Exception.Message)" }
    Start-Sleep -Seconds $PollSeconds
}
Say "game_queue: -Once: nothing left, exiting"
$single.ReleaseMutex()
