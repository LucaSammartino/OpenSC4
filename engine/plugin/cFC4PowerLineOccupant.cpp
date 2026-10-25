// SPDX-License-Identifier: GPL-3.0-or-later
// network-occupants.md 5; occupant-manager.md 13.1. No city access in Read.
#include "cFC4PowerLineOccupant.h"
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
}

class PowerLine final : public LineInterface, public cISC4Occupant, public cIGZSerializable {
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
    bool Init() override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        auto s = State();
        if (s.status & 1) return true;
        s.status |= 1; Put(s);
        if (!(s.status & 2)) PostOccupantMessage(0x99EF1140, 0);
        return true;
    }
    bool Shutdown() override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        SetConnectedPoles(nullptr, nullptr, false);
        auto s = State();
        if (s.status & 1) { s.status &= ~1; Put(s); PostOccupantMessage(0x99EF1141, 0); }
        return true;
    }
    bool IsInitialized() override {
        std::lock_guard<std::recursive_mutex> g(lock_); return (State().status & 1) != 0;
    }
    cISCPropertyHolder* AsPropertyHolder() override { return holder_.AsSdk(); }
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
        auto s = State();
        // Degenerate line bounds still use the base's 0.01 shrink.
        out.topLeftX = static_cast<long>(std::floor((s.position[0] + 0.01f) * 0.0625f));
        out.topLeftY = static_cast<long>(std::floor((s.position[2] + 0.01f) * 0.0625f));
        out.bottomRightX = static_cast<long>(std::floor((s.position[0] - 0.01f) * 0.0625f));
        out.bottomRightY = static_cast<long>(std::floor((s.position[2] - 0.01f) * 0.0625f));
        return true;
    }
    uint32_t Removal(uint32_t f, int delta) {
        auto s = State();
        uint8_t low = static_cast<uint8_t>(s.removal), high = static_cast<uint8_t>(s.removal >> 8);
        if (f & 1) high = static_cast<uint8_t>(high + delta);
        if (f & 2) low = static_cast<uint8_t>(low + delta);
        s.removal = static_cast<uint16_t>((high << 8) | low); Put(s); return s.removal;
    }
    uint32_t SetRemovalFlags(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(lock_); return Removal(f, 1);
    }
    uint32_t UnsetRemovalFlags(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(lock_); return Removal(f, -1);
    }
    bool CanRemove(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(lock_); const auto r = State().removal;
        return (!(f & 1) || !(r >> 8)) && (!(f & 2) || !(r & 255));
    }
    bool PostOccupantMessage(uint32_t type, uint32_t data) override {
        cIGZMessageServer2Ptr ms;
        if (!ms) return true;
        auto* msg = FC4MakeMessage(type, reinterpret_cast<intptr_t>(static_cast<cISC4Occupant*>(this)),
                                   reinterpret_cast<intptr_t>(static_cast<cISC4Occupant*>(this)), GetType());
        msg->SetData4(data);
        ms->MessagePost(msg, false);
        FC4_RELEASE_CREATED(msg);
        return true;
    }
    uint32_t GetHighlight() override {
        std::lock_guard<std::recursive_mutex> g(lock_); return State().highlight;
    }
    bool SetHighlight(uint32_t h, bool post) override {
        std::lock_guard<std::recursive_mutex> g(lock_); auto s = State();
        s.highlight = static_cast<uint8_t>(h & 15); s.status = (s.status & 15) | (s.highlight << 4); Put(s);
        if (post && (s.status & 1)) PostOccupantMessage(0x99EF1145, 0);
        return true;
    }
    uint8_t SetVisibility(bool visible, bool post) override {
        std::lock_guard<std::recursive_mutex> g(lock_); auto s = State();
        s.flags = (s.flags & ~4) | (visible ? 4 : 0); Put(s);
        if (post) PostOccupantMessage(0x99EF1145, 0);
        return s.flags;
    }
    // SPEC-GAP: 13.1 leaves lazy placeable creation unspecified; a line has no model.
    cISC43DPlaceableObject* GetPlaceableObject() override { return nullptr; }
    cISC43DPlaceableObject* GetOrCreatePlaceableObject() override { return nullptr; }
    cISC43DPlaceableObject* SetPlaceableObject(cISC43DPlaceableObject*) override { return nullptr; }
    // SPEC-GAP: no cross-module container contract for occupant group enumeration.
    bool IsOccupantGroup(uint32_t group) override {
        std::vector<uint32_t> groups;
        fc4::props::GetU32s(holder_.AsSdk(), 0xAA1DD396, groups);
        return std::find(groups.begin(), groups.end(), group) != groups.end();
    }
    bool AddOccupantGroup(uint32_t group) override {
        std::vector<uint32_t> groups;
        fc4::props::GetU32s(holder_.AsSdk(), 0xAA1DD396, groups);
        const auto old = groups.size(); groups.push_back(group);
        const bool ok = fc4::props::SetU32s(holder_.AsSdk(), 0xAA1DD396, groups);
        PostOccupantMessage(0xEBFF18A3, static_cast<uint32_t>(old)); return ok;
    }
    bool GetOccupantGroups(std::set<uint32_t>&) override { return false; }
    bool GetOccupantManagerBBox(uint8_t* out) override {
        if (!out) return false;
        std::lock_guard<std::recursive_mutex> g(lock_); auto s = State();
        if (s.rect[0] == 255) return false;
        std::memcpy(out, s.rect, 4); return true;
    }
    bool SetOccupantManagerBBox(uint8_t* in) override {
        if (!in) return false;
        std::lock_guard<std::recursive_mutex> g(lock_); auto s = State();
        std::memcpy(s.rect, in, 4); Put(s); return true;
    }
    bool GetLotTag(uint32_t& tag) override { tag = 0; return false; }
    bool SetLotTag(uint32_t) override { return false; }
    uint32_t SetFlag(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(lock_); auto s = State(); s.flags |= f; Put(s); return s.flags;
    }
    cISC4Occupant* SetAllFlags(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(lock_); auto s = State(); s.flags = static_cast<uint8_t>(f); Put(s); return this;
    }
    uint32_t ClearFlag(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(lock_); auto s = State(); s.flags &= ~f; Put(s); return s.flags;
    }
    bool IsFlagSet(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(lock_); return (State().flags & f) == f;
    }
    uint32_t GetFlags() override {
        std::lock_guard<std::recursive_mutex> g(lock_); return State().flags;
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
