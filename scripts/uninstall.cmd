@echo off
rem Remove CodeWiki: double-click this file in the installation folder.
rem
rem Runs uninstall.ps1 past Windows's default script policy, for that one
rem process only. Arguments are passed on: `uninstall.cmd -PurgeData` also
rem deletes the index cache, the logs and the settings.
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0uninstall.ps1" %*
set "status=%errorlevel%"
rem Started on its own (a double-click), the window closes as soon as this
rem ends. Keep it open: until a key is pressed after a failure, a few seconds
rem after success. Started from a command prompt, return at once.
echo %cmdcmdline% | findstr /i /c:"%~nx0" >nul || exit /b %status%
if not "%status%"=="0" (pause) else (timeout /t 5 >nul 2>nul)
exit /b %status%
