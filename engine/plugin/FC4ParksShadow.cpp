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

#include "FC4ParksShadow.h"

#include "FC4Config.h"
#include "FC4CoreProps.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4WriteCapture.h"
#include "cFC4Parks.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4Lot.h"
#include "cISC4ParkManager.h"
#include "cRZCOMDllDirector.h"

#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgOccupantInserted = 0x99EF1142;
constexpr uint32_t kMsgOccupantRemoved = 0x99EF1143;
constexpr uint32_t kMsgZooEscape = 0xAA667CE2;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr const char* kAgentName = "cSC4ParkManager";  // §3
constexpr size_t kMaxListed = 12;
// §2 offsets from the primary interface pointer (build 1.1.641).
constexpr size_t kOffLatch = 0x15;
constexpr size_t kOffThreshold = 0x34;
// [choice] The shadow core's member generator (its escapes are never compared).
constexpr uint32_t kShadowSeed = 0x5EED0A4Cu;

bool g_enabled = false;
bool g_simulatorReplaced = false;
bool g_parksReplaced = false;
bool g_loggedOurs = false;
bool g_listening = false;

const void* g_sim = nullptr;
cIGZMessageTarget2* g_agent = nullptr;
const void* g_vtable = nullptr;

// The shadow core.
ParkContext g_ctx;
fc4_park_host g_host{};
fc4_park* g_core = nullptr;
const void* g_coreSim = nullptr;
uint32_t g_months = 0, g_cleanMonths = 0, g_followedMessages = 0, g_origEscapes = 0;
// [question PKG-1] The original's threshold after its Init is the code
// default 0.25 in game, not the shipped exemplar's 25.0 (§4.4, §6): ours then
// starts as without the exemplar (threshold 0.25, empty curve) so that the
// pass itself can be compared; every line says so.
bool g_withoutTuning = false;

// Recorded by the shadow host during our month.
struct LotWrite {
    uint64_t lot;
    uint32_t condition;
};
std::vector<LotWrite> g_lotWrites;
uint32_t g_ourEscapes = 0;

void RecordLotCondition(void*, uint64_t lot, uint32_t condition) { g_lotWrites.push_back(LotWrite{lot, condition}); }
void RecordEscape(void*, uint64_t, uint64_t) { ++g_ourEscapes; }
void RecordControllers(void*, const char*, uint64_t) {}

void ReadSettings() { g_enabled = SettingInt("parks_shadow", "FC4_PARKS_SHADOW", 0) != 0; }

bool Active() { return g_enabled && g_simulatorReplaced && !g_parksReplaced; }

cISC4City* CurrentCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    cISC4City* city = sc4->GetCity();
    FC4_RELEASE(sc4);
    return city;
}

cISC4ParkManager* OriginalOf(cISC4City* city) {
    if (city == nullptr) return nullptr;
    cISC4ParkManager* raw = city->GetParkManager();  // slot 74, borrowed
    if (raw == nullptr) return nullptr;
    if (IsOurObject(raw)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("parks shadow: inactive: the city's park manager is FreeCity4's own (do not enable parks)");
        }
        return nullptr;
    }
    return raw;
}

// ---- the original's latch and threshold (raw, §2) -------------------------------------------------

struct RawRead {
    const void* obj = nullptr;
    const void* vtable = nullptr;
    bool vtableOk = false;
    uint8_t latch = 0;
    float threshold = 0.0f;
};

template <class T>
T At(const void* obj, size_t off) {
    T v;
    std::memcpy(&v, static_cast<const uint8_t*>(obj) + off, sizeof(T));
    return v;
}

void ReadRaw(void* ctx) {
    RawRead* r = static_cast<RawRead*>(ctx);
    r->vtableOk = r->vtable != nullptr && At<const void*>(r->obj, 0) == r->vtable;
    if (!r->vtableOk) return;
    r->latch = At<uint8_t>(r->obj, kOffLatch);
    r->threshold = At<float>(r->obj, kOffThreshold);
}

bool ReadOriginal(const void* sim, RawRead* out) {
    *out = RawRead{};
    out->obj = sim;
    out->vtable = g_vtable;
    return g_vtable != nullptr && SehGuard("parks shadow (reading the original's latch)", &ReadRaw, out) &&
           out->vtableOk && out->latch <= 1;
}

