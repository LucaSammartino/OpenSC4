// SPDX-License-Identifier: GPL-3.0-or-later
// Public projection only: network-tools section 2.1 / 14 and
// network-geometry section 3. Not solved pieces, costs or subclass placement.
#include "FC4NetToolDragShadow.h"
#include "FC4Checked.h"
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Subsystems.h"
#include "cFC4NetworkTool.h"
#include "cISC4City.h"
#include "cISC4NetworkManager.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "fc4.h"
#include <vector>
#include <cstring>

namespace fc4 {
namespace {
struct Counts { uint32_t lines = 0, clean = 0, listed = 0; } g_narrow, g_wide;
Counts g_plan, g_power;
struct Probe {
    cFC4ISC4NetworkToolFull* tool;
    uint32_t type;
    SC4Point<uint32_t> start, end;
    bool snap, ok = false;
    uint32_t straight = 0, total = 0;
    SC4Vector<SC4Point<uint32_t>> cells;
};
void Init(void* p) {
    auto* probe = static_cast<Probe*>(p);
    // [abi-ret] network-tools section 14, slot 3: Init returns AL.
    probe->ok = probe->tool->Init();
    probe->tool->EnablePreviewUpdate(false);
}
void Draw(void* p) {
    auto& probe = *static_cast<Probe*>(p);
    probe.ok = (probe.tool->DrawNetworkLine(probe.start, probe.end, probe.snap,
        static_cast<cISC4NetworkOccupant::eNetworkType>(probe.type)) & 0xFFu) != 0;
    probe.tool->GetDraggedCells(probe.cells);
    probe.straight = probe.tool->GetStraightStepCount();
    probe.total = probe.tool->GetTotalStepCount();
}
void Shutdown(void* p) { static_cast<Probe*>(p)->tool->Shutdown(); }

struct PlanProbe {
    Probe drag;
    uint32_t failure = 0, valid = 0;
    int32_t cost = 0;
};
void Plan(void* p) {
    auto& s = *static_cast<PlanProbe*>(p);
    auto& d = s.drag;
    // [abi-ret] network-tools section 3: slot 7 returns AL; mask before use.
    d.ok = (d.tool->PlaceNetworkOccupants(d.start.x, d.start.y, d.end.x, d.end.y,
        0, nullptr, false, static_cast<cISC4NetworkOccupant::eNetworkType>(-1)) & 0xFFu) != 0;
    d.tool->GetDraggedCells(d.cells);
    d.straight = d.tool->GetStraightStepCount();
    d.total = d.tool->GetTotalStepCount();
    s.failure = d.tool->GetFailureState();
    s.valid = d.tool->GetValidStepCount();
    s.cost = d.tool->GetCostOfSolution();
}
void UndoPlan(void* p) {
    auto* tool = static_cast<PlanProbe*>(p)->drag.tool;
    // power-line-tool section 5.1: slot 16 restores reversible draft poles
    // and connections. Do this before Shutdown, including after a failed probe.
    tool->ClearOccupantHighlights();
    tool->ClearCellPreviews();
    tool->Shutdown();
}
struct PlanningContext : NetToolContext {
    fc4_powerline_selection selection{};
    bool selectionReady = false;
};
bool PowerlineDraft(void* u, fc4_nettool_run* run, uint32_t, uint32_t neighbourMode, bool real) {
    auto& context = *static_cast<PlanningContext*>(static_cast<NetToolContext*>(u));
    if (run->type_id != 5 || real || neighbourMode != 0) return false;
    // network-tools sections 5.1/9: slot 53 handles the subclass and returns
    // true, bypassing the base terrain solve even for a one-step drag.
    context.selection = {};
    context.selectionReady = run->dragged_count < 2 ||
        ReadNetToolPowerlineSelection(&context, &context.selection);
    run->valid_count = context.selection.valid;
    // Incomplete observations remain unavailable, never a successful sample.
    return true;
}
bool ClearPlanningOrigin(cISC4City* city, int32_t& x, int32_t& z) {
    auto* occupants = city->GetOccupantManager(); // borrowed
    if (!occupants) return false;
    NetToolContext context;
    context.city = city;
    const auto host = MakeNetGeomHost(&context);
    // SPEC-GAP: demolition-highlights enumeration/pricing and powerline
    // reuse are not wired by these hosts. Probe an explicitly clear L-shaped
    // footprint; never substitute native prices or silently drop a mismatch.
    for (int32_t iz = 2; iz + 4 < static_cast<int32_t>(city->CellCountZ()) - 2; ++iz) {
        for (int32_t ix = 2; ix + 12 < static_cast<int32_t>(city->CellCountX()) - 2; ++ix) {
            bool clear = true;
            for (int32_t i = 0; i < 17 && clear; ++i) {
                const int32_t cx = ix + (i < 13 ? i : 0);
                const int32_t cz = iz + (i < 13 ? 0 : i - 12);
                cISC4Occupant* occupant = nullptr;
                const bool hit = occupants->GetFirstOccupantByStandardCityCell(
                    occupant, cx, cz, static_cast<cISC4OccupantFilter*>(nullptr));
                if (occupant) {
                    FC4_ACQUIRED(occupant, "GetFirstOccupantByStandardCityCell");
                    FC4_RELEASE(occupant);
                }
                clear = !hit && !host.cell_has_water(host.user, cx, cz);
            }
            if (clear) { x = ix; z = iz; return true; }
        }
    }
    return false;
}
void LogPlanningInputs(NetToolContext& context, const fc4_netgeom_host& host, int32_t x, int32_t z) {
    auto* city = context.city;
    const int32_t stride = static_cast<int32_t>(city->CellCountX()) + 1;
    const int32_t v = z * stride + x;
    const int32_t vertices[] = {v, v + stride, v + stride + 1, v + 1};
    float heights[4], water[4];
    for (uint32_t i = 0; i < 4; ++i) {
        heights[i] = host.vertex_altitude(host.user, vertices[i]);
        water[i] = host.surface_water_level(host.user, vertices[i]);
    }
    float lo = heights[0], hi = heights[0];
    for (float h : heights) { if (h < lo) lo = h; if (h > hi) hi = h; }
    int32_t edge = x;
    const int32_t distances[] = {z, static_cast<int32_t>(city->CellCountX()) - 1 - x,
        static_cast<int32_t>(city->CellCountZ()) - 1 - z};
    for (int32_t d : distances) if (d < edge) edge = d;
    auto* occupants = city->GetOccupantManager(); // borrowed
    cISC4Occupant* occupant = nullptr;
    const bool occupied = occupants && occupants->GetFirstOccupantByStandardCityCell(
        occupant, x, z, static_cast<cISC4OccupantFilter*>(nullptr));
    if (occupant) {
        FC4_ACQUIRED(occupant, "GetFirstOccupantByStandardCityCell");
        FC4_RELEASE(occupant);
    }
    auto* zones = reinterpret_cast<cFC4ISC4ZoneManager*>(city->GetZoneManager()); // borrowed
    Log("nettool plan inputs cell=(%d,%d) terrain=(%.9g,%.9g,%.9g,%.9g) "
        "water=%d depths=(%.9g,%.9g,%.9g,%.9g) slope_range_per_16=%.9g edge=%d occupied=%d zone=%u",
        x, z, heights[0], heights[1], heights[2], heights[3], host.cell_has_water(host.user, x, z),
        water[0], water[1], water[2], water[3], (hi - lo) / 16.0f, edge, occupied,
        zones ? zones->GetZoneType(x, z) : 0u);
    fc4_netgeom_cell_info info{};
    if (fc4_netgeom_get_cell_info(context.geom, x, z, &info)) Log(
        "nettool plan inputs retained=(%.9g,%.9g,%.9g,%.9g) flags=%x in_run=%u intersection=%u "
        "vertices=(%d,%d,%d,%d)", info.corners[0], info.corners[1], info.corners[2], info.corners[3],
        info.flags, info.in_run, info.intersection, info.vertices[0], info.vertices[1], info.vertices[2], info.vertices[3]);
}
void RunPlanning(cISC4City* city, int32_t julian, bool power) {
    const char* key = power ? "powerline_plan_shadow" : "nettool_plan_shadow";
    if (!city || SettingInt(key, nullptr, 0) == 0) return;
    auto& n = power ? g_power : g_plan;
    const char* label = power ? "powerline plan shadow" : "nettool plan shadow";
    uint32_t mismatches = 0, unavailable = 0, vectors = 0;
    auto* manager = city->GetNetworkManager(); // borrowed
    size_t count = 0;
    const auto* subsystems = Subsystems(&count);
    for (size_t i = 0; i < count; ++i) {
        if (std::strcmp(subsystems[i].key, "netmgr") == 0 && SubsystemEnabled(subsystems[i])) manager = nullptr;
    }
    // The registry replacement does not own the native manager's private
    // factory. Calling that factory directly works with enable=nettool too.
    int32_t originX = 0, originZ = 0;
    if (manager && !ClearPlanningOrigin(city, originX, originZ)) manager = nullptr;
    constexpr int32_t offsets[][4] = {
        {0,0,0,0}, {0,0,4,0}, {4,0,0,0},
        {0,0,0,4}, {0,4,0,0}, {0,0,12,0},
    };
    const uint32_t types[] = {0, 3, 5};
    for (uint32_t type : types) {
        if ((type == 5) != power) continue;
        for (const auto& offset : offsets) {
            const int32_t v[] = {originX + offset[0], originZ + offset[1],
                originX + offset[2], originZ + offset[3]};
            if (!manager) { ++unavailable; continue; }
            auto* native = manager->GetNetworkTool(type, true); // created, refcount zero
            if (!native) { ++unavailable; continue; }
            FC4_ADDREF(native);
            {
                PlanningContext context;
                context.city = city;
                auto gh = MakeNetGeomHost(&context);
                // No terrain writes, terrain transactions or redisplays in a
                // planning replay. All remaining callbacks read the same city.
                gh.set_altitude_at_vertex = nullptr;
                gh.begin_long_terrain_operation = nullptr;
                gh.redisplay_terrain = nullptr;
                gh.end_long_terrain_operation = nullptr;
                context.geom = fc4_netgeom_create(&gh, nullptr, 0);
                const auto source = MakeNetToolHost(&context);
                fc4_nettool_host th{};
                th.struct_size = sizeof(th);
                th.user = static_cast<NetToolContext*>(&context);
                th.draw_network_line = source.draw_network_line;
                if (power) th.do_network_specific_placement = &PowerlineDraft;
                th.validate_and_solve = source.validate_and_solve;
                th.get_network_occupant = source.get_network_occupant;
                auto* core = fc4_nettool_create(&th, type);
                PlanProbe p{{reinterpret_cast<cFC4ISC4NetworkToolFull*>(native), type,
                    {static_cast<uint32_t>(v[0]), static_cast<uint32_t>(v[1])},
                    {static_cast<uint32_t>(v[2]), static_cast<uint32_t>(v[3])}, false}};
                const bool initialized = SehGuard("nettool plan Init", &Init, &p.drag) && p.drag.ok;
                bool ready = context.geom && core && fc4_nettool_init(core,
                    city->CellCountX(), city->CellCountZ(), 0, 0, 0, 0, !power);
                bool ok = false;
                fc4_netgeom_state state{};
                uint32_t failure = 0, valid = 0;
                int32_t cost = 0;
                std::vector<fc4_netgeom_cell> cells;
                if (ready) {
                    fc4_nettool_enable_preview_update(core, false);
                    ok = fc4_nettool_place_network_occupants(core, v[0], v[1], v[2], v[3], 0, 0, false, -1);
                    ready = fc4_netgeom_get_state(context.geom, &state);
                    failure = fc4_nettool_get_failure_state(core);
                    valid = state.valid;
                    cost = fc4_netgeom_cost(context.geom, 0);
                    cells.resize(fc4_netgeom_get_cells(context.geom, nullptr, 0));
                    fc4_netgeom_get_cells(context.geom, cells.data(), static_cast<uint32_t>(cells.size()));
                    if (power) {
                        const auto& selection = context.selection;
                        ready = context.selectionReady && ready;
                        valid = selection.valid;
                        failure = selection.failure != 0 ? 1u : 0u;
                        cost = selection.cost;
                    }
                }
                const bool observed = initialized && SehGuard("nettool plan draft", &Plan, &p);
                ++vectors;
                if (!ready || !observed) ++unavailable;
                else {
                    uint32_t diff = cells.size() != p.drag.cells.size() ? 1u : 0u;
                    for (size_t i = 0; i < cells.size() && i < p.drag.cells.size(); ++i) {
                        diff += cells[i].x != static_cast<int32_t>(p.drag.cells[i].x) ||
                            cells[i].z != static_cast<int32_t>(p.drag.cells[i].y);
                    }
                    diff += (ok != p.drag.ok) + (failure != p.failure) + (valid != p.valid) +
                        (state.straight_count != p.drag.straight) + (state.dragged_count != p.drag.total) +
                        (cost != p.cost);
                    mismatches += diff;
                    if (diff && n.listed++ < 12) {
                        Log(
                        "%s diff type=%u drag=(%d,%d)-(%d,%d) mismatches=%u ok=%d/%d failure=%u/%u "
                        "valid=%u/%u straight=%u/%u total=%u/%u cells=%u/%u cost=%d/%d",
                        label, type, v[0], v[1], v[2], v[3], diff, p.drag.ok ? 1 : 0, ok ? 1 : 0,
                        p.failure, failure, p.valid, valid, p.drag.straight, state.straight_count,
                        p.drag.total, state.dragged_count, static_cast<uint32_t>(p.drag.cells.size()),
                        static_cast<uint32_t>(cells.size()), p.cost, cost);
                        if (!power && cells.size() == 1) LogPlanningInputs(context, gh, cells[0].x, cells[0].z);
                    }
                }
                if (initialized && !SehGuard("nettool plan undo", &UndoPlan, &p)) ++unavailable;
                if (core) fc4_nettool_destroy(core);
                if (context.geom) fc4_netgeom_destroy(context.geom);
                FC4_X87_CHECK("nettool plan shadow");
            }
            FC4_RELEASE_CREATED(native);
        }
    }
    ++n.lines;
    if (mismatches == 0 && unavailable == 0 && vectors != 0) ++n.clean;
    // SPEC-GAP: no published native cell-info/satisfier or bridge-reader
    // call boundary; public planning results do not verify those owners.
    Log("%s julian=%d: mismatches=%u unavailable=%u vectors=%u scope=clear-land-draft-status-cost "
        "origin=(%d,%d) "
        "fields=return,ordered-cells,failure,valid,straight,total,cost exact=%u/%u",
        label, julian, mismatches + unavailable, unavailable, vectors, originX, originZ, n.clean, n.lines);
}
void Run(cISC4City* city, int32_t julian, bool wide) {
    const char* key = wide ? "nettool_wide_drag_shadow" : "nettool_drag_shadow";
    if (!city || SettingInt(key, nullptr, 0) == 0) return;
    auto& n = wide ? g_wide : g_narrow;
    const char* label = wide ? "nettool wide drag shadow" : "nettool drag shadow";
    uint32_t mismatches = 0, unavailable = 0, vectors = 0;
    auto* manager = city->GetNetworkManager(); // borrowed
    size_t subsystemCount = 0;
    const auto* subsystems = Subsystems(&subsystemCount);
    bool managerReplaced = false;
    for (size_t i = 0; i < subsystemCount; ++i) {
        if (std::strcmp(subsystems[i].key, "netmgr") == 0) managerReplaced = SubsystemEnabled(subsystems[i]);
    }
    if (!manager || NetToolRegistry::Enabled() || managerReplaced) {
        Log("%s julian=%d: mismatches=1 unavailable=original-manager/tool scope=draw-only", label, julian);
        ++n.lines;
        return;
    }
    // Whitelist dimensions only. Draw replay has no write callbacks. Existing
    // network alignment/auto-extension is not supplied by this host; any live
    // difference remains visible, never replaced with the native answer (NG9).
    NetToolContext context;
    context.city = city;
    const auto source = MakeNetGeomHost(&context);
    fc4_netgeom_host host{};
    host.struct_size = sizeof(host);
    host.user = &context;
    host.cells = source.cells;
    fc4_netgeom* core = fc4_netgeom_create(&host, nullptr, 0);
    // Fixed inputs exercise both axes, reversal, one-cell drags, pre-adjust,
    // staircase and snap. They are native/core same-process calls in the city,
    // not an empty-land assertion or a synthetic native result.
    constexpr int32_t ends[][4] = {
        {4,32,4,32}, {4,32,16,32}, {16,32,4,32}, {4,32,4,44},
        {4,44,4,32}, {4,32,16,33}, {4,32,9,35}, {16,44,10,40},
    };
    for (uint32_t type = 0; type < 13 && core; ++type) {
        fc4_netgeom_type_info info{};
        if (!fc4_netgeom_get_type_info(core, type, &info)) { ++unavailable; continue; }
        if ((info.width1 > 1) != wide) continue;
        auto* native = manager->GetNetworkTool(type, true); // created private tool, refcount zero
        if (!native) { ++unavailable; continue; }
        FC4_ADDREF(native);
        {
            Probe p{reinterpret_cast<cFC4ISC4NetworkToolFull*>(native), type, {}, {}, false};
            const bool initialized = SehGuard("nettool drag Init", &Init, &p) && p.ok;
            if (!initialized) ++unavailable;
            for (const auto& v : ends) for (int snap = 0; initialized && snap < 2; ++snap) {
                p.start = {static_cast<uint32_t>(v[0]), static_cast<uint32_t>(v[1])};
                p.end = {static_cast<uint32_t>(v[2]), static_cast<uint32_t>(v[3])};
                p.snap = snap != 0;
                fc4_netgeom_reset(core);
                const bool ok = fc4_netgeom_draw_line(core, type, v[0], v[1], v[2], v[3], p.snap);
                fc4_netgeom_state state{};
                const bool observed = SehGuard("nettool drag Draw", &Draw, &p) &&
                    fc4_netgeom_get_state(core, &state);
                FC4_X87_CHECK("nettool drag shadow");
                ++vectors;
                if (!observed) { ++unavailable; continue; }
                std::vector<fc4_netgeom_cell> cells(fc4_netgeom_get_cells(core, nullptr, 0));
                fc4_netgeom_get_cells(core, cells.data(), static_cast<uint32_t>(cells.size()));
                uint32_t cellDiffs = cells.size() != p.cells.size() ? 1u : 0u;
                for (size_t i = 0; i < cells.size() && i < p.cells.size(); ++i) {
                    if (cells[i].x != static_cast<int32_t>(p.cells[i].x) ||
                        cells[i].z != static_cast<int32_t>(p.cells[i].y)) ++cellDiffs;
                }
                const uint32_t diff = cellDiffs + (ok != p.ok) +
                    (state.straight_count != p.straight) + (state.dragged_count != p.total);
                mismatches += diff;
                if (diff && n.listed++ < 12) Log(
                    "%s diff type=%u drag=(%d,%d)-(%d,%d) snap=%d mismatches=%u "
                    "ok=%d/%d cells=%u/%u cell_diffs=%u straight=%u/%u total=%u/%u",
                    label, type, v[0], v[1], v[2], v[3], snap, diff, p.ok ? 1 : 0, ok ? 1 : 0,
                    static_cast<uint32_t>(p.cells.size()), static_cast<uint32_t>(cells.size()), cellDiffs,
                    p.straight, state.straight_count, p.total, state.dragged_count);
            }
            if (!SehGuard("nettool drag Shutdown", &Shutdown, &p)) ++unavailable;
        } // destroy the SDK vector while the city allocator is alive
        FC4_RELEASE_CREATED(native);
    }
    if (core) fc4_netgeom_destroy(core); else ++unavailable;
    ++n.lines;
    if (mismatches == 0 && unavailable == 0 && vectors != 0) ++n.clean;
    Log("%s julian=%d: mismatches=%u unavailable=%u vectors=%u scope=draw-only "
        "fields=return,ordered-cells,straight,total exact=%u/%u", label, julian,
        mismatches + unavailable, unavailable, vectors, n.clean, n.lines);
}
}
void NetToolDragShadowLoaded(cISC4City* city) {
    g_narrow = {};
    g_wide = {};
    g_plan = {};
    g_power = {};
    Run(city, 0, false);
    Run(city, 0, true);
    RunPlanning(city, 0, false);
    RunPlanning(city, 0, true);
}
void NetToolDragShadowMonth(cISC4City* city, int32_t julian) {
    Run(city, julian, false);
    Run(city, julian, true);
    RunPlanning(city, julian, false);
    RunPlanning(city, julian, true);
}
void NetToolDragShadowShutdown() {
    if (g_narrow.lines) Log("nettool drag shadow: city shutdown: exact %u/%u scope=draw-only", g_narrow.clean, g_narrow.lines);
    if (g_wide.lines) Log("nettool wide drag shadow: city shutdown: exact %u/%u scope=draw-only", g_wide.clean, g_wide.lines);
    if (g_plan.lines) Log("nettool plan shadow: city shutdown: exact %u/%u scope=draft-status-cost", g_plan.clean, g_plan.lines);
    if (g_power.lines) Log("powerline plan shadow: city shutdown: exact %u/%u scope=draft-status-cost", g_power.clean, g_power.lines);
    g_narrow = {};
    g_wide = {};
    g_plan = {};
    g_power = {};
}
}
