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

#include "FC4Config.h"

#include "FC4Log.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cctype>
#include <cstdlib>
#include <cstring>
#include <map>
#include <string>

namespace fc4 {
namespace {

constexpr unsigned kCount = static_cast<unsigned>(Sw::Count);

struct SwitchInfo {
    const char* key;  // ini key; the environment variable is FC4_BUDGET_<KEY>
    const char* env;
};

const SwitchInfo kSwitches[kCount] = {
    {"lot_scan", "FC4_BUDGET_LOT_SCAN"},
    {"property_writes", "FC4_BUDGET_PROPERTY_WRITES"},
    {"property_reads", "FC4_BUDGET_PROPERTY_READS"},
    {"message_posting", "FC4_BUDGET_MESSAGE_POSTING"},
    {"history", "FC4_BUDGET_HISTORY"},
    {"building_name", "FC4_BUDGET_BUILDING_NAME"},
    {"exemplar_loads", "FC4_BUDGET_EXEMPLAR_LOADS"},
    {"disaster", "FC4_BUDGET_DISASTER"},
    {"cheats", "FC4_BUDGET_CHEATS"},
    {"holder_refs", "FC4_BUDGET_HOLDER_REFS"},
    {"ltext", "FC4_BUDGET_LTEXT"},
};

// Keys that are not switches.
const char* const kOtherKeys[] = {"disable", "budget_default", "refaudit", "refaudit_lines", "autotest_seconds",
                                  "autotest_unpause", "digest"};

// The switches that are off, for lock-free readers (autotest verdict).
char g_offList[512] = "-";

volatile LONG g_on[kCount] = {1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1};
volatile LONG g_skipped[kCount] = {};
volatile LONG g_skipLogged[kCount] = {};

std::string Lower(std::string s) {
    for (char& c : s) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    return s;
}

std::string Trim(const std::string& s) {
    size_t a = 0, b = s.size();
    while (a < b && std::isspace(static_cast<unsigned char>(s[a]))) ++a;
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1]))) --b;
    return s.substr(a, b - a);
}

std::string Env(const char* name) {
    if (name == nullptr) return std::string();
    char value[512] = {};
    const DWORD n = GetEnvironmentVariableA(name, value, sizeof(value));
    return (n > 0 && n < sizeof(value)) ? std::string(value, n) : std::string();
}

