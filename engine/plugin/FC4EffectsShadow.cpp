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

// The effects layer's same-process shadow (FC4EffectsShadow.h).

#include "FC4EffectsShadow.h"

#include "FC4Config.h"
#include "FC4EffectsHost.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Diag.h"
#include "FC4Interfaces.h"
#include "GZServPtrs.h"
#include "cIGZCOM.h"
#include "cGZPersistResourceKey.h"
#include "cIGZPersistResourceManager.h"
#include "cIGZPersistDBRecord.h"
#include "cIGZPersistDBSerialRecord.h"
#include "cIGZDBSerialRecordAsIGZFile.h"
#include "cIGZFile.h"
#include "cISC4City.h"
#include "cISC4EffectsManager.h"
#include "cIGZMessage2.h"
#include "cIGZMessageTarget2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "fc4.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <cstdio>
#include <cstring>
#include <map>
#include <string>
#include <vector>

namespace fc4 {
namespace {

struct Shadow {
    bool enabled = false;
    bool effectsReplaced = false;
    cISC4City* city = nullptr;
    fc4_effects* core = nullptr;
    fc4_effects_directory* dir = nullptr;
    fc4_effects_host host = {};
    EffectsHostContext ctx;
    // The mirror's effect ids by name, so an observed Start / Stop finds the
    // mirror's twin.
    std::map<std::string, uint32_t> byName;
    EffectsShadowCounts counts;
    bool loggedOnce = false;
};


// The animation tick (effects.md §12).
constexpr uint32_t kMsgAnimationTick = 0x86AD10EE;

// The shadow's own message target: so the mirror ticks even when the effects
// manager is the original's (the shadow does not hook the original's vtables).
class ShadowTarget final : public cIGZMessageTarget2 {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid == GZIID_cIGZUnknown) {
            *ppvObj = static_cast<cIGZUnknown*>(this);
            AddRef();
            return true;
        }
        *ppvObj = nullptr;
        return false;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override {
        if (refs_ == 0) return 0;
        const uint32_t n = --refs_;
        if (n == 0) delete this;
        return n;
    }
    bool DoMessage(cIGZMessage2* pMessage) override {
        if (pMessage == nullptr || pMessage->GetType() != kMsgAnimationTick) return true;
        uint32_t us = 0;
        cIGZMessage2Standard* m = nullptr;
        if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) && m != nullptr) {
            us = static_cast<uint32_t>(m->GetData2());
            m->Release();
        }
        EffectsShadowOnTick(us);
        return true;
    }

private:
    uint32_t refs_ = 1;
};


ShadowTarget* TheTarget() {
    static ShadowTarget* t = new ShadowTarget();
    return t;
}

Shadow g;
bool g_recordEnabled = false;

void NoteDifference(const char* what) {
    ++g.counts.differences;
    Log("effects shadow: DIFFERENCE %s", what);
}

// The mirror's directory: the same one the live manager uses when it is ours,
// so the comparison is of behaviour, not of data.
bool LoadDirectory() {
    g.dir = fc4_effects_directory_volcano();
    return g.dir != nullptr;
}

}  // namespace

void EffectsShadowStartup(bool effectsReplaced) {
    const long on = SettingInt("effects_shadow", "FC4_EFFECTS_SHADOW", 0);
    g.enabled = on != 0;
    g.effectsReplaced = effectsReplaced;
    if (!g.enabled) return;
    // The old replacement-call mirror is separate from native evidence.
    // Native runs use no ticking core, message subscription or host writes.
    g_recordEnabled = !effectsReplaced;
    if (g_recordEnabled) {
        g.enabled = false;
        Log("effects shadow: enabled native record-only names (EF20 limits collection capture)");
        return;
    }
    Log("effects shadow: enabled (%s; the mirror records every write)",
        effectsReplaced ? "our effects manager is live" : "the original's effects manager is live");
    if (g.core == nullptr) {
        g.ctx = EffectsHostContext();
        g.ctx.shadow = true;
        g.host = MakeEffectsHost(&g.ctx);
        g.core = fc4_effects_create(&g.host, 0);
        if (g.core == nullptr) {
            Log("effects shadow: WARNING the core refused the host table; the shadow is off");
            g.enabled = false;
            return;
        }
        if (LoadDirectory()) fc4_effects_set_directory(g.core, g.dir);
        fc4_effects_set_max_priority(g.core, 5);
        // The mirror finishes outside the draw pass, the same choice as the
        // live manager's default (EF2). A draw_pass run sets both sides.
        const long finish = SettingInt("effects_finish", "FC4_EFFECTS_FINISH", 0);
        fc4_effects_set_finish_mode(g.core, finish != 0 ? FC4_EFFECTS_FINISH_DRAW_PASS : FC4_EFFECTS_FINISH_HOOK);
    }
    cIGZMessageServer2Ptr ms;
    if (ms) {
        ms->AddNotification(static_cast<cIGZMessageTarget2*>(TheTarget()), kMsgAnimationTick);
    } else {
        Log("effects shadow: WARNING no message server; the mirror will not tick");
    }
}

