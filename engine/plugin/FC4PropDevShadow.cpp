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

#include "FC4PropDevShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Interfaces.h"
#include "FC4LotDevInterfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4WriteCapture.h"
#include "FC4PropDevTranscript.h"
#include "FC4PropDevSnapshot.h"
#include "cFC4PropDeveloper.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4City.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cISC4PropDeveloper.h"
#include "cISC4PropManager.h"
#include "cISC4PropOccupant.h"

#include <algorithm>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <iterator>
#include <memory>
#include <string>
#include <utility>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kMsgIdle = 0x6695681A;
constexpr uint32_t kMsgDay = 0x66956814;
constexpr uint32_t kMsgAnimationTick = 0x86AD10EE;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZSerializable = 0xE4FDA3D4;
constexpr uint32_t kCrimeSatisfaction = 0x49A1E996;  // crime.md §11.6
constexpr uint32_t kTickPriority = 0xC0;             // §3.1 step 6
constexpr const char* kAgentName = "cSC4PropDeveloper";
constexpr size_t kMaxListed = 12;
constexpr int32_t kBlock = 8;
// [choice] The shadow cores' session seed (their draws are never compared).
constexpr uint32_t kShadowSeed = 0x5EED0F47u;
// §2 offsets from the primary interface pointer (build 1.1.641).
constexpr size_t kOffScanFlag = 0x14;
constexpr size_t kOffScanner = 0x18;

bool g_enabled = false;
bool g_simulatorReplaced = false;
bool g_replaced = false;
bool g_loggedOurs = false;
bool g_listening = false;
bool g_tickRegistered = false;

const void* g_orig = nullptr;
cIGZMessageTarget2* g_agent = nullptr;
const void* g_vtable = nullptr;  // the original's, when identified

unsigned U(size_t n) { return static_cast<unsigned>(n); }

struct Diff {
    uint32_t mismatches = 0;
    std::string listed;
    size_t n = 0;
    void Add(const char* fmt, ...) {
        ++mismatches;
        if (n++ >= kMaxListed) return;
        char b[192];
        va_list ap;
        va_start(ap, fmt);
        vsnprintf(b, sizeof(b), fmt, ap);
        va_end(ap);
        if (!listed.empty()) listed += ", ";
        listed += b;
    }
};

// ---- what our step does (recorded by the shadow hooks) ---------------------------------------------

bool g_inStep = false;
std::vector<uint64_t> g_removed;  // in call order
uint32_t g_created = 0;
bool g_uncertain = false;
bool g_pendingUncertain = false;
uint32_t g_messageRemovals = 0;
PropSelectorReplay* g_selectorReplay = nullptr;
bool g_nativeOperation = false;
std::vector<fc4_prop_message> g_nested;
size_t g_nestedCursor = 0;
fc4_prop_developer* ReplayCore();

bool ReplayNested(uint32_t type, uint64_t prop) {
    if (g_nestedCursor == g_nested.size()) return false;
    const auto& m = g_nested[g_nestedCursor];
    if (m.msg_type != type || m.object != prop) return false;
    ++g_nestedCursor;
    fc4_prop_developer_do_message(ReplayCore(), &m);
    return true;
}

cISC4PropManager* PropManager() {
    cISC4City* c = propcity::AppCity();
    return c ? c->GetPropManager() : nullptr;
}

// crime.md §11.6: the prop profile's crime incident ID (+0x18).
uint32_t CrimeIncidentOf(uint32_t id) {
    cISC4PropManager* pm = PropManager();
    if (pm == nullptr) return 0xFFFFFFFFu;
    const cISC4PropOccupant::PropProfile& p = pm->GetPropProfile(id);
    uint32_t v = 0;
    std::memcpy(&v, reinterpret_cast<const uint8_t*>(&p) + 24, sizeof(v));
    return v;
}

const fc4_prop_host& RunHost();

void HookRemove(void*, uint64_t prop) {
    if (g_inStep) {
        g_removed.push_back(prop);
        if (!ReplayNested(0x99EF1143, prop)) g_uncertain = true;
    } else {
        ++g_messageRemovals;
    }
}

// A captured insertion supplies its genuine occupant token. Native creation
// arguments and failed attempts are not yet captured; never call this exact.
uint64_t HookCreate(void*, uint64_t, bool, uint32_t, uint32_t id) {
    ++g_created;
    if (g_pendingUncertain) g_uncertain = true;
    g_pendingUncertain = false;  // that select was a creation's (§4.4), not a keep check
    // Insertions observe success, not the native creation attempt's lot/index
    // or failed attempts. This operation remains incomplete.
    g_uncertain = true;
    if (g_nestedCursor == g_nested.size()) return 0;
    const auto& m = g_nested[g_nestedCursor];
    if (m.msg_type != 0x99EF1142) return 0;
    auto* frozen = PropBlockSnapshot::active;
    PropBlockSnapshot::active = nullptr;
    const bool match = PropBlockSnapshot::base.prop_type(nullptr, m.object) == id;
    const uint64_t token = m.object;
    if (match) ReplayNested(0x99EF1142, token);
    PropBlockSnapshot::active = frozen;
    return match ? token : 0;
}

