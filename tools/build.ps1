# FreeCity4 symbol-port + decompile pipeline.
#   .\tools\build.ps1            # match, merge, apply names in Ghidra, export decompilation
#   .\tools\build.ps1 -SkipExport
# Prereqs: Ghidra 12.1.x at C:\tools\ghidra_12.1.4_PUBLIC, JDK 21+, Python with pefile + capstone,
# extracted KillaVolt packages under external\extracted, project copy under ghidra\project.
param([switch]$SkipExport, [switch]$Refeature)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
if (-not $env:JAVA_HOME) { $env:JAVA_HOME = "C:\Program Files\Java\jdk-26.0.1" }
$ghidra = "C:\tools\ghidra_12.1.4_PUBLIC\support\analyzeHeadless.bat"
$gdt = Join-Path $root "external\sc4-ghidra-symbols"

Push-Location $root
try {
    if ($Refeature -or -not (Test-Path re\symport\win_features.pkl)) { python tools\symport\features.py }
    if ($Refeature -or -not (Test-Path re\symport\vtables.pkl)) { python tools\symport\vtables.py }
    python tools\symport\match.py
    python tools\symport\merge_names.py

    $env:MAXMEM = "12G"
    & $ghidra ghidra\project FreeCity4 -process "SimCity 4.exe" -noanalysis -scriptPath ghidra\scripts `
        -postScript FC4ApplyNames.java (Resolve-Path re\names_windows.tsv) "$gdt\SimCity4.gdt" "$gdt\Lua5.0.gdt" `
        "$gdt\Gimex.gdt" "$gdt\DirectX7.gdt" *> ghidra\logs\apply.log
    Select-String ghidra\logs\apply.log -Pattern "renamed=" | ForEach-Object { $_.Line }

    if (-not $SkipExport) {
        if (Test-Path decomp) { Remove-Item -Recurse -Force decomp }
        $env:MAXMEM = "16G"
        & $ghidra ghidra\project FreeCity4 -process "SimCity 4.exe" -noanalysis -readOnly -scriptPath ghidra\scripts `
            -postScript FC4ExportDecomp.java (Join-Path $root decomp) *> ghidra\logs\export.log
        Select-String ghidra\logs\export.log -Pattern "wrote " | ForEach-Object { $_.Line }
    }
}
finally { Pop-Location }
