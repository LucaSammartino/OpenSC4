// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "fc4.h"
#include <map>
#include <tuple>
#include <vector>

namespace fc4 {
// Frozen pre-operation inputs for section 4.3/4.4. No missing read may fall
// back to the changed city. The host's CallPool retains listed occupants.
class PropBlockSnapshot {
    template<class R, class... A> struct Values {
        std::map<std::tuple<A...>, R> values;
        R Get(A... args) {
            const auto it = values.find(std::make_tuple(args...));
            if (it != values.end()) return it->second;
            active->valid = false;
            return R{};
        }
        void Put(R value, A... args) { values[std::make_tuple(args...)] = value; }
    };
    struct Config {
        bool available = false;
        std::vector<fc4_lot_cfg_object> objects;
        std::vector<std::vector<uint32_t>> ids;
        static void Sink(void* u, const fc4_lot_cfg_object* p, size_t n) {
            auto& c = *static_cast<Config*>(u);
            for (size_t i = 0; i < n; ++i) {
                c.objects.push_back(p[i]);
                c.ids.emplace_back();
                if (p[i].id_count) c.ids.back().assign(p[i].ids, p[i].ids + p[i].id_count);
            }
        }
        bool Send(fc4_lot_cfg_objects_sink sink, void* u) {
            for (size_t i = 0; i < objects.size(); ++i) objects[i].ids = ids[i].data();
            if (sink && !objects.empty()) sink(u, objects.data(), objects.size());
            return available;
        }
    };
    static void U64s(void* u, const uint64_t* p, size_t n) {
        if (n) static_cast<std::vector<uint64_t>*>(u)->insert(static_cast<std::vector<uint64_t>*>(u)->end(), p, p+n);
    }
    static void U32s(void* u, const uint32_t* p, size_t n) {
        if (n) static_cast<std::vector<uint32_t>*>(u)->insert(static_cast<std::vector<uint32_t>*>(u)->end(), p, p+n);
    }
    template<class K, class V> static V* Find(std::map<K,V>& m, K key) {
        auto it = m.find(key);
        if (it != m.end()) return &it->second;
        active->valid = false;
        return nullptr;
    }
#define FC4_FROZEN_ONE(N,R,T) \
    Values<R,T> N##Values; \
    static R N(void* u,T a) { return active ? active->N##Values.Get(a) : base.N(u,a); }
    FC4_FROZEN_ONE(lot_livable,bool,uint64_t)
    FC4_FROZEN_ONE(lot_building_type,uint32_t,uint64_t)
    FC4_FROZEN_ONE(lot_occupancy,float,uint64_t)
    FC4_FROZEN_ONE(requester_satisfaction,uint32_t,uint64_t)
    FC4_FROZEN_ONE(requester_custom,bool,uint64_t)
    FC4_FROZEN_ONE(prop_type,uint32_t,uint64_t)
    FC4_FROZEN_ONE(prop_class,uint32_t,uint64_t)
    FC4_FROZEN_ONE(prop_type_valid,bool,uint32_t)
    FC4_FROZEN_ONE(prop_satisfaction,uint32_t,uint32_t)
#undef FC4_FROZEN_ONE
    Values<uint64_t,int32_t,int32_t,bool> lots;
    Values<uint64_t,int32_t,int32_t> networks;
    Values<int32_t,uint64_t> shares;
    Values<std::pair<bool,uint32_t>,uint64_t> tags;
    std::map<uint64_t,Config> configs, netConfigs;
    std::map<uint64_t,std::vector<uint64_t>> lotProps;
    std::map<std::pair<int32_t,int32_t>,std::vector<uint64_t>> streetProps;
    std::map<uint32_t,std::vector<uint32_t>> families;
    std::map<uint64_t,fc4_props*> properties;
    uint32_t date = 0;
    bool view = false;
    static uint64_t lot_at(void* u,int32_t x,int32_t z,bool origin) {
        return active ? active->lots.Get(x,z,origin) : base.lot_at(u,x,z,origin);
    }
    static uint64_t network_lot_at(void* u,int32_t x,int32_t z) {
        return active ? active->networks.Get(x,z) : base.network_lot_at(u,x,z);
    }
    static int32_t requester_share(void* u,uint64_t r,const fc4_cell_rect* rect) {
        if (!active) return base.requester_share(u,r,rect);
        if (!rect || rect->x1 != active->bx*8 || rect->z1 != active->bz*8 ||
            rect->x2 != active->bx*8+7 || rect->z2 != active->bz*8+7) active->valid = false;
        return active->shares.Get(r);
    }
    static bool prop_lot_tag(void* u,uint64_t p,uint32_t* tag) {
        if (!active) return base.prop_lot_tag(u,p,tag);
        const auto result = active->tags.Get(p);
        if (tag) *tag = result.second;
        return result.first;
    }
    static fc4_props* prop_properties(void* u,uint64_t p) {
        if (!active) return base.prop_properties(u,p);
        const auto value = Find(active->properties,p);
        return value ? fc4_props_clone(*value) : nullptr;
    }
    static bool lot_config_objects(void* u,uint64_t l,fc4_lot_cfg_objects_sink sink,void* ctx) {
        if (!active) return base.lot_config_objects(u,l,sink,ctx);
        auto* c = Find(active->configs,l);
        return c && c->Send(sink,ctx);
    }
    static bool network_lot_objects(void* u,uint64_t l,fc4_lot_cfg_objects_sink sink,void* ctx) {
        if (!active) return base.network_lot_objects(u,l,sink,ctx);
        auto* c = Find(active->netConfigs,l);
        return c && c->Send(sink,ctx);
    }
    static void lot_props(void* u,uint64_t l,fc4_u64s_sink sink,void* ctx) {
        if (!active) { base.lot_props(u,l,sink,ctx); return; }
        auto* v = Find(active->lotProps,l);
        if (v && sink && !v->empty()) sink(ctx,v->data(),v->size());
    }
    static void street_props(void* u,int32_t x,int32_t z,fc4_u64s_sink sink,void* ctx) {
        if (!active) { base.street_props(u,x,z,sink,ctx); return; }
        auto* v = Find(active->streetProps,std::make_pair(x,z));
        if (v && sink && !v->empty()) sink(ctx,v->data(),v->size());
    }
    static void props_by_family(void* u,uint32_t id,fc4_u32s_sink sink,void* ctx) {
        if (!active) { base.props_by_family(u,id,sink,ctx); return; }
        auto* v = Find(active->families,id);
        if (v && sink && !v->empty()) sink(ctx,v->data(),v->size());
    }
    static uint32_t sim_date(void* u) { return active ? active->date : base.sim_date(u); }
    static bool view_available(void* u) { return active ? active->view : base.view_available(u); }
    void Profile(void* u,uint32_t id) {
        if (prop_type_validValues.values.count(std::make_tuple(id))) return;
        prop_type_validValues.Put(base.prop_type_valid(u,id),id);
        prop_satisfactionValues.Put(base.prop_satisfaction(u,id),id);
        if (!prop_type_validValues.Get(id)) {
            auto& members = families[id];
            base.props_by_family(u,id,&U32s,&members);
            for (auto m : members) prop_satisfactionValues.Put(base.prop_satisfaction(u,m),m);
        }
    }
    void Props(void* u,const std::vector<uint64_t>& props) {
        for (auto p : props) {
            if (properties.count(p)) continue;
            properties[p] = base.prop_properties(u,p);
            uint32_t tag = 0;
            const bool has = base.prop_lot_tag(u,p,&tag);
            tags.Put({has,tag},p);
            prop_typeValues.Put(base.prop_type(u,p),p);
            prop_classValues.Put(base.prop_class(u,p),p);
        }
    }
public:
    inline static fc4_prop_host base{};
    inline static thread_local PropBlockSnapshot* active = nullptr;
    bool valid = true;
    int32_t bx = 0, bz = 0;
    ~PropBlockSnapshot() { for (auto& p : properties) fc4_props_destroy(p.second); }
    static void Bind(fc4_prop_host& h) {
        base = h;
#define FC4_BIND(N) h.N = &N;
        FC4_BIND(lot_livable) FC4_BIND(lot_building_type) FC4_BIND(lot_occupancy)
        FC4_BIND(requester_satisfaction) FC4_BIND(requester_custom) FC4_BIND(requester_share)
        FC4_BIND(prop_type) FC4_BIND(prop_class) FC4_BIND(prop_type_valid) FC4_BIND(prop_satisfaction)
        FC4_BIND(lot_at) FC4_BIND(network_lot_at) FC4_BIND(prop_lot_tag) FC4_BIND(prop_properties)
        FC4_BIND(lot_config_objects) FC4_BIND(network_lot_objects) FC4_BIND(lot_props)
        FC4_BIND(street_props) FC4_BIND(props_by_family) FC4_BIND(sim_date) FC4_BIND(view_available)
#undef FC4_BIND
    }
    void Capture(void* u,int32_t x,int32_t z,const std::vector<fc4_prop_ref>& requesters) {
        bx=x; bz=z;
        date=base.sim_date(u); view=base.view_available(u);
        const fc4_cell_rect rect{bx*8,bz*8,bx*8+7,bz*8+7};
        for (const auto& r : requesters) if (r.token) {
            shares.Put(base.requester_share(u,r.token,&rect),r.token);
            requester_customValues.Put(base.requester_custom(u,r.token),r.token);
            requester_satisfactionValues.Put(base.requester_satisfaction(u,r.token),r.token);
        }
        for (int32_t cx=bx*8; cx<bx*8+8; ++cx) for (int32_t cz=bz*8; cz<bz*8+8; ++cz) {
            const uint64_t lot=base.lot_at(u,cx,cz,true);
            lots.Put(lot,cx,cz,true);
            lots.Put(base.lot_at(u,cx,cz,false),cx,cz,false);
            const uint64_t net=base.network_lot_at(u,cx,cz);
            networks.Put(net,cx,cz);
            Config* c=nullptr;
            if (lot) {
                lot_livableValues.Put(base.lot_livable(u,lot),lot);
                lot_building_typeValues.Put(base.lot_building_type(u,lot),lot);
                lot_occupancyValues.Put(base.lot_occupancy(u,lot),lot);
                const bool first = configs.find(lot) == configs.end();
                c=&configs[lot];
                // A lot may be returned for several cells. Snapshot its ordered
                // lists once, rather than appending another copy for each cell.
                if (first) {
                    c->available=base.lot_config_objects(u,lot,&Config::Sink,c);
                    auto& props=lotProps[lot]; base.lot_props(u,lot,&U64s,&props); Props(u,props);
                }
            } else if (net) {
                const bool first = netConfigs.find(net) == netConfigs.end();
                c=&netConfigs[net];
                if (first) c->available=base.network_lot_objects(u,net,&Config::Sink,c);
                auto& props=streetProps[{cx,cz}]; base.street_props(u,cx,cz,&U64s,&props); Props(u,props);
            }
            if (c) for (const auto& ids : c->ids) for (auto id : ids) Profile(u,id);
        }
    }
};
}