void EffectsShadowStop() {
    g_recordEnabled = false;
    if (g.enabled && g.core != nullptr) {
        char buf[512] = {};
        Log("%s", EffectsShadowSummary(buf, sizeof(buf)));
    }
    if (g.core != nullptr) {
        fc4_effects_destroy(g.core);
        g.core = nullptr;
    }
    if (g.dir != nullptr) {
        fc4_effects_directory_destroy(g.dir);
        g.dir = nullptr;
    }
    cIGZMessageServer2Ptr ms;
    if (ms) ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(TheTarget()), kMsgAnimationTick);
    g.byName.clear();
    g = Shadow();
}

void EffectsShadowCityLoaded(cISC4City* city) {
    if (!g.enabled) return;
    g.city = city;
    g.ctx.city = city;
    Log("effects shadow: city loaded");
}

void EffectsShadowCityShutdown() {
    if (!g.enabled) return;
    if (g.core != nullptr) fc4_effects_shutdown(g.core);
    g.byName.clear();
    g.city = nullptr;
    g.ctx.city = nullptr;
}

void EffectsShadowOnTick(uint32_t us) {
    if (!g.enabled || g.core == nullptr) return;
    ++g.counts.ticks;
    fc4_effects_tick(g.core, us);
    fc4_effects_finish_hook(g.core);
    fc4_effects_state st = {};
    if (fc4_effects_get_state(g.core, &st)) {
        g.counts.created = st.created;
        g.counts.endPosts = st.end_posts;
        g.counts.selectDraws = st.select_draws;
        g.counts.sweeps = st.sweeps;
        g.counts.brushApplies = g.ctx.brushApplies;
    }
    g.counts.writes = 0;
    for (const auto& kv : g.ctx.writes) g.counts.writes += kv.second;
}

// ---- the live side's calls, mirrored -------------------------------------------------------------

void EffectsShadowObserveCreate(const char* name) {
    if (!g.enabled || g.core == nullptr || name == nullptr) return;
    uint32_t id = 0;
    if (!fc4_effects_create_effect(g.core, name, &id)) {
        // The mirror could not make the effect the live side did.
        NoteDifference(name);
        return;
    }
    g.byName[name] = id;
}

void EffectsShadowObserveEndMessage(const char* name, uint32_t type, uint64_t data1) {
    if (!g.enabled || g.core == nullptr || name == nullptr) return;
    auto it = g.byName.find(name);
    if (it == g.byName.end()) return;
    fc4_effects_set_end_message(g.core, it->second, type, data1);
}

void EffectsShadowObserveStartStop(const char* name, bool start, uint32_t mode) {
    if (!g.enabled || g.core == nullptr || name == nullptr) return;
    auto it = g.byName.find(name);
    if (it == g.byName.end()) return;
    if (start) {
        fc4_effects_start(g.core, it->second, mode);
    } else {
        fc4_effects_stop(g.core, it->second, mode);
    }
}

EffectsShadowCounts EffectsShadowGetCounts() { return g.counts; }

const char* EffectsShadowSummary(char* buf, unsigned cap) {
    if (buf == nullptr || cap == 0) return "";
    std::snprintf(buf, cap,
                  "effects shadow: ticks %llu, created %llu, end posts %llu, select draws %llu, brush applies %llu, "
                  "sweeps %llu, host writes %llu, differences %llu%s",
                  static_cast<unsigned long long>(g.counts.ticks), static_cast<unsigned long long>(g.counts.created),
                  static_cast<unsigned long long>(g.counts.endPosts),
                  static_cast<unsigned long long>(g.counts.selectDraws),
                  static_cast<unsigned long long>(g.counts.brushApplies),
                  static_cast<unsigned long long>(g.counts.sweeps), static_cast<unsigned long long>(g.counts.writes),
                  static_cast<unsigned long long>(g.counts.differences),
                  g.counts.differences == 0 ? "" : "  <-- see docs/lanes/glue22.md");
    return buf;
}

