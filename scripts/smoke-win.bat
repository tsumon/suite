@echo off
REM Wrapper for smoke-suite.ps1 (INTERACTION-P2 §10 / Joe P2 list)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0smoke-suite.ps1"
exit /b %ERRORLEVEL%
