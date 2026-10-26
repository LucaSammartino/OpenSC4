// SPDX-License-Identifier: GPL-3.0-or-later
#include "../plugin/cFC4ConfigRules.h"
int ConfigRulesRegistrationTest(const std::string& dll, const std::string& ini, int mode) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, mode == 0 ? "[subsystems]\n" : mode == 1
        ? "[subsystems]\nenable = cfgrules\n" : "[subsystems]\nenable = cfgrules\ndisable = cfgrules\n");
    HMODULE h = LoadLibraryA(dll.c_str());
    CHECK(h != nullptr);
    if (!h) return 1;
    auto get = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(h, "GZDllGetGZCOMDirector"));
    CHECK(get != nullptr);
    if (get) {
        auto* director = get();
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(std::count(classes.begin(), classes.end(), 0x0A1D8285u) == (mode == 1 ? 1 : 0));
        cFC4ISC4ConfigRules* object = nullptr;
        CHECK(director->GetClassObject(0x0A1D8285, 0x0A1D8284, reinterpret_cast<void**>(&object)) == (mode == 1));
        if (object) {
            void* unknown = nullptr;
            CHECK(object->QueryInterface(1, &unknown) && unknown == object);
            if (unknown) static_cast<cIGZUnknown*>(unknown)->Release();
            void* rejected = reinterpret_cast<void*>(1);
            CHECK(!object->QueryInterface(0xDEADBEEF, &rejected) && rejected == nullptr);
            object->Init(nullptr); // unavailable acquisition refuses initialization
            CHECK(object->GetOptionGroupAutoSetting(0) == 0);
            object->ApplyOptionGroupSetting(UINT32_MAX, UINT32_MAX);
            CHECK(object->Release() == 0);
        }
    }
    FreeLibrary(h);
    return g_failures == 0 ? 0 : 1;
}
