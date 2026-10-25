// SPDX-License-Identifier: GPL-3.0-or-later
// network-rules.md 2.4.3: only the manager/tool host can own a rule solver.
#include "FC4LotDevInterfaces.h"
int NetRulesRegistrationTest(const std::string& dll, const std::string& ini, const std::string& log,
                             const std::string& mode) {
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_NETRULES_SHADOW", nullptr);
    const bool enabled = mode == "--ini-netrules-host" || mode == "--ini-netrules-overlap";
    std::string settings = "[subsystems]\n";
    if (mode == "--ini-netrules-host") settings += "enable = netrules\n";
    if (mode == "--ini-netrules-overlap") settings += "enable = netrules,netmgr,nettool\n";
    if (mode == "--ini-netrules-disabled") settings += "enable = netrules\ndisable = netrules\n";
    if (mode == "--ini-netrules-veto-manager") settings += "enable = netrules\ndisable = netmgr\n";
    if (mode == "--ini-netrules-veto-tool") settings += "enable = netrules\ndisable = nettool\n";
    WriteTextFile(ini, settings.c_str());
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (!hd) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    SetGamePrecision();
    if (director) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        for (uint32_t clsid : {0xC990BD46u, 0x4E543030u}) {
            CHECK(std::count(classes.begin(), classes.end(), clsid) == (enabled ? 1 : 0));
        }
        void* obj = nullptr;
        CHECK(director->GetClassObject(0xC990BD46, 0xA9764C8C, &obj) == enabled);
        if (enabled && obj) {
            auto* manager = static_cast<cFC4ISC4NetworkManager*>(obj);
            CHECK(manager->Init());
            // The cache owns one reference; these two lookups borrow it.
            auto* tool = static_cast<cIGZUnknown*>(manager->GetNetworkTool(0, false));
            CHECK(tool != nullptr);
            CHECK(manager->GetNetworkTool(0, false) == tool);
            CHECK(manager->GetNetworkTool(-1, true) == nullptr);
            CHECK(manager->GetNetworkTool(13, true) == nullptr);
            auto* privateTool = static_cast<cIGZUnknown*>(manager->GetNetworkTool(0, true));
            CHECK(privateTool != nullptr && privateTool != tool);
            if (privateTool) {
                CHECK(privateTool->AddRef() == 1); // native factory returns count zero
                CHECK(privateTool->Release() == 0);
            }
            auto* created = static_cast<cIGZUnknown*>(manager->GetNetworkTool(0, true));
            CHECK(created != nullptr);
            if (created) CHECK(created->Release() == 0); // created-pointer disposal, no retained reference
            if (tool) {
                tool->AddRef(); // our reference must survive the manager/cache
                CHECK(manager->Release() == 0);
                CHECK(tool->Release() == 0);
            } else {
                manager->Release();
            }
        } else {
            CHECK(obj == nullptr);
        }
    }
    FreeLibrary(hd);
    const std::string text = ReadFile(log);
    CHECK((text.find("netrules: enabled through netmgr/nettool") != std::string::npos) == enabled);
    CHECK((text.find("nettool: registry active") != std::string::npos) == enabled);
    CHECK(text.find("netrules shadow: ON") == std::string::npos); // replacement never enables observation
    CHECK(text.find("created (stand-in)") == std::string::npos);
    CHECK(text.find("core creation failed") == std::string::npos);
    CHECK(text.find("WARNING: released last reference") == std::string::npos);
    CHECK(text.find("WARNING: over-release") == std::string::npos);
    return g_failures == 0 ? 0 : 1;
}
