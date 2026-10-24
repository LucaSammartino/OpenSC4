// SPDX-License-Identifier: GPL-3.0-or-later
// The pending owner group must not intercept native direct dispatch calls.
int MySimsFactoryBoundaryTest(const std::string& dll, const std::string& ini,
                             const std::string& log, bool requested) {
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_MYSIMS_SHADOW", nullptr);
    SetEnvironmentVariableA("FC4_MYSIMDISPATCH_SHADOW", nullptr);
    WriteTextFile(ini, requested ? "[subsystems]\nenable = mysims\n" : "[subsystems]\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (!hd) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    if (director) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        const uint32_t ids[] = {0x4A1DBBBFu, 0x0BF606A5u, 0xCBC14674u};
        const uint32_t iids[] = {0x0A1DA42Fu, 0x0A1DA42Fu, 0xEB8B6F9Au};
        for (size_t i = 0; i < 3; ++i) {
            CHECK(std::count(classes.begin(), classes.end(), ids[i]) == 0);
            void* out = nullptr;
            CHECK(!director->GetClassObject(ids[i], iids[i], &out));
            CHECK(out == nullptr);
        }
    }
    FreeLibrary(hd);
    const auto text = ReadFile(log);
    CHECK((text.find("subsystem 'mysims': replacement unavailable; native factories retained") !=
           std::string::npos) == requested);
    CHECK(text.find("subsystem 'mysims': enabled") == std::string::npos);
    CHECK(text.find("mysims shadow: ON") == std::string::npos);
    CHECK(text.find("mysimdispatch shadow ") == std::string::npos);
    return g_failures == 0 ? 0 : 1;
}
