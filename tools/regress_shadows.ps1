# Helpers for regress.ps1. No staging or game side effects when dot-sourced.
function Split-RegressShadows([string]$Ini, [int]$Count) {
    # Seconds projected to the 2167 simulated days of the 2032 house horizon.
    # split11-r2-cost-* isolated probes, same DLL 48533741: traffic 27.781 s / 157 days;
    # resources reached 2026-07-01 in 15.625 s but took 151.734 s more to quit;
    # project only the simulation interval (157 days), then retain that fixed tail.
    # Its final date overshot during shutdown, so final days/s is unsuitable here.
    # Residential uses the measured
    # ~65 s simulation time of split11-s3-fixed group 1; remaining weights are
    # train196 timestamp proxies until isolated measurements replace them.
    # Resource family cost is charged once to scres_file, with its peers kept together.
    $cost = @{ residential=65.0; development=66.596; demand=42.5; pollution=62.6;
        parks=24.575; dbpf=0.0; weather=0.0; scres=0.0; scres_file=367.4;
        power=10.437; landvalue=8.849; flammability=8.432;
        traffic=383.5; occmgr=5.094; automata=4.417; lotconf=4.321; simgrid=4.036;
        plumbing=3.697; netocc=3.261; history=3.028; overlays=2.663; advisors=2.395;
        terrain=2.216; flora=2.132 }
    # Keep stateful families and their configuration in one process. In particular,
    # followed occupants, property/city-detail readers and lot projections share objects;
    # the resource cache readers share state with the weather resource lookup.
    $families = @{
        occmgr='occupants'; propmgr='occupants'; citydetail='occupants'; lotrec='occupants';
        lotconf='occupants'; lotcfg='occupants'; netocc='occupants';
        automata='automata'; automata_owners='automata';
        weather='resources'; scres_file='resources'; scres='resources'; dbpf='resources';
        overlays='grids'; simgrid='grids'
    }
    $common = @(); $units = [ordered]@{}
    foreach ($line in ($Ini -split "`r?`n")) {
        if ($line -notmatch '^\s*([a-z0-9_]+)\s*=') { continue }
        $key = $Matches[1]
        if ($key -eq 'disable') { $common += $line; continue }
        $owner = $key -replace '_shadow.*$', '' -replace '_fixture$', '' -replace '_constructed$', ''
        $family = if ($families.ContainsKey($owner)) { $families[$owner] } else { $owner }
        if (-not $units.Contains($family)) { $units[$family] = @{ name=$family; lines=@(); cost=0.0; owners=@{} } }
        $u = $units[$family]; $u.lines += $line
        if (-not $u.owners.ContainsKey($owner)) {
            $u.owners[$owner] = $true
            $u.cost += $(if ($cost.ContainsKey($owner)) { $cost[$owner] } else { 1.0 })
        }
    }
    if ($Count -gt $units.Count) { throw "-ShadowSplit $Count exceeds $($units.Count) independent shadow families" }
    $groups = @(for ($i=0; $i -lt $Count; $i++) { @{ index=$i; cost=0.0; lines=@($common); families=@() } })
    foreach ($u in ($units.Values | Sort-Object @{Expression={$_.cost};Descending=$true}, @{Expression={$_.name}})) {
        $g = $groups | Sort-Object @{Expression={$_.cost}}, @{Expression={$_.index}} | Select-Object -First 1
        $g.lines += $u.lines; $g.cost += $u.cost; $g.families += $u.name
    }
    foreach ($g in $groups) { [pscustomobject]@{ Ini=($g.lines -join "`n"); Cost=$g.cost; Families=($g.families -join ',') } }
}

function Join-RegressShadowLogs([string]$Out, [string]$Tag, $Names, $Results, $QueueInfo = @{}) {
    $path = Join-Path $Out "regress-$Tag-shadows.log"
    $encoding = New-Object Text.UTF8Encoding($false)
    $writer = New-Object IO.StreamWriter($path, $false, $encoding)
    $worst = 'clean'; $rank = @{ clean=0; warnings=1; problems=2; failed=3; stalled=4; crash=4; blocked=5; NONE=6 }
    try {
        foreach ($name in $Names) {
            $writer.WriteLine("=== shadow group $name ===")
            $log = Join-Path $Out "regress-$Tag-$name.log"
            $v = 'NONE'
            if (Test-Path -LiteralPath $log) {
                foreach ($line in [IO.File]::ReadLines($log)) {
                    $writer.WriteLine($line)
                    if ($line -match 'AUTOTEST VERDICT: ([a-z]+)') { $v = $Matches[1] }
                }
            }
            if (($Results[$name] -join "`n") -match 'BLOCKED|FAIL|CRASH') { $v = 'NONE' }
            if (-not $rank.ContainsKey($v)) { $rank[$v] = 3 }
            foreach ($line in $QueueInfo[$name]) {
                if ($line -match '^verdict: (\S+)') {
                    $qv = $Matches[1]
                    if (-not $rank.ContainsKey($qv)) { $rank[$qv] = 3 }
                    if ($rank[$qv] -gt $rank[$v]) { $v = $qv }
                }
            }
            if ($rank[$v] -gt $rank[$worst]) { $worst = $v }
        }
    } finally { $writer.Dispose() }
    return $worst
}
