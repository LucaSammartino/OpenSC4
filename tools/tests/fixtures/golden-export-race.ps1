param([string]$Root)
$ErrorActionPreference = 'Stop'
$slot = [IO.Path]::GetFullPath((Join-Path $Root 'build-runs/sc4slots/slot6'))
$name = 'FC4UserDir-' + ($slot.TrimEnd('\').ToLower() -replace '[^a-z0-9]', '_')
$mutex = New-Object Threading.Mutex($false, $name)
try {
    if (-not $mutex.WaitOne(10000)) { throw 'test slot busy' }
    $null = New-Item -ItemType Directory -Force (Join-Path $slot 'Plugins'), (Join-Path $slot 'Regions/FC4 Test')
    $text = 'script: loaded ' + (Join-Path $Root 'actions.script')
    [IO.File]::WriteAllText((Join-Path $slot 'Plugins/FreeCity4.log'), $text)
    [IO.File]::WriteAllText((Join-Path $slot 'Regions/FC4 Test/City - C.sc4'), 'original end save')
    Write-Output 'fake run ready'
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    while (-not (Test-Path -LiteralPath (Join-Path $Root 'release'))) {
        if ([DateTime]::UtcNow -gt $deadline) { throw 'test never released run' }
        Start-Sleep -Milliseconds 50
    }
    $mutex.ReleaseMutex()
    [IO.File]::WriteAllText((Join-Path $Root 'expected.log'), $text)
    [IO.File]::WriteAllText((Join-Path $Root 'result.verdict'), 'verdict: clean')
    if (-not $mutex.WaitOne(10000)) { throw 'exporter never released slot' }
    # A queued subsequent runner immediately replaces the source save and log.
    [IO.File]::WriteAllText((Join-Path $slot 'Regions/FC4 Test/City - C.sc4'), 'next run save')
    [IO.File]::WriteAllText((Join-Path $slot 'Plugins/FreeCity4.log'), 'next run log')
    $mutex.ReleaseMutex()
    Write-Output 'fake reuse complete'
} finally { $mutex.Dispose() }
