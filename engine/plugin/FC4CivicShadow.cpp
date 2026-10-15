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

#include "FC4CivicShadow.h"

#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4Watchdog.h"
#include "FC4WriteCapture.h"
#include "cFC4Civic.h"

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
#include "cISC4CivicBuildingSimulator.h"
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
constexpr uint32_t kMsgLotStateChanged = 0xE8BDA2E7;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cISC4Occupant = 0x26BB767E;
constexpr uint32_t kIID_cISC4DepartmentBudget = 0x223E2B67;
constexpr const char* kAgentName = "cSC4CivicBuildingSimulator";  // §3
constexpr size_t kMaxListed = 12;
// [choice] The shadow core's ape-escape generator (never compared).
constexpr uint32_t kShadowSeed = 0x5EED0C1Cu;

bool g_enabled = false;
bool g_simulatorReplaced = false;
bool g_civicReplaced = false;
bool g_loggedOurs = false;
bool g_listening = false;

const void* g_sim = nullptr;
cIGZMessageTarget2* g_agent = nullptr;

// The shadow core.
CivicContext g_ctx;
fc4_civic_host g_host{};
fc4_civic* g_core = nullptr;
const void* g_coreSim = nullptr;
uint32_t g_months = 0, g_cleanMonths = 0, g_followedMessages = 0;

// Recorded by the shadow host during our month.
struct LotWrite {
    uint64_t lot;
    uint32_t condition;
};
std::vector<LotWrite> g_lotWrites;
uint32_t g_escapes = 0, g_fundingReads = 0;
bool (*g_readFunding)(void*, uint64_t, float*) = nullptr;

void RecordLotCondition(void*, uint64_t lot, uint32_t condition) { g_lotWrites.push_back(LotWrite{lot, condition}); }
void RecordControllers(void*, const char*, uint64_t) { ++g_escapes; }
bool CountedFunding(void* u, uint64_t dept, float* out) {
    ++g_fundingReads;
    return g_readFunding != nullptr && g_readFunding(u, dept, out);
}

void ReadSettings() { g_enabled = SettingInt("civic_shadow", "FC4_CIVIC_SHADOW", 0) != 0; }

bool Active() { return g_enabled && g_simulatorReplaced && !g_civicReplaced; }

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

cISC4CivicBuildingSimulator* OriginalOf(cISC4City* city) {
    if (city == nullptr) return nullptr;
    cISC4CivicBuildingSimulator* raw = city->GetCivicBuildingSimulator();  // slot 73, borrowed
    if (raw == nullptr) return nullptr;
    if (IsOurObject(raw)) {
        if (!g_loggedOurs) {
            g_loggedOurs = true;
            Log("civic shadow: inactive: the city's civic simulator is FreeCity4's own (do not enable civic)");
        }
        return nullptr;
    }
    return raw;
}

void Track(cISC4CivicBuildingSimulator* sim) {
    g_sim = sim;
    void* agent = nullptr;
    g_agent = nullptr;
    if (sim->QueryInterface(kIID_cIGZMessageTarget2, &agent) && agent != nullptr) {
        cIGZUnknown* a = static_cast<cIGZUnknown*>(agent);
        FC4_ACQUIRED(a, "QueryInterface");
        g_agent = static_cast<cIGZMessageTarget2*>(agent);
        FC4_RELEASE(a);  // only its identity is kept
    }
    Log("civic shadow: following the original civic simulator %p (agent %p)", static_cast<const void*>(sim),
        static_cast<void*>(g_agent));
}

