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

// The effects layer's host table on the game (FC4EffectsHost.h).

#include "FC4EffectsHost.h"

#include "FC4Interfaces.h"
#include "FC4Log.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include "cISC4City.h"
#include "cISC4Demolition.h"
#include "cISC4DisasterLayer.h"
#include "cISC4DisasterManager.h"
#include "cISC4DisasterManagerFire.h"
#include "cISTETerrain.h"

#include <cmath>
#include <vector>

namespace fc4 {
namespace {

constexpr float kCellSize = 16.0f;  // effects.md CELL_SIZE
// The two disaster managers the game effect's Apply reaches (effects.md §9.3).
constexpr uint32_t kFireDisasterType = 0x621CDA33;
constexpr uint32_t kToxicDisasterType = 0xEBFB0404;
// cISC4DisasterManagerFire (effects.md §9.3: "QI 0x49B43F57").
constexpr uint32_t kIID_DisasterManagerFire = 0x49B43F57;

EffectsHostContext* Ctx(void* u) { return static_cast<EffectsHostContext*>(u); }

cISC4Simulator* Sim(EffectsHostContext* c) {
    return c->city ? c->city->GetSimulator() : nullptr;
}

cISTETerrain* Terrain(EffectsHostContext* c) {
    return c->city ? c->city->GetTerrain() : nullptr;
}

cISC4DisasterLayer* Layer(EffectsHostContext* c) {
    return c->city ? c->city->GetDisasterLayer() : nullptr;
}

// The fire disaster manager (§9.3 step 1: city → layer slot 7 → QI).
cISC4DisasterManagerFire* FireManager(EffectsHostContext* c) {
    cISC4DisasterLayer* l = Layer(c);
    if (l == nullptr) return nullptr;
    cISC4DisasterManager* m = l->GetDisasterManager(kFireDisasterType);  // layer slot 7
    if (m == nullptr) return nullptr;
    cISC4DisasterManagerFire* fire = nullptr;
    if (!m->QueryInterface(kIID_DisasterManagerFire, reinterpret_cast<void**>(&fire)) || fire == nullptr) {
        return nullptr;
    }
    fire->Release();  // the QI AddRef'd it; we borrow the layer's reference
    return fire;
}

// A write: recorded in shadow mode, made in live mode. Returns whether the
// live mode may proceed.
bool MayWrite(EffectsHostContext* c, const char* name) {
    ++c->writes[name];
    return !c->shadow;
}

void NotWired(EffectsHostContext* c, const char* name) {
    if (c->notWired.insert(name).second) {
        Log("effects host: '%s' is not made (logged once); see docs/lanes/glue22.md", name);
    }
}

// ---- host callbacks ------------------------------------------------------------------------------

bool SimulatorCb(void* u, float* dilation) {
    EffectsHostContext* c = Ctx(u);
    cISC4Simulator* s = Sim(c);
    if (s == nullptr) return false;
    if (dilation != nullptr) *dilation = AsGame(s)->GetAnimationTimeDilation();  // slot 32
    return true;
}

void SimPauseCb(void* u) {
    EffectsHostContext* c = Ctx(u);
    if (!MayWrite(c, "sim_pause")) {
        ++c->simPauses;
        return;
    }
    if (cISC4Simulator* s = Sim(c)) AsGame(s)->Pause();  // slot 9
}

void SimResumeCb(void* u) {
    EffectsHostContext* c = Ctx(u);
    if (!MayWrite(c, "sim_resume")) return;
    if (cISC4Simulator* s = Sim(c)) AsGame(s)->Resume();  // slot 12
}

void SimHiddenPauseCb(void* u) {
    EffectsHostContext* c = Ctx(u);
    if (!MayWrite(c, "sim_hidden_pause")) {
        ++c->hiddenPauses;
        return;
    }
    if (cISC4Simulator* s = Sim(c)) AsGame(s)->HiddenPause();  // slot 10
}

void SimHiddenResumeCb(void* u) {
    EffectsHostContext* c = Ctx(u);
    if (!MayWrite(c, "sim_hidden_resume")) return;
    if (cISC4Simulator* s = Sim(c)) AsGame(s)->HiddenResume();  // slot 13
}

void ClockPauseCb(void* u, bool on) {
    EffectsHostContext* c = Ctx(u);
    if (!MayWrite(c, on ? "clock_pause" : "clock_resume")) {
        if (on) ++c->clockPauses;
        return;
    }
    // The 24-hour clock's slot 26 with 1 / 0 (effects.md §9.2).
    if (cISC424HourClock* k = c->city ? c->city->Get24HourClock() : nullptr) {
        fc4::AsGame(k)->HiddenPause(on);  // slot 26
    }
}

void CityCellsCb(void* u, int32_t* x, int32_t* z) {
    EffectsHostContext* c = Ctx(u);
    int32_t cx = 0, cz = 0;
    if (c->city != nullptr) {
        // The city's extent in world units (the spec's cell counts, city slots
        // 93 and 94, are that extent over the 16 m cell).
        cx = static_cast<int32_t>(std::floor(c->city->SizeX() / kCellSize));
        cz = static_cast<int32_t>(std::floor(c->city->SizeZ() / kCellSize));
    }
    if (x != nullptr) *x = cx;
    if (z != nullptr) *z = cz;
}

bool LocationInBoundsCb(void* u, float x, float z) {
    EffectsHostContext* c = Ctx(u);
    return c->city != nullptr && c->city->LocationIsInBounds(x, z);  // slot 98
}

bool ApplyBrushCb(void* u, uint32_t brushId, fc4_brush_params* params) {
    EffectsHostContext* c = Ctx(u);
    if (params == nullptr) return false;
    if (!MayWrite(c, "apply_brush")) {
        ++c->brushApplies;
        return true;  // the shadow records the stroke; nothing is applied
    }
    cISTETerrain* t = Terrain(c);
    if (t == nullptr) return false;
    cISTETerrain::tBrushParameters p = {};
    p.bounds = decltype(p.bounds)(params->from_x, params->from_z, params->to_x, params->to_z);
    p.zoom = params->level;
    p.redisplayTerrain = params->redisplay != 0;
    p.strength = params->strength_scale;
    p.radiusScaling = params->radius_scale;
    p.terrainMessageData4 = params->data4;
    p.altitude = params->target_height;
    const bool ok = t->ApplyBrush(brushId, p);  // terrain slot 57
    params->out.x0 = p.appliedArea.topLeftX;
    params->out.z0 = p.appliedArea.topLeftY;
    params->out.x1 = p.appliedArea.bottomRightX;
    params->out.z1 = p.appliedArea.bottomRightY;
    params->out_valid = 1;
    return ok;
}

void BeginLongTerrainCb(void* u) {
    EffectsHostContext* c = Ctx(u);
    if (!MayWrite(c, "begin_long_terrain_operation")) {
        ++c->longOps;
        c->longOpOpen = true;
        return;
    }
    // Terrain slots 87 and 88 (effects.md §8.3) are not in the SDK's
    // declaration; the stroke's long operation is tracked here and closed by
    // EndLongTerrainCb. See docs/lanes/glue22.md question 4.
    c->longOpOpen = true;
    NotWired(c, "begin_long_terrain_operation");
}

void EndLongTerrainCb(void* u) {
    EffectsHostContext* c = Ctx(u);
    c->longOpOpen = false;
    if (!MayWrite(c, "end_long_terrain_operation")) return;
    NotWired(c, "end_long_terrain_operation");
}

void IgniteCellRadiusCb(void* u, int32_t x, int32_t z, int32_t r0, int32_t r1) {
    EffectsHostContext* c = Ctx(u);
    if (!MayWrite(c, "ignite_cell_radius")) return;
    if (cISC4DisasterManagerFire* f = FireManager(c)) {
        // §9.3 step 1: IgniteCellRadius(cx, cz, 0, burn).
        f->IgniteCellRadius(static_cast<uint32_t>(x), static_cast<uint32_t>(z), static_cast<float>(r0),
                            static_cast<float>(r1));
    }
}

void SetFireCauseCb(void* u, uint32_t cause) {
    EffectsHostContext* c = Ctx(u);
    if (!MayWrite(c, "set_fire_cause")) return;
    if (cISC4DisasterManagerFire* f = FireManager(c)) f->SetCause(cause);  // slot 9
}

void ToxicAttemptStartCb(void* u, int32_t x, int32_t z) {
    EffectsHostContext* c = Ctx(u);
    if (!MayWrite(c, "toxic_attempt_start")) return;
    cISC4DisasterLayer* l = Layer(c);
    if (l == nullptr) return;
    cISC4DisasterManager* m = l->GetDisasterManager(kToxicDisasterType);
    if (m == nullptr) return;
    // §9.3 step 2: AttemptDisasterStart(cx, cz). The amount is ignored (quirk).
    m->AttemptDisasterStart(x, z);
}


// A full rectangle as the game's SC4CellRegion<int32_t> (0x24 bytes: x1, z1,
// x2, z2, then the cell map). The memory is ours; DemolishRegion only reads
// the bits. The same layout cFC4Development.cpp builds for PlaceZone.
// The cRZCellMap layout with a vtable of our own (slot 0, the destructor, is
// the only virtual; DemolishRegion never destroys it).
class EffectsCellMapStorage {
public:
    virtual ~EffectsCellMapStorage() = default;
    uint32_t width = 0;
    uint32_t height = 0;
    uint32_t words = 0;
    uint32_t** rows = nullptr;
};

class RegionBuffer {
public:
    bool Build(const fc4_effects_rect& r) {
        const int64_t w = int64_t{r.x1} - r.x0 + 1;
        const int64_t h = int64_t{r.z1} - r.z0 + 1;
        if (w <= 0 || h <= 0) return false;
        layout_.x1 = r.x0;
        layout_.z1 = r.z0;
        layout_.x2 = r.x1;
        layout_.z2 = r.z1;
        const uint32_t words = static_cast<uint32_t>((h + 31) / 32);
        bits_.assign(static_cast<size_t>(w) * words, 0);
        rows_.resize(static_cast<size_t>(w));
        for (int64_t dx = 0; dx < w; ++dx) {
            uint32_t* row = &bits_[static_cast<size_t>(dx) * words];
            rows_[static_cast<size_t>(dx)] = row;
            for (int64_t dz = 0; dz < h; ++dz) row[dz / 32] |= 1u << (dz % 32);
        }
        layout_.map.width = static_cast<uint32_t>(w);
        layout_.map.height = static_cast<uint32_t>(h);
        layout_.map.words = words;
        layout_.map.rows = rows_.data();
        return true;
    }
    SC4CellRegion<int32_t>& Region() { return *reinterpret_cast<SC4CellRegion<int32_t>*>(&layout_); }

private:
    struct Layout {
        int32_t x1 = 0, z1 = 0, x2 = 0, z2 = 0;
        EffectsCellMapStorage map;
    };
    static_assert(sizeof(void*) != 4 || sizeof(Layout) == 0x24, "SC4CellRegion<int32_t> layout");
    Layout layout_;
    std::vector<uint32_t> bits_;
    std::vector<uint32_t*> rows_;
};

void DemolishRegionCb(void* u, const fc4_effects_rect* rect, uint32_t flags, bool dezone,
                      const fc4_effects_demolish_filter* filter) {
    EffectsHostContext* c = Ctx(u);
    if (rect == nullptr) return;
    if (!MayWrite(c, "demolish_region")) return;
    cISC4Demolition* d = c->city ? c->city->GetDemolitionUtility() : nullptr;
    if (d == nullptr) return;
    // §9.3 step 3: a full SC4CellRegion over the clipped rectangle, privilege
    // 0, the record's flags and the dezone bit. The per-occupant filter is
    // fc4_effects_demolish_filter_skips; it is not an cISC4OccupantFilter the
    // demolisher walks, so no filter is passed (question 5).
    (void)filter;
    RegionBuffer rb;
    if (!rb.Build(*rect)) return;
    d->DemolishRegion(true, rb.Region(), 0, flags, dezone, nullptr, nullptr, 0, nullptr, 0, 0);
}

void ExtinguishCellCb(void* u, int32_t x, int32_t z, int32_t amount) {
    EffectsHostContext* c = Ctx(u);
    if (!MayWrite(c, "extinguish_cell")) return;
    if (cISC4DisasterManagerFire* f = FireManager(c)) {
        f->ExtinguishCell(static_cast<uint32_t>(x), static_cast<uint32_t>(z), static_cast<uint32_t>(amount));
    }
}

void PostCb(void* u, const fc4_effects_post* msg) {
    EffectsHostContext* c = Ctx(u);
    if (msg == nullptr) return;
    ++c->posts[msg->msg_type];
    c->postSums[msg->msg_type] += msg->data1;
    if (msg->data2 == 0 && msg->data3 == 0) c->endPostTypes.insert(msg->msg_type);
    if (!MayWrite(c, "post")) return;
    // A queued standard message (effects.md §12). The post is the last of the
    // writes the specs leave the transport of open; see question 6.
    NotWired(c, "post");
}

bool RenderComponentActiveCb(void* u, uint32_t effect, uint32_t component, uint8_t kind) {
    (void)u;
    (void)effect;
    (void)component;
    (void)kind;
    // Animations are stubbed (FC4EffectsHost.h): a render component is never
    // active, so it never holds an effect. [choice, EF1]
    return false;
}

}  // namespace

void EffectsHostNote(EffectsHostContext* ctx, const char* name) {
    if (ctx != nullptr) ++ctx->writes[name];
}

bool EffectsHostMayWrite(EffectsHostContext* ctx, const char* name) {
    return ctx != nullptr && MayWrite(ctx, name);
}

fc4_effects_host MakeEffectsHost(EffectsHostContext* ctx) {
    fc4_effects_host h = {};
    h.struct_size = sizeof(fc4_effects_host);
    h.user = ctx;
    h.simulator = &SimulatorCb;
    h.sim_pause = &SimPauseCb;
    h.sim_resume = &SimResumeCb;
    h.sim_hidden_pause = &SimHiddenPauseCb;
    h.sim_hidden_resume = &SimHiddenResumeCb;
    h.clock_pause = &ClockPauseCb;
    h.city_cells = &CityCellsCb;
    h.location_in_bounds = &LocationInBoundsCb;
    h.apply_brush = &ApplyBrushCb;
    h.begin_long_terrain_operation = &BeginLongTerrainCb;
    h.end_long_terrain_operation = &EndLongTerrainCb;
    h.ignite_cell_radius = &IgniteCellRadiusCb;
    h.set_fire_cause = &SetFireCauseCb;
    h.toxic_attempt_start = &ToxicAttemptStartCb;
    h.demolish_region = &DemolishRegionCb;
    h.extinguish_cell = &ExtinguishCellCb;
    h.post = &PostCb;
    h.render_component_active = &RenderComponentActiveCb;
    return h;
}

}  // namespace fc4
