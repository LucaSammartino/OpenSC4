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

#include "FC4AuraShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4HostGrid.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Aura.h"

#include "cGZPersistResourceKey.h"
#include "cIGZSerializable.h"
#include "cIGZString.h"
#include "cISC4AuraSimulator.h"
#include "cISC4City.h"

#include <algorithm>
#include <cstring>
#include <vector>

namespace fc4 {
namespace {

// The interface the §11.3 reader asks a reference for (fc4_aura_capture_ref).
constexpr uint32_t kIID_cISC4SimGrid = 0xC9BD939A;
constexpr long kDefaultCells = 4;
constexpr long kMaxCells = 64;
// fc4::Log formats into a fixed buffer: longer shadow lines are split.
constexpr size_t kLogChunk = 2800;

// ---- the capture stream ------------------------------------------------------------------------------
//
// The original's Write goes into memory: primitives little-endian (as the
// game's streams write them), strings as a uint32 length then the bytes, and
// references (WriteGZSerializable) as uint32 objectID (index into the
// reference table + 1, 0 = null) then uint32 classID, each object listed once
// with its cISC4SimGrid identity. The identities are borrowed: the original
// keeps its grids alive (the comparison runs inside one message delivery).
// A process-lifetime object (the reference count is not kept).
class CaptureStream final : public cFC4ISC4DBSegmentOStream {
public:
    void Reset() {
        bytes_.clear();
        refs_.clear();
        ids_.clear();
        good_ = true;
    }

    fc4_aura_capture View(bool writeOk) const {
        fc4_aura_capture c{};
        c.struct_size = sizeof(c);
        c.ok = (writeOk && good_) ? 1u : 0u;
        c.bytes = bytes_.empty() ? nullptr : bytes_.data();
        c.len = bytes_.size();
        c.refs = refs_.empty() ? nullptr : refs_.data();
        c.ref_count = refs_.size();
        return c;
    }

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid == kFC4_IID_cISC4DBSegmentOStream || riid == GZIID_cIGZUnknown) {
            *ppvObj = static_cast<cFC4ISC4DBSegmentOStream*>(this);
            return true;
        }
        return false;
    }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }

    // cIGZOStream
    void Flush() override {}
    bool SetSint8(int8_t v) override { return Put(&v, 1); }
    bool SetUint8(uint8_t v) override { return Put(&v, 1); }
    bool SetSint16(int16_t v) override { return Put(&v, 2); }
    bool SetUint16(uint16_t v) override { return Put(&v, 2); }
    bool SetSint32(int32_t v) override { return Put(&v, 4); }
    bool SetUint32(uint32_t v) override { return Put(&v, 4); }
    bool SetSint64(int64_t v) override { return Put(&v, 8); }
    bool SetUint64(uint64_t v) override { return Put(&v, 8); }
    bool SetFloat32(float v) override { return Put(&v, 4); }
    bool SetFloat64(double v) override { return Put(&v, 8); }
    bool SetRZCharStr(char const* s) override { return PutStr(s, s ? strlen(s) : 0); }
    bool SetGZStr(cIGZString const& s) override { return PutStr(s.Data(), s.Strlen()); }
    bool SetGZSerializable(cIGZSerializable const& s) override {
        if (!const_cast<cIGZSerializable&>(s).Write(*this)) good_ = false;
        return good_;
    }
    bool SetVoid(void const* p, uint32_t n) override {
        if (n != 0 && p == nullptr) {
            good_ = false;
            return false;
        }
        return Put(p, n);
    }
    int32_t GetError() override { return good_ ? 0 : 1; }
    int32_t SetUserData(cIGZVariant*) override { return 0; }
    int32_t GetUserData() override { return 0; }

    // cISC4DBSegmentOStream
    bool Open(cISC4DBSegment*, cGZPersistResourceKey const&, bool) override { return false; }
    bool Close() override { return true; }
    bool IsOpen() override { return true; }
    int32_t GetRecord() override { return 0; }
    int32_t GetSegment() override { return 0; }
    bool WriteGZSerializable(cIGZSerializable const* p) override {
        if (p == nullptr) return SetUint32(0);
        cIGZSerializable* obj = const_cast<cIGZSerializable*>(p);
        size_t i = 0;
        while (i < ids_.size() && ids_[i] != obj) ++i;
        if (i == ids_.size()) {
            fc4_aura_capture_ref r{};
            r.grid = Borrowed(obj, kIID_cISC4SimGrid);
            r.class_id = obj->GetGZCLSID();
            ids_.push_back(obj);
            refs_.push_back(r);
        }
        SetUint32(static_cast<uint32_t>(i + 1));
        return SetUint32(refs_[i].class_id);
    }
    bool WriteResKey(cGZPersistResourceKey const& k) override {
        SetUint32(k.type);
        SetUint32(k.group);
        return SetUint32(k.instance);
    }
    bool WriteVariant(cIGZVariant const&) override {
        good_ = false;  // not part of the aura record (§11.2)
        return false;
    }

