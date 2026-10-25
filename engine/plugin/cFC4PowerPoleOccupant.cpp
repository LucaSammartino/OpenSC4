// SPDX-License-Identifier: GPL-3.0-or-later
// cSC4PowerPoleOccupant (0x09C05C6A), network-occupants.md §4, on the shared
// inherited occupant slots (FC4OccupantAdapter.h). The state, the v5 record,
// connections, StaticInit and the attachment tables live in the core
// (engine/capi/src/netocc_pole.rs over core/src/network_occupants/power_pole.rs).
// The native tool bridge and cell-vector ABI remain open (NO44). Model/Draw
// glue is still unwired; keep the live pole factory native until both are safe.
#include "cFC4PowerPoleOccupant.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4Interfaces.h"
#include "FC4OccupantAdapter.h"
#include "FC4Streams.h"
#include "compat/FC4Compat.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageTarget2.h"
#include "cIGZPersistResource.h"
#include "cIGZSerializable.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4OccupantManager.h"
#include "cISC4VisualEffect.h"
#include "cS3DVector3.h"
#include <atomic>

namespace {
constexpr uint32_t kCLSID = 0x09C05C6A;
constexpr uint32_t kIID_Pole = 0xA8C86142;
constexpr uint32_t kIID_ClassAsIID = 0xA9764C8C;
constexpr uint32_t kIID_Target = 0x452294AA;
constexpr uint32_t kIID_Line = 0xC9C05C5D;
// Private: "this pole is a FreeCity4 pole" (the non-virtual calls of §4.6).
constexpr uint32_t kIID_FC4Pole = 0x7F4C0B6A;

// cISC4PowerPoleOccupant, vtable 0x00AA9EB0 (§4.2), 19 slots.
class PoleInterface : public cIGZUnknown {
public:
    virtual cISC4Occupant* AsOccupant() = 0;                                   // 3
    virtual uint32_t GetConnectionCount() = 0;                                 // 4
    virtual bool GetLineCells(uint32_t index, void* out) = 0;                  // 5
    virtual bool BreakAllConnections(bool quiet) = 0;                          // 6
    virtual cIGZUnknown* GetConnectedPole(uint32_t index) = 0;                 // 7
    virtual bool GetCell(int32_t& x, int32_t& z) = 0;                          // 8
    virtual bool InsertLines() = 0;                                            // 9
    virtual int64_t GetCostOfDemolition() = 0;                                 // 10
    virtual bool IsDistressed() = 0;                                           // 11
    virtual bool SetDistressed(bool on) = 0;                                   // 12
    virtual bool IsPreview() = 0;                                              // 13
    virtual bool SetPreview(bool on) = 0;                                      // 14
    virtual bool Init(cIGZUnknown* exemplar, uint32_t kind) = 0;               // 15
    virtual bool GetLineCellsTo(cIGZUnknown* pole, void* out) = 0;             // 16
    virtual bool GetModelInstances(void*, void**, void*, void*, uint32_t*) = 0;  // 17
    virtual bool ChangeZoomLevel(void* context) = 0;                           // 18
};

struct PoleState {
    uint16_t removal;
    uint8_t highlight, status, flags, rect[4], distressed, preview, subscribed;
    uint32_t kind;
    float position[3];
    int32_t cell[2];
    uint32_t parent[3];
    uint32_t hasParent;
};
static_assert(sizeof(PoleState) == 52, "Rust transport layout");
struct PoleHostVt {
    void* user;
    void (*addRef)(void*, uint64_t);
    void (*release)(void*, uint64_t);
    void (*insertLine)(void*, uint64_t);
    void (*removeLine)(void*, uint64_t);
    void (*reciprocalRemove)(void*, uint64_t, uint64_t, bool);
    void (*postMessage)(void*, uint32_t, uint32_t);
    void (*subscription)(void*, uint32_t, bool);
    uint64_t (*effectCreate)(void*);
    void (*effectPlace)(void*, uint64_t, const float*);
    void (*effectStart)(void*, uint64_t);
    bool (*effectRunning)(void*, uint64_t);
    void (*effectStop)(void*, uint64_t, bool);
};
struct PoleTuning {
    uint32_t present;
    float floats[5];
    uint32_t curvePoints;
    float widths[5];
    int64_t quote;
};
static_assert(sizeof(PoleTuning) == 56, "Rust transport layout");
extern "C" {
void* fc4_netpole_create();
void fc4_netpole_destroy(void*, const PoleHostVt*);
bool fc4_netpole_state(void*, PoleState*, bool);
uint32_t fc4_netpole_connection_count(const void*);
uint64_t fc4_netpole_connected_pole(const void*, uint32_t);
bool fc4_netpole_has_connection(const void*, uint64_t);
void fc4_netpole_break_all(void*, uint64_t, bool, const PoleHostVt*);
void fc4_netpole_remove_connection(void*, uint64_t, bool, const PoleHostVt*);
void fc4_netpole_insert_lines(void*, const PoleHostVt*);
void fc4_netpole_set_distressed(void*, bool, const PoleHostVt*);
void fc4_netpole_on_message(void*, uint32_t, bool, const PoleHostVt*);
void fc4_netpole_shutdown(void*, uint64_t, const PoleHostVt*);
void fc4_netpole_init(void*, const uint32_t*, uint32_t, const PoleHostVt*);
int64_t fc4_netpole_demolition_cost(const PoleHostVt*);
void fc4_netpole_set_highlight(void*, uint8_t);
bool fc4_netpole_bbox(const void*, const float*, float*);
void fc4_netpole_tuning(const PoleTuning*);
bool fc4_netpole_write(void*, const fc4_props*, const fc4_ostream*);
bool fc4_netpole_read(void*, const fc4_istream*, const PoleHostVt*);
void fc4_netpole_props(const void*, fc4_neighbor_property_callback, void*);
}

cIGZUnknown* Ptr(uint64_t t) { return reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(t)); }
uint64_t Tok(const void* p) { return reinterpret_cast<uintptr_t>(p); }

