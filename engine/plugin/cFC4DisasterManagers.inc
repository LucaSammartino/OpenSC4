// SPDX-License-Identifier: GPL-3.0-or-later
// Included in the layer's private namespace. The public layer remains the
// authority for pause/save, assessment and native-format period messages.
constexpr uint32_t kManagerTypes[] = {
    0xC4C90997, 0x621CDA33, 0x42963812, 0x4A1332E8, 0xCA380B06, 0x6A7B4397, 0xEBFB0404
};
class Instance;
class Manager;

class Runtime final : public cIGZMessageTarget2, public std::enable_shared_from_this<Runtime> {
public:
    explicit Runtime(Layer* owner);
    ~Runtime();
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        *out = nullptr;
        if (iid != 1 && iid != 0x452294AA) return false;
        *out = static_cast<cIGZMessageTarget2*>(this); AddRef(); return true;
    }
    // Owned by the layer/session, not by notifications; these references
    // are balanced before the layer drops its shared ownership.
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }
    bool DoMessage(cIGZMessage2*) override;
    void Init();
    void Shutdown();
    void Sync();
    cISC4DisasterInstance* Wrap(uint32_t id, uint32_t type = 0);
    template<class F> auto Call(F f) -> decltype(f()) {
        fc4::CoreLock::Section guard(lock, "disaster manager core");
        auto result = f();
        Sync();
        lock.Defer([this] { fc4::DisasterHostReleaseHeld(&ctx); });
        return result;
    }
    template<class F> void Write(F f) { Call([&] { f(); return true; }); }
    Layer* layer;
    fc4::DisasterHostContext ctx;
    fc4_disasters_host host{};
    fc4_disasters* core = nullptr;
    fc4::CoreLock lock;
    std::map<uint32_t, Manager*> managers; // borrowed; layer owns them
private:
    std::atomic<uint32_t> refs{0};
    std::map<uint32_t, Instance*> instances; // one owned reference per identity
    std::set<uint32_t> notifications;
    std::set<uint32_t> ticking;
    std::map<uint32_t, std::vector<std::tuple<uint64_t, int32_t, int32_t, uint32_t>>> locations;
    bool initialised = false;
};

std::mutex gRuntimeMutex;
std::weak_ptr<Runtime> gRuntime;
void SetRuntime(const std::shared_ptr<Runtime>& r) {
    std::lock_guard<std::mutex> guard(gRuntimeMutex); gRuntime = r;
}
std::shared_ptr<Runtime> CurrentRuntime() {
    std::lock_guard<std::mutex> guard(gRuntimeMutex);
    auto r = gRuntime.lock();
    if (!r) { r = std::make_shared<Runtime>(nullptr); gRuntime = r; }
    return r;
}

