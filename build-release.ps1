$ErrorActionPreference = "Stop"
Write-Host "[1/6] Removing stale production artifacts..." -ForegroundColor Cyan
if (Test-Path "src-tauri\target\release") { Remove-Item -Recurse -Force "src-tauri\target\release" }
if (Test-Path "release-artifacts") { Remove-Item -Recurse -Force "release-artifacts" }

Write-Host "[2/6] Installing dependencies..." -ForegroundColor Cyan
if (Test-Path "package-lock.json") { npm.cmd ci } else { npm.cmd install }

Write-Host "[3/6] Verifying frontend production build..." -ForegroundColor Cyan
npm.cmd run build

Write-Host "[4/6] Building fresh Windows NSIS + MSI packages..." -ForegroundColor Cyan
npm.cmd run tauri build

$exe = Get-ChildItem "src-tauri\target\release\bundle\nsis\*-setup.exe" | Select-Object -First 1
$msi = Get-ChildItem "src-tauri\target\release\bundle\msi\*.msi" | Select-Object -First 1
if (-not $exe) { throw "NSIS installer was not produced." }

New-Item -ItemType Directory -Force "release-artifacts" | Out-Null
Copy-Item $exe.FullName "release-artifacts" -Force
if ($msi) { Copy-Item $msi.FullName "release-artifacts" -Force }

Write-Host "[5/6] Calculating SHA-256 checksums..." -ForegroundColor Cyan
$files = Get-ChildItem "release-artifacts\*.exe","release-artifacts\*.msi" -ErrorAction SilentlyContinue
$hashLines = foreach ($file in $files) {
  $hash = (Get-FileHash -Algorithm SHA256 $file.FullName).Hash
  "$hash  $($file.Name)"
}
$hashLines | Set-Content "release-artifacts\SHA256SUMS.txt"

Write-Host "[6/6] Fresh release artifacts:" -ForegroundColor Green
Get-ChildItem "release-artifacts" | Select-Object Name,Length
Write-Host "`nUse the EXE in release-artifacts for testing. Keep SHA256SUMS.txt beside it." -ForegroundColor Green
