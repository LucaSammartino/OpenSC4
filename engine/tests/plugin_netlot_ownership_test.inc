// SPDX-License-Identifier: GPL-3.0-or-later
#include "cISC4NetworkLot.h"
#include "cISC4NetworkLotConfiguration.h"
#include "SC4Vector.h"

int NetLotOwnershipTest(const std::string& dll, const std::string& ini, int mode = 0) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, mode == 1 ? "[subsystems]\n" : mode == 2
        ? "[subsystems]\nenable = netmgr,netlot,netlotobj\ndisable = netlot,netlotobj\n"
        : mode == 3 ? "[subsystems]\nenable = netlotobj\n"
        : "[subsystems]\nenable = netmgr,netlot,netlotobj\n");
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
        for (uint32_t clsid : {0xE9A6CD3Bu, 0x4E4C3030u, 0x4E4C4330u}) {
            const bool registered = mode == 0 || (mode == 3 && clsid != 0xE9A6CD3Bu);
            CHECK(std::count(classes.begin(), classes.end(), clsid) == (registered ? 1 : 0));
            cIGZUnknown* object = nullptr;
            CHECK(director->GetClassObject(clsid, GZIID_cIGZUnknown,
                reinterpret_cast<void**>(&object)) == registered);
            CHECK((object != nullptr) == registered);
            if (object) {
                CHECK(object->AddRef() == 2);
                CHECK(object->Release() == 1);
                if (clsid == 0x4E4C3030u) {
                    auto* lot = static_cast<cISC4NetworkLot*>(object);
                    CHECK(lot->GetNetworkOccupant() == nullptr);
                    CHECK(lot->GetLotConfigurationObjectCount() == 0);
                    CHECK(lot->GetLotConfigurationObjectArray() != nullptr);
                    CHECK(lot->GetLotConfigurationObjectArray()->size() == 0);
                    void* same = nullptr;
                    CHECK(lot->QueryInterface(0x89A5BA66, &same));
                    CHECK(same == lot);
                    if (same) static_cast<cIGZUnknown*>(same)->Release();
                } else if (clsid == 0x4E4C4330u) {
                    auto* config = static_cast<cISC4NetworkLotConfiguration*>(object);
                    CHECK(config->GetObjectArray() != nullptr);
                    CHECK(config->GetObjectArray()->size() == 0);
                    CHECK(config->GetID() == 0);
                    CHECK(config->GetNetworkPieceId() == 0);
                    cRZBaseString name("sentinel");
                    config->GetName(name);
                    CHECK(name.Strlen() == 0);
                }
                CHECK(object->Release() == 0);
                void* rejected = reinterpret_cast<void*>(1);
                CHECK(!director->GetClassObject(clsid, 0xDEADBEEF, &rejected));
                CHECK(rejected == nullptr);
            }
        }
    }
    if (seam && mode == 0) {
        CHECK(seam(kFC4SeamNetworkCrewOwnership, nullptr, nullptr) == 1);
        ZonesFakeUnknown first, second;
        CHECK(seam(kFC4SeamNetworkLotOwnership, &first, &second) == 1);
        CHECK(first.refs == 1);
        CHECK(second.refs == 1);
    }
    FreeLibrary(hd);
    return g_failures == 0 ? 0 : 1;
}