// Parses FreeCity4.ini; `found` tells whether the file exists.
std::map<std::string, std::string> ReadIni(bool* found) {
    std::map<std::string, std::string> out;
    if (found != nullptr) *found = false;
    const std::string path = IniPath();
    if (path.empty()) return out;
    HANDLE f = CreateFileA(path.c_str(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                           nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (f == INVALID_HANDLE_VALUE) return out;
    if (found != nullptr) *found = true;
    std::string text;
    char buf[4096];
    DWORD got = 0;
    while (ReadFile(f, buf, sizeof(buf), &got, nullptr) && got > 0 && text.size() < 256 * 1024) text.append(buf, got);
    CloseHandle(f);
    if (text.size() >= 3 && static_cast<unsigned char>(text[0]) == 0xEF && static_cast<unsigned char>(text[1]) == 0xBB &&
        static_cast<unsigned char>(text[2]) == 0xBF) {
        text.erase(0, 3);  // UTF-8 BOM (Notepad)
    }
    size_t at = 0;
    while (at < text.size()) {
        size_t end = text.find_first_of("\r\n", at);
        if (end == std::string::npos) end = text.size();
        std::string line = text.substr(at, end - at);
        at = end + 1;
        // A comment starts the line or follows whitespace (so "a;b" lists survive).
        for (size_t i = 0; i < line.size(); ++i) {
            if ((line[i] == '#' || line[i] == ';') && (i == 0 || line[i - 1] == ' ' || line[i - 1] == '\t')) {
                line.erase(i);
                break;
            }
        }
        line = Trim(line);
        if (line.empty() || line[0] == '[') continue;
        const size_t eq = line.find('=');
        if (eq == std::string::npos) continue;
        const std::string key = Lower(Trim(line.substr(0, eq)));
        if (!key.empty()) out[key] = Trim(line.substr(eq + 1));
    }
    return out;
}

// 1 = on, 0 = off, -1 = not a boolean.
int ParseBool(const std::string& value) {
    const std::string v = Lower(Trim(value));
    if (v == "1" || v == "on" || v == "true" || v == "yes") return 1;
    if (v == "0" || v == "off" || v == "false" || v == "no") return 0;
    return -1;
}

bool KnownKey(const std::string& key) {
    for (const SwitchInfo& s : kSwitches) {
        if (key == s.key) return true;
    }
    for (const char* k : kOtherKeys) {
        if (key == k) return true;
    }
    return false;
}

}  // namespace

std::string IniPath() {
    const std::string dll = ModulePath();
    const size_t slash = dll.find_last_of("\\/");
    if (slash == std::string::npos) return std::string();
    return dll.substr(0, slash + 1) + "FreeCity4.ini";
}

bool On(Sw s) { return s < Sw::Count && g_on[static_cast<unsigned>(s)] != 0; }

const char* SwitchesOffList() { return g_offList; }

long SettingInt(const char* iniKey, const char* envName, long fallback) {
    const std::string v = Trim(Setting(iniKey, envName, nullptr));
    if (v.empty()) return fallback;
    const int b = ParseBool(v);
    if (b == 0) return 0;
    char* end = nullptr;
    const long x = strtol(v.c_str(), &end, 10);
    if (end != nullptr && *end == 0) return x;
    return b == 1 ? 1 : fallback;
}

const char* SwitchName(Sw s) { return s < Sw::Count ? kSwitches[static_cast<unsigned>(s)].key : "?"; }

void NoteSkipped(Sw s, const char* what) {
    if (s >= Sw::Count) return;
    const unsigned i = static_cast<unsigned>(s);
    InterlockedIncrement(&g_skipped[i]);
    if (InterlockedIncrement(&g_skipLogged[i]) <= 3) {
        FC4_LOGV("switch '%s' off: %s skipped", kSwitches[i].key, what);
    }
}

void LogSkipCounts(const char* why) {
    std::string list;
    for (unsigned i = 0; i < kCount; ++i) {
        const LONG n = InterlockedExchange(&g_skipped[i], 0);
        if (n == 0) continue;
        char item[64];
        _snprintf_s(item, sizeof(item), _TRUNCATE, "%s%s %ld", list.empty() ? "" : ", ", kSwitches[i].key, n);
        list += item;
    }
    if (!list.empty()) Log("switches (%s): calls skipped since the last report: %s", why, list.c_str());
}

std::string Setting(const char* iniKey, const char* envName, const char** source) {
    if (source != nullptr) *source = "";
    const std::map<std::string, std::string> ini = ReadIni(nullptr);
    auto it = ini.find(Lower(iniKey));
    if (it != ini.end()) {
        if (source != nullptr) *source = "FreeCity4.ini";
        return it->second;
    }
    std::string v = Env(envName);
    if (!v.empty() && source != nullptr) *source = envName;
    return v;
}

void ConfigReload(const char* why) {
    bool found = false;
    const std::map<std::string, std::string> ini = ReadIni(&found);
    for (const auto& kv : ini) {
        if (!KnownKey(kv.first)) Log("config: WARNING unknown key '%s' in FreeCity4.ini (ignored)", kv.first.c_str());
    }

    // budget_default (ini > env FC4_BUDGET_DEFAULT): the value of unset switches.
    int def = 1;
    {
        auto it = ini.find("budget_default");
        const std::string v = it != ini.end() ? it->second : Env("FC4_BUDGET_DEFAULT");
        if (!v.empty()) {
            const int b = ParseBool(v);
            if (b < 0) {
                Log("config: WARNING budget_default='%s' is not 0/1; using 1", v.c_str());
            } else {
                def = b;
            }
        }
    }

    std::string off, fromIni, fromEnv;
    unsigned offCount = 0;
    for (unsigned i = 0; i < kCount; ++i) {
        int value = def;
        auto it = ini.find(kSwitches[i].key);
        std::string raw;
        const char* src = nullptr;
        if (it != ini.end()) {
            raw = it->second;
            src = "ini";
        } else {
            raw = Env(kSwitches[i].env);
            if (!raw.empty()) src = "env";
        }
        if (src != nullptr) {
            const int b = ParseBool(raw);
            if (b < 0) {
                Log("config: WARNING %s='%s' (%s) is not 0/1; keeping %d", kSwitches[i].key, raw.c_str(), src, def);
            } else {
                value = b;
            }
            std::string& list = (src[0] == 'i') ? fromIni : fromEnv;
            list += list.empty() ? "" : ", ";
            list += kSwitches[i].key;
            list += value ? "=1" : "=0";
        }
        InterlockedExchange(&g_on[i], value);
        if (!value) {
            off += off.empty() ? "" : ", ";
            off += kSwitches[i].key;
            ++offCount;
        }
    }
    Log("config (%s): %s %s; budget_default=%d; set by ini: %s; set by env: %s", why, IniPath().c_str(),
        found ? "found" : "not found", def, fromIni.empty() ? "-" : fromIni.c_str(),
        fromEnv.empty() ? "-" : fromEnv.c_str());
    strncpy_s(g_offList, off.empty() ? "-" : off.c_str(), _TRUNCATE);
    for (char* p = g_offList; *p; ++p) {
        if (*p == ' ') *p = ',';  // "a, b" -> "a,,b" is avoided below
    }
    {
        // Compact "a,,b" (from ", ") to "a,b" for a single-token field.
        char* w = g_offList;
        for (const char* r = g_offList; *r; ++r) {
            if (!(*r == ',' && w > g_offList && w[-1] == ',')) *w++ = *r;
        }
        *w = 0;
    }
    if (offCount == 0) {
        Log("config (%s): all %u budget switches ON", why, kCount);
    } else {
        Log("config (%s): budget switches OFF (%u of %u): %s", why, offCount, kCount, off.c_str());
    }
}

}  // namespace fc4
