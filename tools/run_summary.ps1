<#
Summarise a FreeCity4.log from an in-game run (coordinator tooling): the verdict, warning counts, the city's money
over the run, and for every subsystem its month lines (count, "enabled" values, first and last line) and shadow
lines (how many compared clean).

Usage:
  powershell -File tools\run_summary.ps1 -Log build-runs\run69-soak2.log [-Width 220]

Counts that should be 0 in a good run: foreign exceptions, watchdog stalls, reference warnings, Rust panics,
structured exceptions. The x87 control-word warning at start-up is expected (foundations §18.2) and is listed apart.
#>
param(
    [Parameter(Mandatory = $true)][string]$Log,
    [int]$Width = 220
)
$ErrorActionPreference = "Stop"
if (-not (Test-Path $Log)) { Write-Output "no such log: $Log"; exit 2 }

function Body([string]$line) {
    # Drop the "date time [pid N tid M] " prefix.
    $i = $line.IndexOf('] ')
    $b = if ($i -ge 0) { $line.Substring($i + 2) } else { $line }
    if ($b.Length -gt $Width) { $b = $b.Substring(0, $Width) + " ..." }
    return $b
}
function Count([string]$pattern) { return @(Select-String -Path $Log -Pattern $pattern -CaseSensitive).Count }

$verdict = Select-String -Path $Log -Pattern "AUTOTEST VERDICT: .*" -CaseSensitive | Select-Object -Last 1
Write-Output ("verdict:   " + $(if ($verdict) { Body $verdict.Line } else { "(none)" }))
$first = (Select-String -Path $Log -Pattern "^\d{4}-\d\d-\d\d \d" -CaseSensitive | Select-Object -First 1).Line
$lastBeat = Select-String -Path $Log -Pattern "heartbeat \(periodic\): (\S+)" -CaseSensitive | Select-Object -Last 1
if ($lastBeat) { Write-Output ("last date: " + $lastBeat.Matches[0].Groups[1].Value + "   (log " + $first.Substring(0, 19) + " to " + $lastBeat.Line.Substring(0, 19) + ")") }

Write-Output ""
Write-Output "problems (all should be 0):"
$checks = [ordered]@{
    "foreign first-chance exceptions" = "first-chance exception OUTSIDE FreeCity4\.dll"
    "watchdog stalls"                 = "WATCHDOG: main thread.*stalled"
    "reference warnings"              = "WARNING: (released last reference|over-release)"
    "Rust panics"                     = "Rust PANIC"
    "structured exceptions"           = "structured exception in"
    "shadow INCOMPLETE lines"         = "shadow.*INCOMPLETE"
}
foreach ($k in $checks.Keys) { Write-Output ("  {0,-34} {1}" -f $k, (Count $checks[$k])) }
$fpu = @(Select-String -Path $Log -Pattern "WARNING.*(control word changed|not at 24-bit)" -CaseSensitive).Count
$other = @(Select-String -Path $Log -Pattern "WARNING" -CaseSensitive | Where-Object {
        $_.Line -notmatch "control word changed|not at 24-bit|released last reference|over-release|AUTOTEST VERDICT" })
Write-Output ("  {0,-34} {1}" -f "other WARNING lines", $other.Count)
$other | Select-Object -First 5 | ForEach-Object { Write-Output ("    " + (Body $_.Line)) }
Write-Output ("  (expected: {0} x87 start-up warning line(s), foundations 18.2; the plugin re-asserts 24-bit)" -f $fpu)

$obs = @(Select-String -Path $Log -Pattern "observe month \S+: date=(\S+) julian=\d+ funds=(-?\d+) income=(-?\d+) expense=(-?\d+)" -CaseSensitive)
if ($obs.Count -gt 0) {
    $a = $obs[0].Matches[0].Groups; $z = $obs[$obs.Count - 1].Matches[0].Groups
    Write-Output ""
    Write-Output ("money ({0} observed months): {1} funds {2} (income {3}, expense {4})  ->  {5} funds {6} (income {7}, expense {8})" -f
        $obs.Count, $a[1].Value, $a[2].Value, $a[3].Value, $a[4].Value, $z[1].Value, $z[2].Value, $z[3].Value, $z[4].Value)
}

# Month status lines: "<name> month:", "<name> month (<part>):" or "<name> month yyyy-mm:" (crime).
$months = Select-String -Path $Log -Pattern "\] ([a-z][a-z ]*?) month( \([a-z ]+\))?( \d{4}-\d\d)?:" -CaseSensitive
$groups = $months | Group-Object { $_.Matches[0].Groups[1].Value + $_.Matches[0].Groups[2].Value }
if ($groups) {
    Write-Output ""
    Write-Output "month lines:"
    foreach ($g in ($groups | Sort-Object Name)) {
        $en = $g.Group | ForEach-Object { if ($_.Line -match "\benabled=(\d)") { $Matches[1] } } | Group-Object | ForEach-Object { "enabled=$($_.Name) x$($_.Count)" }
        Write-Output ("  {0,-28} {1,5} lines  {2}" -f $g.Name, $g.Count, ($en -join ", "))
        Write-Output ("    first: " + (Body $g.Group[0].Line))
        if ($g.Count -gt 1) { Write-Output ("    last:  " + (Body $g.Group[$g.Count - 1].Line)) }
    }
}

# Shadow comparison lines: any "<name> shadow ... <date> ...: ... mismatches=N" line (budget and demand per day or
# month, "pollution shadow month to <date>" and its day lines, "protection shadow (fire) <date>").
$shadows = Select-String -Path $Log -Pattern "\] ([a-z]+) shadow [^:]*\d{4}-\d\d-\d\d[^:]*:.*mismatches=" -CaseSensitive
$sg = $shadows | Group-Object { $_.Matches[0].Groups[1].Value }
if ($sg) {
    Write-Output ""
    Write-Output "shadow lines:"
    foreach ($g in $sg) {
        $clean = @($g.Group | Where-Object { $_.Line -match "mismatches=0\b" -and $_.Line -notmatch "mismatches=[1-9]" }).Count
        Write-Output ("  {0,-12} {1,5} lines, {2} with every mismatch count 0" -f $g.Name, $g.Count, $clean)
        $bad = $g.Group | Where-Object { $_.Line -match "mismatches=[1-9]" } | Select-Object -First 2
        foreach ($b in $bad) { Write-Output ("    differs: " + (Body $b.Line)) }
    }
}

$shut = Select-String -Path $Log -Pattern "\] [a-zA-Z ]+ Shutdown( \(|$)" -CaseSensitive
if ($shut) {
    Write-Output ""
    Write-Output ("shutdown lines: " + (($shut | ForEach-Object { (Body $_.Line) -replace ' \(.*$', '' } | Select-Object -Unique) -join "; "))
}