class Instance final : public cISC4DisasterInstance, public cISC4DisasterInstanceFire,
                       public cIGZMessageTarget2 {
public:
    Instance(std::shared_ptr<Runtime> r, uint32_t t, uint32_t i) : runtime(std::move(r)), type(t), id(i) {}
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        *out = nullptr;
        if (iid == 1) *out = static_cast<cISC4DisasterInstance*>(this);
        else if (iid == 0x452294AA) *out = static_cast<cIGZMessageTarget2*>(this);
        // SPEC-GAP: no numeric IID for the generic instance is specified;
        // factories expose IUnknown, and typed instances are returned by managers.
        else return false;
        AddRef(); return true;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { auto n = --refs; if (!n) delete this; return n; }
    bool Shutdown() override {
        runtime->Write([&] { if (id) fc4_disasters_instance_shutdown(runtime->core, id); }); return true;
    }
    cISC4DisasterManager* GetManager() override;
    uint32_t GetID() override { return type; }
    bool GetLocation(int32_t& x, int32_t& z) override {
        return runtime->Call([&] {
            fc4_disaster_instance info{};
            if (!id || !fc4_disasters_get_instance(runtime->core, id, &info) || !info.has_location) return false;
            x = info.x; z = info.z; return true;
        });
    }
    int32_t GetPhase() override {
        return runtime->Call([&] {
            fc4_disaster_instance info{};
            return id && fc4_disasters_get_instance(runtime->core, id, &info) ? static_cast<int32_t>(info.phase) : 0;
        });
    }
    bool Init() override {
        runtime->Init();
        // The public fire interface's Init is the lazy fire creation path.
        if (type == 0x621CDA33 && !id)
            id = runtime->Call([&] { return fc4_disasters_fire_instance(runtime->core); });
        return id != 0;
    }
    bool Update() override { runtime->Write([&] { fc4_disasters_fire_update(runtime->core); }); return true; }
    bool Ignite(cISC4Occupant* o, bool force, uint32_t fire, bool focus) override {
        return runtime->Call([&] { return fc4_disasters_ignite(runtime->core, Token(o), force, fire, focus); });
    }
    bool IgniteCell(uint32_t x, uint32_t z, bool probability, bool teleport, uint32_t fire) override {
        return runtime->Call([&] { return fc4_disasters_ignite_cell(runtime->core, x, z, probability, teleport, fire); });
    }
    bool ExtinguishCell(uint32_t x, uint32_t z, uint32_t amount) override {
        runtime->Write([&] { fc4_disasters_extinguish_cell(runtime->core, x, z, amount); }); return true;
    }
    bool QuickBurnCell(uint32_t x, uint32_t z) override {
        runtime->Write([&] { fc4_disasters_quick_burn_cell(runtime->core, x, z); }); return true;
    }
    bool IgniteCellRadius(uint32_t x, uint32_t z, float rx, float rz) override {
        runtime->Write([&] { fc4_disasters_ignite_cell_radius(runtime->core, x, z, static_cast<int32_t>(rx), static_cast<int32_t>(rz)); });
        return true;
    }
    bool Extinguish(cISC4Occupant* o) override {
        runtime->Write([&] { fc4_disasters_extinguish(runtime->core, Token(o)); }); return true;
    }
    bool BurnDown(cISC4Occupant* o) override {
        runtime->Write([&] { fc4_disasters_burn_down(runtime->core, Token(o)); }); return true;
    }
    int32_t SetCause(uint32_t cause) override {
        return runtime->Call([&] { return static_cast<int32_t>(fc4_disasters_set_cause(runtime->core, cause)); });
    }
    int32_t GetCause() override {
        return runtime->Call([&] { return static_cast<int32_t>(fc4_disasters_cause(runtime->core)); });
    }
    int32_t GetFireCountInCellRect(SC4Rect<long>& rect) override {
        const auto* v = reinterpret_cast<const int32_t*>(&rect);
        fc4_cell_rect r{v[0], v[1], v[2], v[3]};
        return runtime->Call([&] { return static_cast<int32_t>(fc4_disasters_fire_count_in_rect(runtime->core, &r)); });
    }
    int32_t GetFireCount() override {
        return runtime->Call([&] { return static_cast<int32_t>(fc4_disasters_fire_count(runtime->core)); });
    }
    bool DoMessage(cIGZMessage2* message) override {
        if (message && message->GetType() == 0x86AD10EE) {
            cIGZMessage2Standard* m = nullptr;
            if (message->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) && m) {
                FC4_ACQUIRED(m, "QueryInterface");
                const auto us = static_cast<uint32_t>(m->GetData2());
                runtime->Write([&] { fc4_disasters_tick(runtime->core, id, us); });
                FC4_RELEASE(m);
            }
        }
        return true;
    }
    std::shared_ptr<Runtime> runtime;
    uint32_t type, id;
private:
    std::atomic<uint32_t> refs{0};
};

