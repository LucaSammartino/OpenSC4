<# Export under the slot user-dir lock, queued behind the active game before
   reuse. The unique script and completed log must match. Legacy callers also
   take the queue lock; they may safely refuse a run that was already reused. #>
param(
    [Parameter(Mandatory=$true)][string]$HarnessRoot,
    [Parameter(Mandatory=$true)][string]$Output,
    [Parameter(Mandatory=$true)][string]$City,
    [string]$Region = 'FC4 Test',
    [ValidateRange(1,16)][int]$Slot = 6,
    [string]$ExpectedLog,
    [string]$VerdictFile,
    [string]$ScriptPath,
    [ValidateRange(60,7200)][int]$WaitSeconds = 600
)
$ErrorActionPreference = 'Stop'
if ($City -notmatch '^[A-Za-z0-9 ._()-]+$' -or $Region -notmatch '^[A-Za-z0-9 ._()-]+$') { throw 'Invalid name' }
$userDir = [IO.Path]::GetFullPath((Join-Path $HarnessRoot "build-runs/sc4slots/slot$Slot"))
$locks = @()
$sourceLog = Join-Path $userDir 'Plugins/FreeCity4.log'
$limit = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
try {
    $names = @("FC4QueueSlot-$Slot", ('FC4UserDir-' + ($userDir.TrimEnd('\').ToLower() -replace '[^a-z0-9]', '_')))
    if ($ScriptPath) {
        # Do not claim before the intended game has taken its user-dir lock.
        # Its unique script path proves this log belongs to that active run.
        $wanted = $ScriptPath.Replace('/', '\').ToLowerInvariant()
        while ($true) {
            $text = ''
            $reader = $null
            try {
                $stream = [IO.File]::Open($sourceLog, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
                $reader = New-Object IO.StreamReader($stream)
                $text = $reader.ReadToEnd().Replace('/', '\').ToLowerInvariant()
            } catch { } finally { if ($reader) { $reader.Dispose() } }
            if ($text.Contains($wanted)) { break }
            if ([DateTime]::UtcNow -ge $limit) { throw 'Intended run never appeared in the slot log' }
            Start-Sleep -Milliseconds 500
        }
        # Queue behind the active game, BEFORE it finishes. Hold only UserDir:
        # the queue can publish its verdict while a later runner waits for us.
        $names = @($names[1])
        Write-Output "golden snapshot: watching active script; queued for slot $Slot user-dir lock"
    }
    foreach ($name in $names) {
        $mutex = New-Object Threading.Mutex($false, $name)
        $held = $false
        try { $held = $mutex.WaitOne($WaitSeconds * 1000) } catch [Threading.AbandonedMutexException] { $held = $true }
        if (-not $held) { $mutex.Dispose(); throw "Slot $Slot remained busy; no save exported" }
        $locks += $mutex
    }
    if ($VerdictFile) {
        while (-not (Test-Path -LiteralPath $VerdictFile)) {
            if ([DateTime]::UtcNow -ge $limit) { throw 'Queue did not publish the completed run' }
            Start-Sleep -Milliseconds 500
        }
    }
    if (-not $ExpectedLog) { $ExpectedLog = Join-Path $Output 'game.log' }
    if ((Get-FileHash -LiteralPath $sourceLog).Hash -ne (Get-FileHash -LiteralPath $expectedLog).Hash) {
        throw "Slot $Slot was reused before export; no save exported. Retry the run."
    }
    # ingame_run mirrors EVERY source region into the slot's sole FC4 Test
    # folder, regardless of the selected source region's name.
    $source = Join-Path $userDir "Regions/FC4 Test/City - $City.sc4"
    Copy-Item -LiteralPath $source -Destination (Join-Path $Output 'end.sc4')
    Write-Output "golden snapshot: slot $Slot, matched completed log, copied end.sc4"
} finally {
    for ($i=$locks.Count-1; $i -ge 0; $i--) { $locks[$i].ReleaseMutex(); $locks[$i].Dispose() }
}
