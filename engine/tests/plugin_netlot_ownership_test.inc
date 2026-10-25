// SPDX-License-Identifier: GPL-3.0-or-later
int NetLotOwnershipTest(const std::string& dll, const std::string& ini) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, "[subsystems]\nenable = netmgr,netlot\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(hd, "FC4TestSeam"));
    CHECK(getDirector != nullptr);
    CHECK(seam != nullptr);
    auto* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    if (director) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        for (uint32_t clsid : {0xC990BD46u, 0xE9A6CD3Bu}) {
            CHECK(std::count(classes.begin(), classes.end(), clsid) == 1);
            cIGZUnknown* object = nullptr;
            CHECK(director->GetClassObject(clsid, GZIID_cIGZUnknown, reinterpret_cast<void**>(&object)));
            CHECK(object != nullptr);
            if (object) object->Release();
        }
    }
    if (seam) {
        ZonesFakeUnknown first, second;
        CHECK(seam(kFC4SeamNetworkLotOwnership, &first, &second) == 1);
        CHECK(first.refs == 1);
        CHECK(second.refs == 1);
    }
    FreeLibrary(hd);
    return g_failures == 0 ? 0 : 1;
}
