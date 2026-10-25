<# Export a completed queued run only while BOTH slot locks are held and its log
   still matches the queue's copy. A reused slot is an error, never a baseline. #>
param(
    [Parameter(Mandatory=$true)][string]$HarnessRoot,
    [Parameter(Mandatory=$true)][string]$Output,
    [Parameter(Mandatory=$true)][string]$City,
    [string]$Region = 'FC4 Test',
    [ValidateRange(1,16)][int]$Slot = 6
)
$ErrorActionPreference = 'Stop'
if ($City -notmatch '^[A-Za-z0-9 ._()-]+$' -or $Region -notmatch '^[A-Za-z0-9 ._()-]+$') { throw 'Invalid name' }
$userDir = [IO.Path]::GetFullPath((Join-Path $HarnessRoot "build-runs/sc4slots/slot$Slot"))
$locks = @()
try {
    foreach ($name in @("FC4QueueSlot-$Slot", ('FC4UserDir-' + ($userDir.TrimEnd('\').ToLower() -replace '[^a-z0-9]', '_')))) {
        $mutex = New-Object Threading.Mutex($false, $name)
        $held = $false
        try { $held = $mutex.WaitOne(60000) } catch [Threading.AbandonedMutexException] { $held = $true }
        if (-not $held) { $mutex.Dispose(); throw "Slot $Slot remained busy; no save exported" }
        $locks += $mutex
    }
    $sourceLog = Join-Path $userDir 'Plugins/FreeCity4.log'
    $expectedLog = Join-Path $Output 'game.log'
    if ((Get-FileHash -LiteralPath $sourceLog).Hash -ne (Get-FileHash -LiteralPath $expectedLog).Hash) {
        throw "Slot $Slot was reused before export; no save exported. Retry the run."
    }
    $source = Join-Path $userDir "Regions/$Region/City - $City.sc4"
    Copy-Item -LiteralPath $source -Destination (Join-Path $Output 'end.sc4')
    Write-Output "golden snapshot: slot $Slot, matched completed log, copied end.sc4"
} finally {
    for ($i=$locks.Count-1; $i -ge 0; $i--) { $locks[$i].ReleaseMutex(); $locks[$i].Dispose() }
}
