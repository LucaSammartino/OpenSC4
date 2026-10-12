// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

#include "FC4Log.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdarg>
#include <cstdio>
#include <cstdlib>
#include <cstring>

// Design notes:
// - The path is chosen once (next to the DLL; %TEMP% only if that file cannot
//   be opened at all, and the switch is written into the log) and never
//   changes afterwards.
// - The file is opened with CreateFileW, FILE_APPEND_DATA and full sharing, so
//   another process reading (or even deleting) the log cannot make writes fail.
// - Each line is one WriteFile on an append handle: once WriteFile returns the
//   data belongs to the OS, so it survives a crash or kill of the game.
// - A failed write is retried once (after reopening). Failures are counted and
//   reported on the next successful write.
// - No heap allocation and no CRT FILE*: the logger is also used from the
//   vectored exception handler.

namespace fc4 {
namespace {

struct LogState {
    CRITICAL_SECTION lock;
    bool resolved = false;
    wchar_t pathW[MAX_PATH * 2] = {};
    char pathA[MAX_PATH * 2] = {};
    HANDLE file = INVALID_HANDLE_VALUE;
    // A second append handle on the same file for writers that must not wait
    // for the lock (exception handler, watchdog). Opened with the first.
    HANDLE spare = INVALID_HANDLE_VALUE;
    unsigned long failedWrites = 0;
    DWORD lastError = 0;
    bool fellBackToTemp = false;
    DWORD primaryError = 0;
    char primaryPath[MAX_PATH * 2] = {};

