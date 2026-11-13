@echo off
rem Install CodeWiki for the current user: double-click this file.
rem
rem Windows's default script policy refuses to run install.ps1 itself. This
rem runs it with the policy bypassed for that one process only. Arguments are
rem passed on, so `install.cmd -NoShortcut` works from a terminal.
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0install.ps1" %*
set "status=%errorlevel%"
rem Started on its own (a double-click), the window closes as soon as this
rem ends. Keep it open: until a key is pressed after a failure, a few seconds
rem after success. Started from a command prompt, return at once.
echo %cmdcmdline% | findstr /i /c:"%~nx0" >nul || exit /b %status%
if not "%status%"=="0" (pause) else (timeout /t 5 >nul 2>nul)
exit /b %status%
