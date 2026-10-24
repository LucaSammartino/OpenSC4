// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4OrdinanceShadow.h"

#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4WriteCapture.h"
#include "cFC4Ordinance.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4Ordinance.h"
#include "cRZCOMDllDirector.h"

#include <cstring>
#include <set>
#include <vector>

namespace fc4 {
namespace {
constexpr uint32_t kMonth = 0x66956816;
bool g_active = false;
uint32_t g_months = 0, g_clean = 0;

cISC4City* CurrentCity() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (!app || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || !sc4) return nullptr;
    FC4_ACQUIRED(sc4, "QueryInterface");
    cISC4City* city = sc4->GetCity();
    FC4_RELEASE(sc4);
    return city;
}

// The existing host's reads use only public game interfaces. None of its
// object-management, messaging or city mutation callbacks can escape this copy.
fc4_ordinance_host ReadOnlyHost() {
    auto h = FC4MakeOrdinanceHost(nullptr);
    h.for_each_exemplar_key = nullptr;
    h.create_ordinance = nullptr;
    h.add_ref = nullptr;
    h.release = nullptr;
    h.foreign_ordinance = nullptr;
    h.ordinance_call = nullptr;
    h.ordinance_text = nullptr;
    h.ordinance_misc_property = nullptr;
    h.post = nullptr;
    h.lots_with_building_type = nullptr;
    h.demolish_lots = nullptr;
    h.hide_conditional_building = nullptr;
    h.remove_view_input_control = nullptr;
    return h;
}

struct Record {
    cISC4Ordinance* original = nullptr;  // QI reference, owned for this delivery
    fc4_ordinance* core = nullptr;
    uint32_t cls = 0;
};

// Only live while the simulator capture is read on the delivery thread. All
// references resolve to private core copies, never to the game's ordinances.
const std::vector<Record>* g_readRecords = nullptr;
bool ReadCoreRef(void* user, uint32_t, fc4_ref* out) {
    auto& in = *static_cast<CaptureIStream*>(user);
    uint32_t id = 0, cls = 0;
    if (!out || !in.Take(&id, 4)) return false;
    *out = {};
    if (id == 0) return true;
    if (!in.Take(&cls, 4) || !g_readRecords || id > g_readRecords->size()) return false;
    const auto& r = (*g_readRecords)[id - 1];
    if (!r.core || cls != r.cls) return false;
    out->token = id;
    out->class_id = cls;
    out->core_handle = r.core;
    return true;
}

struct Month {
    WriteCapture capture{true};
    fc4_ordinance_host host = ReadOnlyHost();
    std::vector<Record> records;
    fc4_ordinance_sim* core = nullptr;
    uint32_t errors = 0, checks = 0, shown = 0;
    const char* phase = "simulator capture";
    uint32_t recordIndex = 0;
    // Section 6.2/6.3 queries are included even when all ordinances are off
    // or their misc holders enumerate only own (not parent) properties.
    std::set<uint32_t> effects{0x0911E117, 0xA8F4EB0C, 0x28ED0380, 0x28F42AA0,
                               0x08F79B8E, 0xE8F79C8B, 0xE8F79C90, 0x8A67E378,
                               0x8A67E373, 0x8A67E374, 0x8A67E376, 0xAA5B8407, 0x8A612FEE,
                               0x2A651010, 0x2A651020, 0x2A651030, 0x2A653110, 0x2A653120,
                               0x2A653130, 0x2A653210, 0x2A653220, 0x2A653230, 0x2A654100,
                               0x2A654200, 0x2A654300, 0x2A654400};

    ~Month() {
        // Destroy the map first: it owns Rc references to the copied records.
        fc4_ordinance_sim_destroy(core);
        for (auto& r : records) {
            fc4_ordinance_destroy(r.core);
            FC4_RELEASE(r.original);
        }
    }

    void Check(uint32_t id, const char* field, int64_t original, int64_t ours) {
        ++checks;
        if (original == ours) return;
        ++errors;
        if (shown++ < 8) Log("ordinance shadow detail: id=%08X %s orig=%lld ours=%lld", id, field,
                            static_cast<long long>(original), static_cast<long long>(ours));
    }

    bool Before(cISC4OrdinanceSimulator* sim) {
        if (!capture.Capture(sim)) return false;
        phase = "simulator header";
        // Simulator record section 9.1: count after version and two s64 totals.
        CaptureIStream countIn(capture);
        uint16_t version = 0;
        uint32_t count = 0;
        if (!countIn.Take(&version, 2) || version != 4 || !countIn.Take(nullptr, 16) ||
            !countIn.Take(&count, 4) || count > 4096) return false;
        records.resize(count);
        for (uint32_t i = 0; i < count; ++i) {
            recordIndex = i;
            phase = "ordinance identity";
            auto& r = records[i];
            auto* serial = capture.SerializableAt(i + 1);
            r.cls = capture.ClassOf(i + 1);
            if (!serial || !FC4IsOrdinanceClass(r.cls) ||
                !serial->QueryInterface(0xAA5E2247, reinterpret_cast<void**>(&r.original)) || !r.original)
                return false;
            FC4_ACQUIRED(r.original, "QueryInterface");
            r.core = fc4_ordinance_create(&host, r.cls, i + 1);
            WriteCapture objectCapture(true);
            phase = "ordinance capture";
            if (!r.core || !objectCapture.Capture(r.original)) return false;
            CaptureIStream in(objectCapture);
            phase = "ordinance core read";
            if (!fc4_ordinance_read(r.core, in.Get()) || !in.atEnd()) return false;
            // Compare every effective property, including parent/cohort effects.
            // Index 0..8 covers the shipped effect rows (section 6) and the
            // traffic nine-value offset; longer mod arrays are outside this sample.
            for (const auto& p : props::EffectiveProperties(AsGame(r.original)->GetMiscProperties()))
                effects.insert(p.id);
        }
        core = fc4_ordinance_sim_create(&host, 0);
        phase = "simulator core read/step";
        CaptureIStream in(capture);
        auto api = *in.Get();
        api.get_ref = &ReadCoreRef;
        g_readRecords = &records;
        const bool ok = core && fc4_ordinance_sim_read(core, &api) && in.atEnd();
        g_readRecords = nullptr;
        return ok && fc4_ordinance_sim_handle_message(core, kMonth);
    }

