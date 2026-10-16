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

#include "FC4NetMgrHost.h"

#include "FC4DevelopmentInterfaces.h"
#include "FC4Fpu.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"

#include "SC4List.h"
#include "SC4Point.h"
#include "SC4Rect.h"
#include "cIGZSerializable.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4City.h"
#include "cISC4Lot.h"
#include "cISC4LotDeveloper.h"
#include "cISC4LotManager.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cISC4ZoneManager.h"
#include "cISTETerrain.h"
#include "cS3DVector3.h"

#include <cstring>

using fc4::FromToken;
using fc4::TokenOf;

namespace {

constexpr uint32_t kIID_cISC4NetworkOccupant = 0xA821EF94;  // §4.1, §6.1
constexpr uint32_t kIID_cISC4Lot = 0xE818FCED;              // §6.2

fc4::NetMgrContext* Ctx(void* u) { return static_cast<fc4::NetMgrContext*>(u); }
cISC4City* City(void* u) { return Ctx(u)->city; }

cISC4NetworkOccupant* NetOf(uint64_t t) { return reinterpret_cast<cISC4NetworkOccupant*>(FromToken(t)); }
cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(FromToken(t)); }
cFC4ISC4Lot* LotOf(uint64_t t) { return fc4::AsGame(reinterpret_cast<cISC4Lot*>(FromToken(t))); }

// QueryInterface with the reference for the caller (0 = it does not answer).
uint64_t QueryToken(uint64_t t, uint32_t iid) {
    if (t == 0) return 0;
    void* p = nullptr;
    cIGZUnknown* u = FromToken(t);
    if (!u->QueryInterface(iid, &p) || p == nullptr) return 0;
    cIGZUnknown* q = static_cast<cIGZUnknown*>(p);
    if (!fc4::IsOurObject(q)) FC4_ACQUIRED(q, "QueryInterface");
    return TokenOf(q);
}

// ---- the stand-in -------------------------------------------------------------------------------------

constexpr uint32_t kStandInMagic = 0x4E455453;  // "NETS"

class NetStandIn final : public cIGZSerializable {
public:
    explicit NetStandIn(uint32_t clsid) : clsid_(clsid) {}
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid != GZIID_cIGZSerializable && riid != GZIID_cIGZUnknown) {
            *ppvObj = nullptr;
            return false;
        }
        *ppvObj = static_cast<cIGZSerializable*>(this);
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override {
        if (refs_ > 1) return --refs_;
        delete this;
        return 0;
    }
    bool Write(cIGZOStream&) override { return true; }
    bool Read(cIGZIStream&) override { return true; }
    uint32_t GetGZCLSID() override { return clsid_; }
    uint32_t magic = kStandInMagic;

private:
    ~NetStandIn() = default;
    uint32_t clsid_;
    uint32_t refs_ = 1;
};

// ---- shared callbacks -----------------------------------------------------------------------------------

void AddRefCb(void*, uint64_t t) { fc4::NetHold(t); }
void ReleaseCb(void*, uint64_t t) { fc4::NetDrop(t); }

// A saved reference: the capture stream (FC4WriteCapture.h CaptureIStream) or
// the segment stream already queried it for `iid` with one reference for the
// reader when the object is the game's; one of ours comes without one.
uint64_t ResolveRef(void*, uint64_t token, uint32_t) {
    if (token != 0 && fc4::IsOurObject(FromToken(token))) FromToken(token)->AddRef();
    return token;
}

void CitySize(void* u, int32_t* x, int32_t* z) {
    cISC4City* c = City(u);
    if (x != nullptr) *x = c ? static_cast<int32_t>(c->CellCountX()) : 0;  // slot 93
    if (z != nullptr) *z = c ? static_cast<int32_t>(c->CellCountZ()) : 0;  // slot 94
}

uint64_t QueryNetworkOccupant(void*, uint64_t occupant) { return QueryToken(occupant, kIID_cISC4NetworkOccupant); }

uint32_t NetworkFlags(void*, uint64_t n) { return n ? NetOf(n)->GetNetworkFlag() : 0; }  // slot 12

void OccupiedCell(void*, uint64_t n, int32_t* x, int32_t* z) {
    uint32_t cx = 0, cz = 0;
    if (n != 0) NetOf(n)->GetOccupiedCell(cx, cz);  // slot 59
    if (x != nullptr) *x = static_cast<int32_t>(cx);
    if (z != nullptr) *z = static_cast<int32_t>(cz);
}

