// SPDX-License-Identifier: GPL-3.0-or-later
// simgrid.md sections 5, 6.2.1/6.2.2 (SG3). Record-only comparisons, not
// simulation advancement or coverage of owner-private/map-view grids.
#include "FC4SimGridRecordShadow.h"
#include "FC4Config.h"
#include "FC4GameMap.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4WriteCapture.h"
#include "FC4Diag.h"
#include "cISC4City.h"
#include "cISC4SimGrid.h"
#include <algorithm>
#include <cstring>
#include <vector>

extern "C" int32_t fc4_simgrids_verify(uint32_t, uint32_t, const uint8_t*, uint32_t,
    const int32_t*, const uint8_t*, uint32_t, uint8_t*, uint32_t);

namespace fc4 { namespace {
template<class R, class... A> R Call(void* p, unsigned slot, A... args) {
    using F = R (__thiscall*)(void*, A...);
    return reinterpret_cast<F>((*static_cast<void***>(p))[slot])(p, args...);
}
template<class T> void Append(std::vector<uint8_t>& b, T v) {
    const auto* p = reinterpret_cast<const uint8_t*>(&v);
    b.insert(b.end(), p, p + sizeof(v));
}
struct Sample {
    cISC4City* city;
    uint32_t roles = 0, exact = 0, differences = 0, unavailable = 0, developers = 0;
    uint64_t cells = 0, bytes = 0;
    uint32_t listed = 0;
    void Missing(const char* owner, unsigned slot, uint32_t selector, const char* reason) {
        ++roles; ++unavailable;
        Log("simgrid-shadow: unavailable owner=%s slot=%u selector=%08X reason=%s",
            owner, slot, selector, reason);
    }
    template<class T> void Grid(void* ptr, uint32_t kind, const char* owner, unsigned slot, uint32_t selector) {
        if (!ptr) { Missing(owner, slot, selector, "null-grid"); return; }
        ++roles;
        auto* grid = static_cast<cISC4SimGrid<T>*>(ptr);
        WriteCapture cap;
        if (!cap.Capture(grid)) {
            ++unavailable;
            Log("simgrid-shadow: unavailable owner=%s slot=%u selector=%08X reason=capture", owner, slot, selector);
            return;
        }
        const auto& record = cap.Bytes();
        uint32_t actualKind = 0;
        if (record.size() >= 7) std::memcpy(&actualKind, record.data() + 3, 4);
        const int32_t nx = grid->GetTractCountX(), nz = grid->GetTractCountZ();
        // Bounded capture domain, never silently omit a role or reinterpret a
        // wrong type. Loaded dimensions, not city dimensions, are authoritative.
        if (actualKind != kind || nx < 0 || nz < 0 ||
            static_cast<uint64_t>(nx) * nz * sizeof(T) > 64u * 1024u * 1024u ||
            record.size() != 43u + static_cast<uint64_t>(nx) * nz * sizeof(T)) {
            ++unavailable;
            Log("simgrid-shadow: unavailable owner=%s slot=%u selector=%08X reason=type-or-geometry clsid=%08X expected=%08X nx=%d nz=%d",
                owner, slot, selector, actualKind, kind, nx, nz);
            return;
        }
        std::vector<uint8_t> native;
        native.reserve(record.size() - 7);
        Append(native, grid->GetInstanceID());
        Append(native, grid->GetTractSize()); Append(native, grid->GetTractShift());
        Append(native, nx); Append(native, nz);
        Append(native, grid->GetTractWidthX()); Append(native, grid->GetTractWidthZ());
        Append(native, grid->GetOneOverTractWidthX()); Append(native, grid->GetOneOverTractWidthZ());
        for (int32_t x = 0; x < nx; ++x)
            for (int32_t z = 0; z < nz; ++z) Append(native, grid->GetTractValue(x, z));
        std::vector<uint8_t> rust(native.size());
        const int32_t args[4]{};
        const int32_t n = fc4_simgrids_verify(kind, 14, record.data(), static_cast<uint32_t>(record.size()),
            args, nullptr, 0, rust.data(), static_cast<uint32_t>(rust.size()));
        cells += static_cast<uint64_t>(nx) * nz; bytes += record.size();
        if (n < 0) {
            ++unavailable;
            Log("simgrid-shadow: unavailable owner=%s slot=%u selector=%08X reason=core-read", owner, slot, selector);
        } else if (static_cast<size_t>(n) == native.size() && rust == native) {
            ++exact;
        } else {
            ++differences;
            if (listed++ < 8) {
                const auto at = std::mismatch(native.begin(), native.end(), rust.begin()).first - native.begin();
                Log("simgrid-shadow: difference owner=%s slot=%u selector=%08X offset=%u native_bytes=%u rust_bytes=%d",
                    owner, slot, selector, static_cast<unsigned>(at), static_cast<unsigned>(native.size()), n);
            }
        }
    }
    template<class T> void Role(void* owner, const char* name, unsigned slot, uint32_t kind,
                                bool selected = false, uint32_t selector = 0) {
        if (!owner) { Missing(name, slot, selector, "null-owner"); return; }
        void* grid = selected ? Call<void*>(owner, slot, selector) : Call<void*>(owner, slot);
        Grid<T>(grid, kind, name, slot, selector);
    }
    template<class T> void Three(void* owner, unsigned slot, uint32_t kind) {
        void* grids[3]{};
        if (owner) {
            // Actual signature is three output references; EAX is NOT bool.
            Call<void>(owner, slot, &grids[0], &grids[1], &grids[2]);
        }
        for (unsigned i = 0; i < 3; ++i) {
            auto* g = static_cast<cISC4SimGrid<T>*>(grids[i]);
            if (g) FC4_ACQUIRED(g, "SG3 residential output");
            if (owner) Grid<T>(g, kind, "residential", slot, i + 1);
            else Missing("residential", slot, i + 1, "null-owner");
            if (g) FC4_RELEASE(g);
        }
    }
    void Run() {
        constexpr uint32_t U8 = 0x49B9E602, I8 = 0x49B9E603, U16 = 0x49B9E604,
            I16 = 0x49B9E605, F32 = 0x49B9E60A;
        if (!city) { Missing("city", 0, 0, "null-city"); return; }
        void* land = city->GetLandValueSimulator();
        Role<uint8_t>(land, "landvalue", 7, U8); Role<uint8_t>(land, "landvalue", 8, U8);
        Role<float>(land, "landvalue", 9, F32);
        void* pollution = city->GetPollutionSimulator();
        for (uint32_t i = 0; i < 4; ++i) Role<int16_t>(pollution, "pollution", 5, I16, true, i);
        Role<uint8_t>(city->GetPowerSimulator(), "power", 35, U8);
        Role<uint8_t>(city->GetPlumbingSimulator(), "plumbing", 7, U8);
        Role<uint8_t>(city->GetCrimeSimulator(), "crime", 5, U8);
        Role<uint8_t>(city->GetFireProtectionSimulator(), "fire", 5, U8);
        Role<uint8_t>(city->GetPoliceSimulator(), "police", 5, U8);
        Role<uint8_t>(city->GetFlammabilitySimulator(), "flammability", 5, U8);
        void* aura = city->GetAuraSimulator();
        Role<int8_t>(aura, "aura", 12, I8);
        Role<int16_t>(aura, "aura", 14, I16); Role<int16_t>(aura, "aura", 15, I16);
        void* traffic = city->GetTrafficSimulator();
        for (unsigned slot : {8u, 9u, 10u, 13u}) Role<uint8_t>(traffic, "traffic", slot, U8);
        void* bds = city->GetBuildingDevelopmentSimulator();
        Role<uint8_t>(bds, "building", 41, U8);
        void* commercial = city->GetCommercialSimulator();
        for (uint32_t key : {0x3320u, 0x3330u}) Role<uint8_t>(commercial, "commercial", 5, U8, true, key);
        void* residential = city->GetResidentialSimulator();
        for (uint32_t wealth = 1; wealth <= 3; ++wealth)
            Role<uint8_t>(residential, "residential", 5, U8, true, wealth);
        Three<uint16_t>(residential, 25, U16);
        Three<float>(residential, 23, F32); Three<float>(residential, 24, F32);
        Role<float>(residential, "residential", 39, F32); Role<float>(residential, "residential", 40, F32);
        Role<uint8_t>(residential, "residential", 41, U8);
        const auto* map = bds ? Call<const GameMapObject*>(bds, 34) : nullptr;
        // Validate the entire dynamic inventory before comparing any developer.
        std::vector<std::pair<uint32_t, void*>> entries;
        const auto status = WalkGameMap(map, 65536, [&](const GameMapNode& node) {
            entries.emplace_back(node.key, reinterpret_cast<void*>(static_cast<uintptr_t>(node.value)));
        });
        if (status != GameMapStatus::Ok) {
            Missing("developers", 34, 0, GameMapStatusName(status));
            return;
        }
        developers = static_cast<uint32_t>(entries.size());
        for (const auto& entry : entries) {
            Role<int16_t>(entry.second, "growth", 13, I16, false, entry.first);
            Role<uint16_t>(entry.second, "growth", 14, U16, false, entry.first);
            Role<int16_t>(entry.second, "growth", 15, I16, false, entry.first);
            Role<uint16_t>(entry.second, "growth", 16, U16, false, entry.first);
        }
    }
};
void Run(void* p) { static_cast<Sample*>(p)->Run(); }
} // namespace

void SimGridRecordShadowObserve(cISC4City* city, int32_t julian, bool loaded) {
    static const bool enabled = SettingInt("simgrid_shadow", "FC4_SIMGRID_SHADOW", 0) != 0;
    if (!enabled) return;
    Sample s{city};
    const bool safe = SehGuard("SG3 simgrid record shadow", &Run, &s);
    if (!safe) ++s.unavailable;
    const uint32_t mismatches = s.differences + s.unavailable;
    Log("simgrid-shadow: phase=%s julian=%d roles=%u expected=%u developers=%u exact=%u/%u mismatches=%u differences=%u unavailable=%u cells=%llu bytes=%llu mode=record-only scope=SG3",
        loaded ? "load" : "month", julian, s.roles, 38u + 4u * s.developers, s.developers,
        s.exact, s.roles, mismatches, s.differences, s.unavailable, s.cells, s.bytes);
}
} // namespace fc4
