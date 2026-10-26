$ErrorActionPreference = 'Stop'
$compiler = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'
if (!(Test-Path -LiteralPath $compiler)) { throw 'Falta Inno Setup' }
& $compiler installer/Luma.iss
if ($LASTEXITCODE) { throw 'El nuevo instalador no compila' }

# Fixture variant embeds ONLY the test EXE to exercise native file operations
# before the beta payload is published. It is never an uploaded release asset.
$fixtureOutput = Join-Path $env:RUNNER_TEMP 'Luma-inno-fixture-output'
$probeFolder = Join-Path $env:RUNNER_TEMP 'Luma-inno-app-fixture'
$preserved = Join-Path $env:RUNNER_TEMP 'Luma-preserved-user-data.txt'
New-Item -ItemType Directory -Path $probeFolder,$fixtureOutput -Force | Out-Null
Set-Content -LiteralPath $preserved -Value 'player data must remain intact'
Set-Content -LiteralPath (Join-Path $probeFolder 'Luma-Setup.exe') -Value 'old setup fixture'
$payload = (Resolve-Path -LiteralPath target/release/luma-launcher.exe).Path
& $compiler "/DFixturePayload=$payload" "/O$fixtureOutput" '/FLuma-Setup-fixture' installer/Luma.iss
if ($LASTEXITCODE) { throw 'La variante de fixture no compila' }
$probeLog = Join-Path $env:RUNNER_TEMP 'Luma-inno-install.log'
$arguments = @('/LUMA_ACTION=install','/VERYSILENT','/NORESTART','/SP-','/SUPPRESSMSGBOXES',"/DIR=$probeFolder","/LOG=$probeLog")
$process = Start-Process -FilePath (Join-Path $fixtureOutput 'Luma-Setup-fixture.exe') -ArgumentList $arguments -WindowStyle Hidden -Wait -PassThru
Get-Content -LiteralPath $probeLog
if ($process.ExitCode) { throw "La instalacion de prueba fallo: $($process.ExitCode)" }
$installed = Join-Path $probeFolder 'Luma.exe'
if ((Get-FileHash -LiteralPath $installed).Hash -ne (Get-FileHash -LiteralPath $payload).Hash) { throw 'El EXE instalado no coincide con la fixture' }
if (Test-Path -LiteralPath (Join-Path $probeFolder 'Luma-Setup.exe')) { throw 'No se retiro el Setup antiguo de la fixture' }
$uninstaller = Join-Path $probeFolder 'unins000.exe'
if (!(Test-Path -LiteralPath $uninstaller)) { throw 'No se creo el desinstalador nativo' }
$uninstallLog = Join-Path $env:RUNNER_TEMP 'Luma-inno-uninstall.log'
$process = Start-Process -FilePath $uninstaller -ArgumentList @('/VERYSILENT','/NORESTART','/SUPPRESSMSGBOXES',"/LOG=$uninstallLog") -WindowStyle Hidden -Wait -PassThru
Get-Content -LiteralPath $uninstallLog
if ($process.ExitCode -or (Test-Path -LiteralPath $installed)) { throw 'No se retiro la aplicacion de prueba' }
if ((Get-Content -LiteralPath $preserved -Raw).Trim() -ne 'player data must remain intact') { throw 'Se alteraron datos ajenos a la instalacion' }

# Exercise pre-Inno application removal without touching any real installation.
$legacyFixture = Join-Path $env:RUNNER_TEMP 'Luma-legacy-app-fixture'
New-Item -ItemType Directory -Path $legacyFixture -Force | Out-Null
foreach ($name in @('Luma.exe','Luma-Setup.exe','Luma.exe.previous','Luma.download','unknown-user-file.txt')) {
    Set-Content -LiteralPath (Join-Path $legacyFixture $name) -Value 'isolated fixture'
}
$process = Start-Process -FilePath (Join-Path $fixtureOutput 'Luma-Setup-fixture.exe') -ArgumentList @('/LUMA_ACTION=legacy-remove','/VERYSILENT','/SP-','/SUPPRESSMSGBOXES') -WindowStyle Hidden -Wait -PassThru
foreach ($name in @('Luma.exe','Luma-Setup.exe','Luma.exe.previous','Luma.download')) {
    if (Test-Path -LiteralPath (Join-Path $legacyFixture $name)) { throw "Quedo el archivo legado de prueba: $name" }
}
if (!(Test-Path -LiteralPath (Join-Path $legacyFixture 'unknown-user-file.txt'))) { throw 'Se elimino contenido desconocido de la instalacion legada' }

# Refresh the real bundle checksums after replacing the old Rust Setup with
# the newly compiled native installer. No fixture binary enters artifacts.
$manifest = Get-Content -LiteralPath artifacts/luma-release.json | ConvertFrom-Json
$checksums = @("artifacts/Luma-$($manifest.version).exe", 'artifacts/Luma-Setup.exe') | ForEach-Object {
    "$((Get-FileHash -LiteralPath $_ -Algorithm SHA256).Hash.ToLowerInvariant())  $([IO.Path]::GetFileName($_))"
}
[IO.File]::WriteAllText((Join-Path $PWD 'artifacts/SHA256SUMS.txt'), (($checksums -join "`n") + "`n"), [Text.UTF8Encoding]::new($false))
Write-Output 'Native installer compile/install/uninstall fixture passed; unrelated data preserved.'
