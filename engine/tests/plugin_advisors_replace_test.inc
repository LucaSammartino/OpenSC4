// SPDX-License-Identifier: GPL-3.0-or-later
// advisors.md §1.1: the opt-in system factory and its public interfaces.
int AdvisorsRegistrationTest(const std::string& dll, const std::string& ini, const std::string& log,
                             bool enabled) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, enabled ? "[subsystems]\nenable = advisors\n" : "[subsystems]\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    SetGamePrecision();
    if (director != nullptr) {
        constexpr uint32_t clsid = 0x4A0DD616;
        constexpr uint32_t iid = 0x4A0DD617;
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(std::count(classes.begin(), classes.end(), clsid) == (enabled ? 1 : 0));
        const uint32_t children[] = {
            0xEAC937B4, 0xCA09F543, 0x2A5F877D, 0x4BE372CD, 0xAA5292D7,
            0x6A5F8755, 0xEA19E825, 0x4A3AD3E1, 0xCA2C2A2F,
            0x8A09F5F4, 0xAA371C32, 0x6A3848DB, 0x6A26D049,
        };
        for (uint32_t child : children)
            CHECK(std::count(classes.begin(), classes.end(), child) == (enabled ? 1 : 0));
        void* obj = nullptr;
        CHECK(director->GetClassObject(clsid, iid, &obj) == enabled);
        if (enabled) {
            CHECK(obj != nullptr);
            if (obj != nullptr) {
                auto* system = static_cast<cIGZUnknown*>(obj);
                for (uint32_t query : {1u, iid, 0x452294AAu, 0xE98B2F57u, 0xE4FDA3D4u}) {
                    void* iface = nullptr;
                    CHECK(system->QueryInterface(query, &iface));
                    CHECK(iface != nullptr);
                    if (iface != nullptr) static_cast<cIGZUnknown*>(iface)->Release();
                }
                for (uint32_t child : children) {
                    void* saved = nullptr;
                    CHECK(director->GetClassObject(child, 0xE4FDA3D4u, &saved));
                    CHECK(saved != nullptr);
                    if (saved) static_cast<cIGZUnknown*>(saved)->Release();
                }
                CHECK(system->Release() == 0);
            }
        } else {
            CHECK(obj == nullptr);
        }
    }
    FreeLibrary(hd);
    const std::string text = ReadFile(log);
    CHECK(text.find(enabled ? "subsystem 'advisors': enabled (14 classes replaced)" :
                              "subsystem 'advisors': off by default") != std::string::npos);
    CHECK(text.find("WARNING: released last reference") == std::string::npos);
    CHECK(text.find("WARNING: over-release") == std::string::npos);
    return g_failures == 0 ? 0 : 1;
}
