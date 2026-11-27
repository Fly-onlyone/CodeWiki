# Runs inside the test guest under Windows PowerShell 5.1, in the logged-in
# user's desktop session, and runs the scripts the host sends it. Started by
# Start-GuestAgent in scripts\vm\Vm.ps1 as the scheduled task "vmtest-agent",
# once per run, by the first guest script that needs it.
#
# Why it exists: every guest script used to be its own scheduled task, a new
# Windows PowerShell 5.1 process and a poll every half second, which cost 3 to
# 4 seconds a call with the script doing nothing. About five hundred calls
# make up a pass. This process is already running, so a call costs the start
# of the script's own process.
#
# The host writes a request into <Dir>\in as <id>.ps1 (the code, as
# Invoke-GuestScript wraps it) and then <id>.json (what to do with it), the
# json file last and by rename. This process takes the json file by renaming it
# to <id>.json.run: of this process and the host, which may cancel the request
# the same way, only one wins. It writes <id>.done when the script has ended:
# the first line says how ("exit", "timeout", or "notstarted <why>" when the
# script never ran, which lets the host run it the old way), the second says
# whether the job object that kills the script's children was in place. The
# script's own output is in <id>.ps1.out, as before.
#
# Lanes (the "kind" in the json file):
#   script   a child powershell.exe, as the scheduled task ran it: the
#            environment block of the user as Windows builds it for a new
#            logon (so PATH after an install and a user variable set a minute
#            ago are seen), no console window, and a job object that ends the
#            script's children with it, as Stop-ScheduledTask did.
#   inproc   a new runspace in this process, for the rig's own instruments
#            that read the screen or the log and start nothing.
#
# The C# helpers the rig's instruments use (window positions) are compiled once,
# here, from <Dir>\types.ps1, which the host writes: a runspace of the inproc
# lane then finds them already loaded. They are compiled in memory, never as
# a file written beside the scripts.
#
# Nothing here is excluded from Windows protection: this is the signed
# powershell.exe running a script file, as every guest script is.
param([Parameter(Mandatory)][string]$Dir)
$ErrorActionPreference = "Continue"

Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;

