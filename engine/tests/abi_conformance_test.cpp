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

// ABI conformance: every interface the plugin implements or calls, as we
// declare it, against docs/abi/interfaces.tsv (read at run time, so a
// regenerated table re-checks everything).
//
// For each declared virtual method the compiler tells us
//   - its slot: decoded from the vcall thunk behind &Class::Method;
//   - its argument bytes: each argument rounded up to 4 (int64/double 8,
//     references 4), plus 4 for the hidden result pointer when a class type
//     is returned by value (MSVC passes it on the stack after `this`);
//   - its return channel: float/double = x87, 64-bit integer = edx:eax.
// and the test asserts, per slot:
//   - argument bytes == callee_pop (wherever the table has it);
//   - declared float/double <=> ret == x87; declared int64 => ret == edx:eax
//     (rows whose ret is pure / forward / via-call are skipped);
// and per interface: the declared slot count == the table's, with no gaps.
//
// Interfaces marked "required" (the plugin's own declarations and the SDK
// ones it calls) fail the test on a mismatch; the rest (SDK declarations we
// replaced, interfaces not used yet) are reported only.
//
// A few SDK declarations disagree with the game in methods the plugin never
// calls; they are waived (kWaivers) for as long as no plugin source calls
// them. Slot-count differences that were reviewed are in kReviewedSlotCounts.
//
// Usage: fc4_abi_conformance_test [--dump] [--plugin-dir DIR] [path/to/interfaces.tsv]
//   --dump  print every declared slot next to its table row

#include "FC4Interfaces.h"

#include "cIGZPersistResourceManager.h"
#include "cIGZString.h"
#include "cIGZVariant.h"
#include "cISC4App.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4City.h"
#include "cISC4ResidentialSimulator.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4Demand.h"
#include "cISC4DemandSimulator.h"
#include "cISC4DepartmentBudget.h"
#include "cISC4DisasterManager.h"
#include "cISC4GameDataRegistry.h"
#include "cISC4HistoryWarehouse.h"
#include "cISC4LineItem.h"
#include "cISC4Lot.h"
#include "cISC4LotManager.h"
#include "cISC4NeighborsSimulator.h"
#include "cISC4Ordinance.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISC4RegionalCity.h"
#include "cISC4Simulator.h"
#include "cISC4WeatherSimulator.h"
#include "cIGZDate.h"
#include "cISCPropertyHolder.h"
#include "cISCResExemplar.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4GrowthDeveloper.h"
#include "cISC4LandValueSimulator.h"
#include "cISC4LotDeveloper.h"
#include "cISC4Region.h"
#include "cISC4SimGrid.h"
#include "cISC4ZoneManager.h"
#include "cISTETerrain.h"

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <map>
#include <regex>
#include <sstream>
#include <string>
#include <type_traits>
#include <vector>

#ifndef FC4_ABI_TSV
#define FC4_ABI_TSV "docs/abi/interfaces.tsv"
#endif
#ifndef FC4_PLUGIN_DIR
#define FC4_PLUGIN_DIR "engine/plugin"
#endif

