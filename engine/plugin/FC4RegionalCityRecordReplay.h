// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4RegionalCityState.h"
#include "FC4WriteCapture.h"
#include "cFC4Neighbors.h"

namespace fc4::regionalcity_replay {
// Isolated shadow elements, never game-owned neighbour objects. The production
// owner transport accepts any host interface through its supplied callbacks.
inline void RetainConnection(void* p) { static_cast<cFC4NeighborConnection*>(p)->AddRef(); }
inline void ReleaseConnection(void* p) { static_cast<cFC4NeighborConnection*>(p)->Release(); }
inline uint32_t ConnectionType(void* p) { return static_cast<uint32_t>(static_cast<cFC4NeighborConnection*>(p)->GetType()); }
inline void ConnectionCell(void* p, int32_t* x, int32_t* z) {
    static_cast<cFC4NeighborConnection*>(p)->GetConnectionCell(*x, *z);
}
inline void RepairConnection(void* p, int32_t cx, int32_t cz, int32_t px, int32_t pz, int32_t sx, int32_t sz) {
    fc4_neighbor_connection_repair(static_cast<cFC4NeighborConnection*>(p)->Handle(), cx, cz, px, pz, sx, sz);
}
inline void RetainDeal(void* p) { static_cast<cFC4NeighborDeal*>(p)->AddRef(); }
inline void ReleaseDeal(void* p) { static_cast<cFC4NeighborDeal*>(p)->Release(); }
inline void CitySide(void* p, int32_t* x, int32_t* z) {
    static_cast<cFC4NeighborDeal*>(p)->GetRegionCellOnCitySideOfConnection(*x, *z);
}
inline void OtherSide(void* p, int32_t* x, int32_t* z) {
    static_cast<cFC4NeighborDeal*>(p)->GetRegionCellOnOtherSideOfConnection(*x, *z);
}
inline void SetCitySide(void* p, int32_t x, int32_t z) {
    static_cast<cFC4NeighborDeal*>(p)->SetRegionCellOnCitySideOfConnection(x, z);
}
inline void SetOtherSide(void* p, int32_t x, int32_t z) {
    static_cast<cFC4NeighborDeal*>(p)->SetRegionCellOnOtherSideOfConnection(x, z);
}
inline uint32_t DealType(void* p) { return static_cast<cFC4NeighborDeal*>(p)->GetType(); }
inline int32_t Amount(void* p) { return static_cast<cFC4NeighborDeal*>(p)->GetAmount(); }
inline uint32_t Flags(void* p) { return static_cast<cFC4NeighborDeal*>(p)->GetFlags(); }
inline bool SetFlags(void* p, uint32_t flags, bool on) {
    static_cast<cFC4NeighborDeal*>(p)->SetFlags(flags, on);
    return true;
}
inline void* Read(void*, uint32_t kind, const fc4_istream* input) {
    if (kind == 0) {
        auto* h = fc4_neighbor_connection_create();
        if (!h) return nullptr;
        if (!fc4_neighbor_connection_read(h, input)) {
            fc4_neighbor_connection_destroy(h);
            return nullptr;
        }
        auto* c = new cFC4NeighborConnection(h); // count 1; transferred to owner
        fc4_neighbor_connection_props(h, &cFC4PropertyHolder::FromCoreProperty, &c->Holder());
        return c;
    }
    auto* h = fc4_neighbor_deal_create();
    if (!h) return nullptr;
    if (!fc4_neighbor_deal_read(h, input)) {
        fc4_neighbor_deal_destroy(h);
        return nullptr;
    }
    auto* d = new cFC4NeighborDeal(h); // count 0 until the factory takes its ref
    d->AddRef();
    return d;
}
inline int32_t Write(void*, uint32_t kind, void* p, const fc4_ostream* output) {
    return kind == 0 ? fc4_neighbor_connection_write(static_cast<cFC4NeighborConnection*>(p)->Handle(), output)
                     : fc4_neighbor_deal_write(static_cast<cFC4NeighborDeal*>(p)->Handle(), output);
}
inline const FC4RegionalCityRecordCalls& Calls() {
    static const FC4RegionalCityRecordCalls calls = {
        nullptr, Read, Write,
        {RetainConnection, ReleaseConnection, ConnectionType, ConnectionCell, RepairConnection},
        {RetainDeal, ReleaseDeal, CitySide, OtherSide, SetCitySide, SetOtherSide, DealType, Amount, Flags, SetFlags}
    };
    return calls;
}
inline bool Roundtrip(const WriteCapture& capture) {
    CaptureIStream input(capture);
    WriteCapture output;
    OStreamAdapter out(output);
    auto* owner = fc4_regionalcity_owner_create(0);
    if (!owner) return false;
    const bool ok = fc4_regionalcity_owner_decode(owner, input.Get(), 0, &Calls())
        && input.good() && input.atEnd()
        && fc4_regionalcity_owner_write(owner, out.Get(), &Calls())
        && output.Bytes() == capture.Bytes();
    fc4_regionalcity_owner_destroy(owner);
    const bool collected = NeighborsCollectReleasedElements();
    return ok && collected;
}
} // namespace fc4::regionalcity_replay
