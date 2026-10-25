// SPDX-License-Identifier: GPL-3.0-or-later
// network-occupants.md 5; occupant-manager.md 13.1 (inherited slots: FC4OccupantAdapter.h).
// No city access in Read.
#include "cFC4PowerLineOccupant.h"
#include "FC4OccupantAdapter.h"
#include "FC4PropertyHolder.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"
#include "cIGZMessageServer2.h"
#include "GZServPtrs.h"
#include "cIGZSerializable.h"
#include "cISC4Occupant.h"
#include "cS3DVector3.h"
#include <algorithm>
#include <atomic>
#include <cmath>
#include <cstring>
#include <mutex>
#include <vector>

namespace {
// The SDK's first GetConnectedPoles output is incorrectly typed as a line.
class LineInterface : public cIGZUnknown {
public:
    virtual bool GetConnectedPoles(cIGZUnknown*&, cIGZUnknown*&, bool&) = 0;
    virtual bool SetConnectedPoles(cIGZUnknown*, cIGZUnknown*, bool) = 0;
    virtual bool GetCell(uint32_t&, uint32_t&) = 0;
    virtual bool IsPreview() = 0;
    virtual bool SetPreview(bool) = 0;
};
struct LineState {
    uint16_t removal;
    uint8_t highlight, status, flags, rect[4], preview, poleFlag;
    float position[3];
    int32_t cell[2];
    uint64_t poles[2];
};
static_assert(sizeof(LineState) == 48, "Rust transport layout");
extern "C" {
void* fc4_netline_create();
void fc4_netline_destroy(void*);
bool fc4_netline_state(void*, LineState*, bool);
bool fc4_netline_position(void*, const float*);
bool fc4_netline_write(void*, const fc4_props*, const fc4_ostream*);
bool fc4_netline_read(void*, const fc4_istream*);
void fc4_netline_props(const void*, fc4_neighbor_property_callback, void*);
bool fc4_netline_parent(const void*, uint32_t*);
}

class PowerLine final : public LineInterface, public FC4OccupantAdapter<PowerLine>, public cIGZSerializable {
public:
    PowerLine() : holder_(static_cast<LineInterface*>(this)), core_(fc4_netline_create()) {}
    ~PowerLine() { DropPoles(State()); fc4_netline_destroy(core_); }
    bool Valid() const { return core_ != nullptr; }
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        switch (iid) {
        case 1: case 0xC98F3B24: case 0xC9C05C5D: case 0xA9764C8C:
            *out = static_cast<LineInterface*>(this); break;
        case 0x26BB767E: *out = static_cast<cISC4Occupant*>(this); break;
        case 0xE4FDA3D4: *out = static_cast<cIGZSerializable*>(this); break;
        case 0x25216283: *out = holder_.AsSdk(); break;
        default: return false;
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
    bool GetConnectedPoles(cIGZUnknown*& a, cIGZUnknown*& b, bool& flag) override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        auto s = State();
        a = Pointer(s.poles[0]); b = Pointer(s.poles[1]); flag = s.poleFlag != 0;
        if (a) FC4_ADDREF_FOR_CALLER(a);
        if (b) FC4_ADDREF_FOR_CALLER(b);
        return true;
    }
    bool SetConnectedPoles(cIGZUnknown* a, cIGZUnknown* b, bool flag) override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        if (a) FC4_ADDREF(a);
        if (b) FC4_ADDREF(b);
        auto s = State();
        const auto old = s;
        s.poles[0] = Token(a); s.poles[1] = Token(b); s.poleFlag = flag;
        Put(s); DropPoles(old);
        return true;
    }
    bool GetCell(uint32_t& x, uint32_t& z) override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        auto s = State(); x = s.cell[0]; z = s.cell[1]; return true;
    }
    bool IsPreview() override {
        std::lock_guard<std::recursive_mutex> g(lock_); return State().preview != 0;
    }
    bool SetPreview(bool b) override {
        std::lock_guard<std::recursive_mutex> g(lock_); auto s = State(); s.preview = b; Put(s); return true;
    }
    // Slot 4 (§5.1): drop the poles, then the base Shutdown.
    bool Shutdown() override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        SetConnectedPoles(nullptr, nullptr, false);
        return BaseShutdown();
    }
    int32_t GetType() override { return 0x0990B2EA; }
    bool GetPosition(cS3DVector3* out) override {
        if (!out) return false;
        std::lock_guard<std::recursive_mutex> g(lock_);
        auto s = State(); *out = cS3DVector3(s.position[0], s.position[1], s.position[2]); return true;
    }
    bool SetPosition(const cS3DVector3* p) override {
        if (!p) return false;
        std::lock_guard<std::recursive_mutex> g(lock_);
        const float xyz[] = {p->fX, p->fY, p->fZ}; return fc4_netline_position(core_, xyz);
    }
    cS3DVector3* GetBoundingBox(cS3DVector3* a, cS3DVector3* b) override {
        GetPosition(a); GetPosition(b); return a;
    }
    bool GetBoundingCityCells(SC4Rect<long>& out) override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        // Degenerate line bounds still use the base's 0.01 shrink.
        PointCells(State().position, out);
        return true;
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
    bool Write(cIGZOStream& stream) override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        fc4::OStreamAdapter out(stream); auto* props = holder_.ToCore();
        const bool ok = fc4_netline_write(core_, props, out.Get()); fc4_props_destroy(props); return ok;
    }
    bool Read(cIGZIStream& stream) override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        fc4::IStreamAdapter input(stream);
        ReadRefs refs{input.Get(), {}};
        auto forwarded = *input.Get();
        auto* previous = activeRead_;
        activeRead_ = &refs;
        forwarded.get_ref = &TrackRead;
        const auto old = State();
        const bool ok = fc4_netline_read(core_, &forwarded);
        activeRead_ = previous;
        if (!ok) { for (auto* p : refs.poles) FC4_RELEASE(p); return false; }
        DropPoles(old);
        holder_.Clear(); fc4_netline_props(core_, &cFC4PropertyHolder::FromCoreProperty, &holder_);
        uint32_t key[3]{};
        ApplyParent(fc4_netline_parent(core_, key) ? key : nullptr);
        return true;
    }
    uint32_t GetGZCLSID() override { return 0xC9C05C5D; }
