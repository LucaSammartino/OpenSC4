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
#include <cstring>
#include <mutex>

namespace fc4 {
namespace {

std::mutex g_mutex;
char g_path[MAX_PATH * 2] = {};
bool g_resolved = false;

bool CanAppend(const char* path) {
    FILE* f = nullptr;
    if (fopen_s(&f, path, "ab") != 0 || f == nullptr) return false;
    fclose(f);
    return true;
}

void ResolvePath() {
    g_resolved = true;
    HMODULE self = nullptr;
    if (GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                           reinterpret_cast<LPCSTR>(&ResolvePath), &self)) {
        char dll[MAX_PATH] = {};
        DWORD n = GetModuleFileNameA(self, dll, MAX_PATH);
        if (n > 0 && n < MAX_PATH) {
            char* slash = strrchr(dll, '\\');
            if (slash != nullptr) {
                slash[1] = '\0';
                _snprintf_s(g_path, sizeof(g_path), _TRUNCATE, "%sFreeCity4.log", dll);
                if (CanAppend(g_path)) return;
            }
        }
    }
    char tmp[MAX_PATH] = {};
    DWORD n = GetTempPathA(MAX_PATH, tmp);
    if (n > 0 && n < MAX_PATH) {
        _snprintf_s(g_path, sizeof(g_path), _TRUNCATE, "%sFreeCity4.log", tmp);
    } else {
        g_path[0] = '\0';
    }
}

}  // namespace

void Log(const char* format, ...) {
    std::lock_guard<std::mutex> lock(g_mutex);
    if (!g_resolved) ResolvePath();
    if (g_path[0] == '\0') return;

    FILE* f = nullptr;
    if (fopen_s(&f, g_path, "ab") != 0 || f == nullptr) return;

    SYSTEMTIME t;
    GetLocalTime(&t);
    fprintf(f, "%04u-%02u-%02u %02u:%02u:%02u.%03u [pid %lu] ", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute,
            t.wSecond, t.wMilliseconds, static_cast<unsigned long>(GetCurrentProcessId()));
    va_list args;
    va_start(args, format);
    vfprintf(f, format, args);
    va_end(args);
    fputs("\r\n", f);
    fclose(f);
}

const char* LogPath() { return g_path; }

}  // namespace fc4