void Track(cISC4ParkManager* sim) {
    g_sim = sim;
    void* agent = nullptr;
    g_agent = nullptr;
    if (sim->QueryInterface(kIID_cIGZMessageTarget2, &agent) && agent != nullptr) {
        cIGZUnknown* a = static_cast<cIGZUnknown*>(agent);
        FC4_ACQUIRED(a, "QueryInterface");
        g_agent = static_cast<cIGZMessageTarget2*>(agent);
        FC4_RELEASE(a);  // only its identity is kept
    }
    g_vtable = compat::ParkManagerVtable();
    RawRead r;
    const bool raw = ReadOriginal(sim, &r);
    Log("parks shadow: following the original park manager %p (agent %p, vtable %s): latch=%s threshold=%s",
        static_cast<const void*>(sim), static_cast<void*>(g_agent), g_vtable ? (raw ? "ok" : "NOT the original's") : "unknown",
        raw ? (r.latch ? "1" : "0") : "-", raw ? std::to_string(r.threshold).c_str() : "-");
}

// The canonical token of an object: its cISC4Occupant interface.
uint64_t Canonical(cIGZUnknown* u) {
    if (u == nullptr) return 0;
    void* p = nullptr;
    if (!u->QueryInterface(kIID_cISC4Occupant, &p) || p == nullptr) return 0;
    cIGZUnknown* q = static_cast<cIGZUnknown*>(p);
    if (IsOurObject(q)) {
        q->Release();
    } else {
        FC4_ACQUIRED(q, "QueryInterface");
        FC4_RELEASE(q);
    }
    return TokenOf(q);
}

// ---- the original's record (§7) -------------------------------------------------------------------

struct Record {
    bool ok = false;
    std::vector<uint64_t> parks, zoos;  // canonical occupant tokens
};

Record Parse(const WriteCapture& cap) {
    Record r;
    CaptureReader b(cap.Bytes());
    uint16_t version = 0;
    if (!b.U16(version) || version != 2) return r;
    for (std::vector<uint64_t>* list : {&r.parks, &r.zoos}) {
        uint32_t n = 0;
        if (!b.U32(n)) return r;
        if (n >= 10000000u) continue;  // civic-buildings.md §7: the shared list reader reads nothing further
        for (uint32_t i = 0; i < n; ++i) {
            uint32_t id = 0;
            if (!b.Ref(id)) return r;
            list->push_back(Canonical(static_cast<cIGZUnknown*>(cap.SerializableAt(id))));
        }
    }
    uint8_t extra = 0;
    r.ok = !b.U8(extra);  // the whole record was read
    return r;
}

// ---- the shadow core ------------------------------------------------------------------------------

void DropCore() {
    if (g_core != nullptr) {
        fc4_park_shutdown(g_core);  // its lists, department and holds
        fc4_park_destroy(g_core);
    }
    g_core = nullptr;
    g_coreSim = nullptr;
}

// A fresh shadow core from the original's capture: Read, then Init with the
// shipped tuning (the read-only host creates and subscribes nothing).
bool Restart(const WriteCapture& cap, const void* sim, bool withTuning) {
    DropCore();
    g_ctx = ParkContext{};
    g_ctx.readOnly = true;
    g_host = MakeParkHost(&g_ctx);
    g_host.set_lot_condition = &RecordLotCondition;
    g_host.post_zoo_escape = &RecordEscape;
    g_host.add_controllers = &RecordControllers;
    g_core = fc4_park_create(&g_host, kShadowSeed, TokenOf(sim));
    if (g_core == nullptr) return false;
    CaptureIStream in(cap);
    const bool ok = fc4_park_read(g_core, in.Get());
    props::CoreProps tuning(withTuning ? ParkTuning() : nullptr);
    fc4_park_init(g_core, tuning.Get());
    if (!ok) {
        DropCore();
        return false;
    }
    g_coreSim = sim;
    return true;
}

std::vector<uint64_t> OurList(uint32_t which) {
    std::vector<uint64_t> v(fc4_park_list(g_core, which, nullptr, 0));
    if (!v.empty()) fc4_park_list(g_core, which, v.data(), v.size());
    return v;
}

fc4_park_state OurState() {
    fc4_park_state st{};
    st.struct_size = sizeof(st);
    if (g_core != nullptr) fc4_park_get_state(g_core, &st);
    return st;
}

uint32_t LotConditionOf(uint64_t lot) {
    cISC4Lot* l = reinterpret_cast<cISC4Lot*>(FromToken(lot));
    return l ? AsGame(l)->GetCondition() : 0;  // slot 35
}

