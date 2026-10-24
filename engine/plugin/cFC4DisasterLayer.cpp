// SPDX-License-Identifier: GPL-3.0-or-later
#include "cFC4DisasterLayer.h"
#include "FC4DisasterHost.h"
#include "FC4HistClock.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cFC4Message2Standard.h"
#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cISC4App.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4DisasterInstance.h"
#include "cISC4DisasterManager.h"
#include "cISC4Occupant.h"
#include <atomic>
#include <cstddef>
#include <map>

namespace {
template<class T> uint64_t Token(T* p) { return reinterpret_cast<uintptr_t>(p); }
template<class T> T* Ptr(uint64_t t) { return reinterpret_cast<T*>(static_cast<uintptr_t>(t)); }
constexpr uint32_t kMessages[] = {0x99EF1143, 0xE94671EC, 0xE94671EB, 0x894671F6, 0xE94671ED};

// Native record layout, disasters.md §2.6. The only external reader reads
// counts, total cost and effectiveness. The vector is owned by this layer.
struct CostPair { uint32_t count = 0, pad = 0; int64_t cost = 0; };
struct CostRecord {
    uint32_t count = 0, pad = 0;
    int64_t total = 0;
    CostPair kinds[4];
    void* special[3] = {};
    float fire = 0, police = 0;
};
static_assert(offsetof(CostRecord, fire) == 0x5c, "disaster statistics ABI");
static_assert(offsetof(CostRecord, police) == 0x60, "disaster statistics ABI");

class Layer final : public cFC4ISC4DisasterLayer, public cIGZMessageTarget2 {
public:
    Layer();
    ~Layer();
    bool QueryInterface(uint32_t iid, void** out) override;
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { const auto n = --refs_; if (!n) delete this; return n; }
    bool Init() override;
    bool Shutdown() override;
    bool EnableDisasters(bool enable) override { return fc4_dlayer_enable(core_, enable); }
    bool DisastersEnabled() override { return fc4_dlayer_enabled(core_); }
    cISC4DisasterManager* GetDisasterManager(uint32_t type) override {
        auto it = managers_.find(type); return it == managers_.end() ? nullptr : it->second;
    }
    bool DisasterHasBegun(cISC4DisasterInstance* p) override {
        fc4::CoreLock::Section s(lock_, "disaster begin");
        const bool ok = fc4_dlayer_begun(core_, Token(p)); Sync(); return ok;
    }
    bool DisasterHasEnded() override {
        fc4::CoreLock::Section s(lock_, "disaster end");
        const bool ok = fc4_dlayer_ended(core_); Sync(); return ok;
    }
    bool IsADisasterOngoing() override { return fc4_dlayer_ongoing(core_) != 0; }
    bool GoToNextDisaster() override {
        fc4::CoreLock::Section s(lock_, "disaster camera"); return fc4_dlayer_go_to_next(core_);
    }
    bool GetMostRecentDisasterLocation(cISC4DisasterInstance* p, int32_t& x, int32_t& z) override {
        return fc4_dlayer_most_recent(core_, Token(p), &x, &z);
    }
    bool RegisterDisasterLocation(cISC4DisasterInstance* p, int32_t x, int32_t z, cISC4Occupant* o,
                                  int32_t zoom, bool focus) override {
        fc4::CoreLock::Section s(lock_, "disaster register");
        return fc4_dlayer_register(core_, Token(p), x, z, Token(o), zoom, focus);
    }
    bool UnregisterDisasterLocations(cISC4DisasterInstance* p) override { return fc4_dlayer_unregister(core_, Token(p)); }
    bool UnregisterDisasterLocationByOccupant(cISC4DisasterInstance* p, cISC4Occupant* o) override {
        return fc4_dlayer_unregister_by_occupant(core_, Token(p), Token(o));
    }
    int32_t GetDisasterLocationCount() override { return fc4_dlayer_location_count(core_); }
    bool GetDisasterLocationByIndex(int32_t n, cISC4DisasterInstance** p, int32_t& x, int32_t& z) override {
        fc4_dlayer_location l{};
        if (!fc4_dlayer_location_at(core_, n, &l)) return false;
        if (p) { *p = Ptr<cISC4DisasterInstance>(l.instance); if (*p) (*p)->AddRef(); }
        x = l.x; z = l.z; return true;
    }
    const void* GetDisasterCost() override { return &record_; }
    bool DoMessage(cIGZMessage2* message) override;
    void Subscribe(bool on);
    void Sync();
    fc4::DisasterHostContext ctx_;
    fc4_disasters_host live_{};
    fc4::CoreLock lock_;
private:
    std::atomic<uint32_t> refs_{0};
    std::map<uint32_t, cISC4DisasterManager*> managers_;
    fc4_dlayer* core_ = nullptr;
    CostRecord record_;
    bool subscribed_ = false;
};

Layer* Self(void* p) { return static_cast<Layer*>(p); }
void Post(Layer* s, uint32_t type, uint32_t count = 0, int32_t x = 0, int32_t z = 0, uint64_t token = 0) {
    auto* instance = Ptr<cISC4DisasterInstance>(token);
    if (instance) instance->AddRef(); // keep the instance until the deferred post has taken its reference
    s->lock_.Defer([type, count, x, z, instance] {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            auto* m = FC4MakeMessage(type, count, x, z);
            if (instance) { m->SetData4(instance->GetID()); m->SetIGZUnknown(instance); }
            ms->MessagePost(m, false); m->Release();
        }
        if (instance) instance->Release();
    });
}