namespace {

// Public SDK raw-record path; never requests a decoded resource or invokes
// its collection reader. OpenDBRecord's borrowed record is closed via rm.
bool CaptureDirectory(std::vector<uint8_t>& bytes) {
    cIGZPersistResourceManagerPtr rm;
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    if (!rm || !com) return false;
    cGZPersistResourceKey key(0xEA5118B0, 0xEA5118B1, 1);
    cIGZPersistDBRecord* rec = nullptr;
    (void)rm->OpenDBRecord(key, &rec, false);
    if (!rec) return false;
    cIGZPersistDBSerialRecord* serial = nullptr;
    cIGZFile* file = nullptr;
    cIGZDBSerialRecordAsIGZFile* binding = nullptr;
    bool ok = rec->QueryInterface(GZIID_cIGZPersistDBSerialRecord, reinterpret_cast<void**>(&serial)) && serial;
    if (serial) FC4_ACQUIRED(serial, "effects raw serial QI");
    ok = ok && com->GetClassObject(GZCLSID_cIGZDBSerialRecordAsIGZFile,
        GZIID_cIGZFile, reinterpret_cast<void**>(&file)) && file;
    if (file) FC4_ACQUIRED(file, "effects raw file adapter");
    ok = ok && file->QueryInterface(GZIID_cIGZDBSerialRecordAsIGZFile,
        reinterpret_cast<void**>(&binding)) && binding;
    if (binding) {
        FC4_ACQUIRED(binding, "effects raw binding QI");
        (void)binding->SetDBSerialRecord(serial);
        FC4_RELEASE(binding);
    }
    if (ok) {
        ok = file->IsOpen() || file->Open(cIGZFile::AccessMode::Read,
            cIGZFile::CreationMode::OpenExisting, cIGZFile::ShareMode::Read);
        const uint32_t n = file->Length();
        ok = ok && n > 0 && n <= 16u * 1024u * 1024u && file->SeekToBegin() == 0;
        if (ok) {
            bytes.resize(n);
            ok = file->Read(bytes.data(), n);
        }
        if (file->IsOpen()) ok = file->Close() && ok;
    }
    if (file) FC4_RELEASE_CREATED(file);
    if (serial) FC4_RELEASE(serial);
    rm->CloseDBRecord(key, &rec);
    return ok;
}

uint32_t NativeHasName(void* user, const char* name) {
    // Borrowed city manager: no AddRef/Release and no mutation.
    // [abi-ret] effects.md section 4.2: slot 5 answers name existence;
    // public SDK declares bool. Compare that boolean only, never an index.
    return static_cast<cISC4EffectsManager*>(user)->HasVisualEffect(name) ? 1u : 0u;
}

struct NameSample {
    cISC4City* city;
    uint32_t counts[3] = {};
    uint32_t bytes = 0;
    bool captured = false;
};

void CompareNames(void* context) {
    auto& s = *static_cast<NameSample*>(context);
    auto* manager = s.city ? s.city->GetEffectsManager() : nullptr;
    std::vector<uint8_t> bytes;
    if (!manager || !CaptureDirectory(bytes)) return;
    s.bytes = static_cast<uint32_t>(bytes.size());
    s.captured = fc4_effects_verify_names(bytes.data(), bytes.size(), NativeHasName, manager, s.counts);
}

} // namespace

void EffectsShadowObserve(cISC4City* city, int32_t julian, bool load) {
    if (!g_recordEnabled) return;
    // City load precedes the observer's first published date. Read the live
    // simulator interface rather than rendering Julian zero as a BC year.
    if (load && city) {
        auto* sim = AsGame(city->GetSimulator());
        if (sim) julian = sim->GetSimDateNumber();
    }
    NameSample sample{city};
    const bool safe = SehGuard("effects record shadow", CompareNames, &sample);
    const uint32_t refusal = safe && sample.captured && sample.counts[1] > 0 ? 0u : 1u;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    Log("effects shadow %04d-%02d-%02d %s: bytes=%u entries=%u queries=%u refusal=%u resyncs=0; mismatches=%u",
        date.year, date.month, date.day_of_month, load ? "load" : "month",
        sample.bytes, sample.counts[0], sample.counts[1], refusal, sample.counts[2] + refusal);
}

}  // namespace fc4