// A host requester's SelectProp (crime, a custom selector of a mod) is never
// called: it would draw from its generator. One ID (a keep check, §4.3 step
// 3.7): a crime prop with incident ID 0 is kept for certain (crime.md §11.6:
// the generic list of one); anything else is answered "kept" and the step
// marked uncertain unless a creation follows (select and create, §4.4).
uint32_t HookSelect(void* user, uint64_t r, const uint32_t* ids, size_t count, int32_t x, int32_t z) {
    if (g_selectorReplay) {
        const auto purpose = fc4_prop_developer_shadow_selector_creation(ReplayCore())
            ? PropSelectorTrace::Purpose::Creation : PropSelectorTrace::Purpose::Kept;
        return g_selectorReplay->Select(static_cast<uintptr_t>(r), purpose, ids, count, x, z);
    }
    if (g_pendingUncertain) g_uncertain = true;
    g_pendingUncertain = false;
    if (ids == nullptr || count == 0) return 0;
    g_pendingUncertain = true;
    if (count == 1) {
        // `user` is the shadow context (hookCtx): the host's own getter (slot 4).
        const fc4_prop_host& h = RunHost();
        const uint32_t sat = h.requester_satisfaction != nullptr ? h.requester_satisfaction(user, r) : 0;
        g_pendingUncertain = sat != kCrimeSatisfaction || CrimeIncidentOf(ids[0]) != 0;
    }
    return ids[0];
}

// ---- shadow cores ------------------------------------------------------------------------------------

struct CoreBox {
    std::unique_ptr<PropDevContext> ctx;
    fc4_prop_host host{};
    fc4_prop_developer* core = nullptr;
    const void* orig = nullptr;

    CoreBox() = default;
    CoreBox(const CoreBox&) = delete;
    CoreBox& operator=(const CoreBox&) = delete;
    ~CoreBox() { Drop(); }

    void Drop() {
        if (core != nullptr) {
            fc4_prop_developer_shutdown(core);  // its lists and requesters
            ReleasePropContext(ctx.get());
            fc4_prop_developer_destroy(core);
        }
        core = nullptr;
        orig = nullptr;
        ctx.reset();
    }
    // Forgets the core after a fault (its references leak).
    void Abandon() {
        core = nullptr;
        orig = nullptr;
        (void)ctx.release();
    }
    void Swap(CoreBox& o) {
        std::swap(ctx, o.ctx);
        std::swap(host, o.host);
        std::swap(core, o.core);
        std::swap(orig, o.orig);
    }
};

CoreBox g_run;  // the running shadow core
fc4_prop_developer* ReplayCore() { return g_run.core; }

const fc4_prop_host& RunHost() { return g_run.host; }

// A core from the original's record (Read, §7); not initialised.
bool ReadInto(CoreBox& b, const WriteCapture& cap, const void* orig) {
    b.Drop();
    b.ctx.reset(new PropDevContext());
    b.ctx->readOnly = true;
    b.ctx->onRemove = &HookRemove;
    b.ctx->onCreate = &HookCreate;
    b.ctx->onSelect = &HookSelect;
    b.ctx->hookCtx = b.ctx.get();
    b.host = MakePropHost(b.ctx.get());
    PropBlockSnapshot::Bind(b.host);
    b.core = fc4_prop_developer_create(&b.host, kShadowSeed, TokenOf(orig));
    if (b.core == nullptr) {
        b.ctx.reset();
        return false;
    }
    CaptureIStream in(cap);
    propcity::RecordingIStream rec(in.Get(), true);
    const bool ok = fc4_prop_developer_read(b.core, rec.Get());
    AfterPropRead(b.ctx.get(), b.core, ok, rec.Refs());
    b.ctx->pool.Drain();
    if (!ok) {
        b.Drop();
        return false;
    }
    b.orig = orig;
    return true;
}

void StartRun() {
    fc4_prop_developer_init(g_run.core);  // the managers; nothing else in the read-only host
    g_run.ctx->pool.Drain();
}

fc4_prop_state StateOf(fc4_prop_developer* core) {
    fc4_prop_state st{};
    st.struct_size = sizeof(st);
    if (core != nullptr) fc4_prop_developer_get_state(core, &st);
    return st;
}

std::vector<fc4_prop_ref> ListOf(fc4_prop_developer* core, uint32_t which) {
    std::vector<fc4_prop_ref> v(fc4_prop_developer_list(core, which, nullptr, 0));
    if (!v.empty()) fc4_prop_developer_list(core, which, v.data(), v.size());
    return v;
}

std::vector<uint64_t> TokensOf(const std::vector<fc4_prop_ref>& v) {
    std::vector<uint64_t> t;
    t.reserve(v.size());
    for (const fc4_prop_ref& r : v) t.push_back(r.token);
    return t;
}

std::vector<std::pair<uint32_t, std::vector<uint64_t>>> TriggersOf(fc4_prop_developer* core) {
    std::vector<std::pair<uint32_t, std::vector<uint64_t>>> out;
    for (uint32_t i = 0;; ++i) {
        uint32_t trig = 0;
        const size_t n = fc4_prop_developer_trigger_entry(core, i, &trig, nullptr, 0);
        if (n == SIZE_MAX) break;
        std::vector<fc4_prop_ref> refs(n);
        if (n != 0) fc4_prop_developer_trigger_entry(core, i, &trig, refs.data(), n);
        std::vector<uint64_t> t = TokensOf(refs);
        std::sort(t.begin(), t.end());
        out.emplace_back(trig, std::move(t));
    }
    std::sort(out.begin(), out.end());
    return out;
}

