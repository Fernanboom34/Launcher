param([ValidateSet('debug', 'release')][string]$Profile = 'release', [string]$MicrosoftClientId = '', [string]$SigningCertificateThumbprint = $env:LUMA_SIGNING_CERT_THUMBPRINT)
$ErrorActionPreference = 'Stop'
$projectRoot = $PSScriptRoot
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
$cargo = Join-Path $cargoBin 'cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) {
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
}
$originalPath = $env:PATH
$originalMicrosoftClientId = $env:LUMA_MICROSOFT_CLIENT_ID
function Sign-LumaBinary([string]$File) {
    if (-not $SigningCertificateThumbprint) { return }
    if ($SigningCertificateThumbprint -notmatch '^[a-fA-F0-9]{40}$') { throw 'La huella del certificado de firma debe tener 40 caracteres hexadecimales.' }
    $certificate = Get-Item -LiteralPath "Cert:\CurrentUser\My\$SigningCertificateThumbprint"
    if (-not $certificate.HasPrivateKey) { throw 'El certificado no tiene una clave privada disponible.' }
    $signature = Set-AuthenticodeSignature -LiteralPath $File -Certificate $certificate -HashAlgorithm SHA256 -TimestampServer 'http://timestamp.digicert.com'
    if ($signature.Status -ne 'Valid') { throw "No se pudo firmar $File`: $($signature.StatusMessage)" }
}
Push-Location $projectRoot
try {
    if ($MicrosoftClientId) { $env:LUMA_MICROSOFT_CLIENT_ID = $MicrosoftClientId }
    $env:PATH = "$cargoBin;$env:PATH"
    $cargoArgs = @()
    $usingGnu = $false
    $mingwBin = Join-Path $projectRoot '.tools\mingw64\bin'
    if (Test-Path -LiteralPath (Join-Path $mingwBin 'gcc.exe')) {
        $env:PATH = "$mingwBin;$env:PATH"
        $cargoArgs += '+stable-x86_64-pc-windows-gnu'
        $usingGnu = $true
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
    Sign-LumaBinary $versionedExe
    try {
        Copy-Item -LiteralPath $versionedExe -Destination (Join-Path $output 'Luma.exe') -Force
    } catch {
        Write-Output 'Luma.exe está en uso o no se pudo reemplazar. Utiliza el ejecutable con número de versión.'
    }
    Write-Output "Listo: $versionedExe"
    Copy-Item -LiteralPath (Join-Path $projectRoot "target\$Profile\luma-setup.exe") -Destination (Join-Path $output 'Luma-Setup.exe') -Force
    Sign-LumaBinary (Join-Path $output 'Luma-Setup.exe')
    if ($usingGnu) {
        $loader = Get-ChildItem -Path (Join-Path $projectRoot "target\$Profile\build\webview2-com-sys-*\out\x64\WebView2Loader.dll") -File | Select-Object -First 1
        if (-not $loader) { throw 'No se encontró WebView2Loader.dll para la compilación GNU.' }
        Copy-Item -LiteralPath $loader.FullName -Destination (Join-Path $output 'WebView2Loader.dll') -Force
        Write-Output 'Desarrollo GNU: conserva WebView2Loader.dll junto al launcher. Los releases públicos se compilan con MSVC y el cargador estático.'
    }
    $manifest = [ordered]@{
        version = $packageVersion
        url = "https://github.com/Fernanboom34/Launcher/releases/download/v$packageVersion/Luma-$packageVersion.exe"
        sha256 = (Get-FileHash -LiteralPath $versionedExe -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    [System.IO.File]::WriteAllText((Join-Path $output 'luma-release.json'), ($manifest | ConvertTo-Json), (New-Object System.Text.UTF8Encoding($false)))
    $checksums = @($versionedExe, (Join-Path $output 'Luma-Setup.exe')) | ForEach-Object {
        $hash = (Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $([IO.Path]::GetFileName($_))"
    }
    [IO.File]::WriteAllLines((Join-Path $output 'SHA256SUMS.txt'), $checksums, (New-Object System.Text.UTF8Encoding($false)))
    Write-Output 'Listo: artifacts\Luma-Setup.exe y artifacts\luma-release.json'
}
finally {
    $env:LUMA_MICROSOFT_CLIENT_ID = $originalMicrosoftClientId
    $env:PATH = $originalPath
    Pop-Location
}
