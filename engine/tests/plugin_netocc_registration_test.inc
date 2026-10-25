// SPDX-License-Identifier: GPL-3.0-or-later
// Observing records must not take over any occupant class factory.
int NetOccShadowRegistrationTest(const std::string& dll, const std::string& ini) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, "[subsystems]\nnetocc_shadow = 1\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    if (director != nullptr) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        for (uint32_t clsid : {0xC9C05C6Eu, 0x49C1A034u, 0x49CC1BCDu, 0x8A4BD52Bu,
                              0x4A10AFC2u, 0x49C05B9Fu, 0xCA16374Fu, 0x8A139184u,
                              0xEA0A5ACFu, 0xCA15ED6Bu, 0x09C05C6Au, 0xC9C05C5Du, 0x49B6D69Bu}) {
            CHECK(std::count(classes.begin(), classes.end(), clsid) == 0);
            void* object = nullptr;
            CHECK(!director->GetClassObject(clsid, GZIID_cIGZUnknown, &object));
            CHECK(object == nullptr);
        }
    }
    FreeLibrary(hd);
    return g_failures == 0 ? 0 : 1;
}