// The running core against a core read from the original's record now.
void CompareCores(fc4_prop_developer* ours, fc4_prop_developer* rec, Diff& d) {
    const fc4_prop_state a = StateOf(rec), b = StateOf(ours);
    if (a.scan_flag != b.scan_flag) d.Add("scan flag %d/%d (orig/ours)", a.scan_flag, b.scan_flag);
    if (a.scanner_state != b.scanner_state || a.scanner_x != b.scanner_x || a.scanner_z != b.scanner_z ||
        a.scanner_step != b.scanner_step || a.clip_x0 != b.clip_x0 || a.clip_z0 != b.clip_z0 ||
        a.clip_x1 != b.clip_x1 || a.clip_z1 != b.clip_z1) {
        d.Add("scanner (%u %d,%d)/(%u %d,%d)", a.scanner_state, a.scanner_x, a.scanner_z, b.scanner_state,
              b.scanner_x, b.scanner_z);
    }
    if (a.wealth_added != b.wealth_added) d.Add("wealthAdded %u/%u", a.wealth_added, b.wealth_added);
    if (a.night_cursor != b.night_cursor || a.night_mode != b.night_mode || a.night_acc != b.night_acc ||
        (a.night_mode != 0 && a.night_batch != b.night_batch)) {
        d.Add("night cursor/mode/batch/acc %u/%u/%u/%u vs %u/%u/%u/%u", a.night_cursor, a.night_mode, a.night_batch,
              a.night_acc, b.night_cursor, b.night_mode, b.night_batch, b.night_acc);
    }
    if (a.is_night != b.is_night) d.Add("isNight %u/%u", a.is_night, b.is_night);
    if (a.signs_visible != b.signs_visible) d.Add("signsVisible %u/%u", a.signs_visible, b.signs_visible);
    if (TokensOf(ListOf(rec, FC4_PROP_LIST_REQUESTERS)) != TokensOf(ListOf(ours, FC4_PROP_LIST_REQUESTERS))) {
        d.Add("requesters %u/%u (objects in order)", a.requester_count, b.requester_count);
    }
    {
        std::vector<uint64_t> x = TokensOf(ListOf(rec, FC4_PROP_LIST_NIGHT)), y = TokensOf(ListOf(ours, FC4_PROP_LIST_NIGHT));
        if (x != y) d.Add("night list %u/%u (in order)", U(x.size()), U(y.size()));
    }
    const struct {
        uint32_t which;
        const char* name;
    } ordered[] = {{FC4_PROP_LIST_TIME_OF_DAY, "time-of-day"},
                   {FC4_PROP_LIST_DATED, "dated"},
                   {FC4_PROP_LIST_POWER, "power"},
                   {FC4_PROP_LIST_SIGNS, "signs"}};
    for (const auto& l : ordered) {
        const std::vector<uint64_t> x = TokensOf(ListOf(rec, l.which)), y = TokensOf(ListOf(ours, l.which));
        if (x != y) d.Add("%s list %u/%u (in order)", l.name, U(x.size()), U(y.size()));
    }
    if (TriggersOf(rec) != TriggersOf(ours)) d.Add("trigger map %u/%u entries", a.trigger_count, b.trigger_count);
}

// ---- the original's scanner (raw, §2) ---------------------------------------------------------------

template <class T>
T At(const void* obj, size_t off) {
    T v;
    std::memcpy(&v, static_cast<const uint8_t*>(obj) + off, sizeof(T));
    return v;
}

struct Raw {
    const void* obj = nullptr;
    bool ok = false;
    int8_t flag = 0;
    uint8_t state = 0;
    int32_t v[7] = {};  // x, z, step, x0, z0, x1, z1
};

void ReadRawFn(void* ctx) {
    Raw* r = static_cast<Raw*>(ctx);
    r->ok = g_vtable != nullptr && At<const void*>(r->obj, 0) == g_vtable;
    if (!r->ok) return;
    r->flag = At<int8_t>(r->obj, kOffScanFlag);
    r->state = At<uint8_t>(r->obj, kOffScanner);
    for (int i = 0; i < 7; ++i) r->v[i] = At<int32_t>(r->obj, kOffScanner + 4 + 4 * static_cast<size_t>(i));
}

bool ReadRaw(const void* orig, Raw* out) {
    *out = Raw{};
    out->obj = orig;
    return g_vtable != nullptr && SehGuard("propdev shadow (reading the original's scanner)", &ReadRawFn, out) &&
           out->ok;
}

bool ScannerEqual(const Raw& r, const fc4_prop_state& s) {
    return r.flag == s.scan_flag && r.state == s.scanner_state && r.v[0] == s.scanner_x && r.v[1] == s.scanner_z &&
           r.v[2] == s.scanner_step && r.v[3] == s.clip_x0 && r.v[4] == s.clip_z0 && r.v[5] == s.clip_x1 &&
           r.v[6] == s.clip_z1;
}

