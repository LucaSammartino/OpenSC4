// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4PropOccupantVerify.h"
#include "FC4PropOccupantCompare.h"
#include "compat/FC4PropOccupantAuthority.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Subsystems.h"
#include "fc4.h"
#include "cIGZCOM.h"
#include "cRZCOMDllDirector.h"
#include <cstring>

namespace fc4 { namespace prop_verify {
// prop-occupant 4.1, exactly 24 slots. The SDK narrows slots12/19/20;
// this local published declaration preserves their clean EAX for comparison.
// AL byte values are uint8_t, including non-boolean chance and usage codes.
struct PlainProp : cIGZUnknown {
    virtual void* AsOccupant() = 0;
    virtual void SetBoundingBox(const float*, const float*) = 0;
    virtual void* GetPropPlacementProperties() = 0;
    virtual void SetPropPlacementProperties(void*) = 0;
    virtual uint32_t GetPropType() const = 0;
    virtual void SetPropType(uint32_t) = 0;
    virtual int32_t GetOrientation() const = 0;
    virtual uint8_t SetOrientation(int32_t) = 0;
    virtual void SetState(uint32_t) = 0;
    virtual uint32_t GetState() const = 0;
    virtual void SetTimeOfDayRange(float, float) = 0;
    virtual void GetTimeOfDayRange(float&, float&) const = 0;
    virtual void SetSimulatorDateRange(uint32_t, uint32_t, uint8_t, uint8_t) = 0;
    virtual void TickSimulatorDate(const void*) = 0;
    virtual void SetRandomChance(uint8_t) = 0;
    virtual uint8_t GetRandomChance() const = 0;
    virtual uint32_t GetPropTimerBitfield() const = 0;
    virtual uint32_t GetPropTimerState(uint8_t) const = 0;
    virtual void SetPropTimerState(uint8_t, bool) = 0;
    virtual uint8_t GetPropUsage() const = 0;
    virtual void SetPropUsage(uint8_t) = 0;
};
constexpr uint32_t clsid = 0x2977aa47, iid = 0xe9793a65;
struct Native {
    PlainProp* p = nullptr;
    unsigned object = 0;
    bool Authority(unsigned slot) {
        compat::PropAuthority a;
        bool ok = compat::OriginalPlainProp(p, slot, a);
        if (!ok) Log("prop-occupant-verify: REFUSED object=%u slot=%u class=%08x iid=%08x ptr=%p table=%p target=%p module=%p reason=%s",
            object, slot, clsid, iid, p, reinterpret_cast<void*>(a.table), reinterpret_cast<void*>(a.target), a.module, a.reason);
        static bool logged[2][24]{};
        if (ok && !logged[object][slot]) {
            logged[object][slot] = true;
            Log("prop-occupant-verify: authority object=%u slot=%u ptr=%p table=%p target=%p module=%p reason=%s",
                object, slot, p, reinterpret_cast<void*>(a.table), reinterpret_cast<void*>(a.target), a.module, a.reason);
        }
        return ok;
    }
    uint32_t Read(unsigned slot, uint8_t mask) {
        switch (slot) {
            case 9: return uint32_t(p->GetOrientation());
            case 12: return p->GetState();
            case 18: return p->GetRandomChance();
            case 19: return p->GetPropTimerBitfield();
            case 20: return p->GetPropTimerState(mask);
            case 22: return p->GetPropUsage();
        }
        return 0; // Only the frozen supported slots reach this adapter.
    }
    uint32_t Write(const Fixture& f) {
        switch (f.slot) {
            case 10: return p->SetOrientation(f.arg);
            case 11: p->SetState(uint8_t(f.arg)); break;
            case 17: p->SetRandomChance(uint8_t(f.arg)); break;
            case 21: p->SetPropTimerState(uint8_t(f.arg), f.on); break;
            case 23: p->SetPropUsage(uint8_t(f.arg)); break;
        }
        return 0; // Placeholder ignored for void slots, never a native result.
    }
};
struct Rust {
    fc4_prop_occupant* p = nullptr;
    uint32_t Read(unsigned slot, uint8_t mask) {
        switch (slot) {
            case 9: return uint32_t(fc4_prop_occupant_get_orientation(p));
            case 12: return fc4_prop_occupant_get_state(p);
            case 18: return fc4_prop_occupant_get_random_chance(p);
            case 19: return fc4_prop_occupant_get_timer_bitfield(p);
            case 20: return uint32_t(fc4_prop_occupant_get_timer_state(p, mask));
            case 22: return fc4_prop_occupant_get_prop_usage(p);
        }
        return 0;
    }
    uint32_t Write(const Fixture& f) {
        switch (f.slot) {
            case 10: return uint32_t(fc4_prop_occupant_set_orientation(p, f.arg));
            case 11: fc4_prop_occupant_set_state(p, uint8_t(f.arg)); break;
            case 17: fc4_prop_occupant_set_random_chance(p, uint8_t(f.arg)); break;
            case 21: fc4_prop_occupant_set_timer_state(p, uint8_t(f.arg), f.on); break;
            case 23: fc4_prop_occupant_set_prop_usage(p, uint8_t(f.arg)); break;
        }
        return 0;
    }
};
struct Host {
    unsigned cityQueries = 0, exemplarQueries = 0, forbidden = 0;
    static int32_t City(void* h) { ++static_cast<Host*>(h)->cityQueries; return 0; }
    static int32_t Exemplar(void* h) { ++static_cast<Host*>(h)->exemplarQueries; return -1; }
    static void Post(void* h, uint32_t, uint32_t) { ++static_cast<Host*>(h)->forbidden; }
    static void Model(void* h, uint8_t) { ++static_cast<Host*>(h)->forbidden; }
    static void Clock(void* h, uint8_t, uint8_t) { ++static_cast<Host*>(h)->forbidden; }
    static uint8_t Draw(void* h) { ++static_cast<Host*>(h)->forbidden; return 0; }
    static int32_t Remove(void* h) { ++static_cast<Host*>(h)->forbidden; return 0; }
};
void Diff(unsigned object, unsigned slot, uint32_t arg, uint32_t a, uint32_t b, uint32_t e) {
    Log("prop-occupant-verify: DIFF object=%u slot=%u arg=%08x native=%08x rust=%08x literal=%08x", object, slot, arg, a, b, e);
}
uint32_t Bits(float f) { uint32_t v; std::memcpy(&v, &f, 4); return v; }
} // prop_verify

void PropOccupantVerify() {
    using namespace prop_verify;
    static bool attempted = false;
    if (attempted || !SettingInt("prop_occupant_verify", "FC4_PROP_OCCUPANT_VERIFY", 0)) return;
    attempted = true;
    Counts counts;
    Lifetime life;
    Native native[2];
    Rust rust[2];
    Host host[2];
    unsigned factoryAttempts = 0, acquisitionRefused = 0, completed = 0;
    auto* director = RZGetCOMDllDirector();
    auto* com = director ? director->GZCOM() : nullptr;
    const Subsystem* replacementOwner = nullptr;
    const auto* replacement = FindEnabledClass(clsid, &replacementOwner);
    Log("prop-occupant-verify: start class=%08x iid=%08x enable=%s disable=%s replacement=%s owner=%s planned_dispatches=1065 natural=0",
        clsid, iid, Setting("enable", "FC4_ENABLE").c_str(), Setting("disable", "FC4_DISABLE").c_str(),
        replacement ? replacement->name : "none", replacementOwner ? replacementOwner->key : "none");
    bool ok = com && compat::IdentifiedVersion();
    if (!ok) { ++acquisitionRefused; Log("prop-occupant-verify: REFUSED no-supported-framework-or-version"); }
    for (unsigned i = 0; ok && i < 2; ++i) {
        native[i].object = i;
        void* out = nullptr;
        ++factoryAttempts;
        const bool acquired = com->GetClassObject(clsid, iid, &out);
        Log("prop-occupant-verify: factory object=%u success=%u class=%08x iid=%08x ptr=%p", i, acquired ? 1u : 0u, clsid, iid, out);
        if (!acquired || !out) { ++acquisitionRefused; ok = false; break; }
        native[i].p = static_cast<PlainProp*>(out);
        ++life.factory;
        FC4_ACQUIRED(native[i].p, "plain prop verifier factory owned");
        compat::PropAuthority identity;
        bool original = compat::OriginalPlainProp(out, 0, identity);
        Log("prop-occupant-verify: identity object=%u table=%p target=%p module=%p reason=%s original=%u",
            i, reinterpret_cast<void*>(identity.table), reinterpret_cast<void*>(identity.target), identity.module, identity.reason, original ? 1u : 0u);
        if (!counts.Enter(native[i], 0)) { ok = false; break; }
        void* qi = nullptr;
        const bool q = native[i].p->QueryInterface(iid, &qi);
        if (q && qi) {
            ++life.qi;
            auto* owned = static_cast<PlainProp*>(qi);
            FC4_ACQUIRED(owned, "plain prop verifier QI owned");
            Native view{owned, i};
            uint32_t refs = 0xffffffffu;
            if (!ReleaseOwned(counts, life, view, false, [&]() { refs = FC4_RELEASE(owned); return refs; })) {
                Diff(i, 2, 0, refs, 1, 1); ok = false;
            }
            Log("prop-occupant-verify: lifecycle object=%u QI_owned=1 released=%u refs=%u", i, refs != 0xffffffffu ? 1u : 0u, refs);
        }
        const uint32_t identityOK = q && qi == out ? 1u : 0u;
        if (!counts.Compare(0, identityOK, uint32_t(fc4_prop_occupant_query_interface(iid) == 0), 1)) {
            Diff(i, 0, iid, identityOK, 1, 1); ok = false;
        }
        if (!ok) break;
        fc4_prop_occupant_host h{};
        h.struct_size = sizeof(h); h.user = &host[i];
        h.is_in_city = Host::City; h.has_model_state_property = Host::Exemplar;
        h.post_occupant_message = Host::Post; h.set_model_state = Host::Model;
        h.set_clock_entry = Host::Clock; h.draw_chance = Host::Draw; h.remove_from_city = Host::Remove;
        rust[i].p = fc4_prop_occupant_create(&h, nullptr);
        if (!rust[i].p) { ++acquisitionRefused; ok = false; break; }
        ++life.rustCreated;
        if (!counts.Enter(native[i], 14)) { ok = false; break; }
        float a = -1, b = -1, c = -1, d = -1;
        native[i].p->GetTimeOfDayRange(a, b);
        fc4_prop_occupant_get_time_of_day_range(rust[i].p, &c, &d);
        if (!counts.Compare(14, Bits(a), Bits(c), 0)) Diff(i, 14, 0, Bits(a), Bits(c), 0);
        if (!counts.Compare(14, Bits(b), Bits(d), 0)) Diff(i, 14, 1, Bits(b), Bits(d), 0);
    }
    if (ok && (native[0].p == native[1].p || rust[0].p == rust[1].p)) {
        ++acquisitionRefused; ok = false;
        Log("prop-occupant-verify: REFUSED objects-not-distinct");
    }
    if (ok) completed = Exercise(counts, native, rust, Diff) ? 1u : 0u;
    for (unsigned i = 0; i < 2; ++i) {
        if (rust[i].p) { fc4_prop_occupant_destroy(rust[i].p); ++life.rustDestroyed; }
        if (native[i].p) {
            uint32_t refs = 0xffffffffu;
            if (!ReleaseOwned(counts, life, native[i], true, [&]() { refs = FC4_RELEASE_CREATED(native[i].p); return refs; })) {
                if (refs != 0xffffffffu) Diff(i, 2, 0, refs, 0, 0);
            }
            if (refs == 0xffffffffu && IsOurObject(native[i].p)) {
                // Owned supported interface cleanup only; never native credit.
                FC4_RELEASE_CREATED(native[i].p);
                ++life.releaseFactory; ++life.foreignCleanup;
            }
            Log("prop-occupant-verify: lifecycle object=%u factory_owned=1 released=%u refs=%u rust_destroyed=%u",
                i, refs != 0xffffffffu ? 1u : 0u, refs, rust[i].p ? 1u : 0u);
        }
        Log("prop-occupant-verify: host object=%u not_in_city_queries=%u no_exemplar_queries=%u forbidden=%u",
            i, host[i].cityQueries, host[i].exemplarQueries, host[i].forbidden);
    }
    unsigned attempts = 0, dispatched = 0, compared = 0, refused = acquisitionRefused, mismatches = 0, checks = 0;
    for (unsigned slot = 0; slot < 24; ++slot) {
        const auto& m = counts.methods[slot];
        attempts += m.attempted; dispatched += m.dispatched; compared += m.compared;
        refused += m.refused; mismatches += m.mismatches; checks += m.checks;
        Log("prop-occupant-verify: method slot=%u attempted=%u dispatched=%u compared_values=%u refused=%u mismatches=%u literal_checks=%u",
            slot, m.attempted, m.dispatched, m.compared, m.refused, m.mismatches, m.checks);
    }
    Log("prop-occupant-verify: totals completed=%u factory_attempts=%u factory_owned=%u/%u qi_owned=%u/%u rust=%u/%u foreign_cleanup=%u lifecycle_balanced=%u attempted=%u dispatched=%u compared_values=%u literal_checks=%u refused=%u mismatches=%u snapshots=%u mutations=%u forbidden=%u natural=0",
        completed, factoryAttempts, life.factory, life.releaseFactory, life.qi, life.releaseQI, life.rustCreated, life.rustDestroyed,
        life.foreignCleanup, life.Balanced() ? 1u : 0u, attempts, dispatched, compared, checks, refused, mismatches,
        counts.snapshots, counts.mutations, host[0].forbidden + host[1].forbidden);
    Log("prop-occupant-verify: excluded=direct-AddRef,AsOccupant,AABB,placement,type-intern,clock-set,date,tick,Init,city,attachment,model,effect,RNG,record,lot-tag,natural-owner,save-owner,whole-owner partial=QI,Release,range-initial,scalar-state,timer-incomplete-low-nibble");
}
} // fc4
