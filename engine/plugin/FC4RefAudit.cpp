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

#include "FC4RefAudit.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Log.h"

#include "cIGZSerializable.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdlib>
#include <map>
#include <unordered_map>

namespace fc4 {
namespace {

struct ClassStats {
    uint32_t clsid = 0;
    bool serializable = false;
    long addRefs = 0;
    long acquired = 0;
    long releases = 0;
    long handedOut = 0;
    long lastRef = 0;
};

struct Held {
    long net = 0;
    uintptr_t vptr = 0;
};

struct State {
    CRITICAL_SECTION lock;
    std::map<uintptr_t, ClassStats> classes;  // by vtable; the class ID cache survives summaries
    std::unordered_map<const void*, Held> held;
    long events = 0;
    State() { InitializeCriticalSection(&lock); }
};

State& S() {
    static State s;
    return s;
}

volatile LONG g_enabled = 1;
volatile LONG g_lineCap = 2000;
volatile LONG g_lines = 0;
volatile LONG g_warnings = 0;
constexpr LONG kWarningCap = 100;

class Lock {
public:
    Lock() { EnterCriticalSection(&S().lock); }
    ~Lock() { LeaveCriticalSection(&S().lock); }
};

void Describe(uintptr_t vptr, const ClassStats& c, char* out, size_t cap) {
    char where[96];
    DescribeAddress(vptr, where, sizeof(where));
    if (c.serializable) {
        _snprintf_s(out, cap, _TRUNCATE, "class 0x%08X, vtable %s", c.clsid, where);
    } else {
        _snprintf_s(out, cap, _TRUNCATE, "class ?, vtable %s", where);
    }
}

}  // namespace

bool IsOurObject(const void* obj) {
    if (obj == nullptr) return false;
    const uintptr_t vptr = *static_cast<const uintptr_t*>(obj);
    return vptr >= ImageBase() && vptr - ImageBase() < ImageSize();
}

void RefAuditReload() {
    const std::string on = Setting("refaudit", "FC4_REFAUDIT");
    InterlockedExchange(&g_enabled, (on == "0" || on == "off" || on == "false" || on == "no") ? 0 : 1);
    const std::string cap = Setting("refaudit_lines", "FC4_REFAUDIT_LINES");
    if (!cap.empty()) InterlockedExchange(&g_lineCap, std::strtol(cap.c_str(), nullptr, 10));
    Log("refaudit: %s (verbose line cap %ld, %ld used)", g_enabled ? "on" : "OFF (refaudit=0)", g_lineCap, g_lines);
}

RefTarget RefBefore(cIGZUnknown* obj) {
    RefTarget t;
    t.obj = obj;
    if (obj == nullptr || g_enabled == 0 || IsOurObject(obj)) return t;
    t.vptr = *reinterpret_cast<const uintptr_t*>(obj);
    t.audited = true;
    bool lookup = false;
    {
        Lock l;
        lookup = S().classes.find(t.vptr) == S().classes.end();
        if (lookup) S().classes[t.vptr] = ClassStats{};
    }
    if (lookup) {
        // Once per vtable, while the object is certainly alive (raw calls, not audited).
        cIGZSerializable* ser = nullptr;
        uint32_t clsid = 0;
        bool ok = false;
        if (obj->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ser)) && ser != nullptr) {
            clsid = ser->GetGZCLSID();
            ok = true;
            ser->Release();
        }
        Lock l;
        ClassStats& c = S().classes[t.vptr];
        c.clsid = clsid;
        c.serializable = ok;
    }
    return t;
}

void RefNote(const RefTarget& t, int delta, long newCount, const char* how, RefSite site, bool owned) {
    if (!t.audited) return;
    char cls[160];
    long net = 0;
    bool overRelease = false;
    {
        Lock l;
        State& s = S();
        ++s.events;
        ClassStats& c = s.classes[t.vptr];
        if (delta > 0) {
            if (!owned) {
                ++c.handedOut;
            } else if (newCount < 0) {
                ++c.acquired;
            } else {
                ++c.addRefs;
            }
        } else {
            ++c.releases;
            if (newCount == 0) ++c.lastRef;
        }
        if (owned) {
            Held& h = s.held[t.obj];
            h.net += delta;
            h.vptr = t.vptr;
            net = h.net;
            overRelease = net < 0;
            if (net == 0) s.held.erase(t.obj);
        }
        Describe(t.vptr, c, cls, sizeof(cls));
    }
    char count[16];
    if (newCount < 0) {
        strcpy_s(count, "?");
    } else {
        _snprintf_s(count, sizeof(count), _TRUNCATE, "%ld", newCount);
    }
    if (LogLevel() >= kLogVerbose) {
        if (InterlockedIncrement(&g_lines) <= g_lineCap) {
            LogVerboseImpl("ref %+d %s %p [%s] -> count %s, ours %ld, at %s:%d", delta, how, t.obj, cls, count, net,
                           site.func, site.line);
        } else {
            NoteSuppressed();
        }
    }
    if (delta < 0 && newCount == 0 && InterlockedIncrement(&g_warnings) <= kWarningCap) {
        Log("WARNING: released last reference to game object %p [%s] at %s:%d (the game still expects to own it)",
            t.obj, cls, site.func, site.line);
    }
    if (overRelease && InterlockedIncrement(&g_warnings) <= kWarningCap) {
        Log("WARNING: over-release of game object %p [%s] at %s:%d: released %ld more time(s) than the audited sites "
            "acquired it",
            t.obj, cls, site.func, site.line, -net);
    }
}

void RefSummary(const char* why) {
    if (g_enabled == 0) return;
    Lock l;
    State& s = S();
    if (s.events == 0 && s.held.empty()) {
        Log("refaudit (%s): no game-object reference events since the last summary", why);
        return;
    }
    Log("refaudit (%s): %ld event(s) since the last summary; per class (net = AddRef + acquired - Release):", why,
        s.events);
    for (auto& kv : s.classes) {
        ClassStats& c = kv.second;
        if (c.addRefs == 0 && c.acquired == 0 && c.releases == 0 && c.handedOut == 0) continue;
        char cls[160];
        Describe(kv.first, c, cls, sizeof(cls));
        Log("  [%s] AddRef %ld, acquired %ld, Release %ld, net %+ld; handed to the game %ld; last-reference "
            "releases %ld%s",
            cls, c.addRefs, c.acquired, c.releases, c.addRefs + c.acquired - c.releases, c.handedOut, c.lastRef,
            (c.addRefs + c.acquired - c.releases) < 0 ? "  <-- NEGATIVE" : "");
        c.addRefs = c.acquired = c.releases = c.handedOut = c.lastRef = 0;
    }
    Log("  objects we still hold references to: %u", static_cast<unsigned>(s.held.size()));
    unsigned shown = 0;
    for (const auto& kv : s.held) {
        if (++shown > 20) {
            Log("    ...");
            break;
        }
        char cls[160];
        Describe(kv.second.vptr, s.classes[kv.second.vptr], cls, sizeof(cls));
        Log("    %p [%s] net %+ld", kv.first, cls, kv.second.net);
    }
    s.events = 0;
}

}  // namespace fc4
