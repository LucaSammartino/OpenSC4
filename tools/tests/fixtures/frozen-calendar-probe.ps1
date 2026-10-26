param([string]$Source, [string]$LogFile, [int]$Limit, [int]$Seconds)
$ErrorActionPreference = 'Stop'
$tokens = $null; $errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($Source, [ref]$tokens, [ref]$errors)
if ($errors) { throw ($errors | Out-String) }
foreach ($fn in $ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq 'FrozenCalendar' }, $false)) {
    Invoke-Expression $fn.Extent.Text
}
$text = [IO.File]::ReadAllText($LogFile)
$state = @{ date = $null; since = $null }
$t0 = Get-Date
$first = FrozenCalendar $text $state $t0 $Limit
$later = FrozenCalendar $text $state $t0.AddSeconds($Seconds) $Limit
[pscustomobject]@{ first = $first; later = $later; date = $state.date } | ConvertTo-Json -Compress
