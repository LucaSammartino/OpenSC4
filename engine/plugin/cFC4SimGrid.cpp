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

// The grid class family (docs/specs/simgrid.md §2): factories for
// 0x49B9E601..0x49B9E60B over FC4SimGridLogic.h, with buffers from the game's
// STL node allocator B and the city read through the application.
//
// Registered only with `enable = simgrids` (FC4Subsystems.cpp). §6.4.2 forbids
// registering these class IDs unless the grids are layout-compatible (§6.3.2):
// these are (vector<vector<T>> in the STLport layout, B buffers), so the
// class store's grids on load, and every grid our own glue creates through
// GZCOM, are then ours, while the original simulators' directly constructed
// grids (§6.2) stay the game's.
#include "cFC4SimGrid.h"

#include "FC4Checked.h"
#include "FC4GameList.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4SimGridLogic.h"

#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cRZCOMDllDirector.h"

#include <atomic>

namespace fc4 {
namespace {

// The city through the application (§3: the original reads the global city).
struct GameSimGridHost {
    static bool Get(SimGridCity& out) {
        cIGZFrameWork* fw = RZGetFrameWork();
        cIGZApp* app = fw ? fw->Application() : nullptr;
        if (app == nullptr) return false;
        FC4_OUT(cISC4App*, sc4, "cISC4App* (QueryInterface out pointer)");
        const bool ok = app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(sc4.ptr()));
        sc4.Check();
        if (!ok || sc4.get() == nullptr) return false;
        FC4_ACQUIRED(sc4.get(), "QueryInterface");
        cISC4City* city = sc4.get()->GetCity();  // borrowed
        bool have = false;
        if (city != nullptr) {
            out.cellCountX = city->CellCountX();
            out.cellCountZ = city->CellCountZ();
            out.cellWidthX = city->CellWidthX();
            out.cellWidthZ = city->CellWidthZ();
            have = true;
        }
        FC4_RELEASE(sc4.get());
        return have;
    }

    // At most this many per-grid read/write lines (a save writes ~30 grids).
    static constexpr uint32_t kTraceCap = 400;

    static void Trace(const char* what, uint32_t clsid, uint32_t instance, int32_t s, int32_t countX,
                      int32_t countZ, uint32_t ifaceWrites, int changed) {
        static std::atomic<uint32_t> lines{0};
        SimGridStats& st = GetSimGridStats();
        if (what[0] == 'a') {  // all-released
            Log("simgrids: all grids released: created=%u destroyed=%u reads=%u read_failures=%u writes=%u "
                "resizes=%u alloc_failures=%u grid_data_calls=%u iface_writes=%u raw_changed=%u",
                st.created.load(), st.destroyed.load(), st.reads.load(), st.readFailures.load(), st.writes.load(),
                st.resizes.load(), st.allocFailures.load(), st.gridDataCalls.load(), st.ifaceWrites.load(),
                st.rawChanged.load());
            return;
        }
        if (lines.fetch_add(1) >= kTraceCap) return;
        Log("simgrids: %s clsid=0x%08X instance=0x%08X S=%d count=%dx%d iface_writes=%u changed=%d "
            "(live=%u grid_data_calls=%u)",
            what, clsid, instance, s, countX, countZ, ifaceWrites, changed,
            st.created.load() - st.destroyed.load(), st.gridDataCalls.load());
    }
};

template <class T>
using GameGrid = cFC4SimGridT<T, StlNodeAllocator, GameSimGridHost>;

// [choice] Refused (nullptr) without the game's allocator B (unknown
// executable): a grid whose buffers came from another heap must not reach
// original code (§6.4.5).
template <class T>
cIGZUnknown* MakeGrid(uint32_t clsid) {
    if (!StlNodeAllocator::Available()) {
        static std::atomic<bool> warned{false};
        if (!warned.exchange(true)) Log("simgrids: WARNING no STL node allocator: grid classes refused");
        return nullptr;
    }
    auto* g = new GameGrid<T>();
    g->SetClsid(clsid);
    return static_cast<cIGZUnknown*>(static_cast<cISC4SimGrid<T>*>(g));
}

}  // namespace
}  // namespace fc4

// §2: "bool" (0x49B9E601) creates a uint8 grid whose CLSID says 0x49B9E602.
cIGZUnknown* CreateFC4SimGridBool() { return fc4::MakeGrid<uint8_t>(0x49B9E602); }
cIGZUnknown* CreateFC4SimGridU8() { return fc4::MakeGrid<uint8_t>(0x49B9E602); }
cIGZUnknown* CreateFC4SimGridS8() { return fc4::MakeGrid<int8_t>(0x49B9E603); }
cIGZUnknown* CreateFC4SimGridU16() { return fc4::MakeGrid<uint16_t>(0x49B9E604); }
cIGZUnknown* CreateFC4SimGridS16() { return fc4::MakeGrid<int16_t>(0x49B9E605); }
cIGZUnknown* CreateFC4SimGridU32() { return fc4::MakeGrid<uint32_t>(0x49B9E606); }
cIGZUnknown* CreateFC4SimGridS32() { return fc4::MakeGrid<int32_t>(0x49B9E607); }
cIGZUnknown* CreateFC4SimGridU64() { return fc4::MakeGrid<uint64_t>(0x49B9E608); }
cIGZUnknown* CreateFC4SimGridS64() { return fc4::MakeGrid<int64_t>(0x49B9E609); }
cIGZUnknown* CreateFC4SimGridF32() { return fc4::MakeGrid<float>(0x49B9E60A); }
cIGZUnknown* CreateFC4SimGridF64() { return fc4::MakeGrid<double>(0x49B9E60B); }
