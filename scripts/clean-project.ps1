param([switch]$Artifacts)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
if (-not (Test-Path -LiteralPath (Join-Path $projectRoot 'Cargo.toml'))) { throw 'No es el proyecto Rust de Luma.' }

# Únicamente dos directorios generados de este checkout, nunca datos de Luma.
$targets = @('target')
if ($Artifacts) { $targets += 'artifacts' }
foreach ($name in $targets) {
    $candidate = Join-Path $projectRoot $name
    if (-not (Test-Path -LiteralPath $candidate)) { continue }
    $resolved = (Resolve-Path -LiteralPath $candidate).Path
    if (-not [IO.Path]::GetFullPath($candidate).Equals($resolved, [StringComparison]::OrdinalIgnoreCase)) { throw 'El destino no coincide con el directorio generado.' }
    if ((Get-Item -LiteralPath $resolved -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'El destino es un enlace; no se limpia.' }
    $links = @(Get-ChildItem -LiteralPath $resolved -Recurse -Force -Attributes ReparsePoint)
    if ($links.Count) { throw 'Hay enlaces en el directorio generado; revísalos antes de limpiar.' }
}

$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
& $cargo clean --manifest-path (Join-Path $projectRoot 'Cargo.toml') --target-dir (Join-Path $projectRoot 'target')
if ($LASTEXITCODE) { throw 'No se completó cargo clean.' }
if ($Artifacts) {
    $artifactRoot = Join-Path $projectRoot 'artifacts'
    if (Test-Path -LiteralPath $artifactRoot) {
        # Destino absoluto verificado arriba y fijado al directorio del checkout.
        Remove-Item -LiteralPath $artifactRoot -Recurse -Force
    }
}
Write-Output 'Retirados compilados y temporales solicitados. Se conservan código, Git, herramientas y datos del jugador.'