namespace {

enum class Ret { Eax, Int64, X87, Hidden };

const char* RetName(Ret r) {
    switch (r) {
        case Ret::X87: return "float/double";
        case Ret::Int64: return "int64";
        case Ret::Hidden: return "class by value (hidden pointer)";
        default: return "32-bit/void";
    }
}

struct AbiMethod {
    const char* name;
    int slot;
    int argBytes;
    Ret ret;
};

struct AbiInterface {
    const char* table;
    const char* cls;
    bool required;
    void (*collect)(std::vector<AbiMethod>&);
};

template <class T>
constexpr int ArgBytes() {
    if constexpr (std::is_reference_v<T>) {
        return 4;
    } else {
        return static_cast<int>((sizeof(T) + 3) / 4 * 4);
    }
}

template <class R>
constexpr Ret RetKind() {
    if constexpr (std::is_void_v<R> || std::is_reference_v<R>) {
        return Ret::Eax;  // a reference is a pointer in eax (its type may be incomplete)
    } else if constexpr (std::is_floating_point_v<R>) {
        return Ret::X87;
    } else if constexpr ((std::is_integral_v<R> || std::is_enum_v<R>) && sizeof(R) == 8) {
        return Ret::Int64;
    } else if constexpr (std::is_class_v<R> || std::is_union_v<R>) {
        return Ret::Hidden;
    } else {
        return Ret::Eax;
    }
}

template <class F>
struct Traits;
template <class R, class C, class... A>
struct Traits<R (C::*)(A...)> {
    using Result = R;
    static constexpr int args = (0 + ... + ArgBytes<A>());
};
template <class R, class C, class... A>
struct Traits<R (C::*)(A...) const> {
    using Result = R;
    static constexpr int args = (0 + ... + ArgBytes<A>());
};
// Variadic members are __cdecl: the caller removes the arguments (callee pops 0).
template <class R, class C, class... A>
struct Traits<R (C::*)(A..., ...)> {
    using Result = R;
    static constexpr int args = 0;
};
template <class R, class C, class... A>
struct Traits<R (C::*)(A..., ...) const> {
    using Result = R;
    static constexpr int args = 0;
};

// The vtable slot behind a pointer to a virtual member function: MSVC points
// it at a vcall thunk, "mov eax, [ecx]; jmp [eax + 4*slot]" (possibly behind
// an incremental-linking jump).
int SlotOf(const void* code) {
    const uint8_t* p = static_cast<const uint8_t*>(code);
    for (int i = 0; i < 4 && p[0] == 0xE9; ++i) {
        int32_t rel;
        std::memcpy(&rel, p + 1, 4);
        p = p + 5 + rel;
    }
    // A variadic (__cdecl) member takes `this` on the stack:
    // "mov eax, [esp+4]; mov eax, [eax]; jmp [eax + 4*slot]".
    if (p[0] == 0x8B && p[1] == 0x44 && p[2] == 0x24 && p[3] == 0x04 && p[4] == 0x8B && p[5] == 0x00) {
        p += 6;
        if (p[0] != 0xFF) return -1;
        if (p[1] == 0x20) return 0;
        if (p[1] == 0x60) return static_cast<int8_t>(p[2]) / 4;
        if (p[1] == 0xA0) {
            int32_t off;
            std::memcpy(&off, p + 2, 4);
            return off / 4;
        }
        return -1;
    }
    if (p[0] != 0x8B || p[1] != 0x01 || p[2] != 0xFF) return -1;
    if (p[3] == 0x20) return 0;
    if (p[3] == 0x60) return static_cast<int8_t>(p[4]) / 4;
    if (p[3] == 0xA0) {
        int32_t off;
        std::memcpy(&off, p + 4, 4);
        return off / 4;
    }
    return -1;
}

template <class F>
AbiMethod AbiM(F f, const char* name) {
    static_assert(sizeof(F) == sizeof(void*), "single-inheritance member function pointer expected");
    const void* code = nullptr;
    std::memcpy(&code, &f, sizeof(code));
    constexpr Ret r = RetKind<typename Traits<F>::Result>();
    return AbiMethod{name, SlotOf(code), Traits<F>::args + (r == Ret::Hidden ? 4 : 0), r};
}

#include "abi_entries.inc"

struct Row {
    std::string calleePop;
    std::string ret;
    std::string method;
    std::string notes;
};

using Table = std::map<std::string, std::map<int, Row>>;

std::vector<std::string> Split(const std::string& line) {
    std::vector<std::string> out;
    std::string cur;
    std::istringstream ss(line);
    while (std::getline(ss, cur, '\t')) out.push_back(cur);
    if (!line.empty() && line.back() == '\t') out.emplace_back();
    return out;
}

bool LoadTable(const char* path, Table& table, const char* keyColumn = "interface", const char* prefix = "") {
    std::ifstream f(path, std::ios::binary);
    if (!f) return false;
    std::string line;
    if (!std::getline(f, line)) return false;
    if (!line.empty() && line.back() == '\r') line.pop_back();
    const std::vector<std::string> head = Split(line);
    auto col = [&](const char* name) {
        for (size_t i = 0; i < head.size(); ++i) {
            if (head[i] == name) return static_cast<int>(i);
        }
        return -1;
    };
    const int cIface = col(keyColumn), cSlot = col("slot"), cPop = col("callee_pop"), cRet = col("ret"),
              cMethod = col("method"), cNotes = col("notes");
    if (cIface < 0 || cSlot < 0 || cPop < 0 || cRet < 0) return false;
    while (std::getline(f, line)) {
        if (!line.empty() && line.back() == '\r') line.pop_back();
        if (line.empty()) continue;
        std::vector<std::string> v = Split(line);
        v.resize(head.size());
        Row r;
        r.calleePop = v[cPop];
        r.ret = v[cRet];
        r.method = cMethod >= 0 ? v[cMethod] : "";
        r.notes = cNotes >= 0 ? v[cNotes] : "";
        table[prefix + v[cIface]][std::atoi(v[cSlot].c_str())] = r;
    }
    return true;
}

// The game's implementing class can have virtual methods of its own after the
// interface's (a virtual destructor, a class-private virtual), or the table can
// stop short of the interface. Each entry is one reviewed case: interface,
// declared slot count, table slot count, and why. A regenerated table with a
// different count fails again.
struct SlotCount {
    const char* table;
    int declared;
    int tableSlots;
    const char* why;
};
const SlotCount kReviewedSlotCounts[] = {
    {"cISC4BudgetSimulator", 59, 61,
     "slot 59 is cSC4BudgetSimulator's destructor and slot 60 its GetSimulatorType: class-private, not part of "
     "the interface (budget spec: 59 interface slots)"},
    {"cISC4LineItem", 20, 21,
     "slot 20 is unnamed and pops 4, the signature of a scalar deleting destructor: class-private"},
    {"cISC4DepartmentBudget", 34, 33,
     "the budget spec lists HasLocallyFundedItemsByPurpose as slot 33; the table stops at 32 but its next "
     "vtable (cSC4LineItem::vftable_1 at 00ab0f3c, locator at 00ab0f38) leaves room for one more entry at "
     "00ab0f34. Kept; the game cannot reach a slot it does not have [spec question]"},
    {"cISC4LandValueSimulator", 12, 13,
     "slot 12 is unnamed and pops 4, the signature of a scalar deleting destructor: class-private "
     "(landvalue spec: 12 interface slots)"},
    {"cISTETerrain", 91, 98,
     "slot 91 is cSTETerrain's destructor and slots 92-97 (GetWaterDepthAtVertex, Flip*Triangulation, "
     "Load/SaveObliterated, Load, Save) are cSTETerrain's own virtuals, past the SDK interface"},
};

const SlotCount* ReviewedSlotCount(const char* table, int declared, int tableSlots) {
    for (const SlotCount& k : kReviewedSlotCounts) {
        if (std::strcmp(k.table, table) == 0 && k.declared == declared && k.tableSlots == tableSlots) return &k;
    }
    return nullptr;
}

// SDK declarations that disagree with the game in methods the plugin never
// calls (the SDK header is vendored and not ours to change). The mismatch is
// printed as "waived" and does not fail the test, as long as no plugin source
// calls the method. Before calling one, declare the interface in
// FC4Interfaces.h with the table's signature (as was done for
// cISC4NeighborsSimulator) and drop the waiver.
struct Waiver {
    const char* table;
    const char* cls;
    int slot;
    const char* method;
    const char* why;
};
const Waiver kWaivers[] = {
    {"cIGZPersistResourceManager", "cIGZPersistResourceManager", 22, "FindObjectFactory",
     "the SDK declares one argument, the game's method pops two"},
    {"cISC4BuildingDevelopmentSimulator", "cISC4BuildingDevelopmentSimulator", 9, "GetCostOfOccupantDemolition",
     "the SDK declares no argument, the game's method pops 8 bytes"},
    {"cISC4BuildingDevelopmentSimulator", "cISC4BuildingDevelopmentSimulator", 45, "GetMaxCapacity",
     "the SDK says int64, the game returns a float in ST0"},
    {"cISC4HistoryWarehouse", "cISC4HistoryWarehouse", 9, "Coalesce",
     "the SDK declares 12 argument bytes, the game's method pops 16"},
    {"cISC4OrdinanceSimulator", "cISC4OrdinanceSimulator", 24, "GetCumulativeOffset",
     "the SDK says int64, the game returns 32 bits in eax"},
    {"vt:cSC4City::vftable", "cISC4City", 96, "CellCornerToPosition", "the game returns in ST0"},
    {"vt:cSC4City::vftable", "cISC4City", 97, "CellCenterToPosition", "the game returns in ST0"},
    {"vt:cSC4ResidentialSimulator::vftable", "cISC4ResidentialSimulator", 10, "ChanceOfSchoolStrike",
     "pops 4 and returns in eax (SDK: no argument, float)"},
    {"vt:cSC4ResidentialSimulator::vftable", "cISC4ResidentialSimulator", 11, "ChanceOfHealthStrike",
     "pops 4 and returns in eax (SDK: no argument, float)"},
    {"vt:cSC4ResidentialSimulator::vftable", "cISC4ResidentialSimulator", 12, "GetSchoolSystemRating",
     "pops 4 and returns in eax (SDK: no argument, float)"},
    {"vt:cSC4ResidentialSimulator::vftable", "cISC4ResidentialSimulator", 13, "GetHealthSystemRating",
     "pops 4 and returns in eax (SDK: no argument, float)"},
    {"vt:cSC4ResidentialSimulator::vftable", "cISC4ResidentialSimulator", 30, "GetCellWorkforcePercent",
     "pops 12 and returns in eax (SDK: 8 bytes, float)"},
    {"vt:cSC4ResidentialSimulator::vftable", "cISC4ResidentialSimulator", 31, "GetGlobalWorkforcePercent",
     "pops 4 and returns in eax (SDK: no argument, float)"},
    {"vt:cSC4ResidentialSimulator::vftable", "cISC4ResidentialSimulator", 34, "GetGlobalLE",
     "returns in eax (SDK: float)"},
    {"vt:cSC4ResidentialSimulator::vftable", "cISC4ResidentialSimulator", 56, "GetAutoBudget",
     "pops 4 (SDK: no argument)"},
    {"vt:cSC4ResidentialSimulator::vftable", "cISC4ResidentialSimulator", 57, "SetAutoBudget",
     "pops 8 (SDK: 4 bytes)"},
    {"vt:cSC4DemandSimulator::vftable", "cISC4DemandSimulator", 8, "CalculateJobsPerUnitOfDemand",
     "the game returns in ST0 (SDK: void)"},
    {"cISC4Demand", "cISC4Demand", 16, "AddToSupplyValue", "the game returns in ST0 (SDK: bool)"},
    {"cISC4Demand", "cISC4Demand", 17, "AddToDemandValue", "the game returns in ST0 (SDK: bool)"},
    {"cISC4Demand", "cISC4Demand", 30, "EndOfCycle", "the game returns in eax (SDK: float)"},
    // The game's grids, used by the land value simulator (simgrid.md §4).
    {"cISC4SimGrid<uint8>", "cISC4SimGrid<uint8_t>", 20, "TractCornerToPosition", "the game returns in ST0 (SDK: bool)"},
    {"cISC4SimGrid<uint8>", "cISC4SimGrid<uint8_t>", 21, "TractCenterToPosition", "the game returns in ST0 (SDK: bool)"},
    {"cISC4SimGrid<int16>", "cISC4SimGrid<int16_t>", 20, "TractCornerToPosition", "the game returns in ST0 (SDK: bool)"},
    {"cISC4SimGrid<int16>", "cISC4SimGrid<int16_t>", 21, "TractCenterToPosition", "the game returns in ST0 (SDK: bool)"},
    {"cISC4SimGrid<float>", "cISC4SimGrid<float>", 20, "TractCornerToPosition", "the game returns in ST0 (SDK: bool)"},
    {"cISC4SimGrid<float>", "cISC4SimGrid<float>", 21, "TractCenterToPosition", "the game returns in ST0 (SDK: bool)"},
    {"cISTETerrain", "cISTETerrain", 6, "CellWidth", "the game returns in ST0 (SDK: 32-bit)"},
    {"cISTETerrain", "cISTETerrain", 27, "GetLerpedNormal", "the game returns in ST0 (SDK: 32-bit)"},
    {"cISTETerrain", "cISTETerrain", 59, "BeginBrushScrub", "pops 8 (SDK: 4 bytes)"},
    {"cISC4LotDeveloper", "cISC4LotDeveloper", 23, "IsAreaDeveloped", "pops 4 (SDK: no argument)"},
    {"cISC4LotDeveloper", "cISC4LotDeveloper", 53, "RemoveLotFoundation", "pops 8 (SDK: 4 bytes)"},
};

const Waiver* FindWaiver(const char* table, const char* cls, int slot) {
    for (const Waiver& w : kWaivers) {
        if (std::strcmp(w.table, table) == 0 && std::strcmp(w.cls, cls) == 0 && w.slot == slot) return &w;
    }
    return nullptr;
}

// Plugin sources that call `method` (->method( or .method( ).
std::vector<std::string> CallersOf(const char* pluginDir, const char* method) {
    std::vector<std::string> out;
    std::error_code ec;
    const std::regex call(std::string(R"((->|\.)\s*)") + method + R"(\s*\()");
    for (const auto& e : std::filesystem::directory_iterator(pluginDir, ec)) {
        const std::string ext = e.path().extension().string();
        if (ext != ".cpp" && ext != ".h") continue;
        std::ifstream f(e.path(), std::ios::binary);
        std::stringstream ss;
        ss << f.rdbuf();
        if (std::regex_search(ss.str(), call)) out.push_back(e.path().filename().string());
    }
    return out;
}

}  // namespace

