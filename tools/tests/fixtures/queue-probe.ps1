param([string]$Source, [string]$Root, [string]$Regions, [string]$Request)
$ErrorActionPreference = 'Stop'
$tokens = $null; $errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($Source, [ref]$tokens, [ref]$errors)
if ($errors) { throw ($errors | Out-String) }
foreach ($fn in $ast.FindAll({ param($n) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] }, $false)) {
    Invoke-Expression $fn.Extent.Text
}
$DocsRegions = $Regions
$Coord = $Root
$q = ParseRequest $Root $Request
$q.peekParallel = PeekParallel $Request
$q | ConvertTo-Json -Depth 4 -Compress
