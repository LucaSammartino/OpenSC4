// SPDX-License-Identifier: GPL-3.0-or-later
// Real DLL boundary oracle for automata.md 7.6.1/7.6.4/7.6.5 (NM17, AT25).
#include "FC4AutomataInterfaces.h"
#include "game_abi.h"
#include "cIGZCOMDirector.h"
#include "cGZPersistResourceKey.h"
#include "cS3DVector3.h"
#include <windows.h>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <iterator>
#include <string>

namespace {
// The DLL's ini takes precedence over FC4_ENABLE, including ini files left by
// other serial DLL tests. Restore the exact previous bytes on every exit path.
struct ScopedIni {
    std::string path, previous;
    bool existed = false, ready = false;
    explicit ScopedIni(const char* dll) {
        const std::string module(dll);
        path = module.substr(0, module.find_last_of("/\\") + 1) + "FreeCity4.ini";
        std::ifstream input(path, std::ios::binary);
        existed = input.is_open();
        previous.assign(std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>());
        input.close();
        std::ofstream output(path, std::ios::binary | std::ios::trunc);
        output << "[subsystems]\nenable = automata\ndisable = simulator\n";
        output.close();
        ready = !output.fail();
    }
    ~ScopedIni() {
        if (existed) {
            std::ofstream output(path, std::ios::binary | std::ios::trunc);
            output << previous;
        } else {
            DeleteFileA(path.c_str());
        }
    }
};

void EnumClass(uint32_t, uint32_t, void*) {}
uint32_t Address(const void* p) { return static_cast<uint32_t>(reinterpret_cast<uintptr_t>(p)); }
uint32_t Word(float f) { uint32_t u; std::memcpy(&u, &f, sizeof(u)); return u; }
float Float(uint32_t u) { float f; std::memcpy(&f, &u, sizeof(f)); return f; }
bool Balanced(const fc4test::SlotCall& call) { return call.espDelta == 0 && call.x87Pushed == 0; }

struct GuardedPosition {
    uint32_t before = 0xDEAD1234;
    cS3DVector3 value;
    uint32_t after = 0xABCD5678;
};
static_assert(sizeof(cS3DVector3) == 12, "native position is exactly three f32 words");

bool Get(void* worker, const cS3DVector3& expected) {
    GuardedPosition out;
    const auto call = fc4test::CallSlot(worker, 7, {Address(&out.value)});
    return Balanced(call) && call.eax == Address(&out.value) &&
        out.before == 0xDEAD1234 && out.after == 0xABCD5678 &&
        Word(out.value.fX) == Word(expected.fX) && Word(out.value.fY) == Word(expected.fY) &&
        Word(out.value.fZ) == Word(expected.fZ);
}

bool Crew(cIGZCOMDirector* director, uint32_t managerClass, bool drainFirst) {
    cFC4ISC4AutomataManager* manager = nullptr;
    if (!director->GetClassObject(managerClass, 0x098F9BF4, reinterpret_cast<void**>(&manager)) || !manager)
        return false;
    manager->SetMaxCount(16);
    void* workers[3]{};
    void* vtables[3]{};
    cS3DVector3 expected[3] = {{10, 7, 30}, {11, 7, 31}, {12.5f, 7, 34.5f}};
    const uint32_t instances[] = {0x0210B000, 0x0210C000, 0x02109000};
    for (int i = 0; i != 3; ++i) {
        cGZPersistResourceKey key;
        key.group = 0x29A5D1EC; key.type = 0x49A593E7; key.instance = instances[i];
        auto* worker = manager->CreateAutomaton(0x4101, expected[i], 0x32000, key, nullptr, 0x32, nullptr);
        if (!worker) return false;
        workers[i] = worker;
        vtables[i] = *reinterpret_cast<void**>(worker);
        cS3DVector3 direction(3, 0, 4);
        if (!Balanced(fc4test::CallSlot(worker, 30, {2, 0})) ||
            !Balanced(fc4test::CallSlot(worker, 14, {Address(&direction)})) ||
            !Balanced(fc4test::CallSlot(worker, 1, {}))) return false;
        // Caller-local creation/orientation/key buffers expire; no input is retained.
        direction = cS3DVector3(-1, -2, -3);
        if (!Get(worker, expected[i])) return false;
    }
    struct Event { const char* name; bool scaffold; uint32_t y; };
    const Event trace[] = {
#include "../conformance/automata-construction-heights.inc"
    };
    for (const auto& event : trace) {
        if (event.scaffold) {
            GuardedPosition roof;
            const auto get = fc4test::CallSlot(workers[2], 7, {Address(&roof.value)});
            if (!Balanced(get) || get.eax != Address(&roof.value) ||
                roof.before != 0xDEAD1234 || roof.after != 0xABCD5678 ||
                std::memcmp(&roof.value, &expected[2], 12) != 0) return false;
            roof.value.fY = Float(event.y);
            if (!Balanced(fc4test::CallSlot(workers[2], 8, {Address(&roof.value)}))) return false;
            expected[2].fY = Float(event.y);
            roof.value = cS3DVector3(-100, -200, -300); // setter must copy, never retain
        }
        for (int i = 0; i != 3; ++i) {
            if (*reinterpret_cast<void**>(workers[i]) != vtables[i] || !Get(workers[i], expected[i])) {
                std::printf("FAIL manager %08X event %s worker %d\n", managerClass, event.name, i);
                return false;
            }
        }
    }
    // D8: removal and final owner Release in creation order. Drain both before
    // and after that owner release; no optional owners or concrete worker view.
    if (drainFirst) manager->SetMaxCount(0);
    for (int i = 0; i != 3; ++i) {
        if (!Balanced(fc4test::CallSlot(manager, 8, {Address(workers[i]), 0})) ||
            !Get(workers[i], expected[i]) || !Balanced(fc4test::CallSlot(workers[i], 2, {}))) return false;
    }
    if (!drainFirst) manager->SetMaxCount(0);
    manager->Release();
    std::printf("construction manager %08X drainFirst=%d: AT25 trace, EAX, stack/x87, guards, lifetime PASS\n",
        managerClass, drainFirst);
    return true;
}
}

int main(int argc, char** argv) {
    if (argc != 2) return 1;
    ScopedIni ini(argv[1]);
    if (!ini.ready) return 5;
    HMODULE dll = LoadLibraryA(argv[1]);
    if (!dll) return 2;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(dll, "GZDllGetGZCOMDirector"));
    auto* director = getDirector ? getDirector() : nullptr;
    if (!director) return 3;
    director->EnumClassObjects(&EnumClass, nullptr);
    for (const uint32_t managerClass : {0xC98F9D17u, 0xA98F9D3Bu})
        for (bool drainFirst : {false, true})
            if (!Crew(director, managerClass, drainFirst)) return 4;
    return 0;
}
