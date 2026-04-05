# Build release and zip a single portable exe into dist/
# Run from repo root:  pwsh -File scripts/package-portable.ps1

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $root

$verLine = Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if (-not $verLine) { throw "Could not read version from Cargo.toml" }
$ver = $verLine.Matches.Groups[1].Value

Write-Host "Building sweep-uninstall v$ver ..."
cargo build --release

$exe = Join-Path $root "target/release/sweep-uninstall.exe"
if (-not (Test-Path $exe)) {
    throw "Missing $exe (wrong binary name? check Cargo.toml)"
}

$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$zipName = "sweep-uninstall-v$ver-windows-x64-portable.zip"
$zipPath = Join-Path $dist $zipName

if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -LiteralPath $exe -DestinationPath $zipPath -CompressionLevel Optimal

$mb = [math]::Round((Get-Item $zipPath).Length / 1048576, 2)
Write-Host "Wrote $zipPath (${mb} MB)"
