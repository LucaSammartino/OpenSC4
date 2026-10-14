<#
Let another SimCity 4 start next to a running one (coordinator tooling, harness side, no game patching).

The game creates a named mutex "SimCity 4 Mutex" at start-up and exits at once (code -1) when it already exists, so
only one game can run at a time. This script finds the game's handle(s) to that mutex in process -Pid, closes them in
the game (DuplicateHandle with DUPLICATE_CLOSE_SOURCE), and puts a fresh UNNAMED mutex in the game in their place, so
the name is free for the next game while the first one's stored handle still refers to a valid mutex (the game only
touches it again at shutdown). The handle table usually reuses the freed slot, so the replacement normally gets the
same handle value; the output says "same" or "DIFFERENT" (a different value is still fine for test runs).

Usage:
  powershell -File tools\sc4mutex.ps1 -Pid <pid> [-WaitSeconds 60] [-Name "SimCity 4 Mutex"]

Prints one line per replaced handle:
  sc4mutex: pid N handle 0x.. closed; replacement 0x.. (same|DIFFERENT); name free: yes|NO
Exit codes: 0 swapped, 1 no such mutex in the process within -WaitSeconds (or the process exited), 2 error.

How it finds the handle: NtQuerySystemInformation(SystemExtendedHandleInformation) lists every handle; the ones of
process -Pid whose object type index is the Mutant type's (learnt from a mutex created here) are duplicated into this
process, their type is checked with NtQueryObject (must be "Mutant") BEFORE their name is asked for (a name query can
hang on some pipe and file handles), and the name must end with "\<Name>".
#>
param(
    [Parameter(Mandatory = $true)][Alias("Pid")][int]$GamePid,
    [int]$WaitSeconds = 60,
    [string]$Name = "SimCity 4 Mutex"
)
$ErrorActionPreference = "Stop"