float OccupantAltitude(void*, uint64_t n, float x, float z) {
    if (n == 0) return 0.0f;
    const float a = NetOf(n)->GetAltitude(x, z);  // slot 56, ST0
    FC4_X87_CHECK("netmgr network occupant GetAltitude");
    return a;
}

float TerrainAltitude(void* u, float x, float z) {
    cISC4City* c = City(u);
    cISTETerrain* t = c ? c->GetTerrain() : nullptr;
    if (t == nullptr) return 0.0f;
    const float a = t->GetAltitude(x, z);  // compiled slot 13, ST0
    FC4_X87_CHECK("netmgr terrain GetAltitude");
    return a;
}

// ---- the network manager --------------------------------------------------------------------------------

bool IsNetworkOccupant(void*, uint64_t occupant) {
    const uint64_t n = QueryToken(occupant, kIID_cISC4NetworkOccupant);
    if (n == 0) return false;
    fc4::NetDrop(n);
    return true;
}

bool IsOfType(void*, uint64_t n, uint32_t type) {
    return n != 0 && NetOf(n)->IsOfType(static_cast<cISC4NetworkOccupant::eNetworkType>(type));  // slot 22
}

uint32_t OccupantFlags(void*, uint64_t occupant) { return occupant ? OccOf(occupant)->GetFlags() : 0; }

bool IsManagedGlobally(void*, uint64_t n) { return n != 0 && NetOf(n)->IsManagedGlobally(); }  // slot 5
bool CanBeLit(void*, uint64_t n) { return n != 0 && NetOf(n)->CanBeLit(); }                    // slot 62

// Occupant manager slot 40 over the rect with no filter, kept when the
// occupant is a network occupant whose network flags meet `mask` (§7.6: an
// OR filter), in query order. The network occupant references are held.
void OccupantsInRect(void* u, const fc4_cell_rect* r, uint32_t mask, fc4_token_callback cb, void* ctx) {
    cISC4City* c = City(u);
    cISC4OccupantManager* om = c ? c->GetOccupantManager() : nullptr;
    if (om == nullptr || r == nullptr || cb == nullptr) return;
    if (r->x1 > r->x2 || r->z1 > r->z2) return;
    SC4List<cISC4Occupant*> list;
    const SC4Point<int> a(r->x1, r->z1);
    const SC4Point<int> b(r->x2, r->z2);
    om->GetOccupantsByStandardCityCells(list, &a, &b, static_cast<cISC4OccupantFilter*>(nullptr), 0xFFFFFFFFu);
    std::vector<uint64_t> found;
    for (cISC4Occupant*& o : list) {
        if (o == nullptr) continue;
        FC4_ACQUIRED(o, "GetOccupantsByStandardCityCells list entry");
        const uint64_t n = QueryToken(TokenOf(o), kIID_cISC4NetworkOccupant);
        FC4_RELEASE(o);
        o = nullptr;  // the list's destructor then only frees its nodes
        if (n == 0) continue;
        Ctx(u)->held.push_back(FromToken(n));
        if ((NetOf(n)->GetNetworkFlag() & mask) != 0) found.push_back(n);
    }
    for (uint64_t n : found) cb(ctx, n);
}

// ---- the network lot manager ------------------------------------------------------------------------------

uint32_t PieceId(void*, uint64_t n) { return n ? NetOf(n)->PieceId() : 0; }                         // slot 23
int32_t Rotation(void*, uint64_t n) { return n ? static_cast<int32_t>(NetOf(n)->GetRotation()) : 0; }  // 24
int32_t Flip(void*, uint64_t n) { return n ? static_cast<int32_t>(NetOf(n)->GetFlip()) : 0; }          // 25

// city.PositionToCell(n.AsOccupant().GetPosition()) (city slot 95).
void PositionCell(void* u, uint64_t n, int32_t* x, int32_t* z) {
    int cx = 0, cz = 0;
    cISC4City* c = City(u);
    cISC4Occupant* o = n ? NetOf(n)->AsOccupant() : nullptr;  // slot 48
    if (c != nullptr && o != nullptr) {
        cS3DVector3 p;
        o->GetPosition(&p);  // occupant slot 8
        c->PositionToCell(p.fX, p.fZ, cx, cz);
    }
    if (x != nullptr) *x = cx;
    if (z != nullptr) *z = cz;
}