private:
    static cIGZUnknown* Pointer(uint64_t t) { return reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(t)); }
    static uint64_t Token(cIGZUnknown* p) { return reinterpret_cast<uintptr_t>(p); }
    LineState State() { LineState s{}; fc4_netline_state(core_, &s, false); return s; }
    void Put(LineState& s) { fc4_netline_state(core_, &s, true); }
    static void DropPoles(const LineState& s) { for (auto t : s.poles) if (auto* p = Pointer(t)) FC4_RELEASE(p); }
    struct ReadRefs {
        const fc4_istream* api;
        std::vector<cIGZUnknown*> poles;
        static bool Get(void* u, uint32_t iid, fc4_ref* out) {
            auto* r = static_cast<ReadRefs*>(u);
            const bool ok = r->api->get_ref(r->api->user, iid, out);
            if (ok && out->token) r->poles.push_back(Pointer(out->token));
            return ok;
        }
    };
    static thread_local ReadRefs* activeRead_;
    static bool TrackRead(void*, uint32_t iid, fc4_ref* out) { return ReadRefs::Get(activeRead_, iid, out); }
    std::atomic<uint32_t> refs_{0};
    std::recursive_mutex lock_;
    cFC4PropertyHolder holder_;
    void* core_;
};
thread_local PowerLine::ReadRefs* PowerLine::activeRead_ = nullptr;
}
cIGZUnknown* CreateFC4PowerLineOccupant() {
    auto* p = new PowerLine;
    if (!p->Valid()) { delete p; return nullptr; }
    return static_cast<LineInterface*>(p);
}