// The vtable is the original's when it lies in the identified executable's
// image and the object's class is 0x89C48F47.
void Track(cISC4PropDeveloper* orig) {
    g_orig = orig;
    g_agent = nullptr;
    g_vtable = nullptr;
    void* agent = nullptr;
    if (orig->QueryInterface(kIID_cIGZMessageTarget2, &agent) && agent != nullptr) {
        cIGZUnknown* a = static_cast<cIGZUnknown*>(agent);
        FC4_ACQUIRED(a, "QueryInterface");
        g_agent = static_cast<cIGZMessageTarget2*>(agent);
        FC4_RELEASE(a);  // only its identity is kept
    }
    uint32_t cls = 0;
    void* s = nullptr;
    if (orig->QueryInterface(kIID_cIGZSerializable, &s) && s != nullptr) {
        cIGZUnknown* su = static_cast<cIGZUnknown*>(s);
        FC4_ACQUIRED(su, "QueryInterface");
        cls = static_cast<cIGZSerializable*>(s)->GetGZCLSID();
        FC4_RELEASE(su);
    }
    const compat::ExecutableVersion* v = compat::IdentifiedVersion();
    const uintptr_t vt = reinterpret_cast<uintptr_t>(*static_cast<void* const*>(static_cast<const void*>(orig)));
    if (v != nullptr && cls == kFC4_CLSID_cSC4PropDeveloper && vt >= v->imageBase &&
        vt < static_cast<uintptr_t>(v->imageBase) + v->sizeOfImage) {
        g_vtable = reinterpret_cast<const void*>(vt);
    }
    Log("propdev shadow: following the original prop developer %p (agent %p, class 0x%08X, raw scanner %s)",
        static_cast<const void*>(orig), static_cast<void*>(g_agent), cls,
        g_vtable ? "readable" : "NOT readable (executable not identified): not compared");
}

// ---- the month's account ----------------------------------------------------------------------------

struct MonthStats {
    const char* followed = nullptr;
    uint32_t steps = 0, blocks = 0, certain = 0, rawCompared = 0;
    uint32_t origRemoved = 0, oursRemoved = 0, notKept = 0, origCreated = 0, oursCreated = 0;
    uint32_t messages = 0, ticks = 0;
    uint32_t selectorEvents = 0, selectorZero = 0;
    uint32_t selectorMatched = 0, replayComplete = 0, replayInvalid = 0;
    uint32_t operationsExact = 0, operationsIncomplete = 0;
    Diff d;
};
MonthStats g_month;
uint32_t g_months = 0, g_cleanMonths = 0;
WriteCapture g_capture;

void Follow(const char* how) {
    // "first" or "resynced" wins over "yes" for the month line.
    if (g_month.followed == nullptr || std::strcmp(g_month.followed, "yes") == 0) g_month.followed = how;
}

bool StartFromOriginal(const void* orig, const char* how) {
    g_run.Drop();
    if (!g_capture.Capture(static_cast<cIGZUnknown*>(const_cast<void*>(orig)))) return false;
    if (!ReadInto(g_run, g_capture, orig)) return false;
    StartRun();
    Follow(how);
    return true;
}

// PV14: read only at the original operation's quiescent boundary. Rebuild
// the next-word index rather than retaining the original generator's pointer.
bool ObserveRandom(const void* original) {
    const auto* version = compat::IdentifiedVersion();
    if (!version || !g_vtable || !g_run.core) return false;
    const uintptr_t expected = version->imageBase + (0x00AB29F8u - 0x00400000u);
    if (reinterpret_cast<uintptr_t>(g_vtable) != expected) return false;
    for (unsigned wealth = 0; wealth != 2; ++wealth) {
        const auto* base = wealth
            ? reinterpret_cast<const uint8_t*>(version->imageBase + (0x00B4A5C0u - 0x00400000u))
            : static_cast<const uint8_t*>(original) + 0x38;
        uint32_t words[624];
        std::memcpy(words, base, sizeof(words));
        const uintptr_t pointer = At<uint32_t>(base, 0x9C0);
        const int32_t remaining = At<int32_t>(base, 0x9C4);
        const uintptr_t start = reinterpret_cast<uintptr_t>(base);
        if (pointer < start || pointer > start + 0x9C0 || (pointer - start) % 4 ||
            remaining < 0 || remaining > 624) return false;
        const uint32_t index = static_cast<uint32_t>((pointer - start) / 4);
        if (!fc4_prop_developer_shadow_random(g_run.core, words, index,
                                              static_cast<uint32_t>(remaining), wealth != 0)) return false;
    }
    return true;
}

// ---- the block step ---------------------------------------------------------------------------------

// The props of block (bx, bz) as prop tokens (sorted), each held in `pool`:
// the lots at their origin cells (GetLotProps), else the cell's street props.
std::vector<uint64_t> BlockProps(int32_t bx, int32_t bz, propcity::CallPool& pool) {
    std::vector<uint64_t> out;
    cISC4City* c = propcity::AppCity();
    cISC4LotManager* lm = c ? c->GetLotManager() : nullptr;
    cISC4PropManager* pm = c ? c->GetPropManager() : nullptr;
    if (lm == nullptr || pm == nullptr) return out;
    for (int32_t x = kBlock * bx; x < kBlock * bx + kBlock; ++x) {
        for (int32_t z = kBlock * bz; z < kBlock * bz + kBlock; ++z) {
            SC4List<cISC4Occupant*> list;
            cISC4Lot* lot = lm->GetLot(x, z, true);  // borrowed
            if (lot != nullptr) {
                pm->GetLotProps(list, lot);  // slot 24
            } else {
                pm->GetStreetProps(list, x, z);  // slot 31
            }
            for (cISC4Occupant*& o : list) {
                if (o == nullptr) continue;
                FC4_ACQUIRED(o, "propdev shadow block list entry");
                pool.Adopt(o);
                const uint64_t t = PropToken(pool, o);
                if (t != 0) out.push_back(t);
                o = nullptr;
            }
        }
    }
    std::sort(out.begin(), out.end());
    out.erase(std::unique(out.begin(), out.end()), out.end());
    return out;
}