// §4.3: the control exemplar's values, read once before the first StaticInit.
void EnsureTuning() {
    static std::once_flag once;
    std::call_once(once, [] {
        PoleTuning t{};
        fc4::props::Exemplar ex;
        if (ex.Load(0x6534284A, 0xE7E2C2DB, 0xC911E35B) && ex.Holder() != nullptr) {
            const uint32_t ids[] = {0xE9A18D3F, 0x49A1E8C4, 0x89A18FC4, 0xE9A1E8D4, 0x49A195C0};
            std::vector<float> f;
            for (int i = 0; i < 5; ++i) {
                if (fc4::props::GetF32s(ex.Holder(), ids[i], f) && !f.empty()) { t.floats[i] = f[0]; t.present |= 1u << i; }
            }
            std::vector<uint32_t> u;
            if (fc4::props::GetU32s(ex.Holder(), 0x89A1E8FB, u) && !u.empty()) { t.curvePoints = u[0]; t.present |= 1u << 5; }
            if (fc4::props::GetF32s(ex.Holder(), 0xE9AC43E7, f) && f.size() >= 5) {
                for (int i = 0; i < 5; ++i) t.widths[i] = f[i];
                t.present |= 1u << 6;
            }
            std::vector<int64_t> q;
            if (fc4::props::GetI64s(ex.Holder(), 0x0A2ECED8, q) && !q.empty()) { t.quote = q[0]; t.present |= 1u << 7; }
            fc4_netpole_tuning(&t);
        }
    });
}

cISC4OccupantManager* OccupantManager() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (!app || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || !sc4) return nullptr;
    cISC4City* city = sc4->GetCity();  // borrowed
    sc4->Release();
    return city ? city->GetOccupantManager() : nullptr;  // borrowed
}

