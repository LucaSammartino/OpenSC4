<#
Compare the simulator clock of two runs on the same save (M1 differential), from the CLOCK lines the plugin writes
with `clock_probe = 1` or `2` (engine/plugin/FC4ClockProbe.h).

Usage:
  powershell -File tools\clock_diff.ps1 -A build-runs\runNN-original.log -B build-runs\runMM-ours.log

Two streams are taken from each log after its first "CLOCK load:" line:
  agent  the probe agent's deliveries at the day/week/month/year levels (name, Julian day, counter, "again");
  post   the day/week/month/year messages the simulator posted (name, Julian day, counter).
Each stream is first checked on its own against simulator.md §2.3/§5.2: consecutive days, a week message on
every Monday and only then (counter = (dayOfYear-1)/7+1), month on the 1st (counter = month), year on 1 January
(counter = year), day counter = day of year, and the order day, week, month, year within a day.
Then A and B are compared entry by entry from the first day both streams saw, up to the shorter one. The
interleaving of the two streams, and the pause/hidden/emergency/speed lines, depend on frame timing and on what
was clicked, so they are listed but not compared.
Exit code: 0 if every check passed, 1 otherwise.
#>
param(
    [Parameter(Mandatory = $true)][string]$A,
    [Parameter(Mandatory = $true)][string]$B
)
$ErrorActionPreference = "Stop"
$JulianOf2000 = 2451545
$Epoch = Get-Date -Year 2000 -Month 1 -Day 1 -Hour 0 -Minute 0 -Second 0 -Millisecond 0
$Rank = @{ day = 0; week = 1; month = 2; year = 3 }

function DateOf([long]$j) { $Epoch.AddDays($j - $JulianOf2000) }

function ReadStreams([string]$path) {
    $s = @{ agent = New-Object System.Collections.ArrayList; post = New-Object System.Collections.ArrayList
            control = New-Object System.Collections.ArrayList; load = $null }
    $seenLoad = $false
    foreach ($line in [IO.File]::ReadLines((Resolve-Path $path).Path)) {
        if ($line -notmatch "CLOCK ") { continue }
        if (-not $seenLoad) {
            if ($line -match "CLOCK load: (.*)$") { $seenLoad = $true; $s.load = $Matches[1] }
            elseif ($line -match "CLOCK post (pause|hidden|emergency|speed) d1=(-?\d+)") {
                [void]$s.control.Add("before load: $($Matches[1]) $($Matches[2])")
            }
            continue
        }
        if ($line -match "CLOCK (agent|post) (day|week|month|year) j=(-?\d+) c=(-?\d+)( again)?") {
            [void]$s[$Matches[1]].Add([pscustomobject]@{ name = $Matches[2]; j = [long]$Matches[3]; c = [long]$Matches[4]
                                                          again = [bool]$Matches[5] })
        } elseif ($line -match "CLOCK post (pause|hidden|emergency|speed) d1=(-?\d+)") {
            [void]$s.control.Add("$($Matches[1]) $($Matches[2])")
        }
    }
    return $s
}

function Key($e) { "{0} j={1} c={2}{3}" -f $e.name, $e.j, $e.c, $(if ($e.again) { " again" } else { "" }) }

# The spec's calendar rules for one stream; returns a list of problems.
function CheckStream($list, [string]$what) {
    $bad = New-Object System.Collections.ArrayList
    $prevDay = $null; $lastRank = -1; $lastJ = $null
    $due = @{}  # name -> julian still expected after a day line
    foreach ($e in $list) {
        if ($e.again) {
            if ($e.name -ne "week" -or $lastJ -ne $e.j) { [void]$bad.Add("$what $(Key $e): 'again' not right after its week") }
            continue
        }
        $d = DateOf $e.j
        if ($e.name -eq "day") {
            foreach ($k in $due.Keys) { [void]$bad.Add("$what day $($due[$k]): no $k message") }
            $due = @{}
            if ($null -ne $prevDay -and $e.j -ne $prevDay + 1) { [void]$bad.Add("$what $(Key $e): previous day was $prevDay") }
            if ($e.c -ne $d.DayOfYear) { [void]$bad.Add("$what $(Key $e): counter should be day of year $($d.DayOfYear)") }
            $prevDay = $e.j; $lastRank = 0
            if ($d.DayOfWeek -eq [DayOfWeek]::Monday) { $due["week"] = $e.j }
            if ($d.Day -eq 1) { $due["month"] = $e.j }
            if ($d.DayOfYear -eq 1) { $due["year"] = $e.j }
        } else {
            if ($null -eq $prevDay) { $lastJ = $e.j; continue }  # the stream starts mid-day
            $want = switch ($e.name) { "week" { [math]::Floor(($d.DayOfYear - 1) / 7) + 1 } "month" { $d.Month } "year" { $d.Year } }
            if ($e.j -ne $prevDay) { [void]$bad.Add("$what $(Key $e): not on the current day $prevDay") }
            elseif (-not $due.ContainsKey($e.name)) { [void]$bad.Add("$what $(Key $e): not due on $($d.ToString('yyyy-MM-dd ddd'))") }
            else { $due.Remove($e.name) }
            if ($e.c -ne $want) { [void]$bad.Add("$what $(Key $e): counter should be $want") }
            if ($Rank[$e.name] -le $lastRank) { [void]$bad.Add("$what $(Key $e): out of order within the day") }
            $lastRank = $Rank[$e.name]
        }
        $lastJ = $e.j
    }
    return , $bad
}