private:
    bool Put(const void* p, size_t n) {
        const uint8_t* b = static_cast<const uint8_t*>(p);
        bytes_.insert(bytes_.end(), b, b + n);  // x86: already little-endian
        return good_;
    }
    bool PutStr(const char* s, size_t n) {
        if (n > 0x00FFFFFF || (n != 0 && s == nullptr)) {
            good_ = false;
            return false;
        }
        SetUint32(static_cast<uint32_t>(n));
        return Put(s, n);
    }
    // The object's identity for `iid` (0 if it does not answer): queried,
    // and the reference dropped at once.
    static uint64_t Borrowed(cIGZSerializable* obj, uint32_t iid) {
        void* q = nullptr;
        if (!obj->QueryInterface(iid, &q) || q == nullptr) return 0;
        cIGZUnknown* u = static_cast<cIGZUnknown*>(q);
        FC4_ACQUIRED(u, "QueryInterface");
        const uint64_t t = TokenOf(u);
        FC4_RELEASE(u);
        return t;
    }

    std::vector<uint8_t> bytes_;
    std::vector<fc4_aura_capture_ref> refs_;
    std::vector<const void*> ids_;
    bool good_ = true;
};

CaptureStream g_capture;

// ---- settings and run state (main thread only) ---------------------------------------------------

bool g_enabled = false;
long g_cells = kDefaultCells;

// Test seam (op 14).
bool g_testMode = false;

bool g_loggedOurs = false;
bool g_loggedNoSerializable = false;
bool g_loggedNoAura = false;

// The shadow core, per city and original simulator. Its reads go to g_game,
// filled with the city's borrowed pointer for one comparison.
FC4AuraGame g_game;
const HostGridContext g_gridCtx{"aura shadow", Sw::AuraGridWrites};
fc4_aura_shadow* g_shadow = nullptr;
const void* g_shadowCity = nullptr;
const void* g_shadowSim = nullptr;
long g_shadowCells = 0;
const void* g_shadowFailedFor = nullptr;

// The comparison being run (SehGuard's context).
struct Run {
    cISC4AuraSimulator* sim = nullptr;
    bool baseline = false;
    int32_t julian = 0;
    bool done = false;
    int32_t result = -1;
};
Run g_run;
intptr_t g_lastResult = -1;

void ReadSettings() {
    g_enabled = SettingInt("aura_shadow", "FC4_AURA_SHADOW", 0) != 0;
    long n = SettingInt("aura_shadow_cells", "FC4_AURA_SHADOW_CELLS", kDefaultCells);
    if (n <= 0) n = kDefaultCells;
    if (n > kMaxCells) n = kMaxCells;
    g_cells = n;
}

