; The setup program on the release page: CodeWiki-<version>-windows-x64-setup.exe.
;
; It does not copy CodeWiki's files itself. It carries the package the zip
; holds, unpacks it to its temporary folder and runs install.ps1 from there;
; its uninstaller runs the installed uninstall.ps1. So the installed folder is
; exactly what install.cmd makes and what self-update expects (uninstall.ps1
; present, install.ps1 absent), plus Inno's unins000.exe and unins000.dat.
;
; Built by scripts\package.ps1 with Inno Setup 6, which passes:
;   /DAppVersion=<full version, e.g. 0.3.1-rc.1>
;   /DFileVersion=<numeric part, e.g. 0.3.1>
;   /DStage=<absolute path of dist\CodeWiki-<version>-windows-x64>
;   /DIcon=<absolute path of crates\codewiki-app\resources\codewiki.ico>

#ifndef AppVersion
  #error AppVersion is not defined. Build the setup program with scripts\package.ps1, which passes /DAppVersion, /DFileVersion, /DStage and /DIcon to ISCC.
#endif
#ifndef FileVersion
  #error FileVersion is not defined. Build the setup program with scripts\package.ps1, which passes /DAppVersion, /DFileVersion, /DStage and /DIcon to ISCC.
#endif
#ifndef Stage
  #error Stage is not defined. Build the setup program with scripts\package.ps1, which passes /DAppVersion, /DFileVersion, /DStage and /DIcon to ISCC.
#endif
#ifndef Icon
  #error Icon is not defined. Build the setup program with scripts\package.ps1, which passes /DAppVersion, /DFileVersion, /DStage and /DIcon to ISCC.
#endif

