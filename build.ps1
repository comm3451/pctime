param([switch]$Test)
$ErrorActionPreference = 'Stop'
$projectRoot = $PSScriptRoot
$localCargo = Join-Path $projectRoot '.tools\cargo\bin\cargo.exe'
if (Test-Path -LiteralPath $localCargo) {
    $env:CARGO_HOME = Join-Path $projectRoot '.tools\cargo'
    $env:RUSTUP_HOME = Join-Path $projectRoot '.tools\rustup'
    $env:Path = (Join-Path $projectRoot '.tools\cargo\bin') + ';' + $env:Path
    $cargo = $localCargo
} else { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
Push-Location -LiteralPath $projectRoot
try {
    if ($Test) { & $cargo test --offline; if ($LASTEXITCODE) { throw 'Tests failed' } }
    & $cargo build --release --locked --offline
    if ($LASTEXITCODE) { throw 'Build failed' }
    New-Item -ItemType Directory -Path (Join-Path $projectRoot 'dist') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $projectRoot 'target\release\pctime.exe') -Destination (Join-Path $projectRoot 'dist\PC-Time.exe') -Force
    Get-Item -LiteralPath (Join-Path $projectRoot 'dist\PC-Time.exe') | Select-Object FullName,Length
} finally { Pop-Location }
