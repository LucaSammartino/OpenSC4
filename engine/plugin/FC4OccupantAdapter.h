// SPDX-License-Identifier: GPL-3.0-or-later
// The inherited cSC4Occupant behaviour (occupant-manager.md §13.1) shared by the
// Rust-backed network occupant factories (enable = netocc). The derived class
// keeps the occupant base block in its core object and supplies, by CRTP:
//   bool OccGet(FC4OccBase&);  void OccPut(const FC4OccBase&);
//   std::recursive_mutex& OccLock();  cFC4PropertyHolder& OccHolder();
//   FC4OccupantStorage& OccStorage();
// and overrides the slots that are its own (type, position, bounds, Init,
// Shutdown, ...). The exemplar parent of the property holder and the lazy
// placeable object (prop-occupant.md §6.1) are kept here.
#pragma once
#include "FC4PropertyHolder.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "GZServPtrs.h"
#include "compat/FC4Compat.h"
#include "cFC4Message2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4Occupant.h"
#include <algorithm>
#include <cmath>
#include <cstring>
#include <memory>
#include <mutex>
#include <vector>

struct FC4OccBase {
    uint16_t removal;  // low byte: holders of removal flag 2, high byte: flag 1
    uint8_t highlight;
    uint8_t status;    // bit 0 initialised, bit 1 loaded, high nibble highlight
    uint8_t flags;     // bit 2 visible
    uint8_t rect[4];   // the stored manager box; rect[0] 0xFF = not indexed
};

// Keep the interface base pointer-sized. Native pole Write passes line+8
// directly as cIGZSerializable (network-occupants.md 4.4 and 5.1).
// Owned storage belongs after the derived class's interface subobjects.
struct FC4OccupantStorage {
    std::unique_ptr<fc4::props::Exemplar> parent;
    cISC43DPlaceableObject* placeable = nullptr;
    ~FC4OccupantStorage() {
        if (placeable) reinterpret_cast<cIGZUnknown*>(placeable)->Release();
    }
};

template <class Self>
class FC4OccupantAdapter : public cISC4Occupant {
public:
    // ---- slots 3-5: the base lifecycle -------------------------------------
    bool Init() override { return BaseInit(); }
    bool Shutdown() override { return BaseShutdown(); }
    bool IsInitialized() override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        return (Get().status & 1) != 0;
    }
    cISCPropertyHolder* AsPropertyHolder() override { return self().OccHolder().AsSdk(); }

    // ---- slots 12-14: removal counters -------------------------------------
    uint32_t SetRemovalFlags(uint32_t f) override { return Removal(f, 1); }
    uint32_t UnsetRemovalFlags(uint32_t f) override { return Removal(f, -1); }
    bool CanRemove(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        const auto r = Get().removal;
        return (!(f & 1) || !(r >> 8)) && (!(f & 2) || !(r & 255));
    }

    // ---- slot 15: messages -------------------------------------------------
    bool PostOccupantMessage(uint32_t type, uint32_t data) override {
        cIGZMessageServer2Ptr ms;
        if (!ms) return true;
        cISC4Occupant* occ = static_cast<cISC4Occupant*>(this);
        auto* msg = FC4MakeMessage(type, reinterpret_cast<intptr_t>(occ), reinterpret_cast<intptr_t>(occ),
                                   static_cast<cISC4Occupant*>(this)->GetType());
        msg->SetData4(data);
        ms->MessagePost(msg, false);
        FC4_RELEASE_CREATED(msg);
        return true;
    }

    // ---- slots 16-18: highlight and visibility -----------------------------
    uint32_t GetHighlight() override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        return Get().highlight;
    }
    bool SetHighlight(uint32_t h, bool post) override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        auto s = Get();
        s.highlight = static_cast<uint8_t>(h & 15);
        s.status = static_cast<uint8_t>((s.status & 15) | (s.highlight << 4));
        Put(s);
        if (post && (s.status & 1)) PostOccupantMessage(0x99EF1145, 0);
        return true;
    }
    uint8_t SetVisibility(bool visible, bool post) override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        auto s = Get();
        s.flags = static_cast<uint8_t>((s.flags & ~4) | (visible ? 4 : 0));
        Put(s);
        if (post) PostOccupantMessage(0x99EF1145, 0);
        return s.flags;
    }

    // ---- slots 19-21: the placeable object (prop-occupant.md §6.1) ---------
    // The occupant holds one game cSC43DPlaceableObject (not an occupant
    // class), made lazily from the game's placeable pool exactly as
    // GetOrCreatePlaceableObject 0x005E91C0 does; the model maker and the
    // view windows hang the models they make on it.
    cISC43DPlaceableObject* GetPlaceableObject() override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        return Storage().placeable;
    }
    cISC43DPlaceableObject* GetOrCreatePlaceableObject() override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        if (Storage().placeable == nullptr) {
            const uintptr_t alloc = fc4::compat::DirectCall(fc4::compat::Direct::PlaceablePoolAllocate);
            const uintptr_t ctor = fc4::compat::DirectCall(fc4::compat::Direct::PlaceableConstruct);
            if (alloc == 0 || ctor == 0) return nullptr;  // logged once by compat
            void* node = reinterpret_cast<void*(__cdecl*)(uint32_t)>(alloc)(0x30);
            if (node == nullptr) return nullptr;
            void* made = reinterpret_cast<void*(__fastcall*)(void*, void*)>(ctor)(node, nullptr);
            SwapPlaceable(static_cast<cISC43DPlaceableObject*>(made));
        }
        return Storage().placeable;
    }
    // 0x0078DAF0: AddRef the new one, Release the old one.
    cISC43DPlaceableObject* SetPlaceableObject(cISC43DPlaceableObject* p) override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        SwapPlaceable(p);
        return Storage().placeable;
    }

    // ---- slots 22-24: groups (property 0xAA1DD396, own or inherited) -------
    bool IsOccupantGroup(uint32_t group) override {
        std::vector<uint32_t> groups;
        fc4::props::GetU32s(self().OccHolder().AsSdk(), 0xAA1DD396, groups);
        return std::find(groups.begin(), groups.end(), group) != groups.end();
    }
    bool AddOccupantGroup(uint32_t group) override {
        std::vector<uint32_t> groups;
        fc4::props::GetU32s(self().OccHolder().AsSdk(), 0xAA1DD396, groups);
        const auto old = groups.size();
        groups.push_back(group);
        const bool ok = fc4::props::SetU32s(self().OccHolder().AsSdk(), 0xAA1DD396, groups);
        PostOccupantMessage(0xEBFF18A3, static_cast<uint32_t>(old));
        return ok;
    }
    // SPEC-GAP: no cross-module std::set layout contract for slot 24.
    bool GetOccupantGroups(std::set<uint32_t>&) override { return false; }

    // ---- slots 25-26: the manager's box ------------------------------------
    bool GetOccupantManagerBBox(uint8_t* out) override {
        if (!out) return false;
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        const auto s = Get();
        if (s.rect[0] == 255) return false;
        std::memcpy(out, s.rect, 4);
        return true;
    }
    bool SetOccupantManagerBBox(uint8_t* in) override {
        if (!in) return false;
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        auto s = Get();
        std::memcpy(s.rect, in, 4);
        Put(s);
        return true;
    }
    bool GetLotTag(uint32_t& tag) override { tag = 0; return false; }
    bool SetLotTag(uint32_t) override { return false; }

    // ---- slots 29-33: the flag byte ----------------------------------------
    uint32_t SetFlag(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        auto s = Get(); s.flags = static_cast<uint8_t>(s.flags | f); Put(s); return s.flags;
    }
    cISC4Occupant* SetAllFlags(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        auto s = Get(); s.flags = static_cast<uint8_t>(f); Put(s); return this;
    }
    uint32_t ClearFlag(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        auto s = Get(); s.flags = static_cast<uint8_t>(s.flags & ~f); Put(s); return s.flags;
    }
    bool IsFlagSet(uint32_t f) override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        return (Get().flags & f) == f;
    }
    uint32_t GetFlags() override {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        return Get().flags;
    }

