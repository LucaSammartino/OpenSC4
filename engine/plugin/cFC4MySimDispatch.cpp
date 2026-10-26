// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

// The MySim dispatch front (cFC4MySimDispatch.h).

#include "cFC4MySimDispatch.h"

#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "GZServPtrs.h"
#include "SC4Rect.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4Occupant.h"
#include "cS3DVector3.h"

namespace {

constexpr uint32_t kIIDUnknown = 0x00000001;
constexpr uint32_t kIIDMessageTarget2 = 0x452294AA;
constexpr uint32_t kIIDSerializable = 0xE4FDA3D4;
constexpr uint32_t kMsgMinute = 0x44778231;     // §4.6.1: +5 minutes
constexpr uint32_t kMsgAnimation = 0x86AD10EE;  // §4.6.1: one state update
constexpr uint32_t kMsgPostCityInit = 0x26D31EC1;
constexpr uint32_t kMsgDispatchNotification = 0x2BA06B99;  // §3.3

cISC4City* City() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) {
        return nullptr;
    }
    FC4_ACQUIRED(reinterpret_cast<cIGZUnknown*>(sc4), "QueryInterface");
    cISC4City* city = sc4->GetCity();
    FC4_RELEASE(sc4);
    return city;
}

cISC4LotManager* Lots() {
    cISC4City* city = City();
    return city ? city->GetLotManager() : nullptr;
}

cISC4Occupant* Occ(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(static_cast<uintptr_t>(t)); }
uint64_t TokenOf(const void* p) { return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(p)); }

bool RectOf(cISC4Lot* lot, int32_t* r) {
    SC4Rect<int32_t> b{};
    if (lot == nullptr || !lot->GetBoundingRect(b)) return false;
    r[0] = b.topLeftX;
    r[1] = b.topLeftY;
    r[2] = b.bottomRightX;
    r[3] = b.bottomRightY;
    return true;
}

// ---- fc4_mysim_dispatch_host (read-only to the game, except the owner call) ----

void HCellCounts(void*, int32_t* x, int32_t* z) {
    cISC4City* city = City();
    *x = city ? static_cast<int32_t>(city->CellCountX()) : 0;
    *z = city ? static_cast<int32_t>(city->CellCountZ()) : 0;
}
bool HOccupantLotRect(void*, uint64_t o, int32_t* r) {
    cISC4LotManager* lm = Lots();
    return lm != nullptr && Occ(o) != nullptr && RectOf(lm->GetOccupantLot(Occ(o)), r);
}
bool HLotRectAt(void*, int32_t x, int32_t z, int32_t* r) {
    cISC4LotManager* lm = Lots();
    return lm != nullptr && RectOf(lm->GetLot(x, z, false), r);
}
uint32_t HLotAtCell(void*, int32_t x, int32_t z, uint64_t* b) {
    cISC4LotManager* lm = Lots();
    cISC4Lot* lot = lm ? lm->GetLot(x, z, false) : nullptr;
    if (lot == nullptr) return 0;
    cISC4BuildingOccupant* building = lot->GetBuilding();  // borrowed
    if (building == nullptr) return 1;
    // The dispatch's locations hold the building's cISC4Occupant (SetByOccupant).
    void* occ = nullptr;
    if (!building->QueryInterface(GZIID_cISC4Occupant, &occ) || occ == nullptr) return 1;
    *b = TokenOf(occ);
    static_cast<cIGZUnknown*>(occ)->Release();  // the lot keeps the building alive
    return 2;
}
bool HOccupantPosition(void*, uint64_t o, float* x, float* z) {
    cISC4Occupant* occ = Occ(o);
    cS3DVector3 p{};
    // [abi-ret] occupant slot 8: only the presence bit gates the out values.
    if (occ == nullptr || !occ->GetPosition(&p)) return false;
    *x = p.fX;
    *z = p.fZ;
    return true;
}
uint32_t HWealth(void* u) {
    auto* d = static_cast<cFC4MySimDispatch*>(u);
    fc4_mysim_state st{};
    st.struct_size = sizeof(st);
    if (d->Owners() == nullptr || !fc4_mysim_get_state(d->Owners(), d->OwnerId(), &st)) return 0;
    return st.wealth;
}
bool HHasOwner(void* u) {
    auto* d = static_cast<cFC4MySimDispatch*>(u);
    return d->Owners() != nullptr && d->OwnerId() != 0;
}
// §3.3: a synchronous owner call, 0x2BA06B99 with data1 = the notice, data2/3 = old/new.
void HNotify(void* u, uint32_t sub, uint32_t a, uint32_t b) {
    auto* d = static_cast<cFC4MySimDispatch*>(u);
    if (d->Owners() == nullptr || d->OwnerId() == 0) return;
    fc4_adv_message m{};
    m.msg_type = kMsgDispatchNotification;
    m.data1 = sub;
    m.data2 = a;
    m.data3 = b;
    fc4_advisors_advisor_do_message(d->Owners(), d->OwnerId(), &m);
}

}  // namespace

cFC4MySimDispatch::cFC4MySimDispatch() {
    host_.struct_size = sizeof(host_);
    host_.user = this;
    host_.cell_counts = &HCellCounts;
    host_.occupant_lot_rect = &HOccupantLotRect;
    host_.lot_rect_at = &HLotRectAt;
    host_.lot_at_cell = &HLotAtCell;
    host_.occupant_position = &HOccupantPosition;
    host_.wealth = &HWealth;
    // [choice] neighbour-city commutes need the region and the neighbours
    // simulator's connection lists: not wired, so a neighbour location is invalid.
    host_.city_across_connection = nullptr;
    host_.region_city = nullptr;
    host_.connections_of_type_to_city = nullptr;
    host_.has_owner = &HHasOwner;
    host_.notify = &HNotify;
    core_ = fc4_mysim_dispatch_create(&host_);
}

