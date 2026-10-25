// SPDX-License-Identifier: GPL-3.0-or-later
// Keep the native class until the live methods have a complete ABI contract.
int LotrecBoundaryTest(const std::string& dll, const std::string& ini, const std::string& log, bool requested) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, requested ? "[subsystems]\nenable = lotrec\nlotrecview_shadow = 1\n" : "[subsystems]\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (!hd) return 1;
    auto get = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto* director = get ? get() : nullptr;
    CHECK(director != nullptr);
    if (director) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(std::count(classes.begin(), classes.end(), 0xC9BD5D4Au) == 0);
        void* object = nullptr;
        CHECK(!director->GetClassObject(0xC9BD5D4A, 0xE818FCED, &object));
        CHECK(object == nullptr);
    }
    FreeLibrary(hd);
    const auto text = ReadFile(log);
    CHECK((text.find("subsystem 'lotrec': replacement unavailable") != std::string::npos) == requested);
    CHECK(text.find("config: WARNING unknown key") == std::string::npos);
    CHECK(text.find("WARNING: released last reference") == std::string::npos);
    CHECK(text.find("WARNING: over-release") == std::string::npos);
    return g_failures == 0 ? 0 : 1;
}