// The canonical token of an object: its cISC4Occupant (or `iid`) interface.
uint64_t Canonical(cIGZUnknown* u, uint32_t iid) {
    if (u == nullptr) return 0;
    void* p = nullptr;
    if (!u->QueryInterface(iid, &p) || p == nullptr) return 0;
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

struct Status {
    uint32_t key = 0;
    uint8_t status = 0;
    uint16_t count = 0;
    std::string text;
};

struct Record {
    bool ok = false;
    std::vector<uint32_t> exclusions;
    std::vector<Status> statuses;
    uint64_t landmarks = 0;  // canonical department token
    std::vector<uint64_t> centers, maintained;  // canonical occupant tokens
};

class Bytes {
public:
    explicit Bytes(const std::vector<uint8_t>& b) : b_(b) {}
    bool Get(void* out, size_t n) {
        if (pos_ + n > b_.size()) {
            good_ = false;
            return false;
        }
        if (n != 0) std::memcpy(out, b_.data() + pos_, n);
        pos_ += n;
        return true;
    }
    template <class T>
    bool Val(T& v) {
        return Get(&v, sizeof(T));
    }
    bool Str(std::string& s) {
        uint32_t n = 0;
        if (!Val(n) || n > b_.size() - pos_) {
            good_ = false;
            return false;
        }
        s.assign(reinterpret_cast<const char*>(b_.data() + pos_), n);
        pos_ += n;
        return true;
    }
    // A reference as WriteCapture writes it: the object number, then the class.
    bool Ref(uint32_t& id, uint32_t& cls) {
        cls = 0;
        if (!Val(id)) return false;
        return id == 0 || Val(cls);
    }
    bool good() const { return good_; }
    bool atEnd() const { return pos_ == b_.size(); }

private:
    const std::vector<uint8_t>& b_;
    size_t pos_ = 0;
    bool good_ = true;
};

Record Parse(const WriteCapture& cap) {
    Record r;
    Bytes b(cap.Bytes());
    uint16_t version = 0;
    if (!b.Val(version) || version != 6) return r;
    uint32_t n = 0;
    if (!b.Val(n)) return r;
    for (uint32_t i = 0; i < n && b.good(); ++i) {
        uint32_t id = 0;
        if (b.Val(id)) r.exclusions.push_back(id);
    }
    if (!b.Val(n)) return r;
    for (uint32_t i = 0; i < n && b.good(); ++i) {
        Status s;
        if (b.Val(s.key) && b.Val(s.status) && b.Val(s.count) && b.Str(s.text)) r.statuses.push_back(s);
    }
    uint32_t id = 0, cls = 0;
    if (!b.Ref(id, cls)) return r;
    r.landmarks = Canonical(static_cast<cIGZUnknown*>(cap.SerializableAt(id)), kIID_cISC4DepartmentBudget);
    for (std::vector<uint64_t>* list : {&r.centers, &r.maintained}) {
        if (!b.Val(n)) return r;
        if (n >= 10000000u) continue;  // §7: the shared list reader reads nothing further
        for (uint32_t i = 0; i < n && b.good(); ++i) {
            if (b.Ref(id, cls)) list->push_back(Canonical(static_cast<cIGZUnknown*>(cap.SerializableAt(id)),
                                                          kIID_cISC4Occupant));
        }
    }
    r.ok = b.good() && b.atEnd();
    return r;
}

// ---- the shadow core ------------------------------------------------------------------------------

void DropCore() {
    if (g_core != nullptr) {
        fc4_civic_shutdown(g_core);  // its lists and the department
        fc4_civic_destroy(g_core);
    }
    g_core = nullptr;
    g_coreSim = nullptr;
}

// A fresh shadow core from the original's capture: Read, then Init (which
// creates nothing with the read-only host and makes Shutdown release).
bool Restart(const WriteCapture& cap, const void* sim) {
    DropCore();
    g_ctx = CivicContext{};
    g_ctx.readOnly = true;
    g_host = MakeCivicHost(&g_ctx);
    g_readFunding = g_host.department_funding;
    g_host.department_funding = &CountedFunding;
    g_host.set_lot_condition = &RecordLotCondition;
    g_host.add_controllers = &RecordControllers;
    g_core = fc4_civic_create(&g_host, kShadowSeed);
    if (g_core == nullptr) return false;
    CaptureIStream in(cap);  // FC4WriteCapture.h
    const bool ok = fc4_civic_read(g_core, in.Get());
    fc4_civic_init(g_core);
    if (!ok) {
        DropCore();
        return false;
    }
    g_coreSim = sim;
    return true;
}

std::vector<uint32_t> OurExclusions() {
    std::vector<uint32_t> v(fc4_civic_exclusions(g_core, nullptr, 0));
    if (!v.empty()) fc4_civic_exclusions(g_core, v.data(), v.size());
    return v;
}

std::vector<uint32_t> OurKeys() {
    std::vector<uint32_t> v(fc4_civic_status_keys(g_core, nullptr, 0));
    if (!v.empty()) fc4_civic_status_keys(g_core, v.data(), v.size());
    return v;
}

std::vector<uint64_t> OurList(uint32_t which) {
    std::vector<uint64_t> v(fc4_civic_list(g_core, which, nullptr, 0));
    if (!v.empty()) fc4_civic_list(g_core, which, v.data(), v.size());
    return v;
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

// Step 1: ours (followed since the last month) against the original's record.
void CompareFollowed(const Record& r, cISC4CivicBuildingSimulator* sim, Diff* d, uint32_t* external,
                     uint32_t* slot5, uint32_t* slot5Total, std::string* counts) {
    const std::vector<uint32_t> ex = OurExclusions();
    if (ex != r.exclusions) d->Add("exclusions %u/%u (orig/ours, hash order)", U(r.exclusions.size()), U(ex.size()));
    for (uint32_t id : ex) {
        ++*slot5Total;
        if (sim->IsBuildingExcluded(id)) {
            ++*slot5;
        } else {
            d->Add("slot 5 IsBuildingExcluded(0x%08X) false on the original", id);
        }
    }
    const std::vector<uint32_t> keys = OurKeys();
    std::vector<uint32_t> theirs;
    for (const Status& s : r.statuses) theirs.push_back(s.key);
    if (keys != theirs) d->Add("status keys %u/%u (hash order)", U(theirs.size()), U(keys.size()));
    uint32_t counted = 0;
    for (const Status& s : r.statuses) {
        uint8_t st = 0;
        uint16_t count = 0;
        char buf[1024];
        size_t len = 0;
        if (!fc4_civic_get_status(g_core, s.key, &st, &count, buf, sizeof(buf), &len)) continue;
        if (count != s.count) d->Add("status 0x%08X count %u/%u", s.key, unsigned{s.count}, unsigned{count});
        if (s.count != 0) ++counted;
        const std::string text(buf, len < sizeof(buf) ? len : sizeof(buf) - 1);
        if (st != s.status || (len < sizeof(buf) && text != s.text)) ++*external;
    }
    char c[48];
    snprintf(c, sizeof(c), "%u", counted);
    *counts = c;
    const std::vector<uint64_t> m = OurList(1), cn = OurList(0);
    if (m != r.maintained) d->Add("maintained %u/%u (objects in order)", U(r.maintained.size()), U(m.size()));
    if (cn != r.centers) d->Add("centers %u/%u (objects in order)", U(r.centers.size()), U(cn.size()));
    const uint32_t lo = sim->GetLandmarkCount(), lu = fc4_civic_landmark_count(g_core);  // slot 8
    if (lo != lu) d->Add("landmark count %u/%u", lo, lu);
    fc4_civic_state st{};
    st.struct_size = sizeof(st);
    if (fc4_civic_get_state(g_core, &st) && st.landmarks != r.landmarks) {
        d->Add("Landmarks department %p/%p", static_cast<void*>(FromToken(r.landmarks)),
               static_cast<void*>(FromToken(st.landmarks)));
    }
}

struct PredictedLot {
    uint64_t occ, lot;
    uint32_t before, predicted;
};

// One month: Before (guarded) -> the original's DoMessage (unguarded, exactly
// once) -> After (guarded).
struct Month {
    cISC4CivicBuildingSimulator* sim = nullptr;
    int32_t julian = 0;
    // Before
    bool compared = false;  // the capture was parsed and ours restarted
    const char* incomplete = nullptr;
    bool followed = false;
    Record rec;
    Diff d;
    uint32_t external = 0, slot5 = 0, slot5Total = 0;
    std::string counts = "-";
    std::vector<PredictedLot> lots;
    uint32_t runDown = 0, recovered = 0;
};

WriteCapture g_capture;

void MonthBefore(void* ctx) {
    Month& mo = *static_cast<Month*>(ctx);
    cISC4CivicBuildingSimulator* sim = mo.sim;
    mo.followed = g_core != nullptr && g_coreSim == sim;
    const bool captured = g_capture.Capture(reinterpret_cast<cIGZUnknown*>(sim));
    if (captured) mo.rec = Parse(g_capture);
    if (!mo.rec.ok) {
        mo.incomplete = captured ? "the original's record could not be parsed" : "the original's record could not be captured";
        DropCore();
        return;
    }
    if (mo.followed) CompareFollowed(mo.rec, sim, &mo.d, &mo.external, &mo.slot5, &mo.slot5Total, &mo.counts);
    // Step 2: ours restarts from the record and runs its month on the same
    // inputs, lot writes recorded.
    if (!Restart(g_capture, sim)) {
        mo.incomplete = "the shadow core could not read the original's record";
        return;
    }
    for (uint64_t occ : mo.rec.maintained) {
        if (occ == 0) continue;
        const uint64_t lot = g_host.occupant_lot(&g_ctx, occ);
        if (lot == 0) continue;
        const uint32_t c = g_host.lot_condition(&g_ctx, lot);
        mo.lots.push_back(PredictedLot{occ, lot, c, c});
    }
    g_lotWrites.clear();
    g_escapes = 0;
    g_fundingReads = 0;
    fc4_civic_do_message(g_core, kMsgSimNewMonth, static_cast<uint32_t>(mo.julian), 0, 0, 0, 0);
    for (const LotWrite& w : g_lotWrites) {
        for (PredictedLot& l : mo.lots) {
            if (l.lot == w.lot) l.predicted = w.condition;
        }
        if (w.condition == 2) ++mo.runDown;
        if (w.condition == 0) ++mo.recovered;
    }
    mo.compared = true;
}

void MonthAfter(void* ctx) {
    Month& mo = *static_cast<Month*>(ctx);
    if (!mo.compared) {
        Log("civic shadow month %s: INCOMPLETE (%s); not compared", DateText(mo.julian).c_str(),
            mo.incomplete ? mo.incomplete : "a fault before the original's month");
        return;
    }
    for (const PredictedLot& l : mo.lots) {
        const uint32_t after = g_host.lot_condition(&g_ctx, l.lot);
        if (after != l.predicted) {
            mo.d.Add("lot of %p condition %u -> orig %u, ours %u", static_cast<void*>(FromToken(l.occ)), l.before,
                     after, l.predicted);
        }
    }
    ++g_months;
    if (mo.d.mismatches == 0) ++g_cleanMonths;
    const Record& rec = mo.rec;
    const std::string line =
        std::string("civic shadow month ") + DateText(mo.julian) + ": followed=" + (mo.followed ? "yes" : "first") +
        " exclusions=" + std::to_string(rec.exclusions.size()) + " statuses=" + std::to_string(rec.statuses.size()) +
        " counted=" + mo.counts + " maintained=" + std::to_string(rec.maintained.size()) +
        " centers=" + std::to_string(rec.centers.size()) +
        " landmarks=" + std::to_string(mo.sim->GetLandmarkCount()) + " excluded-slot5=" + std::to_string(mo.slot5) +
        "/" + std::to_string(mo.slot5Total) + "; external status writes=" + std::to_string(mo.external) +
        "; messages followed=" + std::to_string(g_followedMessages) + "; pass: lots=" +
        std::to_string(mo.lots.size()) + " funding-reads=" + std::to_string(g_fundingReads) +
        " run-down=" + std::to_string(mo.runDown) + " recovered=" + std::to_string(mo.recovered) +
        "; ape escape not compared (centers=" + std::to_string(rec.centers.size()) +
        ", ours escaped=" + std::to_string(g_escapes) + "); mismatches=" + std::to_string(mo.d.mismatches) +
        (mo.d.listed.empty() ? std::string() : "; differences: " + mo.d.listed);
    LogShadowLine("civic shadow (cont.)", reinterpret_cast<const uint8_t*>(line.data()), line.size());
    g_followedMessages = 0;
}

// ---- the followed messages (§3) -------------------------------------------------------------------

struct Msg {
    uint32_t type;
    uint32_t d[3];
    uint64_t occ, unknown;
};

void CallMessage(void* ctx) {
    const Msg* m = static_cast<const Msg*>(ctx);
    fc4_civic_do_message(g_core, m->type, m->d[0], m->d[1], m->d[2], m->occ, m->unknown);
}

void OnMessage(cIGZMessage2* msg) {
    if (msg == nullptr || g_core == nullptr) return;
    const uint32_t type = msg->GetType();
    if (type != kMsgOccupantInserted && type != kMsgOccupantRemoved && type != kMsgLotStateChanged) return;
    void* p = nullptr;
    if (!msg->QueryInterface(GZIID_cIGZMessage2Standard, &p) || p == nullptr) return;
    cIGZMessage2Standard* s = static_cast<cIGZMessage2Standard*>(p);
    FC4_ACQUIRED(s, "QueryInterface");
    Msg m{};
    m.type = type;
    m.d[0] = static_cast<uint32_t>(s->GetData1());
    m.d[1] = static_cast<uint32_t>(s->GetData2());
    m.d[2] = static_cast<uint32_t>(s->GetData3());
    if (type == kMsgLotStateChanged) {
        m.unknown = TokenOf(s->GetIGZUnknown());
    } else {
        m.occ = Canonical(static_cast<cIGZUnknown*>(s->GetVoid1()), kIID_cISC4Occupant);
    }
    ++g_followedMessages;
    SehGuard("civic shadow (followed message)", &CallMessage, &m);
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
        FC4_X87_SCOPE("civic shadow DoMessage", msg->GetType());  // x87 leftover guard (FC4Fpu.h)
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

void Listen() {
    if (g_listening) return;
    g_listening = true;
    cIGZMessageServer2Ptr ms;
    if (!ms) {
        Log("civic shadow: no message server; occupant and lot messages do not reach ours");
        return;
    }
    Events().AddRef();  // our own reference (static object)
    bool ok = true;
    for (uint32_t id : {kMsgLotStateChanged, kMsgOccupantInserted, kMsgOccupantRemoved}) {
        ok = ms->AddNotification(static_cast<cIGZMessageTarget2*>(&Events()), id) && ok;
    }
    if (!ok) Log("civic shadow: a notification was refused; ours can miss messages");
}

}  // namespace

void CivicShadowStartup(bool simulatorReplaced, bool civicReplaced) {
    g_simulatorReplaced = simulatorReplaced;
    g_civicReplaced = civicReplaced;
    ReadSettings();
    if (!g_enabled) return;
    const char* src = "";
    Setting("civic_shadow", "FC4_CIVIC_SHADOW", &src);
    if (civicReplaced) {
        Log("civic shadow: requested (civic_shadow from %s) but INACTIVE: it compares against the ORIGINAL civic "
            "simulator, and civic is enabled (ours)",
            src);
    } else if (!simulatorReplaced) {
        Log("civic shadow: requested (civic_shadow from %s) but INACTIVE: it follows the original's month, which "
            "passes through FreeCity4's simulator, and simulator is not enabled",
            src);
    } else {
        Log("civic shadow: ON (civic_shadow from %s): a shadow core follows the ORIGINAL civic simulator's occupant "
            "and lot messages (read-only host); at each SimNewMonth the original's record (its own Write) is "
            "compared with ours (exclusions and status keys in hash order, building counts, the maintained and "
            "disease-center lists, slots 5 and 8), then ours restarts from that record and runs its month with lot "
            "writes recorded; after the original's month each maintained lot's condition is compared with ours. "
            "Not compared: the ape escape (clock-seeded temporary generator, civic-buildings.md §5); status bytes "
            "and tooltips written through slots 6 / 7 by others are counted as external writes",
            src);
    }
}

void CivicShadowCityShutdown() {
    if (g_core == nullptr && g_sim == nullptr) return;
    if (g_months > 0) Log("civic shadow: city shutdown: %u month lines, %u with mismatches=0", g_months, g_cleanMonths);
    DropCore();
    g_sim = nullptr;
    g_agent = nullptr;
}

bool CivicShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                        int64_t data1, bool* finished) {
    if (type != kMsgSimNewMonth || target == nullptr || msg == nullptr || finished == nullptr || !Active()) {
        return false;
    }
    if (target != g_agent && (agentName == nullptr || strcmp(agentName, kAgentName) != 0)) return false;
    cISC4City* city = CurrentCity();
    cISC4CivicBuildingSimulator* sim = OriginalOf(city);
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
    if (!SehGuard("civic shadow (before the month)", &MonthBefore, &mo)) {
        Log("civic shadow: a fault before the original's month; the shadow core is abandoned (its references leak)");
        g_core = nullptr;
        g_coreSim = nullptr;
        mo.compared = false;
    }
    *finished = target->DoMessage(msg);  // the original's month, exactly once
    if (!SehGuard("civic shadow (after the month)", &MonthAfter, &mo)) {
        Log("civic shadow: a fault after the original's month; the shadow core is abandoned (its references leak)");
        g_core = nullptr;
        g_coreSim = nullptr;
    }
    return true;
}

}  // namespace fc4
