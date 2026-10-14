<#
Compares two unattended runs month by month from their "observe month" lines (observe = 1): funds, income and
expense, R/C/I population, and the power getters (either power class). Coordinator tooling for soak pairs, for
example the same city with and without one replacement.

Usage:
  powershell -File tools\compare_soaks.ps1 -A <log> -B <log> [-Every 12] [-LabelA ours] [-LabelB orig]

Prints one row every -Every months (and the last common month): date, then A | B for each column. A column
the log does not have (power before 7c94135) prints "-". Ends with each run's verdict and last date.
#>
param(
    [Parameter(Mandatory = $true)][string]$A,
    [Parameter(Mandatory = $true)][string]$B,
    [int]$Every = 12,
    [string]$LabelA = "A",
    [string]$LabelB = "B"
)
$ErrorActionPreference = "Stop"

function Read-Months([string]$path) {
    $rows = [ordered]@{}
    foreach ($m in (Select-String -Path $path -Pattern "\] observe month (\d{4}-\d\d): (.*)$" -CaseSensitive)) {
        $date = $m.Matches[0].Groups[1].Value
        $body = $m.Matches[0].Groups[2].Value
        $r = [ordered]@{ funds = "-"; inc = "-"; exp = "-"; R = "-"; C = "-"; I = "-"; req = "-"; prod = "-"; cons = "-"; pow = "-"; unp = "-" }
        if ($body -match "funds=(-?\d+) income=(-?\d+) expense=(-?\d+)") { $r.funds = $Matches[1]; $r.inc = $Matches[2]; $r.exp = $Matches[3] }
        if ($body -match "population R/C/I=(-?\d+)/(-?\d+)/(-?\d+)") { $r.R = $Matches[1]; $r.C = $Matches[2]; $r.I = $Matches[3] }
        if ($body -match "power req/prod/cons/demand=(-?\d+)/(-?\d+)/(-?\d+)/(-?\d+) consumers powered/unpowered=(-?\d+)/(-?\d+)") {
            $r.req = $Matches[1]; $r.prod = $Matches[2]; $r.cons = $Matches[3]; $r.pow = $Matches[5]; $r.unp = $Matches[6]
        }
        $rows[$date] = $r
    }
    return $rows
}

function Verdict([string]$path) {
    $v = Select-String -Path $path -Pattern "AUTOTEST VERDICT: .*" -CaseSensitive | Select-Object -Last 1
    if ($v) { return $v.Matches[0].Value -replace " \(foreign.*", "" }
    return "(no verdict)"
}

$ra = Read-Months $A
$rb = Read-Months $B
$common = @($ra.Keys | Where-Object { $rb.Contains($_) })
Write-Output ("{0}: {1} observed months, {2}" -f $LabelA, $ra.Count, (Verdict $A))
Write-Output ("{0}: {1} observed months, {2}" -f $LabelB, $rb.Count, (Verdict $B))
Write-Output ("common months: {0}" -f $common.Count)
if ($common.Count -eq 0) { exit 0 }

$cols = @(
    @{ n = "funds"; k = "funds"; w = 9 }, @{ n = "income"; k = "inc"; w = 6 }, @{ n = "expense"; k = "exp"; w = 6 },
    @{ n = "pop R"; k = "R"; w = 6 }, @{ n = "pop C"; k = "C"; w = 5 }, @{ n = "pop I"; k = "I"; w = 5 },
    @{ n = "power req"; k = "req"; w = 6 }, @{ n = "prod"; k = "prod"; w = 5 },
    @{ n = "powered"; k = "pow"; w = 4 }, @{ n = "unpowered"; k = "unp"; w = 4 }
)
$hdr = "{0,-7}" -f "month"
foreach ($c in $cols) { $hdr += " | " + ("{0," + (2 * $c.w + 1) + "}") -f ($c.n) }
Write-Output ""
Write-Output ("columns: {0} / {1}" -f $LabelA, $LabelB)
Write-Output $hdr
$i = 0
foreach ($d in $common) {
    $last = ($i -eq $common.Count - 1)
    if (($i % $Every) -eq 0 -or $last) {
        $line = "{0,-7}" -f $d
        foreach ($c in $cols) {
            $line += " | " + ("{0," + $c.w + "}/{1,-" + $c.w + "}") -f $ra[$d][$c.k], $rb[$d][$c.k]
        }
        Write-Output $line
    }
    $i++
}
