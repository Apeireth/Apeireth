@echo off
rem Apeireth resident service management entry (Windows console shim).
rem Usage: apeireth-service install^|start^|stop^|status^|uninstall [options]
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0apeireth-service.ps1" %*
exit /b %ERRORLEVEL%
