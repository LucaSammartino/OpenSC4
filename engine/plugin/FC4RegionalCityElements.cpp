// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4RegionalCityElements.h"
#include "FC4RefAudit.h"
#include "FC4RegionalCityRecordReplay.h"
#include "cISC4NeighborConnection.h"
#include "cISC4NeighborDeal.h"
#include "cIGZSerializable.h"
#include "SC4List.h"
#include <vector>

namespace fc4::regionalcity {
namespace {
void RetainConnection(void* p) { FC4_ADDREF(static_cast<cISC4NeighborConnection*>(p)); }
void ReleaseConnection(void* p) { FC4_RELEASE(static_cast<cISC4NeighborConnection*>(p)); }
uint32_t Type(void* p) { return static_cast<uint32_t>(static_cast<cISC4NeighborConnection*>(p)->GetType()); }
void Cell(void* p, int32_t* x, int32_t* z) { static_cast<cISC4NeighborConnection*>(p)->GetConnectionCell(*x, *z); }
void RetainDeal(void* p) { FC4_ADDREF(static_cast<cISC4NeighborDeal*>(p)); }
void ReleaseDeal(void* p) { FC4_RELEASE(static_cast<cISC4NeighborDeal*>(p)); }
void CitySide(void* p, int32_t* x, int32_t* z) {
    static_cast<cISC4NeighborDeal*>(p)->GetRegionCellOnCitySideOfConnection(*x, *z);
}
void OtherSide(void* p, int32_t* x, int32_t* z) {
    static_cast<cISC4NeighborDeal*>(p)->GetRegionCellOnOtherSideOfConnection(*x, *z);
}
void SetCitySide(void* p, int32_t x, int32_t z) {
    static_cast<cISC4NeighborDeal*>(p)->SetRegionCellOnCitySideOfConnection(x, z);
}
void SetOtherSide(void* p, int32_t x, int32_t z) {
    static_cast<cISC4NeighborDeal*>(p)->SetRegionCellOnOtherSideOfConnection(x, z);
}
uint32_t DealType(void* p) { return static_cast<cISC4NeighborDeal*>(p)->GetType(); }
int32_t Amount(void* p) { return static_cast<cISC4NeighborDeal*>(p)->GetAmount(); }
uint32_t Flags(void* p) { return static_cast<cISC4NeighborDeal*>(p)->GetFlags(); }
bool SetFlags(void* p, uint32_t mask, bool on) {
    static_cast<cISC4NeighborDeal*>(p)->SetFlags(mask, on);
    return true;
}
void* Read(void*, uint32_t kind, const fc4_istream* stream) {
    // The existing neighbour implementations own record decoding and property
    // holders. Return the PRIMARY interface, never a concrete-object layout.
    auto* p = regionalcity_replay::Read(nullptr, kind, stream);
    if (!p) return nullptr;
    if (kind == 0) return static_cast<cISC4NeighborConnection*>(static_cast<cFC4NeighborConnection*>(p));
    return static_cast<cISC4NeighborDeal*>(static_cast<cFC4NeighborDeal*>(p));
}
int32_t Write(void* raw, uint32_t kind, void* object, const fc4_ostream*) {
    const auto* context = static_cast<RecordContext*>(raw);
    if (!context || !context->output || !object) return 0;
    cIGZUnknown* primary = kind == 0
        ? static_cast<cIGZUnknown*>(static_cast<cISC4NeighborConnection*>(object))
        : static_cast<cIGZUnknown*>(static_cast<cISC4NeighborDeal*>(object));
    cIGZSerializable* serial = nullptr;
    if (!primary->QueryInterface(0xE4FDA3D4, reinterpret_cast<void**>(&serial)) || !serial) return -1;
    FC4_ACQUIRED(serial, "regionalcity element serializable");
    const bool ok = serial->Write(*context->output);
    FC4_RELEASE(serial);
    return ok ? 1 : 0;
}
} // namespace
FC4RegionalCityConnectionCalls ConnectionCalls(Repair repair) {
    return {RetainConnection, ReleaseConnection, Type, Cell, repair};
}
const FC4RegionalCityDealCalls& DealCalls() {
    static const FC4RegionalCityDealCalls calls = {
        RetainDeal, ReleaseDeal, CitySide, OtherSide, SetCitySide, SetOtherSide, DealType, Amount, Flags, SetFlags
    };
    return calls;
}
FC4RegionalCityRecordCalls RecordCalls(RecordContext* context, Repair repair) {
    return {context, Read, Write, ConnectionCalls(repair), DealCalls()};
}
bool CheckLiveRecord(const WriteCapture& capture, cFC4ISC4RegionalCity* record) {
    CaptureIStream input(capture);
    WriteCapture output;
    OStreamAdapter stream(output);
    RecordContext context{nullptr, &output};
    const auto calls = RecordCalls(&context, regionalcity_replay::RepairConnection);
    auto* owner = fc4_regionalcity_owner_create(0);
    if (!owner) return false;
    bool ok = fc4_regionalcity_owner_decode(owner, input.Get(), 0, &calls) && input.good() && input.atEnd();
    if (ok) {
        SC4List<cISC4NeighborConnection*> connections;
        SC4List<cISC4NeighborDeal*> deals;
        record->GetAllConnections(connections);
        record->GetLocalDeals(deals);
        std::vector<void*> cp, dp;
        for (auto* p : connections) cp.push_back(p);
        for (auto* p : deals) dp.push_back(p);
        ok = fc4_regionalcity_owner_connections(owner, cp.data(), static_cast<uint32_t>(cp.size()), &calls.connections)
            && fc4_regionalcity_owner_deals(owner, true, dp.data(), static_cast<uint32_t>(dp.size()), &calls.deals)
            && fc4_regionalcity_owner_write(owner, stream.Get(), &calls)
            && output.Bytes() == capture.Bytes();
    }
    fc4_regionalcity_owner_destroy(owner);
    return NeighborsCollectReleasedElements() && ok;
}
} // namespace fc4::regionalcity
