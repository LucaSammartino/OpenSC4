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

#include "FC4FormatsShadow.h"

#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "fc4.h"

#include <windows.h>

#include <cstdarg>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kExemplarType = 0x6534284A;  // data-formats §8

bool g_enabled = false;
bool g_done = false;
long g_maxPerFile = 0;
long g_lines = 8;
long g_shown = 0;

struct Totals {
    uint32_t files = 0, unreadable = 0, entries = 0, compared = 0, exact = 0, differ = 0, coreFail = 0,
             gameMissing = 0, props = 0;
};

void Detail(const char* fmt, ...) {
    if (g_shown++ >= g_lines) return;
    char buf[512];
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(buf, sizeof(buf), _TRUNCATE, fmt, ap);
    va_end(ap);
    Log("formats shadow (cont.) %s", buf);
}

std::string Utf8(const std::wstring& w) {
    if (w.empty()) return {};
    const int n = WideCharToMultiByte(CP_UTF8, 0, w.c_str(), static_cast<int>(w.size()), nullptr, 0, nullptr, nullptr);
    std::string s(static_cast<size_t>(n > 0 ? n : 0), '\0');
    if (n > 0) WideCharToMultiByte(CP_UTF8, 0, w.c_str(), static_cast<int>(w.size()), &s[0], n, nullptr, nullptr);
    return s;
}

// [choice] The install folder is the parent of the folder holding the game
// executable (<install>\Apps\SimCity 4.exe); §3.1 step 1 reads its *.dat.
std::wstring InstallDir() {
    wchar_t buf[MAX_PATH * 2];
    const DWORD n = GetModuleFileNameW(nullptr, buf, static_cast<DWORD>(sizeof(buf) / sizeof(buf[0])));
    if (n == 0 || n >= sizeof(buf) / sizeof(buf[0])) return {};
    std::wstring p(buf, n);
    for (int up = 0; up < 2; ++up) {
        const size_t cut = p.find_last_of(L"\\/");
        if (cut == std::wstring::npos) return {};
        p.resize(cut);
    }
    return p;
}

void AppendBytes(void* ctx, const uint8_t* data, size_t len) {
    static_cast<std::vector<uint8_t>*>(ctx)->assign(data, data + len);
}

std::vector<uint8_t> VariantBytes(const fc4_props* p, uint32_t id) {
    std::vector<uint8_t> out;
    if (!fc4_props_write_variant(p, id, AppendBytes, &out)) out.clear();
    return out;
}