// The original aura simulator of `city` (borrowed), or nullptr (logged once
// when it is ours).
cISC4AuraSimulator* OriginalOf(cISC4City* city) {
    if (city == nullptr) return nullptr;
    cISC4AuraSimulator* raw = city->GetAuraSimulator();  // compiled slot 51, borrowed
    if (raw == nullptr) {
        if (!g_loggedNoAura) {
            g_loggedNoAura = true;
            Log("aura shadow: the city has no aura simulator; nothing compared");
        }
        return nullptr;
    }
    if (IsOurObject(raw)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("aura shadow: inactive: the city's aura simulator is FreeCity4's own (the shadow compares against the "
                "ORIGINAL aura simulator: do not enable aura)");
        }
        return nullptr;
    }
    return raw;
}

void DropShadow() {
    fc4_aura_shadow_destroy(g_shadow);
    g_shadow = nullptr;
    g_shadowCity = nullptr;
    g_shadowSim = nullptr;
}

// A fresh shadow core when the city, the original or the cell setting
// changed (or `fresh`: a city load). g_game.city must be set (the core reads
// the cell widths at creation).
bool EnsureShadow(cISC4City* city, const void* sim, bool fresh) {
    if (!fresh && g_shadow != nullptr && g_shadowCity == city && g_shadowSim == sim && g_shadowCells == g_cells) {
        return true;
    }
    DropShadow();
    const int32_t cx = static_cast<int32_t>(city->CellCountX());  // slot 93
    const int32_t cz = static_cast<int32_t>(city->CellCountZ());  // slot 94
    const fc4_aura_host host = AuraReadOnlyHost(&g_game, &g_gridCtx);  // copied by the core
    g_shadow = fc4_aura_shadow_create(&host, cx, cz, static_cast<uint32_t>(g_cells));
    if (g_shadow == nullptr) {
        if (g_shadowFailedFor != sim) {
            g_shadowFailedFor = sim;
            Log("aura shadow: the shadow core could not be created (city %d x %d cells); nothing compared", cx, cz);
        }
        return false;
    }
    g_shadowCity = city;
    g_shadowSim = sim;
    g_shadowCells = g_cells;
    Log("aura shadow: comparing with the original aura simulator %p (city %p, %d x %d cells); differing tracts "
        "listed per grid: %ld",
        sim, static_cast<void*>(city), cx, cz, g_cells);
    return true;
}

// The original's record through its own Write (cIGZSerializable).
fc4_aura_capture Capture(cISC4AuraSimulator* sim) {
    g_capture.Reset();
    bool ok = false;
    void* p = nullptr;
    if (sim->QueryInterface(GZIID_cIGZSerializable, &p) && p != nullptr) {
        cIGZSerializable* ser = static_cast<cIGZSerializable*>(p);
        FC4_ACQUIRED(ser, "QueryInterface");
        ok = ser->Write(g_capture);
        FC4_RELEASE(ser);
    } else if (!g_loggedNoSerializable) {
        g_loggedNoSerializable = true;
        Log("aura shadow: the original aura simulator does not answer cIGZSerializable; its record (and with it "
            "the grids) cannot be read: nothing is compared");
    }
    return g_capture.View(ok);
}

// fc4::Log has a fixed buffer: a long line goes out in pieces, split at a
// space where there is one.
void Sink(void*, const uint8_t* data, size_t len) {
    const char* p = reinterpret_cast<const char*>(data);
    size_t off = 0;
    bool first = true;
    while (first || off < len) {
        size_t n = std::min(kLogChunk, len - off);
        if (off + n < len) {
            size_t cut = n;
            while (cut > kLogChunk / 2 && p[off + cut - 1] != ' ') --cut;
            if (cut > kLogChunk / 2) n = cut;
        }
        if (first) {
            Log("%.*s", static_cast<int>(n), p + off);
        } else {
            Log("aura shadow (cont.) %.*s", static_cast<int>(n), p + off);
        }
        off += n;
        first = false;
    }
}

