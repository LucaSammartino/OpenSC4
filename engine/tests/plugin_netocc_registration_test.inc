// SPDX-License-Identifier: GPL-3.0-or-later
// Observing records must not take over any occupant class factory.
#include "cISC4Occupant.h"

class NetOccLineTestInterface : public cIGZUnknown {
public:
    virtual bool GetConnectedPoles(cIGZUnknown*&, cIGZUnknown*&, bool&) = 0;
    virtual bool SetConnectedPoles(cIGZUnknown*, cIGZUnknown*, bool) = 0;
    virtual bool GetCell(uint32_t&, uint32_t&) = 0;
    virtual bool IsPreview() = 0;
    virtual bool SetPreview(bool) = 0;
};
class NetOccTestPole final : public cIGZUnknown {
public:
    uint32_t refs = 1;
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out || (iid != 1 && iid != 0xA8C86142)) return false;
        *out = this; AddRef(); return true;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }
};

int NetOccLineRegistrationTest(const std::string& dll, const std::string& ini) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, "[subsystems]\nenable = netocc\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (!hd) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    auto* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    if (director) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(std::count(classes.begin(), classes.end(), 0xC9C05C5Du) == 1);
        // This partial replacement must leave the other occupant factories native,
        // even when netocc itself is enabled.
        for (uint32_t clsid : {0xC9C05C6Eu, 0x49C1A034u, 0x49CC1BCDu, 0x8A4BD52Bu,
                              0x4A10AFC2u, 0x49C05B9Fu, 0xCA16374Fu, 0x8A139184u,
                              0xEA0A5ACFu, 0xCA15ED6Bu, 0x09C05C6Au, 0x49B6D69Bu}) {
            CHECK(std::count(classes.begin(), classes.end(), clsid) == 0);
            void* native = nullptr;
            CHECK(!director->GetClassObject(clsid, GZIID_cIGZUnknown, &native));
            CHECK(native == nullptr);
        }
        void* raw = nullptr;
        CHECK(director->GetClassObject(0xC9C05C5D, 0xC98F3B24, &raw));
        if (raw) {
            auto* line = static_cast<NetOccLineTestInterface*>(raw);
            CHECK(!line->IsPreview());
            CHECK(line->SetPreview(true)); CHECK(line->IsPreview());
            NetOccTestPole pole;
            CHECK(line->SetConnectedPoles(&pole, &pole, true)); CHECK(pole.refs == 3);
            // Factories create independent occupants, and final Release must
            // return held pole references even without an explicit Shutdown.
            void* secondRaw = nullptr;
            CHECK(director->GetClassObject(0xC9C05C5D, 0xC98F3B24, &secondRaw));
            if (secondRaw) {
                auto* second = static_cast<NetOccLineTestInterface*>(secondRaw);
                CHECK(second != line && !second->IsPreview());
                cIGZUnknown* secondA = nullptr; cIGZUnknown* secondB = nullptr;
                bool secondFlag = true;
                CHECK(second->GetConnectedPoles(secondA, secondB, secondFlag));
                CHECK(secondA == nullptr && secondB == nullptr && !secondFlag);
                if (secondA) secondA->Release();
                if (secondB) secondB->Release();
                CHECK(second->SetConnectedPoles(&pole, nullptr, false));
                CHECK(pole.refs == 4);
                second->Release();
                CHECK(pole.refs == 3);
            }
            // Aliasing the old references must acquire before releasing them.
            CHECK(line->SetConnectedPoles(&pole, &pole, true)); CHECK(pole.refs == 3);
            cIGZUnknown* a = nullptr; cIGZUnknown* b = nullptr; bool flag = false;
            CHECK(line->GetConnectedPoles(a, b, flag));
            CHECK(a == &pole && b == &pole && flag && pole.refs == 5);
            a->Release(); b->Release();
            cISC4Occupant* occ = nullptr;
            CHECK(line->QueryInterface(0x26BB767E, reinterpret_cast<void**>(&occ)));
            if (occ) {
                CHECK(occ->GetType() == 0x0990B2EA);
                cS3DVector3 p(-17.0f, 12.0f, -15.0f), mn, mx;
                CHECK(occ->SetPosition(&p));
                uint32_t x = 0, z = 0; CHECK(line->GetCell(x, z));
                CHECK(x == UINT32_MAX && z == 0);
                occ->GetBoundingBox(&mn, &mx);
                CHECK(mn.fX == p.fX && mn.fY == p.fY && mn.fZ == p.fZ);
                CHECK(mx.fX == mn.fX && mx.fY == mn.fY && mx.fZ == mn.fZ);
                uint8_t box[8]{}; CHECK(!occ->GetOccupantManagerBBox(box));
                uint8_t setBox[8] = {1, 2, 3, 4}; CHECK(occ->SetOccupantManagerBBox(setBox));
                CHECK(occ->GetOccupantManagerBBox(box)); CHECK(std::memcmp(box, setBox, 4) == 0);
                occ->SetRemovalFlags(3); CHECK(!occ->CanRemove(1) && !occ->CanRemove(2));
                occ->UnsetRemovalFlags(1); CHECK(occ->CanRemove(1) && !occ->CanRemove(2));
                CHECK(occ->Shutdown()); CHECK(pole.refs == 1);
                occ->Release();
            }
            cIGZSerializable* serial = nullptr;
            CHECK(line->QueryInterface(0xE4FDA3D4, reinterpret_cast<void**>(&serial)));
            if (serial) { CHECK(serial->GetGZCLSID() == 0xC9C05C5D); serial->Release(); }
            line->Release(); CHECK(pole.refs == 1);
        }
    }
    FreeLibrary(hd);
    return g_failures == 0 ? 0 : 1;
}

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