std::string DateText(int32_t julian) {
    if (julian <= 0) return "0000-00-00";
    fc4_date d{};
    fc4_date_from_julian(julian, &d);
    char b[24];
    snprintf(b, sizeof(b), "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
    return b;
}

unsigned U(size_t n) { return static_cast<unsigned>(n); }

struct Diff {
    uint32_t mismatches = 0;
    std::string listed;
    size_t n = 0;
    void Add(const char* fmt, ...) {
        ++mismatches;
        if (n++ >= kMaxListed) return;
        char b[160];
        va_list ap;
        va_start(ap, fmt);
        vsnprintf(b, sizeof(b), fmt, ap);
        va_end(ap);
        if (!listed.empty()) listed += ", ";
        listed += b;
    }
};

struct PredictedLot {
    uint64_t occ, lot;
    uint32_t before, predicted;
};

// One month: Before (guarded) -> the original's DoMessage (unguarded, exactly
// once) -> After (guarded).
struct Month {
    const void* sim = nullptr;
    int32_t julian = 0;
    bool compared = false;
    const char* incomplete = nullptr;
    const char* followed = "yes";
    Record rec;
    Diff d;
    RawRead before, after;
    bool rawBefore = false, rawAfter = false;
    fc4_park_state oursBefore{}, oursAfter{};
    float funding = -1.0f;
    double p = 0.0;
    std::vector<PredictedLot> lots;
    uint32_t runDown = 0, restored = 0, origEscapesBefore = 0;
};

WriteCapture g_capture;

// Ours from the original's record (first month or a resync). With the raw
// threshold readable and equal to the code default 0.25 while the exemplar
// gives another, ours restarts without the exemplar (PKG-1).
bool StartFrom(Month& mo) {
    if (!Restart(g_capture, mo.sim, true)) return false;
    g_withoutTuning = false;
    const fc4_park_state st = OurState();
    if (mo.rawBefore && mo.before.threshold != st.distress_threshold && mo.before.threshold == 0.25f) {
        if (!Restart(g_capture, mo.sim, false)) return false;
        g_withoutTuning = true;
    }
    return true;
}

void MonthBefore(void* ctx) {
    Month& mo = *static_cast<Month*>(ctx);
    mo.rawBefore = ReadOriginal(mo.sim, &mo.before);
    const bool captured = g_capture.Capture(static_cast<cIGZUnknown*>(const_cast<void*>(mo.sim)));
    if (captured) mo.rec = Parse(g_capture);
    if (!mo.rec.ok) {
        mo.incomplete = captured ? "the original's record could not be parsed" : "the original's record could not be captured";
        DropCore();
        return;
    }
    if (g_core == nullptr || g_coreSim != mo.sim) {
        mo.followed = "first";
        if (!StartFrom(mo)) {
            mo.incomplete = "the shadow core could not read the original's record";
            return;
        }
    } else {
        const std::vector<uint64_t> parks = OurList(0), zoos = OurList(1);
        bool differ = false;
        if (parks != mo.rec.parks) {
            mo.d.Add("parks %u/%u (orig/ours, objects in order)", U(mo.rec.parks.size()), U(parks.size()));
            differ = true;
        }
        if (zoos != mo.rec.zoos) {
            mo.d.Add("zoos %u/%u (objects in order)", U(mo.rec.zoos.size()), U(zoos.size()));
            differ = true;
        }
        if (differ) {
            mo.followed = "resynced";
            if (!StartFrom(mo)) {
                mo.incomplete = "the shadow core could not read the original's record";
                return;
            }
        }
    }
    mo.oursBefore = OurState();
    if (mo.rawBefore) {
        if ((mo.before.latch != 0) != (mo.oursBefore.underfunded != 0)) {
            mo.d.Add("latch before %u/%u", unsigned{mo.before.latch}, unsigned{mo.oursBefore.underfunded});
        }
        if (mo.before.threshold != mo.oursBefore.distress_threshold) {
            mo.d.Add("threshold %g/%g", static_cast<double>(mo.before.threshold),
                     static_cast<double>(mo.oursBefore.distress_threshold));
        }
    }
    if (mo.oursBefore.department != 0 && g_host.department_funding != nullptr) {
        g_host.department_funding(&g_ctx, mo.oursBefore.department, &mo.funding);
        mo.p = fc4_park_escape_probability(g_core, mo.funding);
    }
    for (uint64_t occ : mo.rec.parks) {
        if (occ == 0) continue;
        const uint64_t lot = g_host.occupant_position_lot(&g_ctx, occ);
        if (lot == 0) continue;
        const uint32_t c = LotConditionOf(lot);
        mo.lots.push_back(PredictedLot{occ, lot, c, c});
    }
    g_lotWrites.clear();
    g_ourEscapes = 0;
    fc4_park_do_message(g_core, kMsgSimNewMonth, static_cast<uint32_t>(mo.julian), 0, 0, 0);
    for (const LotWrite& w : g_lotWrites) {
        for (PredictedLot& l : mo.lots) {
            if (l.lot == w.lot) l.predicted = w.condition;
        }
        if (w.condition == 2) ++mo.runDown;
        if (w.condition == 0) ++mo.restored;
    }
    mo.oursAfter = OurState();
    mo.origEscapesBefore = g_origEscapes;
    mo.compared = true;
}

void MonthAfter(void* ctx) {
    Month& mo = *static_cast<Month*>(ctx);
    if (!mo.compared) {
        Log("parks shadow month %s: INCOMPLETE (%s); not compared", DateText(mo.julian).c_str(),
            mo.incomplete ? mo.incomplete : "a fault before the original's month");
        return;
    }
    mo.rawAfter = ReadOriginal(mo.sim, &mo.after);
    if (mo.rawAfter && (mo.after.latch != 0) != (mo.oursAfter.underfunded != 0)) {
        mo.d.Add("latch after %u/%u", unsigned{mo.after.latch}, unsigned{mo.oursAfter.underfunded});
    }
    for (const PredictedLot& l : mo.lots) {
        const uint32_t after = LotConditionOf(l.lot);
        if (after != l.predicted) {
            mo.d.Add("lot of %p condition %u -> orig %u, ours %u", static_cast<void*>(FromToken(l.occ)), l.before,
                     after, l.predicted);
        }
    }
    ++g_months;
    if (mo.d.mismatches == 0) ++g_cleanMonths;
    auto latch = [](bool raw, uint8_t v) { return raw ? std::to_string(v) : std::string("-"); };
    char num[64];
    snprintf(num, sizeof(num), "funding=%g p=%.9g", static_cast<double>(mo.funding), mo.p);
    const std::string thr =
        mo.rawBefore ? std::to_string(mo.before.threshold) + "/" + std::to_string(mo.oursBefore.distress_threshold)
                     : "-/" + std::to_string(mo.oursBefore.distress_threshold);
    const std::string line =
        std::string("parks shadow month ") + DateText(mo.julian) + ": followed=" + mo.followed +
        " parks=" + std::to_string(mo.rec.parks.size()) + " zoos=" + std::to_string(mo.rec.zoos.size()) +
        " latch orig " + latch(mo.rawBefore, mo.before.latch) + "->" + latch(mo.rawAfter, mo.after.latch) +
        " ours " + std::to_string(mo.oursBefore.underfunded) + "->" + std::to_string(mo.oursAfter.underfunded) +
        " threshold " + thr +
        (g_withoutTuning ? " tuning=as-without-exemplar (orig threshold 0.25, question PKG-1)" : " tuning=exemplar") +
        "; " + num + "; pass: lots=" + std::to_string(mo.lots.size()) +
        " run-down=" + std::to_string(mo.runDown) + " restored=" + std::to_string(mo.restored) +
        "; zoo escapes orig=" + std::to_string(g_origEscapes - mo.origEscapesBefore) +
        " ours=" + std::to_string(g_ourEscapes) + " (not compared: clock-seeded generator)" +
        "; messages followed=" + std::to_string(g_followedMessages) +
        "; mismatches=" + std::to_string(mo.d.mismatches) +
        (mo.d.listed.empty() ? std::string() : "; differences: " + mo.d.listed);
    LogShadowLine("parks shadow (cont.)", reinterpret_cast<const uint8_t*>(line.data()), line.size());
    g_followedMessages = 0;
}

// ---- the followed messages (§3) -------------------------------------------------------------------

struct Msg {
    uint32_t type;
    uint32_t d[3];
    uint64_t occ;
};

void CallMessage(void* ctx) {
    const Msg* m = static_cast<const Msg*>(ctx);
    fc4_park_do_message(g_core, m->type, m->d[0], m->d[1], m->d[2], m->occ);
}

void OnMessage(cIGZMessage2* msg) {
    if (msg == nullptr) return;
    const uint32_t type = msg->GetType();
    if (type != kMsgOccupantInserted && type != kMsgOccupantRemoved && type != kMsgZooEscape) return;
    void* p = nullptr;
    if (!msg->QueryInterface(GZIID_cIGZMessage2Standard, &p) || p == nullptr) return;
    cIGZMessage2Standard* s = static_cast<cIGZMessage2Standard*>(p);
    FC4_ACQUIRED(s, "QueryInterface");
    if (type == kMsgZooEscape) {
        if (g_sim != nullptr && s->GetVoid1() == g_sim) ++g_origEscapes;
    } else if (g_core != nullptr) {
        Msg m{};
        m.type = type;
        m.d[0] = static_cast<uint32_t>(s->GetData1());
        m.d[1] = static_cast<uint32_t>(s->GetData2());
        m.d[2] = static_cast<uint32_t>(s->GetData3());
        m.occ = Canonical(static_cast<cIGZUnknown*>(s->GetVoid1()));
        ++g_followedMessages;
        SehGuard("parks shadow (followed message)", &CallMessage, &m);
    }
    FC4_RELEASE(s);
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
        FC4_X87_SCOPE("parks shadow DoMessage", msg->GetType());  // x87 leftover guard (FC4Fpu.h)
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

// Registered once, at the first delivery the shadow follows, and left
// registered for the process.
void Listen() {
    if (g_listening) return;
    g_listening = true;
    cIGZMessageServer2Ptr ms;
    if (!ms) {
        Log("parks shadow: no message server; occupant messages do not reach ours");
        return;
    }
    Events().AddRef();  // our own reference (static object)
    bool ok = true;
    for (uint32_t id : {kMsgOccupantInserted, kMsgOccupantRemoved, kMsgZooEscape}) {
        ok = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&Events()), id) && ok;
    }
    if (!ok) Log("parks shadow: a notification was refused; ours can miss messages");
}

}  // namespace