void Step(void*) {
    const fc4_aura_capture c = Capture(g_run.sim);
    if (g_run.baseline) {
        g_run.result = fc4_aura_shadow_baseline(g_shadow, &c, &Sink, nullptr) ? 1 : 0;
    } else {
        g_run.result = fc4_aura_shadow_cycle(g_shadow, &c, g_run.julian, &Sink, nullptr);
    }
    g_run.done = true;
}

// One baseline or cycle on `city`'s original aura simulator.
void RunOn(cISC4City* city, int32_t julian, bool baseline) {
    g_lastResult = -1;
    cISC4AuraSimulator* sim = OriginalOf(city);
    if (sim == nullptr) return;
    g_game.gated = false;  // the original's inputs as they are
    g_game.tag = "aura shadow";
    g_game.city = city;    // borrowed for this call
    if (!EnsureShadow(city, sim, baseline)) {
        g_game.city = nullptr;
        return;
    }
    g_run = Run{};
    g_run.sim = sim;
    g_run.baseline = baseline;
    g_run.julian = julian;
    const bool clean = SehGuard(baseline ? "aura shadow (baseline at city load)" : "aura shadow (cycle)", &Step,
                                nullptr);
    if (!clean && !g_run.done && !baseline) {
        // The capture or our cycle faulted: "no capture" for this cycle.
        g_run.result = fc4_aura_shadow_cycle(g_shadow, nullptr, julian, &Sink, nullptr);
    }
    g_game.DropHeld();  // the population grids of our cycle
    g_game.city = nullptr;
    g_lastResult = g_run.done ? g_run.result : (baseline ? 0 : -2);
    if (!baseline && g_lastResult < 0) g_lastResult = -2;
    g_run = Run{};
}

}  // namespace

void AuraShadowStartup(bool auraReplaced) {
    ReadSettings();
    if (!g_enabled) return;  // off by default: not a word in the log
    const char* src = "";
    Setting("aura_shadow", "FC4_AURA_SHADOW", &src);
    if (!auraReplaced) {
        Log("aura shadow: ON (aura_shadow from %s): at city load the ORIGINAL aura simulator's record becomes the "
            "baseline; at each of its cycle-completed messages (0x2A09B21A) FreeCity4's cycle (states 1-17) runs on "
            "its record with the previous record's base and transient grids, reading the other inputs read-only, "
            "and is compared tract by tract (base, aura, park and landmark maps), with the mayor rating, best / "
            "worst and hotspots; one line per cycle; differing tracts listed per grid: %ld. Not compared: the riot "
            "roll (wall-clock RNG, aura.md §9), and posts, history, riots and strikes (writes)",
            src, g_cells);
    } else {
        Log("aura shadow: requested (aura_shadow from %s) but INACTIVE: it compares against the ORIGINAL aura "
            "simulator, and aura is enabled (ours)",
            src);
    }
}

void AuraShadowCityLoaded(cISC4City* city) {
    if (!g_testMode) ReadSettings();  // FreeCity4.ini may change between city loads
    if (!g_testMode && !g_enabled) return;
    RunOn(city, 0, true);
}

void AuraShadowCycle(cISC4City* city, int32_t julian) {
    if (!g_testMode && !g_enabled) return;
    RunOn(city, julian, false);
}

}  // namespace fc4

intptr_t AuraShadowTestCycle(const FC4AuraShadowTestInput* in) {
    if (in == nullptr) return -1;
    fc4::g_testMode = true;
    fc4::g_shadowFailedFor = nullptr;
    fc4::g_lastResult = -1;
    if (in->baseline != 0) {
        fc4::DropShadow();  // each baseline is a fresh "game"
        fc4::g_loggedOurs = false;
        fc4::g_loggedNoAura = false;
        fc4::g_loggedNoSerializable = false;
        fc4::AuraShadowCityLoaded(in->city);
    } else {
        fc4::AuraShadowCycle(in->city, in->julian);
    }
    fc4::g_testMode = false;
    return fc4::g_lastResult;
}
