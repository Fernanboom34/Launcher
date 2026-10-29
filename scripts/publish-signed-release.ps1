param(
    [Parameter(Mandatory=$true)][string]$PrivateKey,
    [ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version = '0.9.0'
)
$ErrorActionPreference = 'Stop'
$repo = 'Fernanboom34/Launcher'
$tag = "v$Version"
$PrivateKey = (Resolve-Path -LiteralPath $PrivateKey).Path
$gh = (Get-Command gh -ErrorAction Stop).Source
$python = Get-Command py -ErrorAction SilentlyContinue
if (-not $python) { $python = Get-Command python -ErrorAction Stop }
function Read-Draft {
    $json = & $gh release view $tag --repo $repo --json isDraft,tagName
    if ($LASTEXITCODE) { throw 'No se pudo consultar el release. Ejecuta gh auth login.' }
    $release = $json | ConvertFrom-Json
    if (-not $release.isDraft -or $release.tagName -ne $tag) {
        throw 'Solo se permite firmar y publicar un borrador. No se reemplazan assets publicados.'
    }
}
Read-Draft
$folder = Join-Path ([IO.Path]::GetTempPath()) ("luma-sign-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $folder | Out-Null
Write-Host "Descargando los artefactos de $tag a $folder"
& $gh release download $tag --repo $repo --dir $folder --pattern "Luma-$Version.exe" --pattern "Luma-$Version-linux-x64" --pattern 'luma-release.json' --pattern 'luma-release-linux-x64.json' --pattern 'SHA256SUMS-linux-x64.txt'
if ($LASTEXITCODE) { throw 'No se pudieron descargar todos los archivos.' }
$script = Join-Path $PSScriptRoot 'sign-update-manifest.py'
foreach ($platform in @(@('luma-release.json', "Luma-$Version.exe"), @('luma-release-linux-x64.json', "Luma-$Version-linux-x64"))) {
    $manifest = Join-Path $folder $platform[0]
    $binary = Join-Path $folder $platform[1]
    $data = Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
    $expected = "https://github.com/$repo/releases/download/$tag/$($platform[1])"
    if ($data.version -ne $Version -or $data.url -ne $expected) { throw 'Versión o URL de manifiesto incorrecta.' }
    & $python.Source $script $manifest $binary --private-key $PrivateKey
    if ($LASTEXITCODE) { throw 'Firma fallida. No se publicará el release.' }
}
# Linux checksums include the manifest itself: refresh its entry after signing.
$sums = Join-Path $folder 'SHA256SUMS-linux-x64.txt'
$lines = @(Get-Content -LiteralPath $sums)
$entry = @( $lines | Where-Object { $_ -match '^[a-fA-F0-9]{64}  luma-release-linux-x64\.json$' } )
if ($entry.Count -ne 1) { throw 'Falta la entrada del manifiesto en las sumas Linux.' }
$digest = (Get-FileHash -LiteralPath (Join-Path $folder 'luma-release-linux-x64.json') -Algorithm SHA256).Hash.ToLowerInvariant()
$lines = $lines | ForEach-Object {
    if ($_ -eq $entry[0]) { "$digest  luma-release-linux-x64.json" } else { $_ }
}
[IO.File]::WriteAllText($sums, (($lines -join "`n") + "`n"), [Text.UTF8Encoding]::new($false))
# Recheck before replacing unsigned metadata in the draft.
Read-Draft
& $gh release upload $tag --repo $repo --clobber (Join-Path $folder 'luma-release.json') (Join-Path $folder 'luma-release-linux-x64.json') $sums
if ($LASTEXITCODE) { throw 'No se pudieron subir los manifiestos firmados.' }
Read-Draft
& $gh release edit $tag --repo $repo --draft=false --prerelease=false --latest
if ($LASTEXITCODE) { throw 'Los manifiestos están firmados, pero la publicación falló.' }
Write-Host "Publicado: https://github.com/$repo/releases/tag/$tag"
Write-Host 'Tu clave privada no se ha subido. Puedes borrar la carpeta temporal de artefactos.'