Layer::Layer() {
    live_ = fc4::MakeDisasterHost(&ctx_);
    fc4_dlayer_host h{};
    h.struct_size = sizeof(h); h.user = this;
    h.now_ms = [](void* u) { auto* s = Self(u); return s->live_.now_ms(&s->ctx_); };
    h.emergency_pause = [](void* u) { auto* s = Self(u); s->lock_.Defer([s] { s->live_.emergency_pause(&s->ctx_); }); };
    h.emergency_resume = [](void* u) { auto* s = Self(u); s->lock_.Defer([s] { s->live_.emergency_resume(&s->ctx_); }); };
    h.disable_save = [](void* u) { auto* s = Self(u); s->lock_.Defer([s] { s->live_.disable_save(&s->ctx_); }); };
    h.enable_save = [](void* u) { auto* s = Self(u); s->lock_.Defer([s] { s->live_.enable_save(&s->ctx_); }); };
    h.post_started = [](void* u, uint32_t n, int32_t x, int32_t z, uint64_t i) { Post(Self(u), 0xCA1B1E64, n, x, z, i); };
    h.post_ended = [](void* u) { Post(Self(u), 0xAA1B2002); };
    h.subscribe = [](void* u, bool on) { auto* s = Self(u); s->lock_.Defer([s, on] { s->Subscribe(on); }); };
    h.add_ref = [](void* u, uint64_t o) { auto* s = Self(u); s->live_.add_ref(&s->ctx_, o); };
    h.release = [](void* u, uint64_t o) { auto* s = Self(u); s->live_.release(&s->ctx_, o); };
    h.follow_disasters = [](void*) { return true; }; // §2.7 choice: preference unavailable
    h.go_to = [](void* u, int32_t x, int32_t z, uint32_t zoom) {
        auto* s = Self(u);
        s->lock_.Defer([s, x, z, zoom] { s->live_.go_to_location(&s->ctx_, x, z, zoom); });
    };
    h.occupant_in_manager = [](void* u, uint64_t o) { auto* s = Self(u); return s->live_.occupant_in_manager(&s->ctx_, o); };
    h.occupant_cell = [](void* u, uint64_t o, int32_t* x, int32_t* z) { auto* s = Self(u); return s->live_.occupant_cell(&s->ctx_, o, x, z); };
    h.instance_location = [](void*, uint64_t i, int32_t* x, int32_t* z) {
        auto* p = Ptr<cISC4DisasterInstance>(i); return p && p->GetLocation(*x, *z);
    };
    h.dispatch_counts = [](void* u, uint32_t* fire, uint32_t* police) {
        auto* s = Self(u); auto* c = s->ctx_.city;
        if (!c || !c->GetFireProtectionSimulator() || !c->GetPoliceSimulator()) return false;
        *fire = s->live_.fire_dispatch_count(&s->ctx_); *police = s->live_.police_dispatch_count(&s->ctx_); return true;
    };
    core_ = fc4_dlayer_create(&h);
}
Layer::~Layer() { Shutdown(); fc4_dlayer_destroy(core_); fc4::DisasterHostReleaseAll(&ctx_); }
bool Layer::QueryInterface(uint32_t iid, void** out) {
    if (!out) return false;
    if (iid == 1 || iid == 0x29B02A2C) *out = static_cast<cFC4ISC4DisasterLayer*>(this);
    else if (iid == 0x452294AA) *out = static_cast<cIGZMessageTarget2*>(this);
    else return false;
    AddRef(); return true;
}
bool Layer::Init() {
    if (fc4_dlayer_initialised(core_)) return true;
    auto* fw = RZGetFrameWork();
    auto* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4) {
        FC4_ACQUIRED(sc4, "QueryInterface"); ctx_.city = sc4->GetCity(); FC4_RELEASE(sc4);
    }
    // §2.8: keep native managers. Native instances recover their concrete
    // manager by subtracting eight from the interface pointer returned by slot 7.
    constexpr uint32_t classes[] = {0xA9B5AB9F, 0x69C1718E, 0x0A086BD3, 0x4A132FDF, 0x4A38206C, 0x6A7B3E9B, 0xEBFB0402};
    auto* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    for (uint32_t cls : classes) {
        cISC4DisasterManager* m = nullptr;
        if (com && com->GetClassObject(cls, 0x021E9CBF, reinterpret_cast<void**>(&m)) && m) {
            FC4_ACQUIRED(m, "GetClassObject"); managers_.emplace(m->GetID(), m);
        }
    }
    // Initialise the core first: managers may re-enter the layer on Init.
    { fc4::CoreLock::Section s(lock_, "disaster init"); fc4_dlayer_init(core_); }
    for (auto& item : managers_) item.second->Init();
    fc4_dlayer_reset_cost(core_); Sync();
    fc4::Log("disasters replacement: Init (%u native managers)", static_cast<unsigned>(managers_.size()));
    return core_ != nullptr;
}
bool Layer::Shutdown() {
    if (!fc4_dlayer_initialised(core_)) return true;
    // Do not hold the Rust state while native Shutdown calls slots 9 and 14.
    for (auto& item : managers_) item.second->Shutdown();
    for (auto& item : managers_) FC4_RELEASE_CREATED(item.second);
    managers_.clear();
    { fc4::CoreLock::Section s(lock_, "disaster shutdown"); fc4_dlayer_shutdown(core_); }
    ctx_.city = nullptr;
    fc4::Log("disasters replacement: Shutdown refused=%u", fc4_dlayer_refused(core_));
    return true;
}
void Layer::Subscribe(bool on) {
    if (on == subscribed_) return;
    cIGZMessageServer2Ptr ms;
    if (ms) for (uint32_t type : kMessages) {
        if (on) ms->AddNotification(this, type); else ms->RemoveNotification(this, type);
    }
    subscribed_ = on;
}
void Layer::Sync() {
    struct fc4_dlayer_record r{};
    if (!fc4_dlayer_record(core_, &r)) return;
    record_.count = r.building_count; record_.total = r.total_cost;
    for (size_t i = 0; i < 4; ++i) { record_.kinds[i].count = r.kind_count[i]; record_.kinds[i].cost = r.kind_cost[i]; }
    record_.fire = r.fire_effectiveness; record_.police = r.police_effectiveness;
}
bool Layer::DoMessage(cIGZMessage2* message) {
    if (!message) return true;
    cIGZMessage2Standard* m = nullptr;
    if (!message->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) || !m) return true;
    FC4_ACQUIRED(m, "QueryInterface");
    {
        fc4::CoreLock::Section s(lock_, "disaster message");
        if (message->GetType() == 0x99EF1143) {
            auto* c = ctx_.city;
            // void1 is the occupant's unknown, not necessarily its occupant
            // sub-object (occupant-manager.md, removal notification).
            auto* unknown = static_cast<cIGZUnknown*>(m->GetVoid1());
            cISC4Occupant* o = nullptr;
            if (unknown && unknown->QueryInterface(GZIID_cISC4Occupant, reinterpret_cast<void**>(&o)) && o)
                FC4_ACQUIRED(o, "QueryInterface");
            if (c && c->GetEstablished() && o) {
                uint32_t flags = 0, kind = FC4_DLAYER_OCC_OTHER;
                if (o->GetType() == 0x278128A0) {
                    kind = FC4_DLAYER_OCC_BUILDING;
                    cISC4BuildingOccupant* b = nullptr;
                    if (o->QueryInterface(GZIID_cISC4BuildingOccupant, reinterpret_cast<void**>(&b)) && b) {
                        FC4_ACQUIRED(b, "QueryInterface");
                        // Profile flags are at +0x1c, disasters.md §2.5.
                        const auto& profile = b->GetBuildingProfile();
                        flags = static_cast<uint32_t>(profile.buildingCategoryFlags);
                        FC4_RELEASE(b);
                    }
                } else if (o->GetType() == 0xC772BF98) kind = FC4_DLAYER_OCC_NETWORK;
                int64_t cost = 0;
                const bool has = live_.occupant_prop_i64(&ctx_, Token(o), 0x8A1C3E72, &cost);
                fc4_dlayer_occupant_removed(core_, kind, has, cost, flags);
            }
            if (o) FC4_RELEASE(o);
        } else fc4_dlayer_dispatch(core_, message->GetType(), static_cast<int32_t>(m->GetData1()), static_cast<uint32_t>(m->GetData2()));
        Sync();
    }
    FC4_RELEASE(m); return true;
}
} // namespace

cIGZUnknown* CreateFC4DisasterLayer() { return static_cast<cFC4ISC4DisasterLayer*>(new Layer()); }