    void After(cFC4ISC4OrdinanceSimulator* sim) {
        Check(0, "expense", sim->GetOrdinanceMonthlyExpense(), fc4_ordinance_sim_money(core, 0));
        Check(0, "income", sim->GetOrdinanceMonthlyIncome(), fc4_ordinance_sim_money(core, 1));
        for (const auto& r : records) {
            auto* o = AsGame(r.original);
            const uint32_t id = o->GetID();
            Check(id, "available", o->IsAvailable(), fc4_ordinance_call(r.core, FC4_ORD_IS_AVAILABLE, 0));
            Check(id, "on", o->IsOn(), fc4_ordinance_call(r.core, FC4_ORD_IS_ON, 0));
            Check(id, "enabled", o->IsEnabled(), fc4_ordinance_call(r.core, FC4_ORD_IS_ENABLED, 0));
            Check(id, "adjusted", o->GetMonthlyAdjustedIncome(),
                  fc4_ordinance_call(r.core, FC4_ORD_MONTHLY_ADJUSTED_INCOME, 0));
            Check(id, "monthly", o->GetCurrentMonthlyIncome(),
                  fc4_ordinance_call(r.core, FC4_ORD_CURRENT_MONTHLY_INCOME, 0));
            Check(id, "conditions", o->CheckConditions(), fc4_ordinance_call(r.core, FC4_ORD_CHECK_CONDITIONS, 0));
            // [abi-ret] Ordinance section 4.2: these bool queries return found && flag (available/on/enabled).
            Check(id, "sim available", sim->IsOrdinanceAvailable(id), fc4_ordinance_sim_query(core, 0, id));
            Check(id, "sim on", sim->IsOrdinanceOn(id), fc4_ordinance_sim_query(core, 1, id));
            Check(id, "sim enabled", sim->IsOrdinanceEnabled(id), fc4_ordinance_sim_query(core, 3, id));
        }
        for (uint32_t prop : effects) {
            for (uint32_t index = 0; index < 9; ++index) {
                Check(prop, "offset", sim->GetCumulativeOffset32(prop, index),
                      fc4_ordinance_sim_cumulative_offset(core, prop, index));
                const float original = sim->GetCumulativeFactor(prop, index);
                const float ours = fc4_ordinance_sim_cumulative_factor(core, prop, index);
                uint32_t a = 0, b = 0;
                memcpy(&a, &original, 4);
                memcpy(&b, &ours, 4);
                Check(prop, "factor bits", a, b);
            }
        }
    }
};
}  // namespace

void OrdinanceShadowStartup(bool simulatorReplaced, bool ordinanceReplaced) {
    g_active = SettingInt("ordinance_shadow", "FC4_ORDINANCE_SHADOW", 0) != 0 &&
               simulatorReplaced && !ordinanceReplaced;
    if (SettingInt("ordinance_shadow", "FC4_ORDINANCE_SHADOW", 0) == 0) return;
    Log("ordinance shadow: %s; record-only original/core monthly comparison; needs our simulator and original ordinance",
        g_active ? "ON" : "INACTIVE");
}

bool OrdinanceShadowDeliver(cIGZMessageTarget2* target, const char* name, cIGZMessage2* msg,
                            uint32_t type, int64_t julian, bool* finished) {
    if (!g_active || type != kMonth || !target || !msg || !finished || !name ||
        strcmp(name, "cSC4OrdinanceSimulator") != 0) return false;
    cISC4City* city = CurrentCity();
    auto* original = city ? city->GetOrdinanceSimulator() : nullptr;
    if (!original) return false;
    Month month;
    const bool captured = month.Before(original);
    *finished = target->DoMessage(msg);  // exactly one delivery, even on capture failure
    if (captured) month.After(AsGame(original));
    else {
        ++month.errors;
        Log("ordinance shadow detail: capture failed at %s record=%u", month.phase, month.recordIndex);
    }
    ++g_months;
    if (month.errors == 0) ++g_clean;
    fc4_date date{};
    fc4_date_from_julian(static_cast<int32_t>(julian), &date);
    Log("ordinance shadow %04d-%02d-%02d: records=%u checks=%u effects=%u captured=%u mismatches=%u months=%u clean=%u",
        date.year, date.month, date.day_of_month, static_cast<unsigned>(month.records.size()), month.checks,
        static_cast<unsigned>(month.effects.size()), captured ? 1u : 0u, month.errors, g_months, g_clean);
    return true;
}
intptr_t OrdinanceShadowTest(cISC4OrdinanceSimulator* original) {
    Month month;
    return original && month.Before(original) ? static_cast<intptr_t>(month.records.size()) : -1;
}
}  // namespace fc4
