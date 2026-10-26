// SPDX-License-Identifier: GPL-3.0-or-later
int EffectsResourceRegistrationTest(const std::string& dll, const std::string& ini,
                                    const std::string& log, const std::string& mode) {
    const bool enabled = mode == "--effectsres-enabled" || mode == "--effectsres-mixed";
    const bool mixed = mode == "--effectsres-mixed";
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    WriteTextFile(ini, mixed ? "[subsystems]\nenable = effects effectsres\n" :
        enabled ? "[subsystems]\nenable = effectsres\n" :
        mode == "--effectsres-disabled" ? "[subsystems]\nenable = effectsres\ndisable = effectsres\n" :
        "[subsystems]\n");
    HMODULE module = LoadLibraryA(dll.c_str());
    CHECK(module != nullptr);
    if (!module) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(module, "GZDllGetGZCOMDirector"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    if (director) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(std::count(classes.begin(), classes.end(), 0x2A5118ADu) == (enabled ? 1 : 0));
        CHECK(std::count(classes.begin(), classes.end(), 0x49822F75u) == (mixed ? 1 : 0));
        // This host has no game image or native collection service. Creation
        // must fail closed even when enumeration advertises the opt-in class.
        void* object = nullptr;
        CHECK(!director->GetClassObject(0x2A5118AD, 0xCA51189B, &object));
        CHECK(object == nullptr);
    }
    FreeLibrary(module);
    const auto text = ReadFile(log);
    CHECK(text.find("config: WARNING unknown key") == std::string::npos);
    CHECK(text.find("WARNING: over-release") == std::string::npos);
    CHECK(text.find("WARNING: released last reference") == std::string::npos);
    CHECK(text.find("structured exception") == std::string::npos);
    return g_failures == 0 ? 0 : 1;
}