namespace VmTestAgent {
    public sealed class Child : IDisposable {
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        struct STARTUPINFO {
            public int cb; public string lpReserved; public string lpDesktop; public string lpTitle;
            public int dwX, dwY, dwXSize, dwYSize, dwXCountChars, dwYCountChars, dwFillAttribute, dwFlags;
            public short wShowWindow, cbReserved2;
            public IntPtr lpReserved2, hStdInput, hStdOutput, hStdError;
        }
        [StructLayout(LayoutKind.Sequential)]
        struct PROCESS_INFORMATION { public IntPtr hProcess, hThread; public int dwProcessId, dwThreadId; }
        [StructLayout(LayoutKind.Sequential)]
        struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
            public long PerProcessUserTimeLimit, PerJobUserTimeLimit;
            public uint LimitFlags;
            public UIntPtr MinimumWorkingSetSize, MaximumWorkingSetSize;
            public uint ActiveProcessLimit;
            public UIntPtr Affinity;
            public uint PriorityClass, SchedulingClass;
        }
        [StructLayout(LayoutKind.Sequential)]
        struct IO_COUNTERS { public ulong ReadOperationCount, WriteOperationCount, OtherOperationCount, ReadTransferCount, WriteTransferCount, OtherTransferCount; }
        [StructLayout(LayoutKind.Sequential)]
        struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            public JOBOBJECT_BASIC_LIMIT_INFORMATION BasicLimitInformation;
            public IO_COUNTERS IoInfo;
            public UIntPtr ProcessMemoryLimit, JobMemoryLimit, PeakProcessMemoryUsed, PeakJobMemoryUsed;
        }

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        static extern bool CreateProcessW(string app, StringBuilder command, IntPtr processAttributes, IntPtr threadAttributes, bool inherit, uint flags, IntPtr environment, string directory, ref STARTUPINFO startup, out PROCESS_INFORMATION info);
        [DllImport("userenv.dll", SetLastError = true)]
        static extern bool CreateEnvironmentBlock(out IntPtr environment, IntPtr token, bool inherit);
        [DllImport("userenv.dll", SetLastError = true)]
        static extern bool DestroyEnvironmentBlock(IntPtr environment);
        [DllImport("advapi32.dll", SetLastError = true)]
        static extern bool OpenProcessToken(IntPtr process, uint access, out IntPtr token);
        [DllImport("kernel32.dll")]
        static extern IntPtr GetCurrentProcess();
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        static extern IntPtr CreateJobObjectW(IntPtr attributes, string name);
        [DllImport("kernel32.dll", SetLastError = true)]
        static extern bool SetInformationJobObject(IntPtr job, int informationClass, IntPtr information, uint length);
        [DllImport("kernel32.dll", SetLastError = true)]
        static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
        [DllImport("kernel32.dll")]
        static extern bool TerminateJobObject(IntPtr job, uint code);
        [DllImport("kernel32.dll")]
        static extern bool TerminateProcess(IntPtr process, uint code);
        [DllImport("kernel32.dll", SetLastError = true)]
        static extern uint ResumeThread(IntPtr thread);
        [DllImport("kernel32.dll")]
        static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
        [DllImport("kernel32.dll")]
        static extern bool GetExitCodeProcess(IntPtr process, out uint code);
        [DllImport("kernel32.dll")]
        static extern bool CloseHandle(IntPtr handle);

        IntPtr process = IntPtr.Zero;
        IntPtr job = IntPtr.Zero;
        public int Pid;
        // False when the process could not be put in a job object (the helper
        // itself may sit in one that does not allow it).
        public bool Jobbed;

        // Start the command suspended, with the user's own fresh environment,
        // no window, in a job that ends with its handle; then let it run.
        public static Child Start(string command, string directory) {
            IntPtr token;
            if (!OpenProcessToken(GetCurrentProcess(), 0x000E, out token)) { throw new Win32Exception(Marshal.GetLastWin32Error(), "OpenProcessToken"); }
            IntPtr environment = IntPtr.Zero;
            try {
                if (!CreateEnvironmentBlock(out environment, token, false)) { throw new Win32Exception(Marshal.GetLastWin32Error(), "CreateEnvironmentBlock"); }
                STARTUPINFO startup = new STARTUPINFO();
                startup.cb = Marshal.SizeOf(typeof(STARTUPINFO));
                PROCESS_INFORMATION info;
                // CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW
                uint flags = 0x00000004 | 0x00000400 | 0x08000000;
                if (!CreateProcessW(null, new StringBuilder(command), IntPtr.Zero, IntPtr.Zero, false, flags, environment, directory, ref startup, out info)) {
                    throw new Win32Exception(Marshal.GetLastWin32Error(), "CreateProcess");
                }
                Child child = new Child();
                child.process = info.hProcess;
                child.Pid = info.dwProcessId;
                child.job = CreateJobObjectW(IntPtr.Zero, null);
                if (child.job != IntPtr.Zero) {
                    JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits = new JOBOBJECT_EXTENDED_LIMIT_INFORMATION();
                    // JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                    limits.BasicLimitInformation.LimitFlags = 0x2000;
                    int size = Marshal.SizeOf(typeof(JOBOBJECT_EXTENDED_LIMIT_INFORMATION));
                    IntPtr buffer = Marshal.AllocHGlobal(size);
                    try {
                        Marshal.StructureToPtr(limits, buffer, false);
                        // JobObjectExtendedLimitInformation
                        child.Jobbed = SetInformationJobObject(child.job, 9, buffer, (uint)size) && AssignProcessToJobObject(child.job, info.hProcess);
                    } finally {
                        Marshal.FreeHGlobal(buffer);
                    }
                }
                ResumeThread(info.hThread);
                CloseHandle(info.hThread);
                return child;
            } finally {
                if (environment != IntPtr.Zero) { DestroyEnvironmentBlock(environment); }
                CloseHandle(token);
            }
        }

        // True once the process has ended.
        public bool Wait(int milliseconds) {
            return WaitForSingleObject(process, (uint)milliseconds) == 0;
        }

        public int ExitCode {
            get { uint code; return GetExitCodeProcess(process, out code) ? (int)code : -1; }
        }

        // End the process and, in a job, everything it started.
        public void Kill() {
            if (Jobbed) { TerminateJobObject(job, 1); } else { TerminateProcess(process, 1); }
        }

        // Closing the job ends whatever the script left running.
        public void Dispose() {
            if (job != IntPtr.Zero) { CloseHandle(job); job = IntPtr.Zero; }
            if (process != IntPtr.Zero) { CloseHandle(process); process = IntPtr.Zero; }
        }
    }
}
'@

