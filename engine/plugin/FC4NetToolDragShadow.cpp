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
#include "fc4.h"
#include <vector>
#include <cstring>

namespace fc4 {
namespace {
struct Counts { uint32_t lines = 0, clean = 0, listed = 0; } g_narrow, g_wide;
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
    Run(city, 0, false);
    Run(city, 0, true);
}
void NetToolDragShadowMonth(cISC4City* city, int32_t julian) {
    Run(city, julian, false);
    Run(city, julian, true);
}
void NetToolDragShadowShutdown() {
    if (g_narrow.lines) Log("nettool drag shadow: city shutdown: exact %u/%u scope=draw-only", g_narrow.clean, g_narrow.lines);
    if (g_wide.lines) Log("nettool wide drag shadow: city shutdown: exact %u/%u scope=draw-only", g_wide.clean, g_wide.lines);
    g_narrow = {};
    g_wide = {};
}
}
