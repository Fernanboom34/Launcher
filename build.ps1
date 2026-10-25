param([ValidateSet('debug', 'release')][string]$Profile = 'release', [string]$MicrosoftClientId = '')
$ErrorActionPreference = 'Stop'
$projectRoot = $PSScriptRoot
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
$cargo = Join-Path $cargoBin 'cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) {
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
}
$originalPath = $env:PATH
$originalMicrosoftClientId = $env:LUMA_MICROSOFT_CLIENT_ID
Push-Location $projectRoot
try {
    if ($MicrosoftClientId) { $env:LUMA_MICROSOFT_CLIENT_ID = $MicrosoftClientId }
    $env:PATH = "$cargoBin;$env:PATH"
    $cargoArgs = @()
    $mingwBin = Join-Path $projectRoot '.tools\mingw64\bin'
    if (Test-Path -LiteralPath (Join-Path $mingwBin 'gcc.exe')) {
        $env:PATH = "$mingwBin;$env:PATH"
        $cargoArgs += '+stable-x86_64-pc-windows-gnu'
    }
    $cargoArgs += @('build', '--locked')
    if ($Profile -eq 'release') { $cargoArgs += '--release' }
    & $cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw 'La compilación no se completó.' }
    $output = Join-Path $projectRoot 'artifacts'
    New-Item -ItemType Directory -Force -Path $output | Out-Null
    $packageVersion = [regex]::Match([System.IO.File]::ReadAllText((Join-Path $projectRoot 'Cargo.toml')), '(?m)^version = "([^"]+)"').Groups[1].Value
    $versionedExe = Join-Path $output "Luma-$packageVersion.exe"
    Copy-Item -LiteralPath (Join-Path $projectRoot "target\$Profile\luma-launcher.exe") -Destination $versionedExe -Force
    try {
        Copy-Item -LiteralPath $versionedExe -Destination (Join-Path $output 'Luma.exe') -Force
    } catch {
        Write-Output 'Luma.exe está en uso o no se pudo reemplazar. Utiliza el ejecutable con número de versión.'
    }
    Write-Output "Listo: $versionedExe"
    Copy-Item -LiteralPath (Join-Path $projectRoot "target\$Profile\luma-setup.exe") -Destination (Join-Path $output 'Luma-Setup.exe') -Force
    $manifest = [ordered]@{
        version = $packageVersion
        url = "https://github.com/Fernanboom34/Launcher/releases/download/v$packageVersion/Luma-$packageVersion.exe"
        sha256 = (Get-FileHash -LiteralPath $versionedExe -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    [System.IO.File]::WriteAllText((Join-Path $output 'luma-release.json'), ($manifest | ConvertTo-Json), (New-Object System.Text.UTF8Encoding($false)))
    Write-Output 'Listo: artifacts\Luma-Setup.exe y artifacts\luma-release.json'
}
finally {
    $env:LUMA_MICROSOFT_CLIENT_ID = $originalMicrosoftClientId
    $env:PATH = $originalPath
    Pop-Location
}