Add-Type -ErrorAction SilentlyContinue @"
using System; using System.Collections.Generic; using System.Runtime.InteropServices;
public static class FC4Mutex {
    [StructLayout(LayoutKind.Sequential)] public struct Entry {
        public IntPtr Object; public IntPtr UniqueProcessId; public IntPtr HandleValue; public uint GrantedAccess;
        public ushort CreatorBackTraceIndex; public ushort ObjectTypeIndex; public uint HandleAttributes; public uint Reserved;
    }
    [DllImport("ntdll.dll")] static extern int NtQuerySystemInformation(int cls, IntPtr buf, int len, out int retLen);
    [DllImport("ntdll.dll")] static extern int NtQueryObject(IntPtr h, int cls, IntPtr buf, int len, out int retLen);
    [DllImport("kernel32.dll", SetLastError = true)] static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool DuplicateHandle(IntPtr srcProc, IntPtr src, IntPtr dstProc, out IntPtr dst, uint access, bool inherit, uint options);
    [DllImport("kernel32.dll", SetLastError = true)] static extern bool CloseHandle(IntPtr h);
    [DllImport("kernel32.dll")] static extern IntPtr GetCurrentProcess();
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)] static extern IntPtr CreateMutexW(IntPtr attr, bool owner, string name);
    [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)] static extern IntPtr OpenMutexW(uint access, bool inherit, string name);
    [DllImport("kernel32.dll")] static extern uint WaitForSingleObject(IntPtr h, uint ms);
    const uint PROCESS_DUP_HANDLE = 0x40, SYNCHRONIZE = 0x00100000, DUPLICATE_CLOSE_SOURCE = 1, DUPLICATE_SAME_ACCESS = 2;

    static List<Entry> Handles(int pid) {
        int len = 1 << 22;
        while (true) {
            IntPtr buf = Marshal.AllocHGlobal(len);
            try {
                int ret; int st = NtQuerySystemInformation(64, buf, len, out ret);
                if (st == unchecked((int)0xC0000004)) { len = Math.Max(len * 2, ret + (1 << 20)); continue; }
                if (st != 0) throw new Exception("NtQuerySystemInformation failed: 0x" + st.ToString("X8"));
                long n = Marshal.ReadIntPtr(buf).ToInt64();
                int size = Marshal.SizeOf(typeof(Entry));
                List<Entry> r = new List<Entry>();
                for (long i = 0; i < n; i++) {
                    Entry e = (Entry)Marshal.PtrToStructure(new IntPtr(buf.ToInt64() + 2 * IntPtr.Size + i * size), typeof(Entry));
                    if (e.UniqueProcessId.ToInt64() == pid) r.Add(e);
                }
                return r;
            } finally { Marshal.FreeHGlobal(buf); }
        }
    }
    // ObjectNameInformation (1) and ObjectTypeInformation (2) both start with a UNICODE_STRING.
    static string QueryString(IntPtr h, int cls) {
        int len = 0x2000;
        IntPtr buf = Marshal.AllocHGlobal(len);
        try {
            int ret; int st = NtQueryObject(h, cls, buf, len, out ret);
            if (st != 0) return null;
            int l = (ushort)Marshal.ReadInt16(buf);
            IntPtr s = Marshal.ReadIntPtr(buf, IntPtr.Size);
            if (s == IntPtr.Zero || l == 0) return "";
            return Marshal.PtrToStringUni(s, l / 2);
        } finally { Marshal.FreeHGlobal(buf); }
    }
    static int MutantTypeIndex() {
        IntPtr m = CreateMutexW(IntPtr.Zero, false, null);
        try {
            foreach (Entry e in Handles(System.Diagnostics.Process.GetCurrentProcess().Id))
                if (e.HandleValue == m) return e.ObjectTypeIndex;
            return -1;
        } finally { CloseHandle(m); }
    }
    static string Err() { return new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error()).Message; }

    // Returns the game's handle values to the named mutex (possibly none). Throws on errors.
    public static List<long> Find(int pid, string name) {
        int mutant = MutantTypeIndex();
        IntPtr proc = OpenProcess(PROCESS_DUP_HANDLE, false, pid);
        if (proc == IntPtr.Zero) throw new Exception("OpenProcess(" + pid + "): " + Err());
        List<long> found = new List<long>();
        try {
            foreach (Entry e in Handles(pid)) {
                if (mutant >= 0 && e.ObjectTypeIndex != mutant) continue;
                IntPtr local;
                if (!DuplicateHandle(proc, e.HandleValue, GetCurrentProcess(), out local, 0, false, DUPLICATE_SAME_ACCESS)) continue;
                try {
                    if (QueryString(local, 2) != "Mutant") continue;   // the type first: a name query can hang on pipes
                    string n = QueryString(local, 1);
                    if (n != null && n.EndsWith("\\" + name)) found.Add(e.HandleValue.ToInt64());
                } finally { CloseHandle(local); }
            }
        } finally { CloseHandle(proc); }
        return found;
    }
    // Close handle 'old' in the game and put a fresh unnamed mutex there. Returns the replacement's handle value.
    public static long Swap(int pid, long old) {
        IntPtr proc = OpenProcess(PROCESS_DUP_HANDLE, false, pid);
        if (proc == IntPtr.Zero) throw new Exception("OpenProcess(" + pid + "): " + Err());
        IntPtr fresh = CreateMutexW(IntPtr.Zero, false, null);
        if (fresh == IntPtr.Zero) { CloseHandle(proc); throw new Exception("CreateMutex: " + Err()); }
        try {
            IntPtr taken;
            // Move the game's handle into this process (closing it there), then close it here: the named object goes.
            if (!DuplicateHandle(proc, new IntPtr(old), GetCurrentProcess(), out taken, 0, false, DUPLICATE_SAME_ACCESS | DUPLICATE_CLOSE_SOURCE))
                throw new Exception("DuplicateHandle(close source 0x" + old.ToString("X") + "): " + Err());
            CloseHandle(taken);
            IntPtr remote;
            if (!DuplicateHandle(GetCurrentProcess(), fresh, proc, out remote, 0, false, DUPLICATE_SAME_ACCESS))
                throw new Exception("DuplicateHandle(replacement into the game): " + Err());
            return remote.ToInt64();
        } finally { CloseHandle(fresh); CloseHandle(proc); }
    }
    public static bool NameFree(string name) {
        IntPtr h = OpenMutexW(SYNCHRONIZE, false, name);
        if (h == IntPtr.Zero) return true;
        CloseHandle(h); return false;
    }
}
"@

$deadline = (Get-Date).AddSeconds($WaitSeconds)
$found = @()
while ($true) {
    $p = Get-Process -Id $GamePid -ErrorAction SilentlyContinue
    if (-not $p -or $p.HasExited) { Write-Output "sc4mutex: pid $GamePid is not running"; exit 1 }
    try { $found = @([FC4Mutex]::Find($GamePid, $Name)) } catch { Write-Output "sc4mutex: pid ${GamePid}: $($_.Exception.Message)"; exit 2 }
    if ($found.Count -gt 0) { break }
    if ((Get-Date) -ge $deadline) { Write-Output "sc4mutex: pid ${GamePid}: no '$Name' mutex handle after $WaitSeconds s"; exit 1 }
    Start-Sleep -Milliseconds 500
}
$rc = 0
foreach ($old in $found) {
    try {
        $new = [FC4Mutex]::Swap($GamePid, $old)
        $same = if ($new -eq $old) { "same" } else { "DIFFERENT: the game keeps 0x{0:X} (now closed), used only at shutdown" -f $old }
        $free = if ([FC4Mutex]::NameFree($Name)) { "yes" } else { "NO" }
        Write-Output ("sc4mutex: pid {0} handle 0x{1:X} closed; replacement 0x{2:X} ({3}); name free: {4}" -f $GamePid, $old, $new, $same, $free)
        if ($free -ne "yes") { $rc = 2 }
    } catch { Write-Output "sc4mutex: pid ${GamePid}: $($_.Exception.Message)"; $rc = 2 }
}
exit $rc
