$ErrorActionPreference = "Stop"
$exe = Get-ChildItem "release-artifacts\*-setup.exe" | Select-Object -First 1
if (-not $exe) { throw "No NSIS installer found in release-artifacts." }
Write-Host "Installer:" $exe.FullName
Write-Host "Size:" $exe.Length "bytes"
Write-Host "SHA256:" (Get-FileHash -Algorithm SHA256 $exe.FullName).Hash
if (Test-Path "release-artifacts\SHA256SUMS.txt") {
  Write-Host "Expected checksum file: release-artifacts\SHA256SUMS.txt"
}