struct Step {
    const void* orig = nullptr;
    cIGZMessage2* msg = nullptr;
    uint32_t type = 0;
    bool ran = false;
    const char* incomplete = nullptr;
    bool block = false;
    fc4_prop_block blk{};
    std::vector<uint64_t> before;
    propcity::CallPool pool;
    fc4_prop_message message{};
    PropBlockSnapshot inputs;
    const PropSelectorTrace* trace = nullptr;
    bool nativeFinished = false;
};

void StepBefore(void* ctx) {
    Step& s = *static_cast<Step*>(ctx);
    if (g_run.core == nullptr || g_run.orig != s.orig) {
        if (!StartFromOriginal(s.orig, "first")) {
            s.incomplete = "the original's record could not be captured or read";
            return;
        }
    }
    const fc4_prop_state st0 = StateOf(g_run.core);
    if (!ObserveRandom(s.orig)) {
        g_month.d.Add("random input unavailable (PV14)");
        s.incomplete = "random input unavailable (PV14)";
        return;
    }
    if (!PropMessageOf(s.msg, g_run.ctx->pool, &s.message)) {
        s.incomplete = "the message could not be read";
        return;
    }
    g_removed.clear();
    g_created = 0;
    g_uncertain = false;
    g_pendingUncertain = false;
    s.ran = true;
    s.block = fc4_prop_developer_shadow_prepare(g_run.core, &s.blk.bx, &s.blk.bz);
    if (s.block) {
        s.inputs.Capture(g_run.ctx.get(), s.blk.bx, s.blk.bz, ListOf(g_run.core, FC4_PROP_LIST_REQUESTERS));
        s.before = BlockProps(s.blk.bx, s.blk.bz, s.pool);
    }
    if (s.type == 0x66956814 && st0.dated_count) g_uncertain = true;
}

void StepAfter(void* ctx) {
    Step& s = *static_cast<Step*>(ctx);
    if (!s.ran) return;
    const uint32_t mismatchesBefore = g_month.d.mismatches;
    PropSelectorReplay replay(s.trace->Events());
    g_selectorReplay = &replay;
    if (s.block) PropBlockSnapshot::active = &s.inputs;
    g_inStep = true;
    const bool oursFinished = fc4_prop_developer_do_message(g_run.core, &s.message);
    g_inStep = false;
    PropBlockSnapshot::active = nullptr;
    g_selectorReplay = nullptr;
    const bool complete = s.trace->Valid() && replay.Complete();
    g_month.selectorMatched += U(replay.Matched());
    if (complete) ++g_month.replayComplete; else {
        ++g_month.replayInvalid;
        g_month.d.Add("selector replay invalid: consumed=%u matched=%u native=%u", U(replay.Consumed()),
                      U(replay.Matched()), U(s.trace->Events().size()));
        // Bounded independent diagnostics: monthly mismatch lists can be full
        // before the first selector divergence is reached.
        static uint32_t diagnosed = 0;
        if (diagnosed++ < 24) {
            const auto* e = replay.Expected();
            const auto& a = replay.Actual();
            Log("propdev replay invalid: reason=%s type=%08x block=%d,%d consumed=%u matched=%u native=%u frozen=%u",
                replay.Failure(), s.type, s.blk.bx, s.blk.bz, U(replay.Consumed()), U(replay.Matched()),
                U(s.trace->Events().size()), s.inputs.valid ? 1u : 0u);
            if (e) Log("propdev replay args: native/ours requester=%p/%p purpose=%u/%u xy=%d,%d/%d,%d count=%u/%u first=%08x/%08x result=%08x depth=%u",
                reinterpret_cast<void*>(e->requester), reinterpret_cast<void*>(a.requester),
                static_cast<unsigned>(e->purpose), static_cast<unsigned>(a.purpose), e->x, e->z, a.x, a.z,
                U(e->candidates.size()), U(a.candidates.size()), e->candidates.empty() ? 0 : e->candidates.front(),
                a.candidates.empty() ? 0 : a.candidates.front(), e->result, e->depth);
        }
    }
    if (!s.inputs.valid) g_month.d.Add("frozen block input missing");
    if (oursFinished != s.nativeFinished) g_month.d.Add("delivery return differs");
    if (g_nestedCursor != g_nested.size()) g_uncertain = true;
    while (g_nestedCursor < g_nested.size()) {
        const auto m = g_nested[g_nestedCursor++];
        fc4_prop_developer_do_message(g_run.core, &m);
    }
    struct Result {
        Step& step;
        uint32_t before;
        bool eligible;
        ~Result() {
            if (eligible && g_month.d.mismatches == before) {
                WriteCapture cap;
                CoreBox native;
                if (!cap.Capture(static_cast<cIGZUnknown*>(const_cast<void*>(step.orig))) ||
                    !ReadInto(native, cap, step.orig)) g_month.d.Add("operation record unavailable");
                else CompareCores(g_run.core, native.core, g_month.d);
            }
            if (eligible && g_month.d.mismatches == before) ++g_month.operationsExact;
            else ++g_month.operationsIncomplete;
        }
    } result{s, mismatchesBefore, !g_uncertain && complete && s.inputs.valid};
    g_run.ctx->pool.Drain();
    ++g_month.steps;
    const fc4_prop_state st = StateOf(g_run.core);
    Raw r;
    if (ReadRaw(s.orig, &r)) {
        ++g_month.rawCompared;
        if (!ScannerEqual(r, st)) {
            g_month.d.Add("step %u: scanner orig flag %d (%u %d,%d) ours flag %d (%u %d,%d)", g_month.steps, r.flag,
                          r.state, r.v[0], r.v[1], st.scan_flag, st.scanner_state, st.scanner_x, st.scanner_z);
        }
    } else g_month.d.Add("operation scanner unavailable");
    if (!s.block) return;
    s.blk.struct_size = sizeof(s.blk);
    fc4_prop_developer_last_block(g_run.core, &s.blk, nullptr, nullptr, 0);
    ++g_month.blocks;
    const std::vector<uint64_t> after = BlockProps(s.blk.bx, s.blk.bz, s.pool);
    std::vector<uint64_t> removed, created;
    std::set_difference(s.before.begin(), s.before.end(), after.begin(), after.end(), std::back_inserter(removed));
    std::set_difference(after.begin(), after.end(), s.before.begin(), s.before.end(), std::back_inserter(created));
    g_month.origRemoved += U(removed.size());
    g_month.oursRemoved += U(g_removed.size());
    g_month.origCreated += U(created.size());
    g_month.oursCreated += g_created;
    const size_t step5 = std::min<size_t>(s.blk.removed, g_removed.size());
    const size_t notKept = g_removed.size() - step5;
    g_month.notKept += U(notKept);
    if (g_uncertain) {
        g_month.d.Add("operation incomplete (creation attempt, nested event or dated effect)");
        return;
    }
    ++g_month.certain;
    auto oursRemoved = g_removed;
    std::sort(oursRemoved.begin(), oursRemoved.end());
    if (oursRemoved != removed) g_month.d.Add("full removal identities differ");
    if (created.size() != g_created) g_month.d.Add("creation outputs orig %u ours %u", U(created.size()), g_created);
    if (removed.size() != g_removed.size()) {
        g_month.d.Add("block (%d,%d) removed orig %u ours %u", s.blk.bx, s.blk.bz, U(removed.size()),
                      U(g_removed.size()));
    }
    for (size_t i = 0; i < g_removed.size(); ++i) {
        if (!std::binary_search(removed.begin(), removed.end(), g_removed[i])) {
            g_month.d.Add("block (%d,%d) prop %p not kept by ours, still there in the original", s.blk.bx, s.blk.bz,
                          static_cast<void*>(FromToken(g_removed[i])));
        }
    }
}