class Manager final : public cFC4ISC4DisasterManager, public cISC4DisasterManagerFire {
public:
    Manager(std::shared_ptr<Runtime> r, uint32_t t) : runtime(std::move(r)), type(t) {
        fc4::CoreLock::Section guard(runtime->lock, "disaster manager create");
        if (!runtime->managers.count(type)) runtime->managers[type] = this;
    }
    ~Manager() {
        Shutdown();
        fc4::CoreLock::Section guard(runtime->lock, "disaster manager destroy");
        const auto it = runtime->managers.find(type);
        if (it != runtime->managers.end() && it->second == this) runtime->managers.erase(it);
        if (runtime->managers.empty() && !runtime->layer) runtime->Shutdown();
    }
    cISC4DisasterManager* AsNative() { return reinterpret_cast<cISC4DisasterManager*>(static_cast<cFC4ISC4DisasterManager*>(this)); }
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        *out = nullptr;
        if (iid == 1 || iid == 0x021E9CBF) *out = static_cast<cFC4ISC4DisasterManager*>(this);
        else if (iid == 0x49B43F57 && type == 0x621CDA33) *out = static_cast<cISC4DisasterManagerFire*>(this);
        else return false;
        AddRef(); return true;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { auto n = --refs; if (!n) delete this; return n; }
    bool Init() override { runtime->Init(); return true; }
    bool Shutdown() override {
        fc4::CoreLock::Section guard(runtime->lock, "disaster manager shutdown");
        runtime->Write([&] {
            const size_t n = fc4_disasters_manager_instances(runtime->core, type, nullptr, 0);
            std::vector<uint32_t> ids(n);
            fc4_disasters_manager_instances(runtime->core, type, ids.data(), n);
            for (auto id : ids) fc4_disasters_instance_shutdown(runtime->core, id);
        });
        // Snapshot before callbacks: instance Shutdown may edit the list.
        auto list = extra;
        extra.clear();
        for (auto* p : list) { p->Shutdown(); p->Release(); }
        return true;
    }
    uint32_t GetID() override { return type; }
    cISC4DisasterInstance* AttemptDisasterStart(int32_t x, int32_t z) override {
        Init();
        const auto id = runtime->Call([&] { return fc4_disasters_attempt_start(runtime->core, type, x, z); });
        return runtime->Wrap(id);
    }
    cISC4DisasterInstance* AttemptDisasterStart() override {
        Init();
        const auto id = runtime->Call([&] { return fc4_disasters_attempt_start_any(runtime->core, type); });
        return runtime->Wrap(id);
    }
    bool AddInstanceToList(cISC4DisasterInstance* p) override {
        if (!p) return false;
        fc4::CoreLock::Section guard(runtime->lock, "disaster list add");
        if (std::find(extra.begin(), extra.end(), p) == extra.end()) { p->AddRef(); extra.push_back(p); }
        return true;
    }
    bool RemoveInstanceFromList(cISC4DisasterInstance* p) override {
        fc4::CoreLock::Section guard(runtime->lock, "disaster list remove");
        const auto it = std::find(extra.begin(), extra.end(), p);
        if (it != extra.end()) { extra.erase(it); p->Release(); }
        return true;
    }
    int32_t GetActiveInstanceCount() override {
        return runtime->Call([&] {
            auto n = fc4_disasters_active_count(runtime->core, type);
            for (auto* p : extra) if (p->GetPhase() == 1) ++n;
            return static_cast<int32_t>(n);
        });
    }
    const SC4List<cISC4DisasterInstance*>* GetInstances() override {
        fc4::CoreLock::Section guard(runtime->lock, "disaster instance list");
        if (!list) list = std::make_unique<SC4List<cISC4DisasterInstance*>>();
        while (list->begin() != list->end()) list->erase(list->begin());
        const size_t n = fc4_disasters_manager_instances(runtime->core, type, nullptr, 0);
        std::vector<uint32_t> ids(n);
        fc4_disasters_manager_instances(runtime->core, type, ids.data(), n);
        for (auto id : ids) { auto* p = runtime->Wrap(id); list->push_back(p); }
        for (auto* p : extra) list->push_back(p);
        return list.get();
    }
    cISC4DisasterManager* AsISC4DisasterManager() override { return AsNative(); }
    bool IgniteOccupant(cISC4Occupant* o) override {
        Init(); return runtime->Call([&] { return fc4_disasters_ignite_occupant(runtime->core, Token(o)); });
    }
    bool IgniteCell(uint32_t x, uint32_t z, bool probability, bool teleport, uint32_t fire) override {
        Init(); return runtime->Call([&] { return fc4_disasters_manager_ignite_cell(runtime->core, x, z, probability, teleport, fire); });
    }
    bool ExtinguishCell(uint32_t x, uint32_t z, uint32_t amount) override {
        runtime->Write([&] { fc4_disasters_extinguish_cell(runtime->core, x, z, amount); }); return true;
    }
    bool QuickBurnCell(uint32_t x, uint32_t z) override {
        runtime->Write([&] { fc4_disasters_quick_burn_cell(runtime->core, x, z); }); return true;
    }
    bool IgniteCellRadius(uint32_t x, uint32_t z, float rx, float rz) override {
        Init(); runtime->Write([&] { fc4_disasters_ignite_cell_radius(runtime->core, x, z, static_cast<int32_t>(rx), static_cast<int32_t>(rz)); });
        return true;
    }
    int32_t SetCause(uint32_t cause) override {
        Init(); return runtime->Call([&] { return static_cast<int32_t>(fc4_disasters_set_cause(runtime->core, cause)); });
    }
    int32_t GetCause() override {
        return runtime->Call([&] { return static_cast<int32_t>(fc4_disasters_cause(runtime->core)); });
    }
    cISC4DisasterInstanceFire* GetFireInstance() override {
        Init();
        const auto id = runtime->Call([&] { return fc4_disasters_fire_instance(runtime->core); });
        return static_cast<cISC4DisasterInstanceFire*>(static_cast<Instance*>(runtime->Wrap(id)));
    }
    std::shared_ptr<Runtime> runtime;
    uint32_t type;
private:
    std::atomic<uint32_t> refs{0};
    std::vector<cISC4DisasterInstance*> extra;
    std::unique_ptr<SC4List<cISC4DisasterInstance*>> list;
};