bool IsIntersection(void*, uint64_t n) { return n != 0 && NetOf(n)->IsIntersection(); }  // slot 55
uint32_t EdgeTypesFlag(void*, uint64_t n, uint32_t kind) {
    return n ? NetOf(n)->GetEdgeTypesFlag(static_cast<cISC4NetworkOccupant::eNetworkType>(kind)) : 0;  // slot 38
}
uint32_t ActiveEdges(void*, uint64_t n) { return n ? NetOf(n)->GetActiveEdges() : 0; }  // slot 40

cFC4ISC4TrafficNetworkMap* TrafficMap(void* u) {
    cISC4City* c = City(u);
    return c ? reinterpret_cast<cFC4ISC4TrafficNetworkMap*>(c->GetTrafficNetwork()) : nullptr;  // city slot 40
}

bool TrafficMapExists(void* u) { return TrafficMap(u) != nullptr; }

// Traffic network map slot 8 FindNetworkTile(x, z, mask, true): a network
// occupant, borrowed (zones.md §12).
uint64_t FindNetworkTile(void* u, int32_t x, int32_t z, uint32_t mask) {
    cFC4ISC4TrafficNetworkMap* m = TrafficMap(u);
    return m ? TokenOf(m->FindNetworkTile(x, z, mask, true)) : 0;
}

void CellCenter(void* u, int32_t x, int32_t z, float* wx, float* wz) {
    float px = 0.0f, pz = 0.0f;
    if (cISC4City* c = City(u)) c->CellCenterToPosition(x, z, px, pz);  // slot 97
    if (wx != nullptr) *wx = px;
    if (wz != nullptr) *wz = pz;
}

// The zone grid (zone manager slot 12, the signed byte in EAX).
int32_t ZoneType(void* u, int32_t x, int32_t z) {
    cISC4City* c = City(u);
    cISC4ZoneManager* zm = c ? c->GetZoneManager() : nullptr;
    return zm ? fc4::AsGame(zm)->GetZoneType(x, z) : 0;
}

// Lot manager slot 6 GetLot(x, z, false), borrowed.
uint64_t LotAt(void* u, int32_t x, int32_t z) {
    cISC4City* c = City(u);
    cISC4LotManager* lm = c ? c->GetLotManager() : nullptr;
    return lm ? TokenOf(fc4::AsGame(lm)->GetLot(x, z, false)) : 0;
}

int32_t LotWealth(void*, uint64_t lot) { return lot ? static_cast<int32_t>(LotOf(lot)->GetOccupantWealth()) : 0; }
bool LotIsLivable(void*, uint64_t lot) { return lot != 0 && LotOf(lot)->IsInLivableState(); }
uint32_t LotBuildingType(void*, uint64_t lot) { return lot ? LotOf(lot)->GetBuildingType(false) : 0; }

// BDS slot 40 GetBuildingProfilePtr: the profile's wealth (+0x04).
bool BuildingProfileWealth(void* u, uint32_t type, int32_t* wealth) {
    cISC4City* c = City(u);
    cISC4BuildingDevelopmentSimulator* b = c ? c->GetBuildingDevelopmentSimulator() : nullptr;
    const FC4BuildingProfile* p = b ? fc4::AsGame(b)->GetBuildingProfilePtr(type) : nullptr;
    if (p == nullptr) return false;
    if (wealth != nullptr) *wealth = static_cast<int32_t>(p->wealth);
    return true;
}

uint64_t QueryLot(void*, uint64_t unknown) { return QueryToken(unknown, kIID_cISC4Lot); }

void LotBoundingRect(void*, uint64_t lot, fc4_cell_rect* out) {
    if (out == nullptr) return;
    *out = fc4_cell_rect{0, 0, -1, -1};
    SC4Rect<int32_t> r;
    if (lot != 0 && LotOf(lot)->GetBoundingRect(r)) {  // slot 14
        *out = fc4_cell_rect{r.topLeftX, r.topLeftY, r.bottomRightX, r.bottomRightY};
    }
}

