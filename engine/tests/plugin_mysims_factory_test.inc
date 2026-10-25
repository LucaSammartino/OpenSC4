// SPDX-License-Identifier: GPL-3.0-or-later
// The MySim owner group (cSC4MySim, cSC4MyStreetSim, cSC4MySimDispatch) is
// registered whole with `enable = advisors, mysims` and not at all otherwise.
#include "FC4MySimDispatchContract.h"

enum class MySimsIni { Default, Alone, Mixed, Replace };

int MySimsFactoryBoundaryTest(const std::string& dll, const std::string& ini, const std::string& log,
                              MySimsIni mode) {
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_MYSIMS_SHADOW", nullptr);
    SetEnvironmentVariableA("FC4_MYSIMDISPATCH_SHADOW", nullptr);
    const char* text = "[subsystems]\n";
    if (mode == MySimsIni::Alone) text = "[subsystems]\nenable = mysims\n";
    if (mode == MySimsIni::Mixed) text = "[subsystems]\nenable = tutorial,mysims\n";
    if (mode == MySimsIni::Replace) text = "[subsystems]\nenable = advisors,mysims\n";
    WriteTextFile(ini, text);
    const bool replaced = mode == MySimsIni::Replace;
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (!hd) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    if (director) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        // An unavailable group must not prevent another requested factory from
        // registering, nor advertise only part of the three-owner group.
        CHECK((std::count(classes.begin(), classes.end(), 0xAA3836FBu) == 1) == (mode == MySimsIni::Mixed));
        CHECK((std::count(classes.begin(), classes.end(), 0x4A0DD616u) == 1) == replaced);
        const uint32_t ids[] = {0x4A1DBBBFu, 0x0BF606A5u, 0xCBC14674u};
        for (uint32_t id : ids) CHECK((std::count(classes.begin(), classes.end(), id) == 1) == replaced);
        if (!replaced) {
            const uint32_t iids[] = {0x0A1DA42Fu, 0xEB8B6F9Au, 0xE4FDA3D4u, 0x452294AAu};
            for (uint32_t id : ids) {
                for (uint32_t iid : iids) {
                    void* out = nullptr;
                    CHECK(!director->GetClassObject(id, iid, &out));
                    CHECK(out == nullptr);
                }
            }
        } else {
            // The dispatch: ours, with the §7.2.3 constructor contract.
            void* out = nullptr;
            CHECK(director->GetClassObject(0xCBC14674u, 0xEB8B6F9Au, &out));
            CHECK(out != nullptr);
            if (out != nullptr) {
                auto* d = static_cast<fc4::MySimDispatchPublic*>(out);
                CHECK(d->GetRefCount() == 1);
                const auto r = fc4::CheckMySimDispatchConstructor(*d, [](cIGZUnknown* u) { u->Release(); });
                CHECK(r.fields == 12);
                CHECK(r.differences == 0);
                void* target = nullptr;
                CHECK(d->QueryInterface(0x452294AAu, &target) && target != nullptr);
                if (target) static_cast<cIGZUnknown*>(target)->Release();
                void* ser = nullptr;
                CHECK(d->QueryInterface(0xE4FDA3D4u, &ser) && ser != nullptr);
                if (ser) {
                    CHECK(static_cast<cIGZSerializable*>(ser)->GetGZCLSID() == 0xCBC14674u);
                    static_cast<cIGZUnknown*>(ser)->Release();
                }
                CHECK(d->GetRefCount() == 1);
                d->Release();
            }
            // The MySims live in the advisor system's class store: none
            // without a live FC4 advisor system, and no native fallback.
            for (uint32_t id : {0x4A1DBBBFu, 0x0BF606A5u}) {
                void* sim = nullptr;
                CHECK(!director->GetClassObject(id, 0x00000001u, &sim));
                CHECK(sim == nullptr);
            }
        }
    }
    FreeLibrary(hd);
    const auto logText = ReadFile(log);
    const bool unavailable = mode == MySimsIni::Alone || mode == MySimsIni::Mixed;
    CHECK((logText.find("subsystem 'mysims': replacement unavailable without 'advisors'") != std::string::npos) ==
          unavailable);
    CHECK((logText.find("subsystem 'mysims': enabled (3 classes replaced)") != std::string::npos) == replaced);
    CHECK(logText.find("mysims shadow: ON") == std::string::npos);
    CHECK(logText.find("mysimdispatch shadow ") == std::string::npos);
    return g_failures == 0 ? 0 : 1;
}