int main(int argc, char** argv) {
    const char* path = FC4_ABI_TSV;
    const char* pluginDir = FC4_PLUGIN_DIR;
    bool dump = false;
    for (int i = 1; i < argc; ++i) {
        if (std::strcmp(argv[i], "--dump") == 0) {
            dump = true;
        } else if (std::strcmp(argv[i], "--plugin-dir") == 0 && i + 1 < argc) {
            pluginDir = argv[++i];
        } else {
            path = argv[i];
        }
    }
    Table table;
    if (!LoadTable(path, table)) {
        std::fprintf(stderr, "cannot read %s\n", path);
        return 2;
    }
    if (!std::filesystem::is_directory(pluginDir)) {
        std::fprintf(stderr, "plugin source directory %s not found\n", pluginDir);
        return 2;
    }
    // vtables.tsv next to it: whole vtables of the game's classes, for interfaces
    // that interfaces.tsv does not curate (entries named "vt:<vtable>").
    const std::string vtPath = (std::filesystem::path(path).parent_path() / "vtables.tsv").string();
    const bool haveVtables = LoadTable(vtPath.c_str(), table, "class", "vt:");
    std::printf("ABI table: %s (%u interfaces and vtables; vtables.tsv %s)\n", path,
                static_cast<unsigned>(table.size()), haveVtables ? "loaded" : "NOT FOUND");

    int failures = 0, reported = 0, waived = 0, checkedSlots = 0;
    for (const AbiInterface& iface : kAbiInterfaces) {
        std::vector<AbiMethod> methods;
        iface.collect(methods);
        int mismatches = 0;
        // slot < 0: about the whole interface.
        auto report = [&](int slot, const char* fmt, auto... args) {
            const Waiver* w = iface.required && slot >= 0 ? FindWaiver(iface.table, iface.cls, slot) : nullptr;
            std::vector<std::string> callers;
            if (w) callers = CallersOf(pluginDir, w->method);
            const bool isWaived = w && callers.empty();
            std::printf("  %s %s / %s: ", isWaived ? "waived  " : iface.required ? "MISMATCH" : "note    ",
                        iface.table, iface.cls);
            std::printf(fmt, args...);
            if (isWaived) std::printf(" [never called by the plugin: %s]", w->why);
            for (const std::string& c : callers) {
                std::printf(" [waiver void: plugin/%s calls %s - declare it correctly first]", c.c_str(), w->method);
            }
            std::printf("\n");
            if (isWaived) {
                ++waived;
            } else {
                ++mismatches;
            }
        };

        // Slots: decoded, unique, contiguous from 0.
        std::map<int, const AbiMethod*> bySlot;
        for (const AbiMethod& m : methods) {
            if (m.slot < 0) {
                report(-1, "%s: slot could not be decoded from the vcall thunk", m.name);
                continue;
            }
            if (bySlot.count(m.slot)) {
                report(-1, "slot %d declared twice (%s, %s)", m.slot, bySlot[m.slot]->name, m.name);
                continue;
            }
            bySlot[m.slot] = &m;
        }
        const int declared = bySlot.empty() ? 0 : bySlot.rbegin()->first + 1;
        for (int s = 0; s < declared; ++s) {
            if (!bySlot.count(s)) report(-1, "slot %d missing from the declaration (regenerate abi_entries.inc?)", s);
        }

        auto t = table.find(iface.table);
        if (t == table.end()) {
            report(-1, "not in the ABI table");
        } else {
            const std::map<int, Row>& rows = t->second;
            const int tableSlots = rows.empty() ? 0 : rows.rbegin()->first + 1;
            // A whole class vtable (vt:) may have more slots than the interface.
            const bool wholeVtable = std::strncmp(iface.table, "vt:", 3) == 0;
            if (declared != tableSlots && !(wholeVtable && declared < tableSlots)) {
                if (const SlotCount* k = ReviewedSlotCount(iface.table, declared, tableSlots)) {
                    if (iface.required) {
                        std::printf("  reviewed %s: declares %d slots, the table has %d: %s\n", iface.table,
                                    declared, tableSlots, k->why);
                    }
                } else {
                    report(-1, "declares %d slots, the table has %d", declared, tableSlots);
                }
            }
            for (const auto& kv : bySlot) {
                const AbiMethod& m = *kv.second;
                auto r = rows.find(m.slot);
                if (dump) {
                    std::printf("    %3d %-40s args %2d ret %-12s | ", m.slot, m.name, m.argBytes, RetName(m.ret));
                    if (r == rows.end()) {
                        std::printf("(not in the table)\n");
                    } else {
                        std::printf("pop %-3s ret %-11s %s %s\n", r->second.calleePop.c_str(),
                                    r->second.ret.c_str(), r->second.method.c_str(), r->second.notes.c_str());
                    }
                }
                if (r == rows.end()) continue;
                const Row& row = r->second;
                ++checkedSlots;
                if (!row.calleePop.empty()) {
                    const int pop = std::atoi(row.calleePop.c_str());
                    if (pop != m.argBytes) {
                        report(m.slot, "slot %d %s: declared %d argument bytes, the game's method pops %d (%s)",
                               m.slot, m.name, m.argBytes, pop, row.method.c_str());
                    }
                }
                // Return channel. "x87|eax" (low confidence) is either one: only an int64 declaration
                // contradicts it. "edx:eax?" (low) counts as edx:eax for an int64 declaration but does not
                // demand one. pure / forward / via-call / unknown say nothing.
                const std::string& ret = row.ret;
                if (ret.empty() || ret == "pure" || ret == "forward" || ret == "via-call" || ret == "unknown") continue;
                const bool x87 = ret == "x87";
                const bool either = ret == "x87|eax";
                const bool edxEax = ret == "edx:eax" || ret == "edx:eax?";
                if (!either && (m.ret == Ret::X87) != x87) {
                    report(m.slot, "slot %d %s: declared return %s, the game returns in %s (%s)", m.slot, m.name,
                           RetName(m.ret), ret.c_str(), row.method.c_str());
                } else if (m.ret == Ret::Int64 && !edxEax) {
                    report(m.slot, "slot %d %s: declared int64, the game returns in %s (%s)", m.slot, m.name,
                           ret.c_str(), row.method.c_str());
                } else if (ret == "edx:eax" && m.ret != Ret::Int64) {
                    // Beyond the table's stated rules: an implementation returning 32 bits here would leave
                    // garbage in edx for a game caller that reads 64, and a caller would drop the high half.
                    report(m.slot, "slot %d %s: declared return %s, the game returns a 64-bit value in edx:eax (%s)",
                           m.slot, m.name, RetName(m.ret), row.method.c_str());
                }
            }
        }
        std::printf("%s %-36s %-36s %3d slots declared: %s\n", iface.required ? "[required]" : "[report]  ",
                    iface.table, iface.cls, declared, mismatches ? "MISMATCHES ABOVE" : "ok");
        if (iface.required) {
            failures += mismatches;
        } else {
            reported += mismatches;
        }
    }
    std::printf("%d slots compared; %d mismatch(es) in required interfaces, %d waived, %d in reported-only ones\n",
                checkedSlots, failures, waived, reported);
    return failures == 0 ? 0 : 1;
}