// lotDeveloper.GetAreaSlope({x, z, x, z}, true) (lot developer slot 29
// through BDS slot 36; ST0).
float AreaSlope(void* u, int32_t x, int32_t z) {
    cISC4City* c = City(u);
    cISC4BuildingDevelopmentSimulator* b = c ? c->GetBuildingDevelopmentSimulator() : nullptr;
    cISC4LotDeveloper* ld = b ? fc4::AsGame(b)->GetLotDeveloper() : nullptr;
    if (ld == nullptr) return 0.0f;
    const float s = ld->GetAreaSlope(SC4Rect<int32_t>(x, z, x, z), true);
    FC4_X87_CHECK("netlot GetAreaSlope");
    return s;
}

}  // namespace

namespace fc4 {

void NetHold(uint64_t token) {
    if (token == 0) return;
    cIGZUnknown* o = FromToken(token);
    if (IsOurObject(o)) {
        o->AddRef();
    } else {
        FC4_ADDREF(o);
    }
}

void NetDrop(uint64_t token) {
    if (token == 0) return;
    cIGZUnknown* o = FromToken(token);
    if (IsOurObject(o)) {
        o->Release();
    } else {
        FC4_RELEASE(o);
    }
}

uint64_t NewNetStandIn(uint32_t clsid) {
    NetStandIn* s = new NetStandIn(clsid);
    return TokenOf(static_cast<cIGZSerializable*>(s));
}

bool IsNetStandIn(uint64_t token) {
    if (token == 0 || !IsOurObject(FromToken(token))) return false;
    cIGZSerializable* s = reinterpret_cast<cIGZSerializable*>(FromToken(token));
    void* p = nullptr;
    if (!s->QueryInterface(GZIID_cIGZSerializable, &p) || p == nullptr) return false;
    cIGZSerializable* q = static_cast<cIGZSerializable*>(p);
    q->Release();
    NetStandIn* n = dynamic_cast<NetStandIn*>(q);
    return n != nullptr && n->magic == kStandInMagic;
}

fc4_netmgr_host MakeNetMgrReadOnlyHost(NetMgrContext* ctx) {
    fc4_netmgr_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.city_size = &CitySize;
    h.add_ref = &AddRefCb;
    h.release = &ReleaseCb;
    h.resolve_ref = &ResolveRef;
    h.query_network_occupant = &QueryNetworkOccupant;
    h.is_network_occupant = &IsNetworkOccupant;
    h.network_flags = &NetworkFlags;
    h.is_of_type = &IsOfType;
    h.occupant_flags = &OccupantFlags;
    h.occupied_cell = &OccupiedCell;
    h.occupant_altitude = &OccupantAltitude;
    h.is_managed_globally = &IsManagedGlobally;
    h.can_be_lit = &CanBeLit;
    h.occupants_in_rect = &OccupantsInRect;
    h.terrain_altitude = &TerrainAltitude;
    return h;
}

fc4_netlot_host MakeNetLotReadOnlyHost(NetMgrContext* ctx) {
    fc4_netlot_host h{};
    h.struct_size = sizeof(h);
    h.user = ctx;
    h.add_ref = &AddRefCb;
    h.release = &ReleaseCb;
    h.resolve_ref = &ResolveRef;
    h.query_network_occupant = &QueryNetworkOccupant;
    h.network_flags = &NetworkFlags;
    h.piece_id = &PieceId;
    h.rotation = &Rotation;
    h.flip = &Flip;
    h.position_cell = &PositionCell;
    h.occupied_cell = &OccupiedCell;
    h.occupant_altitude = &OccupantAltitude;
    h.is_intersection = &IsIntersection;
    h.edge_types_flag = &EdgeTypesFlag;
    h.active_edges = &ActiveEdges;
    h.traffic_map_exists = &TrafficMapExists;
    h.find_network_tile = &FindNetworkTile;
    h.city_size = &CitySize;
    h.cell_center = &CellCenter;
    h.terrain_altitude = &TerrainAltitude;
    h.zone_type = &ZoneType;
    h.lot_at = &LotAt;
    h.lot_wealth = &LotWealth;
    h.lot_is_livable = &LotIsLivable;
    h.lot_building_type = &LotBuildingType;
    h.building_profile_wealth = &BuildingProfileWealth;
    h.query_lot = &QueryLot;
    h.lot_bounding_rect = &LotBoundingRect;
    h.area_slope = &AreaSlope;
    return h;
}

void NetMgrReleaseHeld(NetMgrContext* ctx) {
    if (ctx == nullptr) return;
    std::vector<cIGZUnknown*> held;
    held.swap(ctx->held);
    for (cIGZUnknown* o : held) NetDrop(TokenOf(o));
}

}  // namespace fc4
