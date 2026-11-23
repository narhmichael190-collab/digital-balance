@echo off
setlocal
powershell -ExecutionPolicy Bypass -File "%~dp0build-release.ps1"
if errorlevel 1 exit /b 1
pause
