// SPDX-License-Identifier: GPL-3.0-or-later
#include "cFC4DisasterLayer.h"
#include "cISC4DisasterManager.h"
#include "cISC4DisasterManagerFire.h"
#include "cISC4DisasterInstance.h"
#include "cISC4DisasterInstanceFire.h"
#include "FC4DisasterFire.h"

struct DisasterFireBaseFixture {
    void** vt;
    int inits = 0, shutdowns = 0;
};
bool __fastcall DisasterFireBaseInit(DisasterFireBaseFixture* f, void*) { ++f->inits; return true; }
bool __fastcall DisasterFireBaseShutdown(DisasterFireBaseFixture* f, void*) { ++f->shutdowns; return true; }
uint32_t __fastcall DisasterFireBaseFlags(DisasterFireBaseFixture*, void*) { return 0x1234; }
bool __fastcall DisasterFireBaseGroup(DisasterFireBaseFixture*, void*, uint32_t group) { return group == 0x5005; }

// A native fire shell with deliberately unusable SetTarget/SetStage: these
// methods require native instance-owned static curves (disasters.md 5.3).
struct DisasterFireStorageFixture final : cISC4FireOccupant {
    cISC4Occupant* base;
    int targetCalls = 0, stageCalls = 0;
    explicit DisasterFireStorageFixture(cISC4Occupant* p) : base(p) {}
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 0; }
    cISC4Occupant* AsOccupant() override { return base; }
    bool SetTarget(cISC4Occupant*) override { ++targetCalls; return false; }
    cISC4Occupant* GetTarget() override { return nullptr; }
    uint32_t GetStage() override { return 0; }
    uint32_t GetMaxStage() override { return 0; }
    bool SetStage(uint32_t) override { ++stageCalls; return false; }
    uint32_t GetCounter() override { return 0; }
    bool SetCounter(uint32_t) override { return false; }
    bool GetHosed() override { return false; }
    bool SetHosed(int32_t) override { return false; }
    uint32_t GetFamily() override { return 0; }
    bool SetFamily(uint32_t) override { return false; }
};

void DisasterFireAdapterTest() {
    void* vt[36]{};
    vt[3] = reinterpret_cast<void*>(&DisasterFireBaseInit);
    vt[4] = reinterpret_cast<void*>(&DisasterFireBaseShutdown);
    vt[23] = reinterpret_cast<void*>(&DisasterFireBaseGroup);
    vt[33] = reinterpret_cast<void*>(&DisasterFireBaseFlags);
    DisasterFireBaseFixture native{vt}, target1{vt}, target2{vt};
    DisasterFireStorageFixture storage(reinterpret_cast<cISC4Occupant*>(&native));
    int retains = 0, drops = 0, storageDrops = 0, hoseDelta = 0;
    auto* fire = new fc4::DisasterFire(&storage, [&](cISC4Occupant*, bool held) {
        if (held) ++retains; else ++drops;
    }, [&] { ++storageDrops; });
    std::vector<uint32_t> messages;
    fire->post = [&](cISC4Occupant* occupant, uint32_t type, uint32_t data) {
        CHECK(occupant == fire->AsOccupant() && data == 0);
        messages.push_back(type);
    };
    CHECK(fire->Init() && fire->Init() && fire->IsInitialized());
    CHECK(messages == std::vector<uint32_t>{0x99EF1140});
    auto* first = reinterpret_cast<cISC4Occupant*>(&target1);
    auto* second = reinterpret_cast<cISC4Occupant*>(&target2);
    CHECK(fire->SetTarget(first) && fire->GetTarget() == first);
    CHECK(fire->SetTarget(first) && retains == 1 && drops == 0);
    CHECK(fire->SetTarget(second) && retains == 2 && drops == 1);
    CHECK(fire->GetType() == 0x69D25C5F);
    CHECK(fire->AsOccupant()->GetFlags() == 0x1234);
    CHECK(fire->SetStage(1) && fire->GetStage() == 1);
    fire->Sync(2, 3, 55, 4);
    CHECK(fire->GetStage() == 2 && fire->GetMaxStage() == 3 && fire->GetCounter() == 55);
    CHECK(fire->GetHosed());
    fire->hose = [&](int32_t delta) { hoseDelta += delta; return true; };
    CHECK(fire->SetHosed(-3) && hoseDelta == -3);
    CHECK(fire->SetFamily(7) && fire->GetFamily() == 7);
    void* occupant = nullptr;
    CHECK(fire->QueryInterface(GZIID_cISC4Occupant, &occupant));
    CHECK(occupant == fire->AsOccupant());
    void* identity = nullptr;
    CHECK(static_cast<cISC4Occupant*>(occupant)->QueryInterface(1, &identity));
    CHECK(identity == static_cast<cISC4FireOccupant*>(fire));
    static_cast<cISC4FireOccupant*>(identity)->Release();
    static_cast<cISC4Occupant*>(occupant)->Release();
    CHECK(fire->Shutdown() && fire->Shutdown() && !fire->IsInitialized());
    CHECK(messages == (std::vector<uint32_t>{0x99EF1140, 0x99EF1141}));
    CHECK(fire->Release() == 0);
    CHECK(retains == 2 && drops == 2 && storageDrops == 1 && native.inits == 0 && native.shutdowns == 0);
    CHECK(storage.targetCalls == 0 && storage.stageCalls == 0);
}