void CompareFile(const std::wstring& path, const std::string& name, Totals& t) {
    const std::string u8 = Utf8(path);
    fc4_dbpf* pkg = fc4_dbpf_open(reinterpret_cast<const uint8_t*>(u8.data()), u8.size());
    ++t.files;
    if (pkg == nullptr) {
        ++t.unreadable;
        Log("formats shadow: %s: not opened by the core's DBPF reader", name.c_str());
        return;
    }
    const size_t count = fc4_dbpf_count_type(pkg, kExemplarType);
    Totals f;
    for (size_t i = 0; i < count; ++i) {
        if (g_maxPerFile > 0 && f.compared >= static_cast<uint32_t>(g_maxPerFile)) break;
        uint32_t group = 0, instance = 0;
        if (!fc4_dbpf_key_of_type(pkg, kExemplarType, i, &group, &instance)) break;
        ++f.entries;
        fc4_props* core = fc4_dbpf_read_exemplar(pkg, kExemplarType, group, instance, nullptr);
        if (core == nullptr) {
            ++f.coreFail;
            Detail("%s %08X-%08X-%08X: the core's reader rejects it", name.c_str(), kExemplarType, group, instance);
            continue;
        }
        props::Exemplar ex;
        if (!ex.Load(kExemplarType, group, instance) || ex.Holder() == nullptr) {
            ++f.gameMissing;
            fc4_props_destroy(core);
            continue;
        }
        ++f.compared;
        fc4_props* game = fc4_props_create();
        uint32_t own = 0;
        for (uint32_t id : props::PropertyIds(ex.Holder())) {
            if (props::CopyToCore(ex.Holder(), id, game)) ++own;
        }
        bool same = fc4_props_count(core) == fc4_props_count(game);
        bool reported = false;
        if (!same) {
            Detail("%s %08X-%08X-%08X: %u own properties in the core's parse, %u in the game's", name.c_str(),
                   kExemplarType, group, instance, static_cast<unsigned>(fc4_props_count(core)), own);
        }
        for (uint32_t id : props::PropertyIds(ex.Holder())) {
            const std::vector<uint8_t> g = VariantBytes(game, id);
            if (g.empty()) continue;  // not copied (object, empty): left out on both sides
            ++f.props;
            if (VariantBytes(core, id) != g) {
                same = false;
                if (!reported) {
                    reported = true;
                    Detail("%s %08X-%08X-%08X: property 0x%08X differs (type, shape or value)", name.c_str(),
                           kExemplarType, group, instance, id);
                }
            }
        }
        fc4_props_destroy(game);
        fc4_props_destroy(core);
        if (same) {
            ++f.exact;
        } else {
            ++f.differ;
        }
    }
    fc4_dbpf_close(pkg);
    Log("formats shadow: %s: %u exemplar entries, %u compared, %u exact, %u differ, %u rejected by the core, "
        "%u not loaded by the game, %u properties compared",
        name.c_str(), f.entries, f.compared, f.exact, f.differ, f.coreFail, f.gameMissing, f.props);
    t.entries += f.entries;
    t.compared += f.compared;
    t.exact += f.exact;
    t.differ += f.differ;
    t.coreFail += f.coreFail;
    t.gameMissing += f.gameMissing;
    t.props += f.props;
}

}  // namespace

void FormatsShadowStartup() {
    g_enabled = SettingInt("formats_shadow", "FC4_FORMATS_SHADOW", 0) != 0;
    if (!g_enabled) return;  // off by default: not a word in the log
    g_maxPerFile = SettingInt("formats_shadow_max", nullptr, 0);
    g_lines = SettingInt("formats_shadow_lines", nullptr, 8);
    const char* src = "";
    Setting("formats_shadow", "FC4_FORMATS_SHADOW", &src);
    Log("formats shadow: ON (formats_shadow from %s): at the first city load each *.dat of the install folder "
        "is read by the core's DBPF and exemplar readers and every exemplar's own properties are compared with "
        "the game's loaded exemplar of the same key (nothing replaced)",
        src);
}

void FormatsShadowCityLoaded() {
    if (!g_enabled || g_done) return;
    g_done = true;
    const std::wstring dir = InstallDir();
    if (dir.empty()) {
        Log("formats shadow: install folder not found; not run");
        return;
    }
    const DWORD start = GetTickCount();
    Totals t;
    WIN32_FIND_DATAW fd;
    HANDLE h = FindFirstFileW((dir + L"\\*.dat").c_str(), &fd);
    if (h != INVALID_HANDLE_VALUE) {
        do {
            if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) continue;
            CompareFile(dir + L"\\" + fd.cFileName, Utf8(fd.cFileName), t);
        } while (FindNextFileW(h, &fd));
        FindClose(h);
    }
    Log("formats shadow: total: %u files (%u unreadable), %u exemplar entries, %u compared, %u exact, %u differ, "
        "%u rejected by the core, %u not loaded by the game, %u properties compared, %lu ms; %s",
        t.files, t.unreadable, t.entries, t.compared, t.exact, t.differ, t.coreFail, t.gameMissing, t.props,
        static_cast<unsigned long>(GetTickCount() - start),
        t.compared > 0 && t.differ == 0 && t.coreFail == 0 && t.unreadable == 0 ? "EXACT" : "DIFFERENCES");
}

}  // namespace fc4
