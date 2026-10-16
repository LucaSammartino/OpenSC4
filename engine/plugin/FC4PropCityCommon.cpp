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

#include "FC4PropCityCommon.h"

#include "FC4Diag.h"
#include "FC4GameList.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4Properties.h"
#include "FC4RefAudit.h"
#include "FC4SafeLotIterator.h"
#include "FC4Streams.h"
#include "FC4WinInterfaces.h"
#include "cFC4ZoneDeveloper.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4View3DWin.h"
#include "cISCPropertyHolder.h"
#include "cSC4LotConfigurationObject.h"

#include <cstdio>

namespace fc4 {
namespace propcity {

namespace {

constexpr uint32_t kMaxRegisteredIterators = 4096;
constexpr uint16_t kVariantUint32 = 0x06;
const uint32_t kWinSC4App = 0x6104489A;
const uint32_t kWinSC4View3DWin = 0x9A47B417;
const uint32_t kIID_cISC4View3DWin = 0xFA47B3F9;

struct Copy {
    safelots::Block* block;
    const void* expectedList;
    void** out;
    uint32_t cap;
    uint32_t count;
    safelots::Status status;
    bool wrongList;
};

void CopyFn(void* c) {
    auto* m = static_cast<Copy*>(c);
    if (m->block != nullptr && m->block->listObject != m->expectedList) {
        m->wrongList = true;
        return;
    }
    m->status = safelots::CopyLots(m->block, m->out, m->cap, &m->count);
}

struct Release {
    safelots::Block* block;
    safelots::FreeFn freeNode;
    safelots::FreeFn freeBlock;
    int removed;
};

void ReleaseFn(void* c) {
    auto* m = static_cast<Release*>(c);
    m->removed = safelots::Release(m->block, m->freeNode, m->freeBlock, kMaxRegisteredIterators);
}

void NoteOnce(bool& done, const char* fmt, const char* why) {
    if (done) return;
    done = true;
    Log(fmt, why);
}

// The master list through slot 15, or false (nothing to release then except
// what is logged).
bool MasterLots(cFC4ISC4LotManager* lm, std::vector<cISC4Lot*>& out, const char** why) {
    static bool leakNoted = false;
    const compat::ExecutableVersion* v = compat::IdentifiedVersion();
    compat::StlNodeFreeFn freeNode = nullptr;
    const uintptr_t freeBlockVa = compat::DirectCall(compat::Direct::GlobalHeapFree);
    if (v == nullptr || freeBlockVa == 0 || !compat::StlNodeAllocator(nullptr, &freeNode)) {
        *why = "executable not identified, or allocator B / C unavailable";
        return false;
    }
    const void* expectedList = cFC4ZoneDeveloper::SafeIteratorListObject(lm, why);
    if (expectedList == nullptr) return false;
    const int32_t lotCount = lm->GetLotCount();  // slot 16
    if (lotCount < 0) {
        *why = "negative GetLotCount";
        return false;
    }
    auto* block = reinterpret_cast<safelots::Block*>(lm->GetSafeLotIterator());  // slot 15
    if (block == nullptr) {
        *why = "GetSafeLotIterator returned null";
        return false;
    }
    std::vector<void*> lots(static_cast<size_t>(lotCount) + 1);
    Copy copy{block, expectedList, lots.data(), static_cast<uint32_t>(lotCount), 0, safelots::Status::NoBlock, false};
    const bool copied = SehGuard("propcity master lot list walk", &CopyFn, &copy);
    Release rel{block, reinterpret_cast<safelots::FreeFn>(freeNode), reinterpret_cast<safelots::FreeFn>(freeBlockVa),
                -1};
    const bool released = copied && SehGuard("propcity safe lot iterator release", &ReleaseFn, &rel);
    if (!released || rel.removed < 0) {
        NoteOnce(leakNoted, "propcity: WARNING a safe lot iterator could not be unregistered (%s); its block is leaked",
                 !copied ? "the walk faulted" : "the release faulted or a broken ring");
    }
    if (!copied || copy.wrongList || copy.status != safelots::Status::Ok) {
        *why = !copied ? "the walk faulted" : copy.wrongList ? "unexpected list object" : safelots::StatusName(copy.status);
        return false;
    }
    out.clear();
    for (uint32_t i = 0; i < copy.count; ++i) {
        if (lots[i] != nullptr) out.push_back(static_cast<cISC4Lot*>(lots[i]));
    }
    return true;
}

// ---- the recording stream -----------------------------------------------------------------------------

RecordingIStream* Self(void* u) { return static_cast<RecordingIStream*>(u); }

}  // namespace

cISC4City* AppCity() {
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

void Hold(cIGZUnknown* o) {
    if (o == nullptr) return;
    if (IsOurObject(o)) {
        o->AddRef();
    } else {
        FC4_ADDREF(o);
    }
}

void Drop(cIGZUnknown* o) {
    if (o == nullptr) return;
    if (IsOurObject(o)) {
        o->Release();
    } else {
        FC4_RELEASE(o);
    }
}

void DropHeld(cIGZUnknown* o) {
    if (o == nullptr) return;
    if (IsOurObject(o)) {
        o->Release();
    } else {
        FC4_RELEASE_LAST_HOLDER(o);
    }
}

void CallPool::Drain() {
    // Taken before the releases: a release can run game code. The entries
    // are list references (allocators.md: one per entry) and query results;
    // the call may have removed the object meanwhile (RemovePropA,
    // DemolishFloraOccupant), so ours can be the last reference, as the
    // original's own list is when it frees it: FC4_RELEASE_LAST_HOLDER.
    std::vector<cIGZUnknown*> v;
    v.swap(held_);
    for (cIGZUnknown* o : v) DropHeld(o);
}

void* QueryInto(CallPool& pool, cIGZUnknown* u, uint32_t iid) {
    if (u == nullptr) return nullptr;
    void* p = nullptr;
    if (!u->QueryInterface(iid, &p) || p == nullptr) return nullptr;
    cIGZUnknown* q = static_cast<cIGZUnknown*>(p);
    if (!IsOurObject(q)) FC4_ACQUIRED(q, "QueryInterface");
    pool.Adopt(q);
    return p;
}

void SinkCfgObjects(SC4Vector<cSC4LotConfigurationObject>* objects, fc4_lot_cfg_objects_sink sink, void* ctx) {
    if (sink == nullptr) return;
    std::vector<fc4_lot_cfg_object> items;
    std::vector<std::vector<uint32_t>> ids;
    if (objects != nullptr) {
        const size_t n = objects->size();
        items.reserve(n);
        ids.resize(n);
        cSC4LotConfigurationObject* o = objects->begin();
        for (size_t i = 0; i < n; ++i, ++o) {
            for (uint32_t id : o->data) ids[i].push_back(id);
            fc4_lot_cfg_object c{};
            c.object_type = static_cast<uint8_t>(o->type);
            c.tag = o->objectID;
            c.flags = o->flags;
            c.id_count = static_cast<uint32_t>(ids[i].size());
            c.ids = ids[i].empty() ? nullptr : ids[i].data();
            items.push_back(c);
        }
    }
    sink(ctx, items.empty() ? nullptr : items.data(), items.size());
}

std::vector<cISC4Lot*> AllLots(cISC4City* city, cISC4LotManager* sdkLm, CallPool& pool, bool* ordered) {
    static bool fallbackNoted = false;
    std::vector<cISC4Lot*> out;
    if (ordered != nullptr) *ordered = false;
    if (sdkLm == nullptr) return out;
    cFC4ISC4LotManager* lm = AsGame(sdkLm);
    const char* why = nullptr;
    if (MasterLots(lm, out, &why)) {
        if (ordered != nullptr) *ordered = true;
        return out;
    }
    NoteOnce(fallbackNoted,
             "propcity: the lot manager's list is not walked raw (%s); the lots of GetLotsInCellRect are used "
             "instead (lot order: question CD6)",
             why != nullptr ? why : "?");
    out.clear();
    if (city == nullptr || !StlNodeAllocator::Available()) return out;
    GameList<cISC4Lot*> list;
    if (!list.valid()) return out;
    lm->GetLotsInCellRect(list, 0, 0, static_cast<int32_t>(city->CellCountX()) - 1,
                          static_cast<int32_t>(city->CellCountZ()) - 1, false);  // slot 7
    for (cISC4Lot*& lot : list) {
        if (lot == nullptr) continue;
        FC4_ACQUIRED(lot, "GetLotsInCellRect list entry");
        pool.Adopt(reinterpret_cast<cIGZUnknown*>(lot));
        out.push_back(lot);
        lot = nullptr;
    }
    return out;
}

cISC4View3DWin* View3D() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(sc4, "QueryInterface");
    cISC4View3DWin* view = nullptr;
    auto* main = reinterpret_cast<cFC4IGZWin*>(sc4->GetMainWindow());                // borrowed
    cFC4IGZWin* parent = main ? main->GetChildWindowFromID(kWinSC4App) : nullptr;  // slot 34, borrowed
    if (parent == nullptr ||
        !parent->GetChildAs(kWinSC4View3DWin, kIID_cISC4View3DWin, reinterpret_cast<void**>(&view)) ||  // slot 36
        view == nullptr) {
        view = nullptr;
    } else {
        FC4_ACQUIRED(view, "GetChildAs");
    }
    FC4_RELEASE(sc4);
    return view;
}

bool U32Property(const cISCPropertyHolder* h, uint32_t id, uint32_t* out) {
    if (out != nullptr) *out = 0;
    if (h == nullptr) return false;
    std::vector<uint32_t> v;
    if (!props::GetU32s(h, id, v) || v.empty()) return false;
    if (out != nullptr) *out = v[0];
    return true;
}

void SetU32Property(cISCPropertyHolder* h, uint32_t id, uint32_t value) {
    if (h == nullptr) return;
    if (props::Has(h, id)) {
        props::SetTyped(h, id, kVariantUint32, false, &value, 1);
    } else {
        AsGame(h)->AddPropertyU32(id, value, false);  // holder slot 13
    }
}

// ---- RecordingIStream ---------------------------------------------------------------------------------

RecordingIStream::RecordingIStream(const fc4_istream* inner, bool addRefOurs) : inner_(inner), addRefOurs_(addRefOurs) {
    api_.struct_size = sizeof(api_);
    api_.user = this;
    api_.get_u8 = [](void* u, uint8_t* out) -> bool {
        const fc4_istream* in = Self(u)->inner_;
        return in->get_u8 != nullptr && in->get_u8(in->user, out);
    };
    api_.get_u16 = [](void* u, uint16_t* out) -> bool {
        const fc4_istream* in = Self(u)->inner_;
        return in->get_u16 != nullptr && in->get_u16(in->user, out);
    };
    api_.get_u32 = [](void* u, uint32_t* out) -> bool {
        const fc4_istream* in = Self(u)->inner_;
        return in->get_u32 != nullptr && in->get_u32(in->user, out);
    };
    api_.get_u64 = [](void* u, uint64_t* out) -> bool {
        const fc4_istream* in = Self(u)->inner_;
        return in->get_u64 != nullptr && in->get_u64(in->user, out);
    };
    api_.get_f32 = [](void* u, float* out) -> bool {
        const fc4_istream* in = Self(u)->inner_;
        return in->get_f32 != nullptr && in->get_f32(in->user, out);
    };
    api_.get_f64 = [](void* u, double* out) -> bool {
        const fc4_istream* in = Self(u)->inner_;
        return in->get_f64 != nullptr && in->get_f64(in->user, out);
    };
    api_.get_bytes = [](void* u, uint8_t* out, size_t n) -> bool {
        const fc4_istream* in = Self(u)->inner_;
        return in->get_bytes != nullptr && in->get_bytes(in->user, out, n);
    };
    api_.get_str = [](void* u, fc4_bytes_sink sink, void* sinkCtx) -> bool {
        const fc4_istream* in = Self(u)->inner_;
        return in->get_str != nullptr && in->get_str(in->user, sink, sinkCtx);
    };
    api_.get_ref = &RecordingIStream::GetRef;
    api_.skip = [](void* u, size_t n) -> bool {
        const fc4_istream* in = Self(u)->inner_;
        return in->skip != nullptr && in->skip(in->user, n);
    };
    api_.error = [](void* u) -> int32_t {
        const fc4_istream* in = Self(u)->inner_;
        return in->error != nullptr ? in->error(in->user) : 0;
    };
}

bool RecordingIStream::GetRef(void* user, uint32_t iid, fc4_ref* out) {
    RecordingIStream* s = Self(user);
    const fc4_istream* in = s->inner_;
    if (in->get_ref == nullptr || !in->get_ref(in->user, iid, out)) return false;
    if (out != nullptr && out->token != 0) {
        cIGZUnknown* o = FromToken(out->token);
        if (s->addRefOurs_ && IsOurObject(o)) o->AddRef();
        s->refs_.push_back(Taken{iid, out->token, out->class_id});
    }
    return true;
}

void DropRefs(const std::vector<RecordingIStream::Taken>& refs) {
    for (const RecordingIStream::Taken& t : refs) Drop(FromToken(t.token));
}

const char* DateText(int32_t julian, char (&buf)[16]) {
    if (julian <= 0) {
        std::snprintf(buf, sizeof(buf), "0000-00-00");
        return buf;
    }
    fc4_date d{};
    fc4_date_from_julian(julian, &d);
    std::snprintf(buf, sizeof(buf), "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
    return buf;
}

}  // namespace propcity
}  // namespace fc4