void ParksShadowStartup(bool simulatorReplaced, bool parksReplaced) {
    g_simulatorReplaced = simulatorReplaced;
    g_parksReplaced = parksReplaced;
    ReadSettings();
    if (!g_enabled) return;
    const char* src = "";
    Setting("parks_shadow", "FC4_PARKS_SHADOW", &src);
    if (parksReplaced) {
        Log("parks shadow: requested (parks_shadow from %s) but INACTIVE: it compares against the ORIGINAL park "
            "manager, and parks is enabled (ours)",
            src);
    } else if (!simulatorReplaced) {
        Log("parks shadow: requested (parks_shadow from %s) but INACTIVE: it follows the original's month, which "
            "passes through FreeCity4's simulator, and simulator is not enabled",
            src);
    } else {
        Log("parks shadow: ON (parks_shadow from %s): a shadow core (read-only host) starts from the ORIGINAL park "
            "manager's record at its first month and follows its occupant messages; at each SimNewMonth the "
            "park and zoo lists are compared with the original's record, the latch (+0x15) and threshold (+0x34) "
            "with the original's (raw, vtable-checked) before and after, and every park lot's condition after the "
            "original's month with ours (lot writes recorded, not made). Not compared: the zoo escape "
            "(clock-seeded generator, parks.md §5)",
            src);
    }
}

