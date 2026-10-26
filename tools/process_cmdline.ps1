# Read-only fallback when WMI/CIM is unavailable in the lane's Windows token.
# PROCESS_QUERY_LIMITED_INFORMATION does not read process memory or change it.
if (-not ('FC4ProcessQuery' -as [type])) {
    Add-Type -ErrorAction Stop @"
using System;
using System.Runtime.InteropServices;
public static class FC4ProcessQuery {
    [DllImport("kernel32.dll", SetLastError=true)]
    static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    [DllImport("ntdll.dll")]
    static extern int NtQueryInformationProcess(IntPtr handle, int kind, IntPtr data, uint size, out uint needed);
    public static string CommandLine(int pid) {
        IntPtr handle = OpenProcess(0x1000, false, pid); // query limited information
        if (handle == IntPtr.Zero) return null;
        IntPtr data = IntPtr.Zero;
        try {
            const uint size = 131072;
            data = Marshal.AllocHGlobal((int)size);
            uint needed;
            if (NtQueryInformationProcess(handle, 60, data, size, out needed) != 0) return null;
            int bytes = (ushort)Marshal.ReadInt16(data);
            IntPtr text = Marshal.ReadIntPtr(data, IntPtr.Size == 8 ? 8 : 4);
            long offset = text.ToInt64() - data.ToInt64();
            if ((bytes & 1) != 0 || offset < 0 || offset + bytes > size) return null;
            return Marshal.PtrToStringUni(text, bytes / 2);
        } catch { return null; }
        finally {
            if (data != IntPtr.Zero) Marshal.FreeHGlobal(data);
            CloseHandle(handle);
        }
    }
}
"@
}

function Get-HarnessGameProcesses {
    try { $all = @(Get-CimInstance Win32_Process -Filter "Name='SimCity 4.exe'" -ErrorAction Stop) }
    catch {
        $all = @(Get-Process -Name 'SimCity 4' -ErrorAction SilentlyContinue | ForEach-Object {
            [pscustomobject]@{ ProcessId = $_.Id; CommandLine = $null }
        })
    }
    foreach ($item in $all) {
        $command = $item.CommandLine
        if (-not $command) { $command = [FC4ProcessQuery]::CommandLine([int]$item.ProcessId) }
        [pscustomobject]@{ ProcessId = [int]$item.ProcessId; CommandLine = $command }
    }
}
