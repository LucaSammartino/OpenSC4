// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4PropDevSnapshot.h"
#include <cstdio>

// This fixture owns no property holders; the snapshot must preserve null replies.
extern "C" fc4_props* fc4_props_clone(const fc4_props*) { return nullptr; }
extern "C" void fc4_props_destroy(fc4_props*) {}
namespace {
unsigned configs = 0, lists = 0, emptyConfigs = 0;
uint64_t Lot(void*, int32_t x, int32_t, bool) { return x < 4 ? 1 : 0; }
uint64_t Network(void*, int32_t x, int32_t) { return x >= 4 ? 2 : 0; }
bool Config(void*, uint64_t, fc4_lot_cfg_objects_sink sink, void* u) {
    ++configs;
    const uint32_t ids[]{17,23,17};
    fc4_lot_cfg_object object{};
    object.ids = ids; object.id_count = 3;
    sink(u,&object,1); return true;
}
bool EmptyConfig(void*, uint64_t, fc4_lot_cfg_objects_sink, void*) { ++emptyConfigs; return true; }
void Props(void*, uint64_t, fc4_u64s_sink sink, void* u) {
    ++lists; const uint64_t p[]{10,11}; sink(u,p,2);
}
void Street(void*, int32_t, int32_t, fc4_u64s_sink, void*) {}
bool BoolToken(void*, uint64_t) { return true; }
uint32_t UintToken(void*, uint64_t) { return 0; }
uint32_t UintId(void*, uint32_t) { return 0; }
bool Valid(void*, uint32_t) { return true; }
float Occupancy(void*, uint64_t) { return 1; }
fc4_props* Properties(void*, uint64_t) { return nullptr; }
bool Tag(void*, uint64_t, uint32_t*) { return false; }
uint32_t Date(void*) { return 0; }
bool View(void*) { return true; }
void Objects(void* u, const fc4_lot_cfg_object* p, size_t n) {
    auto* count=static_cast<unsigned*>(u);
    if (n==1 && p[0].id_count==3 && p[0].ids[0]==17 && p[0].ids[2]==17) ++*count;
}
void Tokens(void* u,const uint64_t*,size_t n) { *static_cast<size_t*>(u)+=n; }
}
int main() {
    fc4_prop_host h{};
    h.lot_at=&Lot; h.network_lot_at=&Network;
    h.lot_config_objects=&Config; h.network_lot_objects=&EmptyConfig;
    h.lot_props=&Props; h.street_props=&Street;
    h.lot_livable=&BoolToken; h.lot_building_type=&UintToken; h.lot_occupancy=&Occupancy;
    h.prop_type=&UintToken; h.prop_class=&UintToken;
    h.prop_type_valid=&Valid; h.prop_satisfaction=&UintId;
    h.prop_properties=&Properties; h.prop_lot_tag=&Tag;
    h.sim_date=&Date; h.view_available=&View;
    fc4::PropBlockSnapshot::Bind(h);
    fc4::PropBlockSnapshot snapshot;
    snapshot.Capture(nullptr,0,0,{});
    if (configs!=1 || lists!=1 || emptyConfigs!=1) return 1;
    fc4::PropBlockSnapshot::active=&snapshot;
    unsigned objects=0; size_t tokens=0;
    if (!h.lot_config_objects(nullptr,1,&Objects,&objects)) return 2;
    h.lot_props(nullptr,1,&Tokens,&tokens);
    if (objects!=1 || tokens!=2 || !snapshot.valid) return 3;
    h.lot_props(nullptr,99,&Tokens,&tokens);
    if (snapshot.valid) return 4;
    fc4::PropBlockSnapshot::active=nullptr;
    std::puts("propdev snapshot: repeated lot and empty network config captured once; missing input invalidates");
    return 0;
}