$in = Join-Path $Dir "in"
$alive = Join-Path $Dir "alive"
$shell = Join-Path $env:SystemRoot "System32\WindowsPowerShell\v1.0\powershell.exe"
$home32 = Join-Path $env:SystemRoot "System32"
New-Item -ItemType Directory -Force $in | Out-Null
[System.IO.File]::WriteAllText((Join-Path $Dir "pid"), "$PID")
[System.IO.File]::WriteAllText($alive, "$PID")
# The first ConvertFrom-Json loads its module; do it before the first request.
$null = '{"kind":"script","timeoutSec":1}' | ConvertFrom-Json
$types = Join-Path $Dir "types.ps1"
if (Test-Path $types) { . $types }

$beat = [System.Diagnostics.Stopwatch]::StartNew()
function Set-Beat {
    if ($beat.ElapsedMilliseconds -ge 1000) {
        try { [System.IO.File]::SetLastWriteTimeUtc($alive, [DateTime]::UtcNow) } catch { }
        $beat.Restart()
    }
}

# <id>.done is written beside a temporary name and renamed, so the host never
# reads half of it.
function Write-Done {
    param([string]$Id, [string]$Status, [bool]$Jobbed)
    $done = Join-Path $in "$Id.done"
    [System.IO.File]::WriteAllText("$done.tmp", "$Status`n" + $(if ($Jobbed) { "jobs=true" } else { "jobs=false" }))
    [System.IO.File]::Move("$done.tmp", $done)
}

# Run the text in a new runspace of this process. Returns "exit" with the
# lines the text wrote in $lines, or "timeout" when it did not end in time.
function Invoke-InProcess {
    param([string]$Text, [int]$Timeout, [string]$OutFile)
    $ps = [powershell]::Create()
    try {
        [void]$ps.AddScript($Text)
        $async = $ps.BeginInvoke()
        $clock = [System.Diagnostics.Stopwatch]::StartNew()
        while (-not $async.AsyncWaitHandle.WaitOne(100)) {
            Set-Beat
            if ($clock.Elapsed.TotalSeconds -ge $Timeout) {
                [void]$ps.BeginStop($null, $null)
                [void]$async.AsyncWaitHandle.WaitOne(2000)
                return "timeout"
            }
        }
        $lines = @($ps.EndInvoke($async) | ForEach-Object { "$_" })
        [System.IO.File]::WriteAllLines($OutFile, [string[]]$lines, (New-Object System.Text.UTF8Encoding $true))
        "exit"
    } finally {
        $ps.Dispose()
    }
}

$stopAfter = $false
while (-not $stopAfter) {
    Set-Beat
    if ($beat.ElapsedMilliseconds -lt 5 -and (Test-Path (Join-Path $Dir "stop"))) { break }
    $next = $null
    foreach ($file in [System.IO.Directory]::GetFiles($in, "*.json")) {
        if ($file.EndsWith(".json")) { $next = $file; break }
    }
    if (-not $next) { Start-Sleep -Milliseconds 30; continue }
    # Of this process and the host's cancel, one renames the file.
    try { [System.IO.File]::Move($next, "$next.run") } catch { continue }
    $id = [System.IO.Path]::GetFileNameWithoutExtension($next)
    $jobbed = $false
    $status = "notstarted"
    try {
        $request = [System.IO.File]::ReadAllText("$next.run") | ConvertFrom-Json
        $timeout = [int]$request.timeoutSec
        $script = Join-Path $in "$id.ps1"
        if ($request.kind -eq "script") {
            $command = '"' + $shell + '" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + $script + '"'
            $child = [VmTestAgent.Child]::Start($command, $home32)
            try {
                $jobbed = $child.Jobbed
                $clock = [System.Diagnostics.Stopwatch]::StartNew()
                $status = "exit"
                while (-not $child.Wait(200)) {
                    Set-Beat
                    if ($clock.Elapsed.TotalSeconds -ge $timeout) { $child.Kill(); $status = "timeout"; break }
                }
            } finally {
                $child.Dispose()
            }
        } elseif ($request.kind -eq "inproc") {
            $status = Invoke-InProcess -Text ([System.IO.File]::ReadAllText($script)) -Timeout $timeout -OutFile "$script.out"
            # A runspace that would not stop may still be running its code: end
            # this process; the host starts a new helper for the next call.
            if ($status -eq "timeout") { $stopAfter = $true }
        } else {
            $status = "notstarted unknown kind '$($request.kind)'"
        }
    } catch {
        # Only a script that never started may say so: the host then runs it
        # again the old way. One that started is never run twice.
        if ($status -eq "notstarted") { $status = "notstarted $($_.Exception.Message -replace '\s+', ' ')" }
    }
    Write-Done -Id $id -Status $status -Jobbed $jobbed
}