int DisastersFactoryTest(const std::string& dll, const std::string& ini, bool enabled) {
    if (enabled) DisasterFireAdapterTest();
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, enabled ? "[subsystems]\nenable = disasters\n" : "[subsystems]\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (!hd) return 1;
    auto get = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto* director = get ? get() : nullptr;
    CHECK(director != nullptr);
    SetGamePrecision();
    if (director) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK((std::find(classes.begin(), classes.end(), 0x61F6ABF5) != classes.end()) == enabled);
        const uint32_t managerClasses[] = {0xA9B5AB9F, 0x69C1718E, 0x0A086BD3, 0x4A132FDF, 0x4A38206C, 0x6A7B3E9B, 0xEBFB0402};
        const uint32_t instanceClasses[] = {0x29B5AB89, 0x69C1718F, 0xAA086D86, 0x0A133012, 0x4A38206D, 0x6A7B3E9C, 0xEBFB0403};
        const uint32_t types[] = {0xC4C90997, 0x621CDA33, 0x42963812, 0x4A1332E8, 0xCA380B06, 0x6A7B4397, 0xEBFB0404};
        for (size_t n = 0; n < 7; ++n) {
            CHECK((std::find(classes.begin(), classes.end(), managerClasses[n]) != classes.end()) == enabled);
            CHECK((std::find(classes.begin(), classes.end(), instanceClasses[n]) != classes.end()) == enabled);
            void* manager = nullptr;
            CHECK(director->GetClassObject(managerClasses[n], 0x021E9CBF, &manager) == enabled);
            if (manager) {
                auto* m = static_cast<cFC4ISC4DisasterManager*>(manager);
                CHECK(m->GetID() == types[n]);
                CHECK(m->Init());
                CHECK(m->GetActiveInstanceCount() == 0);
                // Independent of the compiler's overloaded-virtual ordering.
                using StartAny = cISC4DisasterInstance* (__thiscall*)(void*);
                auto** slots = *reinterpret_cast<void***>(m);
                CHECK(reinterpret_cast<StartAny>(slots[7])(m) == nullptr);
                CHECK(m->Shutdown());
                CHECK(m->Release() == 0);
            }
            void* instance = nullptr;
            CHECK(director->GetClassObject(instanceClasses[n], 1, &instance) == enabled);
            if (instance) {
                auto* i = static_cast<cISC4DisasterInstance*>(instance);
                CHECK(i->GetID() == types[n]);
                CHECK(i->GetPhase() == 0);
                int32_t x = 7, z = 8;
                CHECK(!i->GetLocation(x, z));
                CHECK(x == 7 && z == 8);
                CHECK(i->Shutdown());
                CHECK(i->Release() == 0);
            }
        }
        void* obj = nullptr;
        CHECK(director->GetClassObject(0x61F6ABF5, 0x29B02A2C, &obj) == enabled);
        if (obj) {
            auto* layer = static_cast<cFC4ISC4DisasterLayer*>(obj);
            CHECK(layer->Init());
            CHECK(layer->DisastersEnabled());
            CHECK(!layer->EnableDisasters(false));
            CHECK(!layer->EnableDisasters(false));
            CHECK(!layer->EnableDisasters(true));
            CHECK(layer->EnableDisasters(true));
            auto* manager = layer->GetDisasterManager(0x621CDA33); // borrowed
            CHECK(manager != nullptr);
            void* fireInterface = nullptr;
            CHECK(manager && manager->QueryInterface(0x49B43F57, &fireInterface));
            if (fireInterface) {
                auto* fire = static_cast<cISC4DisasterManagerFire*>(fireInterface);
                CHECK(fire->AsISC4DisasterManager() == manager);
                auto* instance = fire->GetFireInstance(); // borrowed, stable identity
                CHECK(instance != nullptr && instance == fire->GetFireInstance());
                CHECK(instance && instance->GetFireCount() == 0);
                CHECK(fire->GetCause() == 0);
                CHECK(fire->SetCause(2) == 2);
                CHECK(fire->SetCause(3) == 2);
                fire->Release();
            }
            // Slot 6 has two coordinates; empty test city contains no pipe.
            auto* pipe = reinterpret_cast<cFC4ISC4DisasterManager*>(layer->GetDisasterManager(0x4A1332E8));
            if (pipe) {
                using StartAt = cISC4DisasterInstance* (__thiscall*)(void*, int32_t, int32_t);
                auto** slots = *reinterpret_cast<void***>(pipe);
                CHECK(reinterpret_cast<StartAt>(slots[6])(pipe, 5, 6) == nullptr);
            } else CHECK(false);
            CHECK(!layer->IsADisasterOngoing());
            CHECK(layer->RegisterDisasterLocation(nullptr, 12, 34, nullptr, 3, false));
            CHECK(layer->GetDisasterLocationCount() == 1);
            cISC4DisasterInstance* instance = reinterpret_cast<cISC4DisasterInstance*>(1);
            int32_t x = 0, z = 0;
            CHECK(layer->GetDisasterLocationByIndex(0, &instance, x, z));
            CHECK(instance == nullptr && x == 12 && z == 34);
            CHECK(!layer->GetDisasterLocationByIndex(-1, &instance, x, z));
            CHECK(layer->DisasterHasBegun(nullptr));
            CHECK(layer->IsADisasterOngoing());
            CHECK(layer->DisasterHasEnded());
            CHECK(!layer->IsADisasterOngoing());
            const void* cost = layer->GetDisasterCost();
            CHECK(cost != nullptr && *static_cast<const uint32_t*>(cost) == 0);
            CHECK(layer->UnregisterDisasterLocations(nullptr));
            CHECK(layer->GetDisasterLocationCount() == 0);
            CHECK(layer->Shutdown());
            CHECK(layer->GetDisasterCost() == cost); // stable record
            CHECK(layer->Release() == 0);
        }
    }
    FreeLibrary(hd);
    return g_failures == 0 ? 0 : 1;
}
