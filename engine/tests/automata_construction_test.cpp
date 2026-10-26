// SPDX-License-Identifier: GPL-3.0-or-later
// Exercise the real replacement's native construction setup through slot calls.
#include "FC4AutomataInterfaces.h"
#include "game_abi.h"
#include "cIGZCOMDirector.h"
#include "cGZPersistResourceKey.h"
#include "cS3DVector3.h"
#include <windows.h>
#include <cstdio>

namespace {
void EnumClass(uint32_t, uint32_t, void*) {}
}

int main(int argc, char** argv) {
    if (argc != 2) return 1;
    SetEnvironmentVariableA("FC4_ENABLE", "automata");
    SetEnvironmentVariableA("FC4_DISABLE", "simulator");
    HMODULE dll = LoadLibraryA(argv[1]);
    if (!dll) return 2;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(dll, "GZDllGetGZCOMDirector"));
    auto* director = getDirector ? getDirector() : nullptr;
    if (!director) return 3;
    director->EnumClassObjects(&EnumClass, nullptr);
    for (const uint32_t managerClass : {0xC98F9D17u, 0xA98F9D3Bu}) {
    cFC4ISC4AutomataManager* manager = nullptr;
    if (!director->GetClassObject(managerClass, 0x098F9BF4, reinterpret_cast<void**>(&manager)) || !manager)
        return 4;
    manager->SetMaxCount(16);
    cS3DVector3 position{};
    position.fX = 12.5f;
    position.fY = 7.0f;
    position.fZ = 34.5f;
    cGZPersistResourceKey key;
    auto* vehicle = manager->CreateAutomaton(0x4101, position, 0x32000, key, nullptr, 0x32, nullptr);
    if (!vehicle) return 5;
    cS3DVector3 direction{};
    direction.fX = 3.0f;
    direction.fZ = 4.0f;
    const auto flags = fc4test::CallSlot(vehicle, 30, {2, 0});
    const auto orient = fc4test::CallSlot(vehicle, 14,
        {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(&direction))});
    cS3DVector3 roof{};
    const auto get = fc4test::CallSlot(vehicle, 7,
        {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(&roof))});
    const bool spawn = roof.fX == 12.5f && roof.fY == 7.0f && roof.fZ == 34.5f;
    roof.fY = 9.0f;
    const auto set = fc4test::CallSlot(vehicle, 8,
        {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(&roof))});
    roof = {};
    fc4test::CallSlot(vehicle, 7, {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(&roof))});
    std::printf("construction manager %08X: stack slots30/14/7/8=%d/%d/%d/%d; roof=%g,%g,%g\n",
        managerClass, flags.espDelta, orient.espDelta, get.espDelta, set.espDelta, roof.fX, roof.fY, roof.fZ);
    const bool ok = spawn && roof.fX == 12.5f && roof.fY == 9.0f && roof.fZ == 34.5f &&
        get.espDelta == 0 && set.espDelta == 0 && get.x87Pushed == 0 && set.x87Pushed == 0 &&
        flags.espDelta == 0 && orient.espDelta == 0 &&
        flags.x87Pushed == 0 && orient.x87Pushed == 0;
    // CreateAutomaton returns a borrowed pointer; only the factory's manager
    // reference is ours. The manager owns the wrapper until its destruction.
    manager->Release();
    if (!ok) return 6;
    }
    return 0;
}
