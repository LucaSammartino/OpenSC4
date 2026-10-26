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
    cFC4ISC4AutomataManager* manager = nullptr;
    if (!director->GetClassObject(0xA98F9D3B, 0x098F9BF4, reinterpret_cast<void**>(&manager)) || !manager)
        return 4;
    manager->SetMaxCount(16);
    cS3DVector3 position{};
    cGZPersistResourceKey key;
    auto* vehicle = manager->CreateAutomaton(1, position, 0, key, nullptr, 0xFF, nullptr);
    if (!vehicle) return 5;
    cS3DVector3 direction{};
    direction.fX = 3.0f;
    direction.fZ = 4.0f;
    const auto flags = fc4test::CallSlot(vehicle, 30, {0x900, 1});
    const auto orient = fc4test::CallSlot(vehicle, 14,
        {static_cast<uint32_t>(reinterpret_cast<uintptr_t>(&direction))});
    std::printf("construction vehicle stack: slot30=%d slot14=%d; x87=%d,%d\n",
        flags.espDelta, orient.espDelta, flags.x87Pushed, orient.x87Pushed);
    const bool ok = flags.espDelta == 0 && orient.espDelta == 0 &&
        flags.x87Pushed == 0 && orient.x87Pushed == 0;
    // CreateAutomaton returns a borrowed pointer; only the factory's manager
    // reference is ours. The manager owns the wrapper until its destruction.
    manager->Release();
    return ok ? 0 : 6;
}