    LogState() { InitializeCriticalSection(&lock); }
};

LogState& State() {
    static LogState s;  // constructed on first use (thread-safe static init)
    return s;
}

HANDLE OpenShared(const wchar_t* path) {
    return CreateFileW(path, FILE_APPEND_DATA, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, nullptr,
                       OPEN_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
}

void Resolve(LogState& s) {
    s.resolved = true;
    // 1. Next to the DLL.
    HMODULE self = nullptr;
    if (GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                           reinterpret_cast<LPCWSTR>(&Resolve), &self)) {
        wchar_t dll[MAX_PATH] = {};
        const DWORD n = GetModuleFileNameW(self, dll, MAX_PATH);
        wchar_t* slash = (n > 0 && n < MAX_PATH) ? wcsrchr(dll, L'\\') : nullptr;
        if (slash != nullptr) {
            slash[1] = L'\0';
            _snwprintf_s(s.pathW, _countof(s.pathW), _TRUNCATE, L"%sFreeCity4.log", dll);
            s.file = OpenShared(s.pathW);
            if (s.file != INVALID_HANDLE_VALUE) {
                s.spare = OpenShared(s.pathW);
                return;
            }
            s.primaryError = GetLastError();
            WideCharToMultiByte(CP_UTF8, 0, s.pathW, -1, s.primaryPath, sizeof(s.primaryPath), nullptr, nullptr);
        }
    }
    // 2. %TEMP%, recorded in the log itself (never a silent switch).
    wchar_t tmp[MAX_PATH] = {};
    const DWORD n = GetTempPathW(MAX_PATH, tmp);
    if (n > 0 && n < MAX_PATH) {
        _snwprintf_s(s.pathW, _countof(s.pathW), _TRUNCATE, L"%sFreeCity4.log", tmp);
        s.file = OpenShared(s.pathW);
        s.spare = OpenShared(s.pathW);
        s.fellBackToTemp = true;
    } else {
        s.pathW[0] = L'\0';
    }
}

bool WriteAll(HANDLE h, const char* data, DWORD len) {
    DWORD written = 0;
    return h != INVALID_HANDLE_VALUE && WriteFile(h, data, len, &written, nullptr) && written == len;
}

// Writes one already-formatted line (without CRLF). Caller holds the lock.
void WriteLine(LogState& s, const char* text, size_t textLen) {
    if (!s.resolved) {
        Resolve(s);
        WideCharToMultiByte(CP_UTF8, 0, s.pathW, -1, s.pathA, sizeof(s.pathA), nullptr, nullptr);
        if (s.fellBackToTemp) {
            char note[MAX_PATH * 3];
            const int k = _snprintf_s(note, sizeof(note), _TRUNCATE,
                                      "[log] could not open %s (error %lu); logging to this file instead\r\n",
                                      s.primaryPath, static_cast<unsigned long>(s.primaryError));
            if (k > 0) WriteAll(s.file, note, static_cast<DWORD>(k));
        }
    }
    if (s.pathW[0] == L'\0') return;

    char line[4096];
    size_t len = textLen < sizeof(line) - 2 ? textLen : sizeof(line) - 2;
    memcpy(line, text, len);
    line[len++] = '\r';
    line[len++] = '\n';

    bool ok = WriteAll(s.file, line, static_cast<DWORD>(len));
    if (!ok) {
        // Retry once with a fresh handle (same path).
        s.lastError = GetLastError();
        if (s.file != INVALID_HANDLE_VALUE) CloseHandle(s.file);
        s.file = OpenShared(s.pathW);
        ok = WriteAll(s.file, line, static_cast<DWORD>(len));
        if (!ok) s.lastError = GetLastError();
    }
    if (!ok) {
        ++s.failedWrites;
        return;
    }
    if (s.failedWrites != 0) {
        char note[160];
        const int k = _snprintf_s(note, sizeof(note), _TRUNCATE,
                                  "[log] %lu earlier log line(s) could not be written (last error %lu)\r\n",
                                  s.failedWrites, static_cast<unsigned long>(s.lastError));
        if (k > 0 && WriteAll(s.file, note, static_cast<DWORD>(k))) s.failedWrites = 0;
    }
}

volatile LONG g_unlocked = 0;  // lines written through the spare handle
volatile LONG g_dropped = 0;   // lines lost (log not open yet)
volatile LONG g_resolvedFlag = 0;

// Formats the standard prefix + message into `text` (no allocation).
size_t Format(char* text, size_t cap, const char* tag, const char* format, va_list args) {
    SYSTEMTIME t;
    GetLocalTime(&t);
    int n = _snprintf_s(text, cap, _TRUNCATE, "%04u-%02u-%02u %02u:%02u:%02u.%03u [pid %lu tid %lu] %s", t.wYear,
                        t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds,
                        static_cast<unsigned long>(GetCurrentProcessId()),
                        static_cast<unsigned long>(GetCurrentThreadId()), tag);
    if (n < 0) n = static_cast<int>(strlen(text));
    const int m = _vsnprintf_s(text + n, cap - n, _TRUNCATE, format, args);
    return m < 0 ? strlen(text) : static_cast<size_t>(n + m);
}

void VLog(const char* tag, const char* format, va_list args) {
    char text[3072];
    SYSTEMTIME t;
    GetLocalTime(&t);
    int n = _snprintf_s(text, sizeof(text), _TRUNCATE, "%04u-%02u-%02u %02u:%02u:%02u.%03u [pid %lu tid %lu] %s",
                        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond, t.wMilliseconds,
                        static_cast<unsigned long>(GetCurrentProcessId()),
                        static_cast<unsigned long>(GetCurrentThreadId()), tag);
    if (n < 0) n = static_cast<int>(strlen(text));
    int m = _vsnprintf_s(text + n, sizeof(text) - n, _TRUNCATE, format, args);
    const size_t total = m < 0 ? strlen(text) : static_cast<size_t>(n + m);

    LogState& s = State();
    EnterCriticalSection(&s.lock);
    WriteLine(s, text, total);
    if (s.resolved) InterlockedExchange(&g_resolvedFlag, 1);
    LeaveCriticalSection(&s.lock);
}

// Writes without the lock through the spare handle (one WriteFile with
// FILE_APPEND_DATA is one atomic append). No allocation, no other lock.
void WriteUnlocked(LogState& s, char* text, size_t len, size_t cap) {
    if (!g_resolvedFlag || s.spare == INVALID_HANDLE_VALUE) {
        InterlockedIncrement(&g_dropped);
        return;
    }
    if (len + 2 > cap) len = cap - 2;
    text[len++] = '\r';
    text[len++] = '\n';
    if (WriteAll(s.spare, text, static_cast<DWORD>(len))) {
        InterlockedIncrement(&g_unlocked);
    } else {
        InterlockedIncrement(&g_dropped);
    }
}

int ReadLevel() {
    char value[16] = {};
    const DWORD n = GetEnvironmentVariableA("FC4_LOG_LEVEL", value, sizeof(value));
    if (n == 0 || n >= sizeof(value)) return kLogVerbose;  // default: verbose (diagnostic builds)
    const int v = atoi(value);
    return v < kLogOff ? kLogOff : (v > kLogVerbose ? kLogVerbose : v);
}

}  // namespace

int LogLevel() {
    static const int level = ReadLevel();
    return level;
}

void Log(const char* format, ...) {
    if (LogLevel() < kLogInfo) return;
    va_list args;
    va_start(args, format);
    VLog("", format, args);
    va_end(args);
}

void LogVerboseImpl(const char* format, ...) {
    va_list args;
    va_start(args, format);
    VLog("[v] ", format, args);
    va_end(args);
}

void LogWait(unsigned waitMs, const char* format, ...) {
    if (LogLevel() < kLogInfo) return;
    char text[3072];
    va_list args;
    va_start(args, format);
    const size_t len = Format(text, sizeof(text) - 2, "", format, args);
    va_end(args);
    LogState& s = State();
    // The lock is never taken recursively here: a line from inside a write
    // (an exception raised while this thread holds the lock) goes unlocked.
    const bool ownedHere =
        reinterpret_cast<DWORD>(s.lock.OwningThread) == GetCurrentThreadId() && s.lock.RecursionCount > 0;
    if (!ownedHere) {
        const DWORD start = GetTickCount();
        for (;;) {
            if (TryEnterCriticalSection(&s.lock)) {
                WriteLine(s, text, len);
                if (s.resolved) InterlockedExchange(&g_resolvedFlag, 1);
                LeaveCriticalSection(&s.lock);
                return;
            }
            if (GetTickCount() - start >= waitMs) break;
            Sleep(1);
        }
    }
    WriteUnlocked(s, text, len, sizeof(text));
}

unsigned long LogUnlockedLines() { return static_cast<unsigned long>(g_unlocked); }
unsigned long LogDroppedLines() { return static_cast<unsigned long>(g_dropped); }

const char* LogPath() { return State().pathA; }

const char* ModulePath() {
    static char path[MAX_PATH] = {};
    if (path[0] == '\0') {
        HMODULE self = nullptr;
        if (GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                               reinterpret_cast<LPCSTR>(&ModulePath), &self)) {
            GetModuleFileNameA(self, path, MAX_PATH);
        }
    }
    return path;
}

void LogClose() {
    LogState& s = State();
    EnterCriticalSection(&s.lock);
    if (s.file != INVALID_HANDLE_VALUE) {
        CloseHandle(s.file);
        s.file = INVALID_HANDLE_VALUE;
    }
    LeaveCriticalSection(&s.lock);
}

}  // namespace fc4