[Setup]
; Fixed, so a newer setup program finds and replaces the Apps entry of an
; older one. The registry key is {7DEA7959-917F-46EC-80A8-625E2A9B24F5}_is1
; under HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall; self-update
; writes the new version there. "{{" is a literal "{".
AppId={{7DEA7959-917F-46EC-80A8-625E2A9B24F5}
AppName=CodeWiki
AppVersion={#AppVersion}
AppPublisher=CodeWiki
VersionInfoVersion={#FileVersion}
; The Apps entry's name; without it Inno shows "CodeWiki version <version>".
UninstallDisplayName=CodeWiki
UninstallDisplayIcon={app}\codewiki-app.exe
SetupIconFile={#Icon}
; Per user, no administrator rights: %LOCALAPPDATA%\Programs\CodeWiki, the
; folder install.cmd uses.
PrivilegesRequired=lowest
DefaultDirName={userpf}\CodeWiki
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Ready to install, progress, finish: no welcome, folder, Start Menu group or
; licence page (the licence ships as LICENSE). install.ps1 makes the shortcut.
DisableWelcomePage=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=no
; install.ps1 stops a running CodeWiki under the install folder itself.
CloseApplications=no
WizardStyle=modern
Compression=lzma2
SolidCompression=yes
; package.ps1 passes /O and /F; these keep a compile by hand in dist\ too.
OutputDir=..\..\dist
OutputBaseFilename=CodeWiki-{#AppVersion}-windows-x64-setup

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
; The whole package, never copied by Inno: PrepareToInstall unpacks it to
; {tmp}\pkg and runs install.ps1 from there. First in the list, because with
; solid compression Setup decompresses everything before a file to reach it.
Source: "{#Stage}\*"; DestDir: "{tmp}\pkg"; Flags: dontcopy recursesubdirs

[Run]
; The way the Start Menu shortcut starts it: the app takes a working folder
; that is a repository as the one to open.
Filename: "{app}\codewiki-app.exe"; WorkingDir: "{app}"; Description: "Start CodeWiki"; Flags: postinstall nowait skipifsilent

[UninstallDelete]
; Only what CodeWiki leaves once uninstall.ps1 has run: the script itself,
; uninstall.cmd, and what a self-update may leave (<name>.old, <name>.old2 ...
; files and folders, and the .update folder). The wildcard goes to Windows'
; FindFirstFile: "*.old???" is a last name part of .old and up to three more
; characters, and filesandordirs deletes a matching folder with its contents.
; Then the folder, only if it is empty: anything else the user put there
; stays. Inno deletes its unins000.exe and unins000.dat itself and tries the
; folder again after that. None of this runs when uninstall.ps1 failed: the
; uninstaller aborts first.
Type: files; Name: "{app}\uninstall.ps1"
Type: files; Name: "{app}\uninstall.cmd"
Type: filesandordirs; Name: "{app}\*.old???"
Type: filesandordirs; Name: "{app}\.update"
Type: dirifempty; Name: "{app}"

[Code]
// Windows PowerShell 5.1, which every Windows CodeWiki runs on has. In 64-bit
// install mode Exec turns WOW64 file system redirection off, so this is the
// 64-bit PowerShell: the scripts find running CodeWiki programs by
// Get-Process ... .Path, which a 32-bit PowerShell cannot read for them.
function PowerShellExe: String;
begin
  Result := ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe');
end;

// A PowerShell single-quoted string, in which nothing is expanded. PowerShell
// takes the typographic quotes U+2018 to U+201B for single quotes too, so
// each is doubled like ' (a profile folder O'Neil written with U+2019).
function Quoted(const S: String): String;
var
  Text: String;
begin
  Text := S;
  StringChangeEx(Text, '''', '''''', True);
  StringChangeEx(Text, #$2018, #$2018#$2018, True);
  StringChangeEx(Text, #$2019, #$2019#$2019, True);
  StringChangeEx(Text, #$201A, #$201A#$201A, True);
  StringChangeEx(Text, #$201B, #$201B#$201B, True);
  Result := '''' + Text + '''';
end;

// Runs Script -Target Target hidden and waits for it. Everything the script
// prints, and the error that stopped it, goes to LogFile (UTF-8) and comes
// back in Output. True when the script ran to its end.
//
// -Command, not -File: with -File the redirection would reach the script as
// an argument. A script that throws is not caught by *>, so catch appends
// its error to the log and the exit code says it failed.
function RunScript(const Script, Target, LogFile: String; var Output: String): Boolean;
var
  Command: String;
  ResultCode, I: Integer;
  Lines: TArrayOfString;
begin
  DeleteFile(LogFile);
  Command :=
    '$PSDefaultParameterValues[''Out-File:Encoding''] = ''utf8''; ' +
    'try { & ' + Quoted(Script) + ' -Target ' + Quoted(Target) + ' *> ' + Quoted(LogFile) + ' } ' +
    'catch { $_.ToString() | Out-File -Append -LiteralPath ' + Quoted(LogFile) + '; exit 1 }';
  Log('Running ' + Script + ' -Target ' + Target);
  if not Exec(PowerShellExe, '-NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "' + Command + '"',
    '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
  begin
    Output := 'Windows PowerShell could not be started: ' + SysErrorMessage(ResultCode);
    Log(Output);
    Result := False;
    Exit;
  end;
  Output := '';
  if LoadStringsFromFile(LogFile, Lines) then
    for I := 0 to GetArrayLength(Lines) - 1 do
      Output := Output + Lines[I] + #13#10;
  Output := Trim(Output);
  Log(Format('PowerShell exit code %d; output:', [ResultCode]) + #13#10 + Output);
  if (ResultCode <> 0) and (Output = '') then
    Output := Format('PowerShell ended with exit code %d and printed nothing.', [ResultCode]);
  Result := ResultCode = 0;
end;

// Before Inno writes its uninstaller and Apps entry. A non-empty result stops
// Setup on the Preparing to Install page with that message (exit code 7).
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Output: String;
begin
  Result := '';
  try
    // Matched against the unexpanded DestDir; files land in {tmp}\pkg with
    // their subfolders (plugins\...).
    ExtractTemporaryFiles('{tmp}\pkg\*');
  except
    Result := 'CodeWiki could not be installed:' + #13#10 + GetExceptionMessage;
    Exit;
  end;
  if not RunScript(ExpandConstant('{tmp}\pkg\install.ps1'), ExpandConstant('{app}'),
    ExpandConstant('{tmp}\install.log'), Output) then
    Result := 'CodeWiki could not be installed:' + #13#10 + Output;
end;

// Data and settings stay, as with uninstall.cmd without -PurgeData. When
// uninstall.ps1 fails, Abort here stops the uninstaller before it deletes
// anything more: the Apps entry stays, the exit code is not 0, and the user
// can close what holds a file and uninstall again.
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Output: String;
begin
  if CurUninstallStep <> usUninstall then
    Exit;
  if not RunScript(ExpandConstant('{app}\uninstall.ps1'), ExpandConstant('{app}'),
    ExpandConstant('{tmp}\uninstall.log'), Output) then
  begin
    if not UninstallSilent then
      MsgBox('CodeWiki could not be uninstalled:' + #13#10 + Output, mbError, MB_OK);
    Abort;
  end;
end;