// ---- the followed messages (§3.3) -------------------------------------------------------------------

struct Msg {
    cIGZMessage2* msg;
};

void CallMessage(void* ctx) {
    cIGZMessage2* msg = static_cast<Msg*>(ctx)->msg;
    fc4_prop_message m{};
    if (PropMessageOf(msg, g_run.ctx->pool, &m)) {
        const auto before = StateOf(g_run.core);
        // A broadcast listener does not observe the original's scoped CRT
        // draws. Preserve the independent continuation, but never count an
        // unobserved shuffle as exact (PV14 sections 5.2/5.3).
        if ((m.msg_type == 0xC9DA96E9 || m.msg_type == 0xC9DA96E8) && before.night_count > 1)
            g_month.d.Add("night shuffle CRT input unavailable (PV14)");
        fc4_prop_developer_do_message(g_run.core, &m);
        if (m.msg_type == 0x26D31EC1 && !g_run.ctx->lightingAvailable)
            g_month.d.Add("PostCityInit lighting input unavailable (PV15)");
    }
    g_run.ctx->pool.Drain();
}

void OnMessage(cIGZMessage2* msg) {
    if (msg == nullptr || g_run.core == nullptr) return;
    if (msg->GetType() == kMsgAnimationTick) {
        ++g_month.ticks;
    } else {
        ++g_month.messages;
    }
    if (g_nativeOperation) {
        fc4_prop_message m{};
        if (PropMessageOf(msg, g_run.ctx->pool, &m)) g_nested.push_back(m);
        return;
    }
    Msg m{msg};
    if (!SehGuard("propdev shadow (followed message)", &CallMessage, &m)) {
        Log("propdev shadow: a fault in a followed message; the shadow core is abandoned (its references leak)");
        g_run.Abandon();
    }
}

class EventTarget final : public cIGZMessageTarget2 {
public:
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid != kIID_cIGZMessageTarget2 && riid != GZIID_cIGZUnknown) return false;
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { return refs_ > 0 ? --refs_ : 0u; }
    bool DoMessage(cIGZMessage2* msg) override {
        if (msg == nullptr) return true;
        FC4_X87_SCOPE("propdev shadow DoMessage", msg->GetType());  // x87 leftover guard (FC4Fpu.h)
        OnMessage(msg);
        return true;
    }

private:
    uint32_t refs_ = 0;
};

EventTarget& Events() {
    static EventTarget t;
    return t;
}

cFC4ISC4AnimationTickManager* TickManager() {
    cIGZFrameWork* fw = RZGetFrameWork();
    void* p = nullptr;
    if (fw == nullptr || !fw->GetSystemService(kFC4_SID_AnimationTickManager, kFC4_IID_AnimationTickManager, &p) ||
        p == nullptr) {
        return nullptr;
    }
    auto* m = static_cast<cFC4ISC4AnimationTickManager*>(p);
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(m), "GetSystemService");
    return m;
}