protected:
    ~FC4OccupantAdapter() = default;

    // occupant-manager §13: loaded Init calls SetHighlight(0, false) and posts
    // no Create; a fresh Init posts Create (0x99EF1140).
    bool BaseInit() {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        auto s = Get();
        if (s.status & 1) return true;
        if (s.status & 2) { s.highlight = 0; s.status &= 15; }
        s.status |= 1;
        Put(s);
        if (!(s.status & 2)) PostOccupantMessage(0x99EF1140, 0);
        return true;
    }
    // Slot 4: if initialised, clear bit 0 and post Destroy (0x99EF1141).
    bool BaseShutdown() {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        auto s = Get();
        if (s.status & 1) { s.status &= ~1; Put(s); PostOccupantMessage(0x99EF1141, 0); }
        return true;
    }
    // The base's GetBoundingCityCells (0x005E8E20) over a degenerate box at `p`.
    static void PointCells(const float* p, SC4Rect<long>& out) {
        out.topLeftX = static_cast<long>(std::floor((p[0] + 0.01f) * 0.0625f));
        out.topLeftY = static_cast<long>(std::floor((p[2] + 0.01f) * 0.0625f));
        out.bottomRightX = static_cast<long>(std::floor((p[0] - 0.01f) * 0.0625f));
        out.bottomRightY = static_cast<long>(std::floor((p[2] - 0.01f) * 0.0625f));
    }
    // Resolves the holder's exemplar parent (foundations §10.3: against the
    // current data files) so inherited properties answer like the game's
    // cSCExemplarPropertyHolder. `key` null or not found: no parent.
    void ApplyParent(const uint32_t* key) {
        auto& holder = self().OccHolder();
        holder.SetParent(nullptr);
        Storage().parent.reset();
        if (key == nullptr) return;
        auto res = std::make_unique<fc4::props::Exemplar>();
        if (!res->Load(key[0], key[1], key[2])) return;
        Storage().parent = std::move(res);
        holder.SetParent(Storage().parent->Holder());
    }

private:
    Self& self() { return *static_cast<Self*>(this); }
    FC4OccupantStorage& Storage() { return self().OccStorage(); }
    FC4OccBase Get() { FC4OccBase s{}; self().OccGet(s); return s; }
    void Put(const FC4OccBase& s) { self().OccPut(s); }
    uint32_t Removal(uint32_t f, int delta) {
        std::lock_guard<std::recursive_mutex> g(self().OccLock());
        auto s = Get();
        uint8_t low = static_cast<uint8_t>(s.removal), high = static_cast<uint8_t>(s.removal >> 8);
        if (f & 1) high = static_cast<uint8_t>(high + delta);
        if (f & 2) low = static_cast<uint8_t>(low + delta);
        s.removal = static_cast<uint16_t>((high << 8) | low);
        Put(s);
        return s.removal;
    }
    void SwapPlaceable(cISC43DPlaceableObject* p) {
        cISC43DPlaceableObject* old = Storage().placeable;
        if (p == old) return;
        if (p) Unknown(p)->AddRef();
        Storage().placeable = p;
        if (old) Unknown(old)->Release();
    }
    // Slots 0-2 of the placeable are cIGZUnknown's (prop-occupant.md §6.1).
    static cIGZUnknown* Unknown(cISC43DPlaceableObject* p) { return reinterpret_cast<cIGZUnknown*>(p); }
};