void ParksShadowCityShutdown() {
    if (g_core == nullptr && g_sim == nullptr) return;
    if (g_months > 0) Log("parks shadow: city shutdown: %u month lines, %u with mismatches=0", g_months, g_cleanMonths);
    DropCore();
    g_sim = nullptr;
    g_agent = nullptr;
}

bool ParksShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                        int64_t data1, bool* finished) {
    if (type != kMsgSimNewMonth || target == nullptr || msg == nullptr || finished == nullptr || !Active()) {
        return false;
    }
    if (target != g_agent && (agentName == nullptr || strcmp(agentName, kAgentName) != 0)) return false;
    cISC4City* city = CurrentCity();
    cISC4ParkManager* sim = OriginalOf(city);
    if (sim == nullptr) return false;
    if (sim != g_sim) {
        DropCore();
        Track(sim);
    }
    if (target != g_agent) return false;
    Listen();
    Month mo;
    mo.sim = sim;
    mo.julian = static_cast<int32_t>(data1);
    if (!SehGuard("parks shadow (before the month)", &MonthBefore, &mo)) {
        Log("parks shadow: a fault before the original's month; the shadow core is abandoned (its references leak)");
        g_core = nullptr;
        g_coreSim = nullptr;
        mo.compared = false;
    }
    *finished = target->DoMessage(msg);  // the original's month, exactly once
    if (!SehGuard("parks shadow (after the month)", &MonthAfter, &mo)) {
        Log("parks shadow: a fault after the original's month; the shadow core is abandoned (its references leak)");
        g_core = nullptr;
        g_coreSim = nullptr;
    }
    return true;
}

}  // namespace fc4
