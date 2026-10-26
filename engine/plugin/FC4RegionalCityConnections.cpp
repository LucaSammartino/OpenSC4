// SPDX-License-Identifier: GPL-3.0-or-later
#include "cFC4RegionalCity.h"
#include "FC4RegionalCityElements.h"
#include "FC4RegionalCityHost.h"
#include "FC4RegionalCityBorder.h"
#include "cFC4ZoneManager.h"
#include "FC4Properties.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"
#include "GZServPtrs.h"
#include "cIGZMessageServer2.h"
#include "cISC4Demolition.h"
#include "cISC4NetworkManager.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantFilter.h"
#include "cISC4RegionalCity.h"
#include "cISC4Region.h"
#include "cISC4NeighborConnection.h"
#include "SC4CellRegion.h"
#include "SC4List.h"
#include "SC4Point.h"
#include <algorithm>
#include <atomic>
#include <map>
#include <memory>
#include <vector>

namespace fc4::regionalcity {
namespace {
struct OccupantQueries {
    cISC4Occupant* object;
    bool Interface(uint32_t id) {
        cIGZUnknown* p = nullptr;
        if (!object || !object->QueryInterface(id, reinterpret_cast<void**>(&p)) || !p) return false;
        FC4_ACQUIRED(p, "regionalcity layer classifier"); FC4_RELEASE(p); return true;
    }
    bool Has(uint32_t id) { return object && props::Has(object->AsPropertyHolder(), id); }
    bool Group(uint32_t id) { return object && object->IsOccupantGroup(id); }
    bool Bool(uint32_t id) {
        std::vector<uint32_t> v;
        return object && props::GetU32s(object->AsPropertyHolder(), id, v) && !v.empty() && v[0] != 0;
    }
};
class LayerFilter final : public cISC4OccupantFilter {
public:
    explicit LayerFilter(uint32_t level) : level_(level) {}
    bool QueryInterface(uint32_t id, void** out) override {
        if (!out) return false;
        *out = nullptr;
        if (id != 1 && id != 0x46A80026) return false;
        *out = static_cast<cISC4OccupantFilter*>(this); AddRef(); return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { const auto n = --refs_; if (!n) delete this; return n; }
    bool IsOccupantIncluded(cISC4Occupant* p) override { OccupantQueries q{p}; return p && ViewLevel(q) == level_; }
    bool IsOccupantTypeIncluded(uint32_t) override { return true; }
    bool IsPropertyHolderIncluded(cISCPropertyHolder*) override { return true; }
private:
    std::atomic<uint32_t> refs_{1};
    uint32_t level_;
};
struct Context {
    cFC4RegionalCity& record;
    cISC4City* city;
    cFC4ISC4NeighborsSimulator* neighbors;
    std::vector<std::unique_ptr<SC4List<cISC4Occupant*>>> lists;
    std::map<uint64_t, uint32_t> networks;
};
Context& C(void* p) { return *static_cast<Context*>(p); }
cISC4NeighborConnection* Conn(void* p) { return static_cast<cISC4NeighborConnection*>(p); }
bool OtherEstablished(void* raw, void* c) {
    int32_t x = 0, z = 0; Conn(c)->GetConnectionCell(x, z);
    auto* other = C(raw).neighbors->FindCityOnOtherSideOfConnection(x, z);
    return other && AsGame(other)->GetEstablished();
}
void Adjacent(void* raw, void* sink, void (*emit)(void*, int32_t, int32_t, int32_t, int32_t, void*)) {
    SC4List<cISC4RegionalCity*> cities;
    C(raw).neighbors->GetAdjacentCities(cities, true);
    for (auto* city : cities) if (city) {
        auto* r = AsGame(city); int32_t x = 0, z = 0, sx = 0, sz = 0;
        r->GetPosition(x, z); r->GetCitySize(sx, sz);
        SC4List<cISC4NeighborConnection*> connections; r->GetAllConnections(connections);
        for (auto* c : connections) if (c) emit(sink, x, z, sx, sz, c);
    }
}
bool PointsHere(void* raw, void* c) {
    auto* other = Conn(c)->GetCityOnOtherSideOfConnection();
    if (!other) return false;
    int32_t x = 0, z = 0, ox = 0, oz = 0;
    C(raw).record.GetPosition(x, z); AsGame(other)->GetPosition(ox, oz);
    return x == ox && z == oz;
}
cFC4ISC4NetworkTool* Tool(Context& c, uint32_t n) {
    auto* manager = c.city->GetNetworkManager();
    return manager ? reinterpret_cast<cFC4ISC4NetworkTool*>(manager->GetNetworkTool(static_cast<int32_t>(n), false)) : nullptr;
}
bool NetworkExists(void* p, uint32_t n) { return Tool(C(p), n) != nullptr; }
bool Find(void* p, uint32_t t, int32_t x, int32_t z) {
    return C(p).neighbors->FindConnection(static_cast<cISC4NeighborsSimulator::ConnectionType>(t), x, z) != nullptr;
}
void Change(void* p, uint32_t t, int32_t x, int32_t z, bool add, bool dir) {
    const auto type = static_cast<cISC4NeighborsSimulator::ConnectionType>(t);
    if (add) C(p).neighbors->CreateConnection(type, x, z, dir);
    else C(p).neighbors->DeleteConnection(type, x, z);
}
std::unique_ptr<SC4List<cISC4Occupant*>> Query(Context& c, int32_t x, int32_t z, uint32_t type) {
    auto out = std::make_unique<SC4List<cISC4Occupant*>>();
    auto* manager = AsGame(c.city->GetOccupantManager());
    const SC4Point<int> cell{x, z};
    QueryBorderOccupants(manager, *out, cell, type);
    return out;
}
bool Present(void* p, int32_t x, int32_t z, uint32_t kind) {
    auto& c = C(p);
    if (kind == 0) {
        auto* map = reinterpret_cast<cFC4ISC4TrafficNetworkMap*>(c.city->GetTrafficNetwork());
        return TrafficPresent(map, x, z);
    }
    auto poles = Query(c, x, z, 0x2890D4DE);
    if (!poles->empty()) return true;
    return kind == 1 && !Query(c, x, z, 0x0990B2EA)->empty();
}
void Cells(void* p, uint32_t action, const int32_t (*cells)[2], uint32_t n, uint32_t layer) {
    // A masked strip can be empty. Still issue the demolition request, as §6.2.1 requires.
    int32_t x0 = 0, z0 = 0, x1 = 0, z1 = 0;
    if (n) {
        x0 = x1 = cells[0][0]; z0 = z1 = cells[0][1];
        for (uint32_t i = 1; i < n; ++i) {
            x0 = std::min(x0, cells[i][0]); x1 = std::max(x1, cells[i][0]);
            z0 = std::min(z0, cells[i][1]); z1 = std::max(z1, cells[i][1]);
        }
    }
    const size_t width = static_cast<size_t>(x1 - x0 + 1);
    std::vector<uint8_t> mask(width * static_cast<size_t>(z1 - z0 + 1), 0);
    for (uint32_t i = 0; i < n; ++i)
        mask[static_cast<size_t>(cells[i][1] - z0) * width + static_cast<size_t>(cells[i][0] - x0)] = 1;
    fc4_zone_region input{x0, z0, x1, z1, mask.data(), mask.size()};
    ZoneRegionBuffer buffer;
    if (!buffer.Build(input)) return;
    auto& region = buffer.Game();
    if (action == 0) {
        auto* filter = new LayerFilter(layer);
        DemolishBorder(C(p).city->GetDemolitionUtility(),
            reinterpret_cast<const SC4CellRegion<int32_t>&>(region), filter);
        filter->Release();
    } else {
        auto* zones = AsGame(C(p).city->GetZoneManager());
        // SPEC-GAP: §4.6 specifies dezoning but no zone-placement policy; use the ordinary free dezone call.
        if (zones) zones->PlaceZone(region, 0,
            true, true, false, true, false, nullptr, nullptr, nullptr);
    }
}
void Segment(void* p, uint32_t n, int32_t x, int32_t z, int32_t ex, int32_t ez) {
    auto* tool = Tool(C(p), n);
    if (!tool) return;
    FC4_ADDREF(tool);
    tool->Init();
    PlaceBorderSegment(tool, x, z, ex, ez);
    tool->Shutdown();
    FC4_RELEASE(tool);
}
void Occupants(void* p, uint32_t n, int32_t x, int32_t z, void* sink, void (*emit)(void*, uint64_t)) {
    auto& c = C(p);
    auto list = Query(c, x, z, 0xC772BF98);
    for (auto* o : *list) if (o) {
        cISC4NetworkOccupant* net = nullptr;
        if (!o->QueryInterface(GZIID_cISC4NetworkOccupant, reinterpret_cast<void**>(&net)) || !net) continue;
        FC4_ACQUIRED(net, "regionalcity edge occupant");
        if (net->IsOfType(static_cast<cISC4NetworkOccupant::eNetworkType>(n))) {
            const auto token = TokenOf(o); c.networks[token] = n; emit(sink, token);
        }
        FC4_RELEASE(net);
    }
    c.lists.push_back(std::move(list)); // pin tokens until reconciliation returns
}
uint32_t Word(void* p, uint64_t token) {
    auto* o = reinterpret_cast<cISC4Occupant*>(static_cast<uintptr_t>(token));
    cISC4NetworkOccupant* net = nullptr;
    if (!o->QueryInterface(GZIID_cISC4NetworkOccupant, reinterpret_cast<void**>(&net)) || !net) return 0;
    FC4_ACQUIRED(net, "regionalcity edge word");
    const auto word = net->GetEdgeTypesFlag(static_cast<cISC4NetworkOccupant::eNetworkType>(C(p).networks[token]));
    FC4_RELEASE(net); return word;
}
void SetWord(void* p, uint64_t token, uint32_t word) {
    auto* o = reinterpret_cast<cISC4Occupant*>(static_cast<uintptr_t>(token));
    cISC4NetworkOccupant* net = nullptr;
    if (!o->QueryInterface(GZIID_cISC4NetworkOccupant, reinterpret_cast<void**>(&net)) || !net) return;
    FC4_ACQUIRED(net, "regionalcity edge word write");
    net->SetEdgeTypesFlag(static_cast<cISC4NetworkOccupant::eNetworkType>(C(p).networks[token]), word);
    FC4_RELEASE(net);
}
void Post(void*, uint32_t type) {
    cIGZMessageServer2Ptr server;
    if (!server) return;
    auto* msg = FC4MakeMessage(type, 0, 0, 0);
    server->MessagePost(msg, false); msg->Release();
}
}
void RepairLiveConnection(void* p, int32_t, int32_t, int32_t x, int32_t z, int32_t sx, int32_t sz) {
    cFC4RegionalCity geometry;
    geometry.SetPosition(x, z, false); geometry.SetCitySize(sx, sz);
    Conn(p)->Repair(geometry.Primary());
}
void Reconcile(cFC4RegionalCity& record, cISC4City* city, bool forced) {
    auto* neighbors = city ? AsGame(city->GetNeighborsSimulator()) : nullptr;
    if (!neighbors) return;
    Context context{record, city, neighbors, {}, {}};
    props::Exemplar tuning;
    std::vector<uint32_t> side;
    bool drive = true;
    if (tuning.Load(0x6534284A, 0xA998D30B, 1) && props::GetU32s(tuning.Holder(), 0x49BE9565, side) && !side.empty())
        drive = side[0] != 0;
    const FC4RegionalCityConnectionHost host{&context, ConnectionCalls(RepairLiveConnection), drive,
        OtherEstablished, Adjacent, PointsHere, NetworkExists, Find, Change, Present, Cells,
        Segment, Occupants, Word, SetWord, Post};
    fc4_regionalcity_owner_reconcile(record.Owner(), city->GetEstablished(), forced, &host);
}
}