cISC4DisasterManager* Instance::GetManager() {
    fc4::CoreLock::Section guard(runtime->lock, "disaster instance manager");
    auto it = runtime->managers.find(type);
    return it == runtime->managers.end() ? nullptr : it->second->AsNative();
}

Runtime::Runtime(Layer* owner) : layer(owner) {
    host = fc4::MakeDisasterHost(&ctx);
    // The public layer owns the emergency nesting and assessment period.
    host.emergency_pause = nullptr; host.emergency_resume = nullptr;
    host.disable_save = nullptr; host.enable_save = nullptr;
    ctx.postOverride = [this](const fc4_disaster_post& post) {
        lock.Defer([this, post] {
            if (post.msg_type == 0xCA1B1E64) {
                if (layer) layer->DisasterHasBegun(Wrap(post.void1));
            } else if (post.msg_type == 0xAA1B2002) {
                if (layer) layer->DisasterHasEnded();
            } else {
                // Use the host's normal post mapper after leaving the core.
                auto override = std::move(ctx.postOverride);
                ctx.postOverride = nullptr;
                host.post(&ctx, &post);
                ctx.postOverride = std::move(override);
            }
        });
    };
    core = fc4_disasters_create(&host, static_cast<uint32_t>(GetTickCount()));
}
Runtime::~Runtime() {
    Shutdown(); fc4_disasters_destroy(core); fc4::DisasterHostReleaseAll(&ctx);
}
void Runtime::Init() {
    fc4::CoreLock::Section guard(lock, "disaster runtime init");
    if (initialised) return;
    initialised = true;
    fc4::DisasterHostLoadTuning(core);
    fc4_disasters_init(core);
    Sync();
}
void Runtime::Shutdown() {
    if (!initialised) return;
    Write([&] { fc4_disasters_shutdown(core); });
    initialised = false;
    auto owned = std::move(instances);
    instances.clear();
    for (auto& item : owned) item.second->Release();
    ctx.city = nullptr;
}
cISC4DisasterInstance* Runtime::Wrap(uint32_t id, uint32_t type) {
    if (!id) return nullptr;
    fc4::CoreLock::Section guard(lock, "disaster instance identity");
    auto it = instances.find(id);
    if (it != instances.end()) return static_cast<cISC4DisasterInstance*>(it->second);
    fc4_disaster_instance info{};
    if (!fc4_disasters_get_instance(core, id, &info) && !type) return nullptr;
    auto* p = new Instance(shared_from_this(), type ? type : info.manager_type, id);
    p->AddRef(); instances.emplace(id, p);
    return static_cast<cISC4DisasterInstance*>(p);
}

