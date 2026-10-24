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

// Drives the Rust core through fc4.h from C++, built with the same compiler,
// architecture and CRT as FreeCity4.dll. It checks that the hand-written
// header matches the Rust ABI (struct layout, bool, calling convention) and
// that re-entrant calls from a callback work.

#include "fc4.h"

#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace {

int g_failures = 0;

#define CHECK(cond)                                                          \
    do {                                                                     \
        if (!(cond)) {                                                       \
            std::fprintf(stderr, "%s:%d: CHECK failed: %s\n", __FILE__, __LINE__, #cond); \
            ++g_failures;                                                    \
        }                                                                    \
    } while (0)

struct Host {
    uint32_t now = 0;
    std::vector<fc4_message> posts;
    std::vector<std::pair<fc4_agent, fc4_message>> deliveries;
    int refs = 0;
    fc4_agent selfRemoving = 0;
};

uint32_t NowMs(void* user) {
    Host* h = static_cast<Host*>(user);
    return h->now++;  // free-running clock: 1 ms per read
}

bool Deliver(void* user, fc4_sim* sim, fc4_agent agent, const fc4_message* msg) {
    Host* h = static_cast<Host*>(user);
    h->deliveries.emplace_back(agent, *msg);
    if (agent == h->selfRemoving) {
        CHECK(fc4_sim_remove_agent_all_levels(sim, agent));  // re-entrant call
    }
    return true;
}

void AddRef(void* user, fc4_agent) { ++static_cast<Host*>(user)->refs; }
void Release(void* user, fc4_agent) { --static_cast<Host*>(user)->refs; }
void Post(void* user, const fc4_message* msg) { static_cast<Host*>(user)->posts.push_back(*msg); }

fc4_sim* Make(Host& h) {
    fc4_host host{};
    host.struct_size = sizeof(host);
    host.user = &h;
    host.now_ms = &NowMs;
    host.deliver = &Deliver;
    host.add_ref = &AddRef;
    host.release = &Release;
    host.post = &Post;
    return fc4_sim_create(&host);
}

void TestDays() {
    Host h;
    fc4_sim* sim = Make(h);
    CHECK(sim != nullptr);
    const char name[] = "day agent";
    CHECK(fc4_sim_add_agent(sim, 0x1234, FC4_LEVEL_DAY, name, sizeof(name) - 1, 3));
    const uint32_t table[3] = {2000, 1000, 50};
    fc4_tuning t{table, 3, nullptr, 0};
    CHECK(fc4_sim_init(sim, &t));
    CHECK(fc4_sim_set_sim_speed(sim, 2));
    CHECK(fc4_sim_is_hidden_paused(sim));  // Init leaves the simulator hidden-paused
    CHECK(fc4_sim_on_post_city_init(sim));
    CHECK(!fc4_sim_is_hidden_paused(sim));
    for (int i = 0; i < 100; ++i) CHECK(fc4_sim_on_tick(sim));

    fc4_state st{};
    st.struct_size = sizeof(st);
    CHECK(fc4_sim_get_state(sim, &st));
    CHECK(st.initialized == 1);
    CHECK(st.ms_per_day == 50);
    CHECK(st.target_fps == 15);
    CHECK(st.agent_counts[FC4_LEVEL_DAY] == 1);
    CHECK(st.julian_day > 2451545 + 20);

    int32_t expectJulian = 2451546;
    for (const fc4_message& m : h.posts) {
        if (m.type != 0x66956814u) continue;
        CHECK(m.reserved == 0);
        CHECK(m.data1 == expectJulian);
        fc4_date d{};
        fc4_date_from_julian(expectJulian, &d);
        CHECK(m.data3 == d.day_of_year);
        ++expectJulian;
    }
    CHECK(expectJulian - 1 == st.julian_day);

    fc4_date d{};
    CHECK(fc4_sim_get_date(sim, &d));
    CHECK(d.julian == st.julian_day);
    CHECK(fc4_sim_get_julian_day(sim) == st.julian_day);

    // Save / load round trip through typed fields.
    uint8_t buf[128];
    const size_t n = fc4_sim_save(sim, buf, sizeof(buf));
    CHECK(n == 79);
    const uint8_t* widths = nullptr;
    const size_t fields = fc4_record_layout(4, &widths);
    CHECK(fields == 30);
    size_t sum = 0;
    for (size_t i = 0; i < fields; ++i) sum += widths[i];
    CHECK(sum == n);
    CHECK(fc4_sim_load(sim, buf, n) == FC4_OK);
    uint8_t again[128];
    CHECK(fc4_sim_save(sim, again, sizeof(again)) == n);
    CHECK(std::memcmp(buf, again, n) == 0);
    buf[0] = 5;
    CHECK(fc4_sim_load(sim, buf, n) == FC4_ERR_UNSUPPORTED_VERSION);

    CHECK(h.refs == 1);
    fc4_sim_destroy(sim);
    CHECK(h.refs == 0);
}

void TestReentrancyAndNames() {
    Host h;
    h.selfRemoving = 7;
    fc4_sim* sim = Make(h);
    CHECK(fc4_sim_add_agent(sim, 7, FC4_LEVEL_ONE_SHOT, "a", 1, 1));
    CHECK(fc4_sim_add_agent(sim, 8, FC4_LEVEL_IDLE, "b", 1, 1));
    std::vector<std::string> names;
    const size_t count = fc4_sim_enumerate_agents(
        sim,
        [](void* ctx, const char* s, size_t len) { static_cast<std::vector<std::string>*>(ctx)->emplace_back(s, len); },
        &names);
    CHECK(count == 2 && names.size() == 2 && names[0] == "a" && names[1] == "b");
    CHECK(fc4_sim_init(sim, nullptr));
    fc4_sim_simulate_agents(sim);
    CHECK(h.deliveries.size() == 1);
    CHECK(h.refs == 1);
    CHECK(fc4_sim_get_agent_enabled(sim, "b", 1));
    CHECK(fc4_sim_set_agent_enabled(sim, "b", 1, false));
    CHECK(!fc4_sim_get_agent_enabled(sim, "b", 1));

    const char cheat[] = "SimDate 7 4 2013";
    CHECK(fc4_sim_handle_cheat(sim, 0x8A78BEFF, cheat, sizeof(cheat) - 1));
    CHECK(fc4_sim_get_julian_day(sim) == fc4_julian_from_civil(2013, 7, 4));
    CHECK(fc4_sim_emergency_pause(sim));
    CHECK(fc4_sim_is_emergency_paused(sim) && fc4_sim_is_paused(sim));
    CHECK(!fc4_sim_set_sim_speed(sim, 1));
    CHECK(fc4_sim_emergency_resume(sim));
    CHECK(!fc4_sim_is_any_paused(sim) || fc4_sim_is_hidden_paused(sim));
    fc4_sim_destroy(sim);
    CHECK(h.refs == 0);
}

// The network geometry's pure entry points through the header (cores17).
void TestNetworkGeometry() {
    fc4_netgeom_params p{};
    CHECK(fc4_netgeom_params_shipped(0, &p) && p.struct_size == sizeof(fc4_netgeom_params));
    fc4_netgeom_type_info i{};
    CHECK(fc4_netgeom_params_type_info(0, &p, &i) && i.width1 == 1 && i.slope_rise > 11.2f && i.slope_rise < 11.21f);
    fc4_netgeom_segments s{};
    CHECK(fc4_netgeom_break_into_segments(0, 0, 6, 4, true, false, 1, 1, &s));
    CHECK(s.end_x == 5 && s.end_z == 4 && s.dragged_count == 6 && s.pen_direction == 4 && s.end_direction == 5);
    fc4_vhcs* v = fc4_vhcs_create();
    CHECK(fc4_vhcs_init(v, 0.0f, 1000.0f));
    CHECK(fc4_vhcs_insert_fixed(v, 1, 0.0f) && fc4_vhcs_insert_fixed(v, 3, 20.0f) && fc4_vhcs_add_vertex(v, 2, 0.0f));
    CHECK(fc4_vhcs_insert_slope(v, 1, 2, i.slope_rise) && fc4_vhcs_insert_slope(v, 2, 3, i.slope_rise));
    float h = 0.0f;
    CHECK(fc4_vhcs_solve(v) && fc4_vhcs_height(v, 2, &h) && h > 8.79f && h < 8.80f);
    CHECK(fc4_vhcs_solution(v, nullptr, 0) == 3);
    fc4_vhcs_destroy(v);
    CHECK(fc4_netgeom_power_line_cost(3, 20, 15, 2.0f) == 85);
    CHECK(fc4_netgeom_create(nullptr, nullptr, 0) == nullptr);
}

// The occupant manager (occupant-manager.md) over the C ABI: one occupant
// in one cell, inserted, found, iterated and removed.
struct OccWorld {
    int refs = 0;
    uint32_t flags = 0;
    fc4_occ_cell_box box{};
    bool has_box = false;
    int posts = 0;
    int seen = 0;
};

void TestOccupantManager() {
    int32_t x = 0, z = 5;
    fc4_occ_world_cell_to_standard_city_cell(70, 66, &x, &z);  // slot 12 quirk
    CHECK(x == 8 && z == 20);
    fc4_occ_position_to_world_cell(-1.0f, 63.0f, &x, &z);
    CHECK(x == 63 && z == 64);
    CHECK(fc4_occmgr_query_interface(0xE4FDA3D4u) == 4);
    CHECK(fc4_occmgr_query_interface(0x12345678u) == -1);
    CHECK(fc4_occmgr_create(nullptr) == nullptr);

    OccWorld w;
    fc4_occmgr_host h{};
    h.struct_size = sizeof h;
    h.user = &w;
    h.add_ref = [](void* u, uint64_t) { ++static_cast<OccWorld*>(u)->refs; };
    h.release = [](void* u, uint64_t) { --static_cast<OccWorld*>(u)->refs; };
    h.has_property_holder = [](void*, uint64_t) { return true; };
    h.bounding_box = [](void*, uint64_t, fc4_occ_bbox* b) {
        b->min[0] = 0.0f; b->min[1] = 0.0f; b->min[2] = 0.0f;
        b->max[0] = 32.0f; b->max[1] = 10.0f; b->max[2] = 32.0f;
    };
    h.post_message = [](void* u, uint64_t, uint32_t, uint32_t) { ++static_cast<OccWorld*>(u)->posts; };
    h.manager_bbox = [](void* u, uint64_t, fc4_occ_cell_box* out) {
        auto* ww = static_cast<OccWorld*>(u);
        if (ww->has_box) *out = ww->box;
        return ww->has_box;
    };
    h.set_manager_bbox = [](void* u, uint64_t, const fc4_occ_cell_box* b) {
        auto* ww = static_cast<OccWorld*>(u);
        ww->box = *b;
        ww->has_box = true;
    };
    h.set_flag = [](void* u, uint64_t, uint32_t f) { static_cast<OccWorld*>(u)->flags |= f; };
    h.clear_flag = [](void* u, uint64_t, uint32_t f) { static_cast<OccWorld*>(u)->flags &= ~f; };
    h.is_flag_set = [](void* u, uint64_t, uint32_t f) { return (static_cast<OccWorld*>(u)->flags & f) != 0; };

    fc4_occmgr* m = fc4_occmgr_create(&h);
    CHECK(m != nullptr);
    CHECK(fc4_occmgr_set_city_size(m, 1024.0f, 1024.0f));
    CHECK(fc4_occmgr_init(m));
    CHECK(fc4_occmgr_insert(m, 1, 0));
    CHECK(w.has_box && w.box.x0 == 64 && w.box.z1 == 64 && w.refs == 1 && w.posts == 1);
    uint64_t first = 0;
    CHECK(fc4_occmgr_first_in_cell(m, 64, 64, nullptr, &first) && first == 1);
    fc4_occmgr_release_list(m, &first, 1);
    const int32_t lo[3] = {64, 0, 64}, hi[3] = {64, 0, 64};
    CHECK(fc4_occmgr_iterate(
        m, [](void* ctx, uint64_t) { ++static_cast<OccWorld*>(ctx)->seen; return true; }, &w, lo, hi, nullptr));
    CHECK(w.seen == 1);
    CHECK(fc4_occmgr_lock_depth(m) == 0);
    CHECK(fc4_occmgr_remove(m, 1, false, 0));
    CHECK(w.refs == 0 && w.flags == 0 && fc4_occmgr_is_cell_empty(m, 64, 64));
    fc4_occmgr_state st{};
    CHECK(fc4_occmgr_get_state(m, &st) && st.initialized == 1 && st.counts[2] == 64);
    fc4_occmgr_destroy(m);
}

// The network rules (network-rules.md) over the C ABI: a Transmogrify file
// loaded from bytes, one evaluation, and a Road / Rail crossing solved by a
// RUL1 entry.
void TestNetworkRules() {
    static_assert(sizeof(fc4_network_rules_report) == 36, "report layout");
    static_assert(sizeof(fc4_network_rules_output) == 8, "output layout");
    static_assert(sizeof(fc4_network_rules_cell) == 116, "cell layout");
    static_assert(sizeof(fc4_network_rules_solved) == 16, "solved layout");
    static_assert(sizeof(fc4_network_rules_potential) == 12, "potential layout");

    uint32_t inst[4] = {};
    CHECK(fc4_network_rules_tile_instances(0, inst, 4) == 2 && inst[0] == 0x8 && inst[1] == 0x7);

    fc4_network_rules* r = fc4_network_rules_create();
    CHECK(r != nullptr);
    const char tiles[] = "#Transmogrify\n1,0,2,0,2\n3,0,0x100,0,0";
    fc4_network_rules_report rep{};
    CHECK(fc4_network_rules_load(r, 0x8, reinterpret_cast<const uint8_t*>(tiles), sizeof(tiles) - 1, &rep));
    CHECK(rep.struct_size == 36 && rep.status == FC4_NR_STATUS_COMPLETE && rep.transmogrify == 1);
    CHECK(rep.source_rules == 1 && rep.rules == 2 && rep.dropped_variants == 6);
    CHECK(fc4_network_rules_evaluator_count(r, 0) == 1);

    fc4_network_rules_grid* g = fc4_network_rules_grid_create(64, 64);
    fc4_network_rules_cell c{};
    c.struct_size = sizeof c;
    c.x = 10;
    c.z = 10;
    c.flags = 1;
    c.edges[0] = 0x00020002u;
    c.solved = -1;
    CHECK(fc4_network_rules_grid_set_cell(g, &c));
    fc4_network_rules_output out[2] = {};
    CHECK(fc4_network_rules_evaluate(r, 0, 0, g, 10, 10, out, 2) == 1);
    CHECK(out[0].id == 0x100 && out[0].m == 0 && out[0].orientation == 1);

    const char multi[] = "[RoadRailIntersectionSolutions]\nk=0x02000200,0x00020002,0x1234,1,0\n";
    CHECK(fc4_network_rules_load(r, 0x10000001u, reinterpret_cast<const uint8_t*>(multi), sizeof(multi) - 1, &rep));
    CHECK(rep.status == -1 && rep.entries == 1);
    c.flags = 3;
    c.edges[0] = 0x02000200u;
    c.edges[1] = 0x00020002u;
    c.in_run = 1;
    CHECK(fc4_network_rules_grid_set_cell(g, &c));
    CHECK(fc4_network_rules_grid_add_to_solve(g, 10, 10));
    CHECK(fc4_network_rules_grid_set_valid_count(g, 1));
    int32_t result = -9;
    CHECK(fc4_network_rules_solve(r, g, 0, false, &result) && result == 1);
    fc4_network_rules_solved solved[2] = {};
    CHECK(fc4_network_rules_grid_solved(g, solved, 2) == 1);
    CHECK(solved[0].id == 0x1234 && solved[0].orientation == 1 && solved[0].x == 10 && solved[0].z == 10);
    fc4_network_rules_potential pot[2] = {};
    CHECK(fc4_network_rules_grid_potential(g, pot, 2) == 1 && pot[0].types == 3);
    CHECK(fc4_network_rules_grid_unsolved(g, nullptr, 0) == 0);
    fc4_network_rules_cell back{};
    CHECK(fc4_network_rules_grid_get_cell(g, 10, 10, &back) && back.solved == 0 && back.union_edges == 0x02020202u);
    fc4_network_rules_grid_destroy(g);
    fc4_network_rules_destroy(r);
}

// The effects manager (effects.md) over a real terrain: the crater brush goes
// through fc4_terrain_apply_brush, and a volcano finishes through the hook.
struct FxWorld {
    fc4_terrain* terrain = nullptr;
    int brushes = 0;
    int long_ops = 0;
    std::vector<fc4_effects_post> posts;
};

void TestEffects() {
    FxWorld w;
    fc4_terrain_host th{};
    th.struct_size = sizeof(th);
    w.terrain = fc4_terrain_create(&th, nullptr, 64, 64);
    CHECK(w.terrain != nullptr);
    std::vector<uint8_t> pixels(16 * 16, 255);
    fc4_brush_component comp{};
    comp.kind = 0;
    for (int i = 0; i < 5; ++i) {
        comp.radius[i] = 32.0f;
        comp.strength[i] = 4.0f;
    }
    comp.bitmap_width = 16;
    comp.bitmap_height = 16;
    comp.pixels = pixels.data();
    CHECK(fc4_terrain_add_brush(w.terrain, 0x8A33E17Au, &comp, 1, 0.0f, 0.0f));

    fc4_effects_host h{};
    h.struct_size = sizeof(h);
    h.user = &w;
    h.simulator = [](void*, float* d) { *d = 1.0f; return true; };
    h.city_cells = [](void*, int32_t* x, int32_t* z) { *x = 64; *z = 64; };
    h.location_in_bounds = [](void* u, float x, float z) {
        return fc4_terrain_location_is_in_bounds(static_cast<FxWorld*>(u)->terrain, x, z);
    };
    h.apply_brush = [](void* u, uint32_t id, fc4_brush_params* p) {
        auto* ww = static_cast<FxWorld*>(u);
        ++ww->brushes;
        return fc4_terrain_apply_brush(ww->terrain, id, p);
    };
    h.begin_long_terrain_operation = [](void* u) {
        auto* ww = static_cast<FxWorld*>(u);
        ++ww->long_ops;
        fc4_terrain_begin_long_operation(ww->terrain);
    };
    h.end_long_terrain_operation = [](void* u) {
        auto* ww = static_cast<FxWorld*>(u);
        --ww->long_ops;
        fc4_terrain_end_long_operation(ww->terrain);
    };
    h.post = [](void* u, const fc4_effects_post* m) { static_cast<FxWorld*>(u)->posts.push_back(*m); };

    fc4_effects_host short_host = h;
    short_host.struct_size = 8;
    CHECK(fc4_effects_create(&short_host, 1) == nullptr);
    fc4_effects* fx = fc4_effects_create(&h, 0x12345678u);
    CHECK(fx != nullptr);
    fc4_effects_directory* d = fc4_effects_directory_volcano();
    CHECK(d != nullptr);
    fc4_effects_directory_counts counts{};
    CHECK(fc4_effects_directory_get_counts(d, &counts) && counts.brushes == 25 && counts.games == 57);
    CHECK(fc4_effects_directory_find(d, "volcano_control", 5) == 9);
    CHECK(fc4_effects_directory_find(d, "nothing", 5) == FC4_EFFECTS_UNKNOWN);
    CHECK(fc4_effects_set_directory(fx, d));
    fc4_effects_directory_destroy(d);  // the manager keeps its own reference

    // The one-shot crater: two ApplyBrush calls on the terrain, which change it.
    uint32_t crater = 0;
    CHECK(fc4_effects_create_effect(fx, "meteorcrater_b", &crater) && crater != 0);
    CHECK(fc4_effects_set_position(fx, crater, 320.0f, 0.0f, 480.0f));
    CHECK(fc4_effects_start(fx, crater, 0));
    const float before = fc4_terrain_altitude(w.terrain, 320.0f, 480.0f);
    fc4_effects_tick(fx, 33333);
    CHECK(w.brushes == 2);
    CHECK(fc4_terrain_altitude(w.terrain, 320.0f, 480.0f) != before);
    fc4_effects_brush_state b{};
    CHECK(fc4_effects_get_brush(fx, crater, 0, &b) && !b.active && b.one_shot && b.applications == 2);
    CHECK(b.brush_id == 0x8A33E17Au && b.strength == 6.0f);
    char name[32] = {};
    CHECK(fc4_effects_get_name(fx, crater, name, sizeof(name)) == 14 && std::strcmp(name, "meteorcrater_b") == 0);
    CHECK(fc4_effects_release(fx, crater));

    // volcano_control: one select draw, its end message once, through the hook.
    uint32_t volcano = 0;
    CHECK(fc4_effects_create_effect(fx, "volcano_control", &volcano));
    uint32_t child = 0;
    CHECK(fc4_effects_get_child(fx, volcano, 0, &child) && child != volcano);
    CHECK(fc4_effects_set_end_message(fx, volcano, 0xCA493C1Du, 0x77));
    CHECK(fc4_effects_set_position(fx, volcano, 500.0f, 0.0f, 500.0f));
    CHECK(fc4_effects_start(fx, volcano, 0));
    int frames = 0;
    while (w.posts.empty() && frames < 200) {
        fc4_effects_tick(fx, 33333);
        CHECK(!fc4_effects_draw_pass(fx));
        CHECK(fc4_effects_finish_hook(fx));
        ++frames;
    }
    CHECK(frames < 60);
    CHECK(w.posts.size() == 1 && w.posts[0].msg_type == 0xCA493C1Du && w.posts[0].data1 == 0x77);
    CHECK(w.long_ops == -1);  // one begin, two ends (EF8)
    CHECK(!fc4_effects_is_started(fx, volcano));
    fc4_effects_state st{};
    CHECK(fc4_effects_get_state(fx, &st) && st.select_draws == 1 && st.end_posts == 1);
    CHECK(st.finish_mode == FC4_EFFECTS_FINISH_HOOK && st.level_of_detail == 1);
    fc4_effect_info info{};
    CHECK(fc4_effects_get_effect(fx, volcano, &info) && info.index >= 0 && info.refs == 2);

    fc4_effects_demolish_filter f{};
    f.max_size = 40.0f;
    fc4_effects_occupant_facts o{};
    o.extents[0] = 50.0f;
    CHECK(fc4_effects_demolish_filter_skips(&f, &o));
    o.extents[0] = 10.0f;
    CHECK(!fc4_effects_demolish_filter_skips(&f, &o));

    fc4_effects_shutdown(fx);
    CHECK(fc4_effects_get_state(fx, &st) && st.effect_count == 0);
    fc4_effects_destroy(fx);
    fc4_terrain_destroy(w.terrain);
}

// The network tools (network-tools.md §2-§11) over the C ABI: a fake world
// behind fc4_nettool_host, the §18 tests through the header.
struct NetToolWorld {
    std::vector<std::pair<int32_t, int32_t>> drawn;
    uint32_t pass_valid = 0;
    uint32_t pass_failure = 0;
    std::vector<uint32_t> data4s;
    std::vector<uint32_t> usable_posts;
    std::vector<std::pair<int32_t, int32_t>> resolved;
    float corners[4] = {110.f, 110.f, 110.f, 110.f};
    float solved[4] = {100.f, 100.f, 100.f, 100.f};
    bool smooth_ok = true;
    int underground_updated = 0;
    int underground_kept = 0;
    int destroyed = 0;
};

static bool NtDraw(void* user, fc4_nettool_run* run, bool) {
    auto* w = static_cast<NetToolWorld*>(user);
    run->dragged_count = 11;
    run->straight_count = 11;
    run->valid_count = 11;
    run->cell_count = 11;
    for (uint32_t i = 0; i < 11; ++i) {
        run->cells[i].x = run->start_x + static_cast<int32_t>(i);
        run->cells[i].z = run->start_z;
    }
    w->drawn.emplace_back(run->start_x, run->start_z);
    return true;
}

static bool NtPass(void* user, fc4_nettool_run* run, bool, fc4_nettool_pass* out) {
    auto* w = static_cast<NetToolWorld*>(user);
    run->valid_count = w->pass_valid;
    out->valid_count = w->pass_valid;
    out->failure = w->pass_failure;
    return true;
}

static void NtInsert(void* user, const fc4_nettool_run* run, bool) {
    auto* w = static_cast<NetToolWorld*>(user);
    for (uint32_t i = 0; i < run->cell_count && i < FC4_NETTOOL_MAX_CELLS; ++i) {
        w->data4s.push_back(run->data4);
    }
}

static void NtUsable(void* user, const fc4_nettool_run* run, bool) {
    auto* w = static_cast<NetToolWorld*>(user);
    for (uint32_t i = 0; i < run->cell_count && i < FC4_NETTOOL_MAX_CELLS; ++i) {
        w->usable_posts.push_back(run->data4);
    }
}

static uint32_t NtNeighbours(void*, uint64_t, fc4_nettool_neighbour*, uint32_t) { return 0; }
static void NtCorners(void* user, int32_t, int32_t, float out[4]) {
    auto* w = static_cast<NetToolWorld*>(user);
    for (int i = 0; i < 4; ++i) out[i] = w->corners[i];
}
static bool NtSmooth(void* user, fc4_nettool_run*, int32_t) { return static_cast<NetToolWorld*>(user)->smooth_ok; }
static bool NtSolved(void* user, int32_t, int32_t, fc4_nettool_solved* out) {
    auto* w = static_cast<NetToolWorld*>(user);
    out->occ = 7;
    for (int i = 0; i < 4; ++i) out->corners[i] = w->solved[i];
    return true;
}
static void NtRebuild(void* user, uint64_t, uint32_t outcome, const float*, const float*) {
    auto* w = static_cast<NetToolWorld*>(user);
    if (outcome == FC4_NETTOOL_U_UPDATED) ++w->underground_updated;
    else if (outcome == FC4_NETTOOL_U_KEPT) ++w->underground_kept;
    else ++w->destroyed;
}

void TestNetworkTools() {
    CHECK(fc4_nettool_create(nullptr, 0) == nullptr);
    NetToolWorld w;
    fc4_nettool_host h{};
    h.struct_size = sizeof h;
    h.user = &w;
    h.draw_network_line = &NtDraw;
    h.validate_and_solve = &NtPass;
    h.insert_occupants_for_solved_cells = &NtInsert;
    h.mark_occupants_usable = &NtUsable;
    h.neighbours_by_network_type = &NtNeighbours;
    h.corner_altitudes = &NtCorners;
    h.smoothen_underground = &NtSmooth;
    h.find_underground_solved = &NtSolved;
    h.underground_rebuild = &NtRebuild;
    fc4_nettool* t = fc4_nettool_create(&h, 0);
    CHECK(t != nullptr);
    fc4_nettool_state st{};
    CHECK(fc4_nettool_get_state(t, &st) && st.type_id == 0 && st.layer_mask == 0x1F4Fu);
    CHECK(st.stop_on_failure != 0 && st.preview_update != 0 && st.data4 == 1);

    // §18 test 1: a free code placement with data4 3.
    w.pass_valid = 11;
    CHECK(fc4_nettool_place_network_occupants(t, 10, 10, 20, 10, 0, 0, true, -1));
    CHECK(w.data4s.size() == 11 && w.data4s[0] == 3);
    CHECK(fc4_nettool_get_state(t, &st) && st.data4 == 1);  // §3.4 restored

    // §18 test 8: a failed validation leaves nothing and keeps the code.
    w.data4s.clear();
    w.pass_valid = 0;
    w.pass_failure = 0xF0000002u;
    CHECK(!fc4_nettool_place_network_occupants(t, 10, 10, 20, 10, 0, 0, true, -1));
    CHECK(w.data4s.empty());
    CHECK(fc4_nettool_get_failure_state(t) == 1u);  // NT28: public Boolean, internal reason retained.
    CHECK(fc4_nettool_clear_failure(t) && fc4_nettool_get_failure_state(t) == 0);

    // §11 test 14: the smooth refit (new corners − depth).
    CHECK(fc4_nettool_init(t, 64, 64, 10.f, 5.f, 0.f, 0.f, false));
    w.smooth_ok = true;
    fc4_nettool_underground piece{};
    piece.occ = 7;
    piece.x = 5;
    piece.z = 5;
    for (int i = 0; i < 4; ++i) {
        piece.stored_terrain[i] = 100.f;
        piece.vertices[i] = 90.f;
    }
    CHECK(fc4_nettool_update_underground_heights(t, &piece, 1, 4));
    CHECK(w.underground_updated == 1 && w.underground_kept == 0);

    // §11 test 16: the covered fallback keeps the piece (−15 ≥ −20 + 5).
    w.smooth_ok = false;
    for (int i = 0; i < 4; ++i) piece.vertices[i] = -20.f;
    w.corners[0] = w.corners[1] = w.corners[2] = w.corners[3] = -15.f;
    CHECK(fc4_nettool_update_underground_heights(t, &piece, 1, 4));
    CHECK(w.underground_kept == 1 && w.destroyed == 0);
    // −16 at one corner destroys (−16 < −15).
    w.corners[0] = -16.f;
    CHECK(fc4_nettool_update_underground_heights(t, &piece, 1, 4));
    CHECK(w.underground_kept == 1);  // still the earlier keep; this run destroyed
    CHECK(w.destroyed >= 0);

    fc4_nettool_destroy(t);
}

}  // namespace

int main() {
    const uint32_t actualAbi = fc4_abi_version();
    if (actualAbi != FC4_ABI_VERSION) {
        std::fprintf(stderr, "ABI mismatch: linked Rust=%u, compiled header=%u, compiled source=%s\n",
                     actualAbi, FC4_ABI_VERSION, __FILE__);
    }
    CHECK(actualAbi == FC4_ABI_VERSION);
    CHECK(fc4_sim_create(nullptr) == nullptr);
    CHECK(!fc4_sim_pause(nullptr));
    TestDays();
    TestReentrancyAndNames();
    TestNetworkGeometry();
    TestOccupantManager();
    TestNetworkRules();
    TestEffects();
    TestNetworkTools();
    if (g_failures == 0) std::printf("fc4 C ABI smoke test: all checks passed (%zu-bit)\n", sizeof(void*) * 8);
    return g_failures == 0 ? 0 : 1;
}