class PowerPole;
PowerPole* FromPole(cIGZUnknown* pole);

class PowerPole final : public PoleInterface,
                        public cIGZMessageTarget2,
                        public FC4OccupantAdapter<PowerPole>,
                        public cIGZSerializable {
public:
    PowerPole() : holder_(static_cast<PoleInterface*>(this)), core_(fc4_netpole_create()) {
        vt_ = {this, &HostAddRef, &HostRelease, &HostInsertLine, &HostRemoveLine, &HostReciprocal, &HostPost,
               &HostSubscribe, &HostEffectCreate, &HostEffectPlace, &HostEffectStart, &HostEffectRunning,
               &HostEffectStop};
    }
    ~PowerPole() {
        if (State().subscribed) Subscribe(0x99EF1142, false);
        fc4_netpole_destroy(core_, &vt_);
    }
    bool Valid() const { return core_ != nullptr; }
    FC4OccupantStorage& OccStorage() { return occStorage_; }
    uint64_t Self() const { return Tok(static_cast<const PoleInterface*>(this)); }

    // ---- cIGZUnknown (§4.1) ----
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        switch (iid) {
        case kIID_Pole: case kCLSID: case kIID_ClassAsIID: case kIID_FC4Pole:
            *out = static_cast<PoleInterface*>(this); break;
        case kIID_Target: *out = static_cast<cIGZMessageTarget2*>(this); break;
        case 1: case 0x26BB767E: *out = static_cast<cISC4Occupant*>(this); break;
        case 0xE4FDA3D4: *out = static_cast<cIGZSerializable*>(this); break;
        case 0x25216283: *out = holder_.AsSdk(); break;
        default: return false;  // 0x26A68448: no model instance (NO33)
        }
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override {
        const auto n = --refs_;
        if (!n) delete this;
        return n;
    }

    // ---- cISC4PowerPoleOccupant ----
    cISC4Occupant* AsOccupant() override { return this; }
    uint32_t GetConnectionCount() override { Lock g(lock_); return fc4_netpole_connection_count(core_); }
    // SPEC-GAP: the native cell-vector output ABI is missing (NO44).
    bool GetLineCells(uint32_t, void*) override { return false; }
    bool BreakAllConnections(bool quiet) override {
        Lock g(lock_); fc4_netpole_break_all(core_, Self(), quiet, &vt_); return true;
    }
    cIGZUnknown* GetConnectedPole(uint32_t i) override { Lock g(lock_); return Ptr(fc4_netpole_connected_pole(core_, i)); }
    bool GetCell(int32_t& x, int32_t& z) override {
        Lock g(lock_); const auto s = State(); x = s.cell[0]; z = s.cell[1]; return true;
    }
    bool InsertLines() override { Lock g(lock_); fc4_netpole_insert_lines(core_, &vt_); return true; }
    int64_t GetCostOfDemolition() override { EnsureTuning(); return fc4_netpole_demolition_cost(&vt_); }
    bool IsDistressed() override { Lock g(lock_); return State().distressed != 0; }
    bool SetDistressed(bool on) override { Lock g(lock_); fc4_netpole_set_distressed(core_, on, &vt_); return true; }
    bool IsPreview() override { Lock g(lock_); return State().preview != 0; }
    bool SetPreview(bool on) override {
        Lock g(lock_); auto s = State(); s.preview = on ? 1 : 0; Put(s); return true;
    }
    bool Init(cIGZUnknown* exemplar, uint32_t kind) override {
        Lock g(lock_);
        EnsureTuning();
        if (State().status & 1) return true;
        Init();
        uint32_t key[3]{};
        cIGZPersistResource* res = nullptr;
        const bool hasKey = exemplar && exemplar->QueryInterface(GZIID_cIGZPersistResource, reinterpret_cast<void**>(&res)) && res;
        if (hasKey) { key[0] = res->GetType(); key[1] = res->GetGroup(); key[2] = res->GetInstance(); res->Release(); }
        fc4_netpole_init(core_, hasKey ? key : nullptr, kind, &vt_);
        if (hasKey) ApplyParent(key);
        return true;
    }
    bool GetLineCellsTo(cIGZUnknown*, void*) override { return false; }
    bool GetModelInstances(void*, void**, void*, void*, uint32_t* count) override {
        if (count) *count = 0;  // Model host (4.7.3) is not wired yet.
        return false;
    }
    bool ChangeZoomLevel(void*) override { return true; }

    // ---- cIGZMessageTarget2 (§4.3): the Insert of this pole inserts its lines ----
    bool DoMessage(cIGZMessage2* msg) override {
        if (!msg) return true;
        cIGZMessage2Standard* m = nullptr;
        if (!msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) || !m) return true;
        const bool isThis = m->GetVoid1() == static_cast<cISC4Occupant*>(this);
        const uint32_t type = m->GetType();
        m->Release();
        Lock g(lock_);
        fc4_netpole_on_message(core_, type, isThis, &vt_);
        return true;
    }

    // ---- cISC4Occupant (own slots) ----
    // Slot 3 (§4.3): the base Init, then the Insert subscription.
    bool Init() override {
        Lock g(lock_);
        BaseInit();
        auto s = State();
        if (!s.subscribed) { Subscribe(0x99EF1142, true); s = State(); s.subscribed = 1; Put(s); }
        return true;
    }
    bool Shutdown() override { Lock g(lock_); fc4_netpole_shutdown(core_, Self(), &vt_); return true; }
    int32_t GetType() override { return 0x2890D4DE; }
    bool GetPosition(cS3DVector3* out) override {
        if (!out) return false;
        Lock g(lock_); const auto s = State(); *out = cS3DVector3(s.position[0], s.position[1], s.position[2]);
        return true;
    }
    // SPEC-GAP: §4.7 needs the terrain's cell width and corner heights; the
    // cell uses the 16 m city cell and the floor height is not rebuilt.
    bool SetPosition(const cS3DVector3* p) override {
        if (!p) return false;
        Lock g(lock_); auto s = State();
        s.position[0] = p->fX; s.position[1] = p->fY; s.position[2] = p->fZ;
        s.cell[0] = static_cast<int32_t>(p->fX * 0.0625f); s.cell[1] = static_cast<int32_t>(p->fZ * 0.0625f);
        Put(s);
        if (effect_) HostEffectPlace(this, Tok(effect_), s.position);
        return true;
    }
    // §4.7.2: the cells of the pole and of its wired records. The cell size
    // is the city's 16 m (the native statics come from city slots 91, 92).
    cS3DVector3* GetBoundingBox(cS3DVector3* a, cS3DVector3* b) override {
        Lock g(lock_);
        const float cell[2] = {16.0f, 16.0f};
        float box[6]{};
        fc4_netpole_bbox(core_, cell, box);
        if (a) *a = cS3DVector3(box[0], box[1], box[2]);
        if (b) *b = cS3DVector3(box[3], box[4], box[5]);
        return a;
    }
    bool GetBoundingCityCells(SC4Rect<long>& out) override {
        Lock g(lock_); PointCells(State().position, out); return true;
    }
    // Occupant slot 17 (§4.2 quirk): the draw attributes only; GetHighlight stays.
    bool SetHighlight(uint32_t h, bool) override {
        Lock g(lock_); fc4_netpole_set_highlight(core_, static_cast<uint8_t>(h)); return true;
    }

    // ---- cIGZSerializable (§4.4) ----
    bool Write(cIGZOStream& stream) override {
        Lock g(lock_);
        fc4::OStreamAdapter out(stream);
        auto* props = holder_.ToCore();
        const bool ok = fc4_netpole_write(core_, props, out.Get());
        fc4_props_destroy(props);
        return ok;
    }
    bool Read(cIGZIStream& stream) override {
        Lock g(lock_);
        EnsureTuning();
        fc4::IStreamAdapter input(stream);
        const bool ok = fc4_netpole_read(core_, input.Get(), &vt_);
        holder_.Clear();
        fc4_netpole_props(core_, &cFC4PropertyHolder::FromCoreProperty, &holder_);
        const auto s = State();
        ApplyParent(s.hasParent ? s.parent : nullptr);
        return ok;
    }
    uint32_t GetGZCLSID() override { return kCLSID; }

    // §4.6 RemoveConnection(pole, quiet), the non-virtual call.
    void RemoveConnectionTo(uint64_t pole, bool quiet) {
        Lock g(lock_); fc4_netpole_remove_connection(core_, pole, quiet, &vt_);
    }

    // ---- FC4OccupantAdapter hooks ----
    std::recursive_mutex& OccLock() { return lock_; }
    cFC4PropertyHolder& OccHolder() { return holder_; }
    bool OccGet(FC4OccBase& b) {
        const auto s = State();
        b.removal = s.removal; b.highlight = s.highlight; b.status = s.status; b.flags = s.flags;
        std::memcpy(b.rect, s.rect, 4);
        return true;
    }
    void OccPut(const FC4OccBase& b) {
        auto s = State();
        s.removal = b.removal; s.highlight = b.highlight; s.status = b.status; s.flags = b.flags;
        std::memcpy(s.rect, b.rect, 4);
        Put(s);
    }