function FirstDay($list) { foreach ($e in $list) { if ($e.name -eq "day" -and -not $e.again) { return $e.j } }; return $null }

function Compare-Stream([string]$name, $la, $lb) {
    $fa = FirstDay $la; $fb = FirstDay $lb
    if ($null -eq $fa -or $null -eq $fb) { return @($false, "${name}: no day line in $(if ($null -eq $fa) { 'A' } else { 'B' })") }
    $start = [math]::Max($fa, $fb)
    $ia = 0; while ($ia -lt $la.Count -and -not ($la[$ia].name -eq "day" -and $la[$ia].j -eq $start -and -not $la[$ia].again)) { $ia++ }
    $ib = 0; while ($ib -lt $lb.Count -and -not ($lb[$ib].name -eq "day" -and $lb[$ib].j -eq $start -and -not $lb[$ib].again)) { $ib++ }
    if ($ia -ge $la.Count -or $ib -ge $lb.Count) { return @($false, "${name}: the streams share no day (A from $fa, B from $fb)") }
    $n = [math]::Min($la.Count - $ia, $lb.Count - $ib)
    $days = 0
    for ($k = 0; $k -lt $n; $k++) {
        $x = Key $la[$ia + $k]; $y = Key $lb[$ib + $k]
        if ($x -ne $y) {
            return @($false, "${name}: MISMATCH at entry $k after day $start (+$days days): A '$x' vs B '$y'")
        }
        if ($la[$ia + $k].name -eq "day" -and -not $la[$ia + $k].again) { $days++ }
    }
    $last = $la[$ia + $n - 1].j
    return @($true, ("${name}: identical, {0} entries, {1} days ({2} .. {3})" -f $n, $days,
                     (DateOf $start).ToString("yyyy-MM-dd"), (DateOf $last).ToString("yyyy-MM-dd")))
}

$sa = ReadStreams $A; $sb = ReadStreams $B
$ok = $true
foreach ($p in @(@("A", $A, $sa), @("B", $B, $sb))) {
    $s = $p[2]
    if ($null -eq $s.load) { Write-Output "$($p[0]): no 'CLOCK load:' line in $($p[1]) (clock_probe off, or no city loaded)"; $ok = $false; continue }
    Write-Output ("{0}: {1}; agent {2}, post {3} entries" -f $p[0], $s.load, $s.agent.Count, $s.post.Count)
    foreach ($w in @("agent", "post")) {
        $bad = CheckStream $s[$w] "$($p[0]) $w"
        if ($bad.Count -gt 0) {
            $ok = $false
            Write-Output "  $w stream: $($bad.Count) problem(s) against the spec:"
            $bad | Select-Object -First 10 | ForEach-Object { Write-Output "    $_" }
        } else { Write-Output "  $w stream: follows the spec's calendar rules" }
    }
    $pre = @($s.control | Where-Object { $_ -like "before load: *" } | ForEach-Object { $_.Substring(13) })
    $post = @($s.control | Where-Object { $_ -notlike "before load: *" } | Select-Object -First 12)
    Write-Output ("  control lines before the load: {0}; after: {1}" -f $(if ($pre.Count) { $pre -join ", " } else { "none" }),
                  $(if ($post.Count) { $post -join ", " } else { "none" }))
}
if ($null -ne $sa.load -and $null -ne $sb.load) {
    foreach ($w in @("agent", "post")) {
        $r = Compare-Stream $w $sa[$w] $sb[$w]
        if (-not $r[0]) { $ok = $false }
        Write-Output $r[1]
    }
}
Write-Output $(if ($ok) { "CLOCK DIFF: PASS" } else { "CLOCK DIFF: FAIL" })
exit $(if ($ok) { 0 } else { 1 })