// The notifications: once, left registered for the process. The animation
// tick: while a city is followed.
void Listen() {
    if (!g_listening) {
        g_listening = true;
        cIGZMessageServer2Ptr ms;
        if (!ms) {
            Log("propdev shadow: no message server; the messages of §3.3 do not reach ours");
        } else {
            Events().AddRef();  // our own reference (static object)
            bool ok = true;
            for (uint32_t id : kPropDevSubscriptions) {
                ok = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&Events()), id) && ok;
            }
            if (!ok) Log("propdev shadow: a notification was refused; ours can miss messages");
        }
    }
    if (!g_tickRegistered) {
        cFC4ISC4AnimationTickManager* t = TickManager();
        if (t == nullptr) {
            Log("propdev shadow: no animation-tick manager; ours runs no night batches");
            return;
        }
        // [abi-ret] lot-developer.md 6.7.8 gives AddTickableObject no result (the target always goes in a
        // bucket) and the table row is eax-or-void: the value is not read, as FC4LotDevHost does.
        t->AddTickableObject(static_cast<cIGZMessageTarget2*>(&Events()), kTickPriority);  // slot 5
        g_tickRegistered = true;
        FC4_RELEASE(static_cast<cIGZUnknown*>(t));
    }
}

void Unlisten() {
    if (!g_tickRegistered) return;
    g_tickRegistered = false;
    cFC4ISC4AnimationTickManager* t = TickManager();
    if (t == nullptr) return;
    t->RemoveTickableObject(static_cast<cIGZMessageTarget2*>(&Events()), kTickPriority);
    FC4_RELEASE(static_cast<cIGZUnknown*>(t));
}

// ---- the month ---------------------------------------------------------------------------------------

struct Month {
    const void* orig = nullptr;
    bool compared = false;
    bool resynced = false;
    const char* incomplete = nullptr;
    fc4_prop_state st{};
};

void MonthCompare(void* ctx) {
    Month& mo = *static_cast<Month*>(ctx);
    if (g_run.core == nullptr || g_run.orig != mo.orig) {
        mo.incomplete = "no shadow core yet";
        return;
    }
    if (!g_capture.Capture(static_cast<cIGZUnknown*>(const_cast<void*>(mo.orig)))) {
        mo.incomplete = "the original's record could not be captured";
        return;
    }
    CoreBox rec;
    if (!ReadInto(rec, g_capture, mo.orig)) {
        mo.incomplete = "the original's record could not be read";
        return;
    }
    CompareCores(g_run.core, rec.core, g_month.d);
    mo.compared = true;
    // Comparison never replaces the independent running state.
    mo.st = StateOf(g_run.core);
}

void ReadSettings() { g_enabled = SettingInt("propdev_shadow", "FC4_PROPDEV_SHADOW", 0) != 0; }

bool Active() { return g_enabled && g_simulatorReplaced && !g_replaced; }

cISC4PropDeveloper* OriginalOf(cISC4City* city) {
    cISC4PropDeveloper* p = city ? city->GetPropDeveloper() : nullptr;  // slot 41, borrowed
    if (p == nullptr) return nullptr;
    if (IsOurObject(p)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("propdev shadow: inactive: the city's prop developer is FreeCity4's own (do not enable propdev)");
        }
        return nullptr;
    }
    return p;
}

}  // namespace

void PropDevShadowStartup(bool simulatorReplaced, bool propdevReplaced) {
    g_simulatorReplaced = simulatorReplaced;
    g_replaced = propdevReplaced;
    ReadSettings();
    if (!g_enabled) return;
    const char* src = "";
    Setting("propdev_shadow", "FC4_PROPDEV_SHADOW", &src);
    if (propdevReplaced) {
        Log("propdev shadow: requested (propdev_shadow from %s) but INACTIVE: it compares against the ORIGINAL prop "
            "developer, and propdev is enabled (ours)",
            src);
    } else if (!simulatorReplaced) {
        Log("propdev shadow: requested (propdev_shadow from %s) but INACTIVE: it follows the original's agent "
            "deliveries, which pass through FreeCity4's simulator, and simulator is not enabled",
            src);
    } else {
        Log("propdev shadow: ON (propdev_shadow from %s): a shadow core (read-only host) starts from the ORIGINAL "
            "prop developer's record at its first block step and follows the messages of section 3.3 and the "
            "animation tick; block inputs freeze before the native operation, genuine selectors replay after it. "
            "Compare complete selector arguments/order/count, scanner, all removal identities and saved-state "
            "projections per eligible operation. Creation attempts/failures and dated effects remain incomplete; "
            "operation-output-matches does not mean full-operation exactness. No native state resynchronization",
            src);
    }
}