private:
    using Lock = std::lock_guard<std::recursive_mutex>;
    PoleState State() { PoleState s{}; fc4_netpole_state(core_, &s, false); return s; }
    void Put(PoleState& s) { fc4_netpole_state(core_, &s, true); }
    void Subscribe(uint32_t type, bool on) {
        cIGZMessageServer2Ptr ms;
        if (!ms) return;
        if (on) ms->AddNotification(static_cast<cIGZMessageTarget2*>(this), type);
        else ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(this), type);
    }

    static PowerPole* From(void* u) { return static_cast<PowerPole*>(u); }
    static void HostAddRef(void*, uint64_t t) { if (auto* p = Ptr(t)) FC4_ADDREF(p); }
    static void HostRelease(void* u, uint64_t t) {
        if (auto* p = Ptr(t)) {
            if (p == From(u)->effect_) From(u)->effect_ = nullptr;
            FC4_RELEASE(p);
        }
    }
    static cISC4Occupant* AsOcc(uint64_t t) {
        cISC4Occupant* occ = nullptr;
        if (auto* p = Ptr(t)) p->QueryInterface(0x26BB767E, reinterpret_cast<void**>(&occ));
        return occ;
    }
    static void HostInsertLine(void*, uint64_t t) {
        cISC4OccupantManager* om = OccupantManager();
        if (cISC4Occupant* occ = AsOcc(t)) { if (om) om->InsertOccupant(occ, 0); occ->Release(); }
    }
    static void HostRemoveLine(void*, uint64_t t) {
        cISC4OccupantManager* om = OccupantManager();
        if (cISC4Occupant* occ = AsOcc(t)) { if (om) om->RemoveOccupant(occ, true, 0); occ->Release(); }
    }
    static void HostReciprocal(void*, uint64_t other, uint64_t self, bool quiet);
    static void HostPost(void* u, uint32_t m, uint32_t d) { From(u)->PostOccupantMessage(m, d); }
    static void HostSubscribe(void* u, uint32_t m, bool on) { From(u)->Subscribe(m, on); }
    static uint64_t HostEffectCreate(void* u) {
        cISC4AppPtr app;
        auto* city = app ? app->GetCity() : nullptr;
        auto* manager = city ? city->GetEffectsManager() : nullptr; // borrowed
        cISC4VisualEffect* effect = nullptr;
        if (!manager || !reinterpret_cast<cFC4ISC4EffectsManager*>(manager)->CreateVisualEffectByName(
                            "powerpolezap", &effect) || !effect) return 0;
        FC4_ACQUIRED(effect);
        From(u)->effect_ = reinterpret_cast<cIGZUnknown*>(effect); // borrowed from the core's owned reference
        return Tok(effect);
    }
    static cFC4ISC4VisualEffect* Effect(uint64_t t) {
        return reinterpret_cast<cFC4ISC4VisualEffect*>(static_cast<uintptr_t>(t));
    }
    static void HostEffectPlace(void*, uint64_t t, const float* position) {
        // network-occupants 4.1/4.7: counter, mask, padding, matrix, translation, scale.
        struct Transform {
            uint8_t counter = 1, mask = 1, padding[2]{};
            float matrix[9] = {1,0,0,0,1,0,0,0,1};
            float translation[3]{};
            float scale = 1;
        } transform;
        static_assert(sizeof(Transform) == 0x38, "S3DTransform");
        std::memcpy(transform.translation, position, sizeof(transform.translation));
        Effect(t)->SetTransform(reinterpret_cast<const cS3DTransform&>(transform));
    }
    static void HostEffectStart(void*, uint64_t t) { Effect(t)->Start(0); }
    static bool HostEffectRunning(void*, uint64_t t) { return Effect(t)->IsStarted(); }
    static void HostEffectStop(void*, uint64_t t, bool immediate) { Effect(t)->Stop(immediate ? 1u : 0u); }

    std::atomic<uint32_t> refs_{0};
    std::recursive_mutex lock_;
    FC4OccupantStorage occStorage_;
    cFC4PropertyHolder holder_;
    void* core_;
    PoleHostVt vt_{};
    cIGZUnknown* effect_ = nullptr;
};