void Runtime::Sync() {
    std::set<uint32_t> wanted;
    for (const auto& sub : ctx.subscriptions) {
        if (std::get<0>(sub) != FC4_DISASTER_TARGET_LAYER && std::get<2>(sub) != 0x86AD10EE)
            wanted.insert(std::get<2>(sub));
    }
    auto before = notifications;
    notifications = wanted;
    if (before != wanted) lock.Defer([this, before, wanted] {
        cIGZMessageServer2Ptr ms;
        if (!ms) return;
        for (auto type : before) if (!wanted.count(type)) ms->RemoveNotification(this, type);
        for (auto type : wanted) if (!before.count(type)) ms->AddNotification(this, type);
    });
    auto tickBefore = ticking;
    ticking = ctx.ticks;
    const auto tickAfter = ticking;
    // Keep wrapper identities for ended instances until session shutdown.
    // A native consumer may still hold a reference after list removal.
    for (auto id : tickAfter) Wrap(id);
    if (tickBefore != tickAfter) lock.Defer([this, tickBefore, tickAfter] {
        auto* fw = RZGetFrameWork(); void* service = nullptr;
        if (!fw || !fw->GetSystemService(kFC4_SID_AnimationTickManager, kFC4_IID_AnimationTickManager, &service) || !service) return;
        auto* tick = static_cast<cFC4ISC4AnimationTickManager*>(service);
        FC4_ACQUIRED(tick, "GetSystemService");
        for (auto id : tickBefore) if (!tickAfter.count(id)) {
            auto it = instances.find(id);
            if (it != instances.end()) tick->RemoveTickableObject(it->second, it->second->type == 0xEBFB0404 ? 0xFF : 0xC0);
        }
        for (auto id : tickAfter) if (!tickBefore.count(id)) {
            auto it = instances.find(id);
            if (it != instances.end()) tick->AddTickableObject(it->second, it->second->type == 0xEBFB0404 ? 0xFF : 0xC0);
        }
        FC4_RELEASE(tick);
    });
    decltype(locations) next;
    fc4_disaster_location loc{};
    for (size_t n = 0; fc4_disasters_get_location(core, n, &loc); ++n)
        next[loc.instance].emplace_back(loc.occupant, loc.x, loc.z, loc.zoom);
    if (next == locations) return;
    auto old = locations;
    locations = next;
    for (const auto& item : next) Wrap(item.first);
    lock.Defer([this, old, next] {
        if (!layer) return;
        for (const auto& item : old) {
            auto it = next.find(item.first);
            if (it == next.end() || it->second != item.second) layer->UnregisterDisasterLocations(Wrap(item.first));
        }
        for (const auto& item : next) {
            auto it = old.find(item.first);
            if (it != old.end() && it->second == item.second) continue;
            for (const auto& l : item.second)
                layer->RegisterDisasterLocation(Wrap(item.first), std::get<1>(l), std::get<2>(l),
                    Ptr<cISC4Occupant>(std::get<0>(l)), std::get<3>(l), false);
        }
    });
}
bool Runtime::DoMessage(cIGZMessage2* message) {
    if (!message) return true;
    cIGZMessage2Standard* m = nullptr;
    if (!message->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) || !m) return true;
    FC4_ACQUIRED(m, "QueryInterface");
    fc4_disaster_message input{};
    input.msg_type = message->GetType(); input.data1 = m->GetData1();
    input.data2 = m->GetData2(); input.data3 = m->GetData3(); input.void1 = Token(m->GetVoid1());
    Write([&] {
        const auto subscriptions = ctx.subscriptions;
        for (const auto& sub : subscriptions)
            if (std::get<0>(sub) != FC4_DISASTER_TARGET_LAYER && std::get<2>(sub) == input.msg_type)
                fc4_disasters_do_message(core, std::get<0>(sub), std::get<1>(sub), &input);
    });
    FC4_RELEASE(m); return true;
}