bool PropDevShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                          int64_t, bool* finished) {
    if ((type != kMsgIdle && type != kMsgDay) || target == nullptr || msg == nullptr || finished == nullptr ||
        !Active()) {
        return false;
    }
    if (target != g_agent && (agentName == nullptr || std::strcmp(agentName, kAgentName) != 0)) return false;
    cISC4PropDeveloper* orig = OriginalOf(propcity::AppCity());
    if (orig == nullptr) return false;
    if (orig != g_orig) {
        g_run.Drop();
        Track(orig);
    }
    if (target != g_agent) return false;
    Listen();
    Step s;
    s.orig = orig;
    s.msg = msg;
    s.type = type;
    if (!SehGuard("propdev shadow (before the step)", &StepBefore, &s)) {
        Log("propdev shadow: a fault before the original's step; the shadow core is abandoned (its references leak)");
        g_run.Abandon();
        s.ran = false;
    }
    std::vector<void*> requesters;
    if (g_run.core && g_vtable) {
        for (const auto& ref : ListOf(g_run.core, FC4_PROP_LIST_REQUESTERS))
            if (ref.token) requesters.push_back(FromToken(ref.token));
    }
    PropSelectorTrace trace;
    const auto* version = compat::IdentifiedVersion();
    const bool recording = version && trace.Begin(requesters, version->imageBase);
    g_nested.clear();
    g_nestedCursor = 0;
    g_nativeOperation = true;
    *finished = target->DoMessage(msg);  // the original's step, exactly once
    g_nativeOperation = false;
    trace.Finish();
    s.trace = &trace;
    s.nativeFinished = *finished;
    if (!recording || !trace.Valid()) g_month.d.Add("selector transcript capture unavailable");
    g_month.selectorEvents += U(trace.Events().size());
    for (const auto& event : trace.Events()) if (event.result == 0) ++g_month.selectorZero;
    if (!SehGuard("propdev shadow (after the step)", &StepAfter, &s)) {
        Log("propdev shadow: a fault after the original's step; the shadow core is abandoned (its references leak)");
        g_run.Abandon();
    }
    s.pool.Drain();
    PropBlockSnapshot::active = nullptr;
    g_selectorReplay = nullptr;
    static bool loggedIncomplete = false;
    if (s.incomplete != nullptr && !loggedIncomplete) {
        loggedIncomplete = true;
        Log("propdev shadow: step not followed: %s", s.incomplete);
    }
    return true;
}

void PropDevShadowMonth(cISC4City* city, int32_t julian) {
    if (!Active() || g_orig == nullptr) return;
    cISC4PropDeveloper* orig = OriginalOf(city);
    if (orig == nullptr || orig != g_orig) return;
    Month mo;
    mo.orig = orig;
    if (!SehGuard("propdev shadow (the month's record)", &MonthCompare, &mo)) {
        Log("propdev shadow: a fault comparing the record; the shadow core is abandoned (its references leak)");
        g_run.Abandon();
        mo.compared = false;
        mo.incomplete = "a fault";
    }
    char date[16];
    propcity::DateText(julian, date);
    if (!mo.compared) {
        Log("propdev shadow month %s: INCOMPLETE (%s); not compared", date, mo.incomplete ? mo.incomplete : "?");
        g_month = MonthStats{};
        return;
    }
    ++g_months;
    if (g_month.d.mismatches == 0) ++g_cleanMonths;
    char head[512];
    snprintf(head, sizeof(head),
             "propdev shadow month %s: followed=%s steps=%u blocks=%u certain=%u raw-compared=%u removed orig/ours "
             "%u/%u (not kept %u) created orig/ours %u/%u (not compared) messages=%u ticks=%u message-removals=%u; "
             "record: scan=%d scanner=(%u %d,%d) requesters=%u night=%u tod=%u dated=%u power=%u signs=%u "
             "triggers=%u isNight=%u%s",
             date, g_month.followed ? g_month.followed : "yes", g_month.steps, g_month.blocks, g_month.certain,
             g_month.rawCompared, g_month.origRemoved, g_month.oursRemoved, g_month.notKept, g_month.origCreated,
             g_month.oursCreated, g_month.messages, g_month.ticks, g_messageRemovals, mo.st.scan_flag,
             mo.st.scanner_state, mo.st.scanner_x, mo.st.scanner_z, mo.st.requester_count, mo.st.night_count,
             mo.st.time_of_day_count, mo.st.dated_count, mo.st.power_count, mo.st.sign_count, mo.st.trigger_count,
             mo.st.is_night, mo.resynced ? " (resynced from the record)" : "");
    const std::string line = std::string(head) + "; mismatches=" + std::to_string(g_month.d.mismatches) +
                             "; selector-events=" + std::to_string(g_month.selectorEvents) +
                             " selector-zero=" + std::to_string(g_month.selectorZero) +
                             " selector-matched=" + std::to_string(g_month.selectorMatched) +
                             " replay-complete=" + std::to_string(g_month.replayComplete) +
                             " replay-invalid=" + std::to_string(g_month.replayInvalid) +
                             " operation-output-matches=" + std::to_string(g_month.operationsExact) +
                             " full-operations-exact=0" +
                             " operations-incomplete=" + std::to_string(g_month.operationsIncomplete) +
                             (g_month.d.listed.empty() ? std::string() : "; differences: " + g_month.d.listed);
    LogShadowLine("propdev shadow (cont.)", reinterpret_cast<const uint8_t*>(line.data()), line.size());
    g_month = MonthStats{};
    g_messageRemovals = 0;
}

void PropDevShadowCityShutdown() {
    if (g_run.core == nullptr && g_orig == nullptr) return;
    if (g_months > 0) {
        Log("propdev shadow: city shutdown: %u month lines, %u with mismatches=0", g_months, g_cleanMonths);
    }
    g_run.Drop();
    Unlisten();
    g_orig = nullptr;
    g_agent = nullptr;
    g_vtable = nullptr;
    g_month = MonthStats{};
}

}  // namespace fc4