PowerPole* FromPole(cIGZUnknown* pole) {
    void* p = nullptr;
    if (!pole || !pole->QueryInterface(kIID_FC4Pole, &p) || !p) return nullptr;
    auto* ours = static_cast<PowerPole*>(static_cast<PoleInterface*>(p));
    ours->Release();  // borrowed: the caller holds `pole`
    return ours;
}

// The other pole's RemoveConnection(this, quiet): ours through the core, the
// game's through its non-virtual body (docs/abi/direct-calls.md).
using NativeRemoveFn = void(__fastcall*)(void* self, void* edx, void* other, uint32_t flag);
void PowerPole::HostReciprocal(void*, uint64_t other, uint64_t self, bool quiet) {
    cIGZUnknown* o = Ptr(other);
    if (!o) return;
    if (PowerPole* ours = FromPole(o)) { ours->RemoveConnectionTo(self, quiet); return; }
    const uintptr_t va = fc4::compat::DirectCall(fc4::compat::Direct::PoleRemoveConnection);
    void* base = nullptr;
    if (va == 0 || !o->QueryInterface(kIID_Pole, &base) || !base) {
        fc4::Log("netocc: pole %p: RemoveConnection on a game pole %p skipped", Ptr(self), o);
        return;
    }
    reinterpret_cast<NativeRemoveFn>(va)(base, nullptr, Ptr(self), quiet ? 1u : 0u);
    static_cast<cIGZUnknown*>(base)->Release();
}
}  // namespace

cIGZUnknown* CreateFC4PowerPoleOccupant() {
    auto* p = new PowerPole;
    if (!p->Valid()) { delete p; return nullptr; }
    return static_cast<PoleInterface*>(p);
}

bool FC4NetOccRemovePoleConnection(cIGZUnknown* first, cIGZUnknown* second, bool quiet) {
    PowerPole* ours = FromPole(first);
    if (!ours) return false;
    void* other = nullptr;
    uint64_t token = 0;
    if (second && second->QueryInterface(kIID_Pole, &other) && other) {
        token = Tok(other);
        static_cast<cIGZUnknown*>(other)->Release();
    }
    ours->RemoveConnectionTo(token, quiet);
    return true;
}
