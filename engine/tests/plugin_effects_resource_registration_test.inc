// SPDX-License-Identifier: GPL-3.0-or-later
int EffectsResourceRegistrationTest(const std::string& dll, const std::string& ini,
                                    const std::string& log, const std::string& mode) {
    const bool enabled = mode == "--effectsres-enabled" || mode == "--effectsres-mixed";
    const bool mixed = mode == "--effectsres-mixed";
    const bool disabled = mode == "--effectsres-disabled";
    const bool house = mixed || disabled;
    // Exercise the registration interaction with the complete train288 house
    // list, including effects, regionalcity and gzfw. Enumeration must not
    // instantiate the resource or duplicate another owner's class.
    const std::string houseIni = "[subsystems]\nenable = landvalue, weather, ordinance, demand, power, "
        "plumbing, flammability, pollution, protection, crime, aura, flora, landfill, residential, neighbors, "
        "zones, development, tract, history, clock24, commercial, industrial, civic, parks, lotdev, zonedev, "
        "traffic, automata, netmgr, netlot, nettool, demolition, occmgr, propmgr, lotcfg, citysit, propdev, "
        "simgrids, citydetail, ports, dispatch, exemplars, span1555, span4444, span565, span8, span8888, "
        "tutorial, advisors, terrain, disasters, effects, netrules, mysims, regionalcity, gzfw, effectsres\n";
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    WriteTextFile(ini, house ? houseIni + (disabled ? "disable = effectsres\n" : "") :
        enabled ? "[subsystems]\nenable = effectsres\n" :
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
        CHECK(std::count(classes.begin(), classes.end(), 0x49822F75u) == (house ? 1 : 0));
        if (house) {
            CHECK(std::count(classes.begin(), classes.end(), 0x6A037C24u) == 1); // regionalcity
            CHECK(std::count(classes.begin(), classes.end(), 0xC470D325u) == 1); // gzfw buffer
            auto sorted = classes;
            std::sort(sorted.begin(), sorted.end());
            CHECK(std::adjacent_find(sorted.begin(), sorted.end()) == sorted.end());
        }
        CHECK(ReadFile(log).find("factory enter: cSC4EffectsResource") == std::string::npos);
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
