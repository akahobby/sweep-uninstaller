# Build release and copy the portable exe into dist/ (for manual upload or testing).
# Run from repo root:
#   powershell -ExecutionPolicy Bypass -File scripts/package-portable.ps1

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
$outName = "sweep-uninstall-v$ver-windows-x64.exe"
$outPath = Join-Path $dist $outName

Copy-Item -LiteralPath $exe -Destination $outPath -Force

$mb = [math]::Round((Get-Item $outPath).Length / 1048576, 2)
Write-Host "Wrote $outPath (${mb} MB)"