cFC4MySimDispatch::~cFC4MySimDispatch() {
    Listen(false);
    Subscribe(kMsgPostCityInit, false);
    fc4_mysim_dispatch_destroy(core_);
    core_ = nullptr;
}

bool cFC4MySimDispatch::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    *ppvObj = nullptr;  // §7.2.3: cleared before the test
    switch (riid) {
        case kFC4_IID_cSC4MySimDispatch:
        case kIIDUnknown: *ppvObj = static_cast<cFC4IMySimDispatch*>(this); break;
        case kIIDMessageTarget2: *ppvObj = static_cast<cIGZMessageTarget2*>(this); break;
        case kIIDSerializable: *ppvObj = static_cast<cIGZSerializable*>(this); break;
        case kFC4_IID_FC4MySimDispatchConcrete: *ppvObj = this; break;
        default: return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4MySimDispatch::AddRef() { return ++refCount_; }

uint32_t cFC4MySimDispatch::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    delete this;
    return 0;
}

uint32_t cFC4MySimDispatch::RemoveRef() { return refCount_ != 0 ? --refCount_ : 0; }
uint32_t cFC4MySimDispatch::GetRefCount() { return refCount_; }

bool cFC4MySimDispatch::CanUDriveIt() { return fc4_mysim_dispatch_u_drive(core_, 0); }
bool cFC4MySimDispatch::EnterUDriveIt() { return fc4_mysim_dispatch_u_drive(core_, 1); }
bool cFC4MySimDispatch::ExitUDriveIt() { return fc4_mysim_dispatch_u_drive(core_, 2); }

bool cFC4MySimDispatch::DoMessage(cIGZMessage2* msg) {
    if (msg == nullptr) return true;
    switch (msg->GetType()) {
        case kMsgMinute: fc4_mysim_dispatch_minute(core_); break;
        case kMsgAnimation: fc4_mysim_dispatch_animation(core_); break;
        case kMsgPostCityInit:
            if (setupPending_) {
                setupPending_ = false;
                Subscribe(kMsgPostCityInit, false);
                if (fc4_mysim_dispatch_deferred_setup(core_)) Listen(true);
            }
            break;
        default: break;
    }
    return true;
}

bool cFC4MySimDispatch::Write(cIGZOStream& out) {
    fc4::OStreamAdapter a(out);
    return fc4_mysim_dispatch_write(core_, a.Get());
}

bool cFC4MySimDispatch::Read(cIGZIStream& in) {
    fc4::IStreamAdapter a(in);
    const bool ok = fc4_mysim_dispatch_read(core_, a.Get());
    // §6.2.1: the restored values are replayed at the next PostCityInit.
    if (!setupPending_) Subscribe(kMsgPostCityInit, true);
    setupPending_ = true;
    return ok;
}

void cFC4MySimDispatch::SetOwner(fc4_advisors* owners, int32_t ownerId) {
    owners_ = owners;
    ownerId_ = owners != nullptr ? ownerId : 0;
}

bool cFC4MySimDispatch::Init() {
    const bool ok = fc4_mysim_dispatch_init(core_);
    if (ok) Listen(true);
    return ok;
}

void cFC4MySimDispatch::Shutdown() {
    Listen(false);
    fc4_mysim_dispatch_shutdown(core_);
}

void cFC4MySimDispatch::SetHome(uint64_t home) { fc4_mysim_dispatch_set_home(core_, home); }
uint32_t cFC4MySimDispatch::Commute(bool hasResidence, uint8_t state, int32_t hour) {
    return fc4_mysim_dispatch_commute(core_, hasResidence, state, hour);
}
bool cFC4MySimDispatch::FindNewJob() { return fc4_mysim_dispatch_find_new_job(core_); }
bool cFC4MySimDispatch::ChangeJob(uint64_t b) { return fc4_mysim_dispatch_change_job(core_, b); }
bool cFC4MySimDispatch::JobDestination(fc4_mysim_job_destination* out) const {
    return fc4_mysim_dispatch_job_destination(core_, out);
}
bool cFC4MySimDispatch::Snapshot(fc4_mysim_dispatch* out) const { return fc4_mysim_dispatch_snapshot(core_, out); }
bool cFC4MySimDispatch::CheckFlags(uint32_t mask) const { return fc4_mysim_dispatch_check_flags(core_, mask); }
bool cFC4MySimDispatch::State(fc4_mysim_dispatch_state* out) const { return fc4_mysim_dispatch_get_state(core_, out); }

cFC4MySimDispatch* cFC4MySimDispatch::FromUnknown(cIGZUnknown* u) {
    void* p = nullptr;
    if (u == nullptr || !u->QueryInterface(kFC4_IID_FC4MySimDispatchConcrete, &p) || p == nullptr) return nullptr;
    auto* d = static_cast<cFC4MySimDispatch*>(p);
    d->Release();  // the caller's reference keeps it alive
    return d;
}

void cFC4MySimDispatch::Subscribe(uint32_t type, bool on) {
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    if (on) ms->AddNotification(static_cast<cIGZMessageTarget2*>(this), type);
    else ms->RemoveNotification(static_cast<cIGZMessageTarget2*>(this), type);
}

void cFC4MySimDispatch::Listen(bool on) {
    if (listening_ == on) return;
    listening_ = on;
    Subscribe(kMsgMinute, on);
    Subscribe(kMsgAnimation, on);
}

cIGZUnknown* CreateFC4MySimDispatch() {
    auto* d = new cFC4MySimDispatch();
    return static_cast<cFC4IMySimDispatch*>(d);
}
