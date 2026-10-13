// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

// Loads the real FreeCity4.dll the way a GZCOM host does (LoadLibrary +
// GZDllGetGZCOMDirector), without a framework, and drives the director and
// the simulator through their vtables. Then checks that the diagnostics log
// recorded the calls.
//
// Usage: fc4_plugin_load_test <path to FreeCity4.dll>

#include "cIGZCOMDirector.h"
#include "cIGZDate.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cS3DVector2.h"
#include "cIGZSystemService.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4DepartmentBudget.h"
#include "cISC4LandValueSimulator.h"
#include "cISC4LineItem.h"
#include "cISC4Ordinance.h"
#include "cISC4OrdinanceSimulator.h"
#include "cISCPropertyHolder.h"
#include "cRZBaseVariant.h"
#include "cSCBaseProperty.h"
#include "cISC4Simulator.h"
#include "SC4Percentage.h"
#include "cRZBaseString.h"
#include "FC4BudgetShadow.h"
#include "FC4DemandShadow.h"
#include "FC4PollutionShadow.h"
#include "cIGZOStream.h"
#include "FC4Interfaces.h"
#include "FC4TestSeam.h"
#include "fc4.h"
#include "game_abi.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <map>
#include <sstream>
#include <string>
#include <utility>
#include <vector>

namespace {

int g_failures = 0;

#define CHECK(cond)                                                                       \
    do {                                                                                  \
        if (!(cond)) {                                                                    \
            std::fprintf(stderr, "%s:%d: CHECK failed: %s\n", __FILE__, __LINE__, #cond); \
            ++g_failures;                                                                 \
        }                                                                                 \
    } while (0)

constexpr uint32_t kCLSID_Simulator = 0x2990C1E5;
constexpr uint32_t kIID_cISC4Simulator = 0x8695664E;
constexpr uint32_t kDirectorID = 0x5C7A43E1;
constexpr uint32_t kCLSID_Budget = 0xE990BE01;
constexpr uint32_t kCLSID_Department = 0xE990BFFC;
constexpr uint32_t kCLSID_LineItem = 0xAA313C9F;
constexpr uint32_t kIID_cISC4BudgetSimulator = 0x6772BD23;
constexpr uint32_t kIID_cISC4DepartmentBudget = 0x223E2B67;
constexpr uint32_t kCLSID_LandValue = 0xE990C18E;
constexpr uint32_t kIID_cISC4LandValueSimulator = 0xE7781D8C;
constexpr uint32_t kCLSID_Weather = 0x2990C1BC;
constexpr uint32_t kIID_cISC4WeatherSimulator = 0xC6C66245;
constexpr uint32_t kCLSID_OrdinanceSim = 0xA990C08E;
constexpr uint32_t kIID_cISC4OrdinanceSimulator = 0x41193C3A;
constexpr uint32_t kCLSID_FreeClinics = 0x40D07236;
constexpr uint32_t kCLSID_DemandSim = 0x2990BFF2;
constexpr uint32_t kIID_cISC4DemandSimulator = 0x47967E16;
constexpr uint32_t kCLSID_Demand = 0x8990C12D;
constexpr uint32_t kIID_cISC4Demand = 0x0797A4E8;
constexpr uint32_t kCLSID_Plumbing = 0x0990C075;
constexpr uint32_t kIID_cISC4PlumbingSimulator = 0x47782CCC;
constexpr uint32_t kCLSID_Pollution = 0x8990C065;
constexpr uint32_t kIID_cISC4PollutionSimulator = 0x877181EC;
constexpr uint32_t kIID_cISC4PropRequester = 0x89C498BE;

void CollectClass(uint32_t clsid, uint32_t, void* ctx) { static_cast<std::vector<uint32_t>*>(ctx)->push_back(clsid); }

// A handled access violation in the HOST's code (not the DLL's): the DLL's
// vectored handler must log it with the call ring.
int ForeignFault() {
    __try {
        *static_cast<volatile int*>(nullptr) = 1;
    } __except (EXCEPTION_EXECUTE_HANDLER) {
        return 1;
    }
    return 0;
}

// A building's property holder with just what the budget reads and writes
// (budget-item arrays), laid out like the game's (foundations §17.2): the
// plugin calls holders only through those slots.
class FakeHolder final : public cFC4ISCPropertyHolder {
public:
    ~FakeHolder() { RemoveAllProperties(false); }
    bool QueryInterface(uint32_t riid, void** pp) override {
        if (riid != GZIID_cISCPropertyHolder && riid != GZIID_cIGZUnknown) return false;
        *pp = this;
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }  // stack object: never deleted

    bool HasProperty(uint32_t id) const override { return props.count(id) != 0; }
    bool GetPropertyList(cIGZUnknownList**) const override { return false; }
    bool GetPropertyData(uint32_t, void*, uint32_t&) const override { return false; }
    bool GetPropertyInterface(uint32_t, uint32_t, void**) const override { return false; }
    bool GetPropertyString(uint32_t, cIGZString&) const override { return false; }
    bool GetPropertyU32(uint32_t, uint32_t&) const override { return false; }
    cISCProperty* GetProperty(uint32_t id) const override {
        auto it = props.find(id);
        return it == props.end() ? nullptr : it->second;
    }
    bool AddPropertyData(uint32_t, void*, uint32_t, bool) override { return false; }
    bool AddPropertyObject(uint32_t, cIGZUnknown*, bool) override { return false; }
    bool AddPropertyString(uint32_t, cIGZString const&, bool) override { return false; }
    bool AddPropertyU32(uint32_t, uint32_t, bool) override { return false; }
    bool AddPropertyVariant(uint32_t id, cFC4IGZVariant const* fv, bool) override {
        cIGZVariant const* v = fc4::AsSdk(fv);  // a game holder copies through the SDK slots
        RemoveProperty(id, false);
        cSCBaseProperty* p = new cSCBaseProperty(id, v);  // deep copy
        p->AddRef();
        props[id] = p;
        return true;
    }
    bool AddPropertyRecord(cISCProperty*, bool) override { return false; }
    bool CopyAddProperty(cISCProperty*, bool) override { return false; }
    bool RemoveProperty(uint32_t id, bool) override {
        auto it = props.find(id);
        if (it == props.end()) return false;
        it->second->Release();
        props.erase(it);
        return true;
    }
    bool RemoveAllProperties(bool) override {
        for (auto& kv : props) kv.second->Release();
        props.clear();
        return true;
    }
    bool EnumPropertiesFiltered(FilterFn, EnumFn, void*) const override { return false; }
    bool EnumProperties(EnumFn, void*) const override { return false; }
    bool CompactProperties() override { return true; }

    cISCPropertyHolder* AsSdk() { return reinterpret_cast<cISCPropertyHolder*>(static_cast<cFC4ISCPropertyHolder*>(this)); }

    template <class T>
    void Set(uint32_t id, T value) {
        cRZBaseVariant v;
        SetArray(v, &value);
        AddPropertyVariant(id, fc4::AsGame(static_cast<cIGZVariant const*>(&v)), false);
    }
    float Get32f(uint32_t id) const {
        const cISCProperty* p = GetProperty(id);
        return p != nullptr ? p->GetPropertyValue()->RefFloat32()[0] : -1.0f;
    }

    uint32_t refs = 1;

private:
    static void SetArray(cRZBaseVariant& v, uint32_t* x) { v.RefUint32(x, 1); }
    static void SetArray(cRZBaseVariant& v, int64_t* x) { v.RefSint64(x, 1); }
    static void SetArray(cRZBaseVariant& v, float* x) { v.RefFloat32(x, 1); }
    std::map<uint32_t, cSCBaseProperty*> props;
};

// A day agent that reports "not finished" for its first 150 calls.
class SlowAgent final : public cIGZMessageTarget2 {
public:
    uint32_t refs = 0;
    uint32_t calls = 0;
    uint32_t badMessages = 0;
    int32_t lastJulian = 0;
    bool QueryInterface(uint32_t riid, void** pp) override {
        if (riid == GZIID_cIGZUnknown) {
            *pp = static_cast<cIGZUnknown*>(this);
            AddRef();
            return true;
        }
        return false;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return refs > 0 ? --refs : 0; }
    bool DoMessage(cIGZMessage2* msg) override {
        ++calls;
        // Read the message the way game agents do: cast to the standard interface.
        auto* m = static_cast<cIGZMessage2Standard*>(msg);
        if (msg->GetType() != 0x66956814 || !m->GetHasData1() || m->GetData2() != 0 || m->GetData1() < 2451545) {
            ++badMessages;
        }
        lastJulian = static_cast<int32_t>(m->GetData1());
        return calls > 150;
    }
};

// A day agent whose first call blocks for 6.5 s (the watchdog's stall threshold is 5 s).
class BlockingAgent final : public cIGZMessageTarget2 {
public:
    uint32_t refs = 0;
    uint32_t calls = 0;
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return refs > 0 ? --refs : 0; }
    bool DoMessage(cIGZMessage2*) override {
        if (calls++ == 0) Sleep(6500);
        return true;
    }
};

// A DLL plugin's ordinance (ordinance §10.1): the simulator calls it through
// the interface only.
class FakeOrdinance final : public cFC4ISC4Ordinance {
public:
    uint32_t refs = 0;
    bool available = true, on = false, enabled = true;
    cRZBaseString name{"Fake Ordinance"};
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return refs > 0 ? --refs : 0; }
    bool Init() override { return true; }
    bool Shutdown() override { return true; }
    int64_t GetCurrentMonthlyIncome() override { return -7; }
    uint32_t GetID() const override { return 0x12345678; }
    cIGZString* GetName() override { return &name; }
    cIGZString* GetDescription() override { return &name; }
    uint32_t GetYearFirstAvailable() override { return 0; }
    uint32_t GetChanceAvailability() override { return 0; }
    int64_t GetEnactmentIncome() override { return -10; }
    int64_t GetRetracmentIncome() override { return -5; }
    int64_t GetMonthlyConstantIncome() override { return -7; }
    float GetMonthlyIncomeFactor() override { return 0.0f; }
    uint32_t GetAdvisorID() override { return 0; }
    cISCPropertyHolder* GetMiscProperties() override { return nullptr; }
    bool IsAvailable() override { return available; }
    bool IsOn() override { return available && on; }
    bool IsEnabled() override { return enabled; }
    int64_t GetMonthlyAdjustedIncome() override { return -7; }
    bool CheckConditions() override { return enabled; }
    bool IsIncomeOrdinance() override { return false; }
    bool Simulate() override { return true; }
    bool SetAvailable(bool v) override { return available = v, true; }
    bool SetOn(bool v) override { return on = v, true; }
    bool SetEnabled(bool v) override { return enabled = v, true; }
    bool ForceAvailable(bool v) override { return SetAvailable(v); }
    bool ForceOn(bool v) override { return SetOn(v); }
    bool ForceMonthlyAdjustedIncome(int64_t) override { return true; }
};

// A message with only a type (the ordinance simulator reads nothing else, §5).
class TypeMessage final : public cIGZMessage2 {
public:
    explicit TypeMessage(uint32_t t) : type(t) {}
    uint32_t type;
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    bool Create(uint32_t, void**) const override { return false; }
    uint32_t GetType() const override { return type; }
    void SetType(uint32_t t) override { type = t; }
    bool operator==(cIGZMessage2 const&) const override { return false; }
    bool operator<(cIGZMessage2 const&) const override { return false; }
};

size_t CountOf(const std::string& text, const std::string& what) {
    size_t n = 0;
    for (size_t at = text.find(what); at != std::string::npos; at = text.find(what, at + 1)) ++n;
    return n;
}

std::string ReadFile(const std::string& path) {
    std::ifstream f(path, std::ios::binary);
    std::stringstream ss;
    ss << f.rdbuf();
    return ss.str();
}

void WriteTextFile(const std::string& path, const std::string& text) {
    std::ofstream f(path, std::ios::binary | std::ios::trunc);
    f << text;
}

// ---- budget §10.1: what the posted messages carry (test seam op 1) ----------------------------

struct PostedMessage {
    uint32_t type;
    void* void1;
    void* unknown;
    intptr_t d1, d2, d3;
};

void RecordPost(void* ctx, cIGZMessage2Standard* m) {
    static_cast<std::vector<PostedMessage>*>(ctx)->push_back(
        {m->GetType(), m->GetVoid1(), static_cast<void*>(m->GetIGZUnknown()), m->GetData1(), m->GetData2(),
         m->GetData3()});
}

size_t CountPosts(const std::vector<PostedMessage>& posts, uint32_t type) {
    size_t n = 0;
    for (const PostedMessage& p : posts) n += p.type == type ? 1 : 0;
    return n;
}

// ---- the x87 control word (foundations §18.2) ---------------------------------------------------
// The game runs its main thread at 24-bit x87 precision; nothing in
// FreeCity4.dll may change that. The normal run sets it like the game and
// checks it after every stage.

uint16_t RawCw() {
    uint16_t cw = 0;
    __asm fnstcw cw
    return cw;
}

uint16_t g_cwExpected = 0;
bool g_cwChanged = false;

void SetGamePrecision() {
    unsigned int unused = 0;
    _controlfp_s(&unused, _PC_24, _MCW_PC);
    g_cwExpected = RawCw();
}

void CheckCw(const char* where) {
    const uint16_t cw = RawCw();
    if (cw != g_cwExpected && !g_cwChanged) {
        g_cwChanged = true;
        std::fprintf(stderr, "x87 control word changed by FreeCity4.dll (%s): 0x%04X, expected 0x%04X\n", where, cw,
                     g_cwExpected);
        ++g_failures;
    }
}

// ---- a fake city for the observer (test seam op 2) -------------------------------------------
// Game objects are called through their vtables only, so a fake is a table of
// __fastcall functions (ECX = this, like __thiscall; callee-pop) with traps in
// every slot the observer must not call.

template <size_t N>
void __fastcall Trap(void*, void*) {
    std::fprintf(stderr, "fake game object: unexpected call of slot %u\n", static_cast<unsigned>(N));
    std::abort();
}

template <size_t... I>
std::array<void*, sizeof...(I)> MakeTraps(std::index_sequence<I...>) {
    return {{reinterpret_cast<void*>(&Trap<I>)...}};
}

struct FakeVtable {
    std::array<void*, 160> slots = MakeTraps(std::make_index_sequence<160>());
};

struct FakeObject {
    void** vtbl;
};

struct FakeDemand {
    void** vtbl;
    uint32_t id;
    float supply;
    float demand;
};

struct FakeWorld {
    void* sim = nullptr;
    void* budget = nullptr;
    int demandCalls = 0;
    int demandBadIndex = 0;
    int saveCalls = 0;
    FakeVtable cityVt, resVt, demandSimVt, demandVt, appVt;
    FakeObject city{}, res{}, demandSim{}, app{};
    FakeDemand demands[7] = {
        {nullptr, 0x1010, 1000.0f, 250.5f},  // R$
        {nullptr, 0x3110, 300.0f, -12.0f},   // Cs$
        {nullptr, 0x3320, 200.0f, 40.0f},    // Co$$
        {nullptr, 0x4100, 150.0f, 7.5f},     // I-R
        {nullptr, 0x4400, 50.0f, 0.0f},      // I-HT
        {nullptr, 0x3001, 480.0f, 0.0f},     // C census (P1: the digest reads its supply)
        {nullptr, 0x4001, 160.0f, 0.0f},     // I census
    };
};

FakeWorld* g_world = nullptr;

void* __fastcall CityGetSimulator(void*, void*) { return g_world->sim; }                   // cISC4City 50
void* __fastcall CityGetBudgetSimulator(void*, void*) { return g_world->budget; }          // cISC4City 52
void* __fastcall CityGetDemandSimulator(void*, void*) { return &g_world->demandSim; }      // cISC4City 56
void* __fastcall CityGetResidentialSimulator(void*, void*) { return &g_world->res; }       // cISC4City 68
int32_t __fastcall ResGetPopulation(void*, void*) { return 1234; }                          // residential 16
void* __fastcall DemandSimGetDemand(void*, void*, uint32_t id, uint32_t index) {           // demand sim 6
    ++g_world->demandCalls;
    if (index != 0) ++g_world->demandBadIndex;
    for (FakeDemand& d : g_world->demands) {
        if (d.id == id) return &d;
    }
    return nullptr;
}
// cISC4App 16 = SaveCity(bool) (pop 4); true only for a full save, so the
// log line "returned 1" shows the argument arrived.
bool __fastcall AppSaveCity(void*, void*, bool fast) {
    ++g_world->saveCalls;
    return !fast;
}
float __fastcall DemandQuerySupply(void* self, void*) { return static_cast<FakeDemand*>(self)->supply; }  // 8
float __fastcall DemandQueryDemand(void* self, void*) { return static_cast<FakeDemand*>(self)->demand; }  // 9

void InitFakeWorld(FakeWorld& w) {
    g_world = &w;
    w.cityVt.slots[50] = reinterpret_cast<void*>(&CityGetSimulator);
    w.cityVt.slots[52] = reinterpret_cast<void*>(&CityGetBudgetSimulator);
    w.cityVt.slots[56] = reinterpret_cast<void*>(&CityGetDemandSimulator);
    w.cityVt.slots[68] = reinterpret_cast<void*>(&CityGetResidentialSimulator);
    w.resVt.slots[16] = reinterpret_cast<void*>(&ResGetPopulation);
    w.demandSimVt.slots[6] = reinterpret_cast<void*>(&DemandSimGetDemand);
    w.demandVt.slots[8] = reinterpret_cast<void*>(&DemandQuerySupply);
    w.demandVt.slots[9] = reinterpret_cast<void*>(&DemandQueryDemand);
    w.appVt.slots[16] = reinterpret_cast<void*>(&AppSaveCity);
    w.app.vtbl = w.appVt.slots.data();
    w.city.vtbl = w.cityVt.slots.data();
    w.res.vtbl = w.resVt.slots.data();
    w.demandSim.vtbl = w.demandSimVt.slots.data();
    for (FakeDemand& d : w.demands) d.vtbl = w.demandVt.slots.data();
}

// ---- the flammability fire path (run 37) ---------------------------------------------------------
// A fire manager as the game hands it out: the disaster layer returns it
// BORROWED (flammability.md §6.8); a QueryInterface for
// cISC4DisasterManagerFire returns a sub-object that shares its count, with one
// AddRef. `refs` starts at the layer's own reference.
struct FakeFireCity {
    FakeVtable cityVt, layerVt, mgrVt, fireVt;
    FakeObject city{}, layer{}, mgr{}, fire{};
    int refs = 1;
    int qis = 0, ignites = 0, causes = 0;  // qis: for cISC4DisasterManagerFire only
    uint32_t cause = 0;
};

FakeFireCity* g_fire = nullptr;

void* __fastcall FireCityGetDisasterLayer(void*, void*) { return &g_fire->layer; }  // cISC4City 72
void* __fastcall FireLayerGetManager(void*, void*, uint32_t type) {                  // layer 7
    return type == 0x621CDA33 ? &g_fire->mgr : nullptr;
}
bool __fastcall FireMgrQi(void*, void*, uint32_t iid, void** out) {  // 0, both interfaces
    if (iid != 0x49B43F57 || out == nullptr) return false;  // (the reference audit asks other IIDs)
    ++g_fire->qis;
    *out = &g_fire->fire;
    ++g_fire->refs;
    return true;
}
uint32_t __fastcall FireMgrAddRef(void*, void*) { return static_cast<uint32_t>(++g_fire->refs); }   // 1
uint32_t __fastcall FireMgrRelease(void*, void*) { return static_cast<uint32_t>(--g_fire->refs); }  // 2
bool __fastcall FireIgniteCell(void*, void*, uint32_t, uint32_t, bool, bool, uint32_t) {  // fire 5
    ++g_fire->ignites;
    return true;
}
uint32_t __fastcall FireSetCause(void*, void*, uint32_t c) {  // fire 9
    ++g_fire->causes;
    g_fire->cause = c;
    return 0;
}

void InitFakeFireCity(FakeFireCity& f) {
    g_fire = &f;
    f.cityVt.slots[72] = reinterpret_cast<void*>(&FireCityGetDisasterLayer);
    f.layerVt.slots[7] = reinterpret_cast<void*>(&FireLayerGetManager);
    for (FakeVtable* vt : {&f.mgrVt, &f.fireVt}) {
        vt->slots[0] = reinterpret_cast<void*>(&FireMgrQi);
        vt->slots[1] = reinterpret_cast<void*>(&FireMgrAddRef);
        vt->slots[2] = reinterpret_cast<void*>(&FireMgrRelease);
    }
    f.fireVt.slots[5] = reinterpret_cast<void*>(&FireIgniteCell);
    f.fireVt.slots[9] = reinterpret_cast<void*>(&FireSetCause);
    f.city.vtbl = f.cityVt.slots.data();
    f.layer.vtbl = f.layerVt.slots.data();
    f.mgr.vtbl = f.mgrVt.slots.data();
    f.fire.vtbl = f.fireVt.slots.data();
}

constexpr uint32_t kMsgPostCityInit = 0x26D31EC1;
constexpr uint32_t kMsgSimNewDay = 0x66956814;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;
constexpr uint32_t kMsgSimIdle = 0x6695681A;

// ---- the budget shadow (test seam op 10) -------------------------------------------------------
// An "original" budget simulator (not ours: its vtable is in this
// executable) with its month agent. Only getters have slots; any other call,
// a write included, hits a trap and aborts. DoMessage(SimNewMonth) "settles":
// it publishes the values in `settle*`.
struct FakeOrigBudget {
    FakeVtable budgetVt, agentVt, cityVt, otherVt;
    FakeObject budget{}, agent{}, city{}, other{};
    int refs = 1;
    int deliveries = 0;
    int64_t funds = 50000, ytdInc = 100, ytdExp = 60, bonds = 0;
    int64_t wa[12] = {}, tax[12] = {}, total = 0;
    int64_t settleWa[12] = {}, settleTax[12] = {}, settleTotal = 0, settleIncome = 0, settleExpense = 0;
};

FakeOrigBudget* g_orig = nullptr;

bool __fastcall OrigQi(void*, void*, uint32_t iid, void** out) {  // 0 (budget and agent)
    if (iid != 0x452294AA || out == nullptr) return false;  // cIGZMessageTarget2
    *out = &g_orig->agent;
    ++g_orig->refs;
    return true;
}
uint32_t __fastcall OrigAddRef(void*, void*) { return static_cast<uint32_t>(++g_orig->refs); }   // 1
uint32_t __fastcall OrigRelease(void*, void*) { return static_cast<uint32_t>(--g_orig->refs); }  // 2
int64_t __fastcall OrigFunds(void*, void*) { return g_orig->funds; }                           // 6
int64_t __fastcall OrigYtdIncome(void*, void*) { return g_orig->ytdInc; }                      // 10
int64_t __fastcall OrigYtdExpense(void*, void*) { return g_orig->ytdExp; }                     // 12
int64_t __fastcall OrigTaxOf(void*, void*, int32_t t) { return t >= 0 && t < 12 ? g_orig->tax[t] : 0; }  // 18
int64_t __fastcall OrigTax(void*, void*) { return g_orig->total; }                             // 19
float __fastcall OrigRate(void*, void*, uint32_t) { return 9.0f; }                             // 36
int64_t __fastcall OrigWa(void*, void*, uint32_t t) { return t < 12 ? g_orig->wa[t] : 0; }     // 38
int64_t __fastcall OrigBonds(void*, void*) { return g_orig->bonds; }                           // 45
bool __fastcall OrigDoMessage(void*, void*, cIGZMessage2* m) {                                 // agent 3
    ++g_orig->deliveries;
    if (m == nullptr || m->GetType() != kMsgSimNewMonth) return true;
    for (int t = 0; t < 12; ++t) {
        g_orig->wa[t] = g_orig->settleWa[t];
        g_orig->tax[t] = g_orig->settleTax[t];
    }
    g_orig->total = g_orig->settleTotal;
    g_orig->ytdInc += g_orig->settleIncome;
    g_orig->ytdExp += g_orig->settleExpense;
    g_orig->funds += g_orig->settleIncome - g_orig->settleExpense;
    return true;
}
int32_t __fastcall OrigCityDifficulty(void*, void*) { return 2; }                    // city 24
void* __fastcall OrigCityBudget(void*, void*) { return &g_orig->budget; }            // city 52
void* __fastcall OrigCityNone(void*, void*) { return nullptr; }                      // city 62, 63

void InitFakeOrigBudget(FakeOrigBudget& o) {
    g_orig = &o;
    for (FakeVtable* vt : {&o.budgetVt, &o.agentVt}) {
        vt->slots[0] = reinterpret_cast<void*>(&OrigQi);
        vt->slots[1] = reinterpret_cast<void*>(&OrigAddRef);
        vt->slots[2] = reinterpret_cast<void*>(&OrigRelease);
    }
    o.budgetVt.slots[6] = reinterpret_cast<void*>(&OrigFunds);
    o.budgetVt.slots[10] = reinterpret_cast<void*>(&OrigYtdIncome);
    o.budgetVt.slots[12] = reinterpret_cast<void*>(&OrigYtdExpense);
    o.budgetVt.slots[18] = reinterpret_cast<void*>(&OrigTaxOf);
    o.budgetVt.slots[19] = reinterpret_cast<void*>(&OrigTax);
    o.budgetVt.slots[36] = reinterpret_cast<void*>(&OrigRate);
    o.budgetVt.slots[38] = reinterpret_cast<void*>(&OrigWa);
    o.budgetVt.slots[45] = reinterpret_cast<void*>(&OrigBonds);
    o.agentVt.slots[3] = reinterpret_cast<void*>(&OrigDoMessage);
    o.otherVt.slots[3] = reinterpret_cast<void*>(&OrigDoMessage);
    o.cityVt.slots[24] = reinterpret_cast<void*>(&OrigCityDifficulty);
    o.cityVt.slots[52] = reinterpret_cast<void*>(&OrigCityBudget);
    o.cityVt.slots[62] = reinterpret_cast<void*>(&OrigCityNone);
    o.cityVt.slots[63] = reinterpret_cast<void*>(&OrigCityNone);
    o.budget.vtbl = o.budgetVt.slots.data();
    o.agent.vtbl = o.agentVt.slots.data();
    o.city.vtbl = o.cityVt.slots.data();
    o.other.vtbl = o.otherVt.slots.data();
}

// One shadowed month: the "original" settles to `wa` (types 0, 3, 6), with
// its per-type tax the GetTaxIncome(t) formula gives at 9 % with the code
// default multiplier (budget §9: 100000): 1000 -> 750000, 2000 -> 1500000,
// 2500 -> 1875000, 3000 -> 2250000 (float32, checked by hand).
intptr_t ShadowMonth(FC4TestSeamFn seam, FakeOrigBudget& o, cIGZMessageTarget2* target, int64_t wa3, int64_t tax3) {
    // Lots (code-default tuning, land value 0: assessed = building value).
    static fc4_lot_info lots[5] = {};
    lots[0] = fc4_lot_info{1, 1, 0, 0, 1, 1, 1, 1000, 1, 1};  // R$
    lots[1] = fc4_lot_info{1, 1, 0, 0, 2, 1, 1, 2000, 2, 1};  // CS$
    lots[2] = fc4_lot_info{1, 0, 0, 0, 2, 1, 1, 500, 1, 1};   // CS$, not livable
    lots[3] = fc4_lot_info{1, 1, 0, 0, 3, 2, 2, 3000, 1, 2};  // CO$$
    lots[4] = fc4_lot_info{};                                // no building
    static FC4ShadowTestLot feed[5] = {};
    for (int i = 0; i < 5; ++i) feed[i] = FC4ShadowTestLot{&lots[i], i, i + 1};
    for (int t = 0; t < 12; ++t) o.settleWa[t] = o.settleTax[t] = 0;
    o.settleWa[0] = 1000;
    o.settleTax[0] = 750000;
    o.settleWa[3] = wa3;
    o.settleTax[3] = tax3;
    o.settleWa[6] = 3000;
    o.settleTax[6] = 2250000;
    o.settleTotal = 4500000;
    o.settleIncome = 4500000;
    o.settleExpense = 0;
    FC4ShadowTestInput in{reinterpret_cast<cISC4City*>(&o.city), target, feed, 5, 2451576};  // 2000-02-01
    return seam(kFC4SeamBudgetShadow, &in, nullptr);
}

// ---- the demand shadow (test seam op 11) -------------------------------------------------------
// An "original" demand simulator (not ours: its vtables are in this
// executable) with its agent and three nodes. Only getters have slots; any
// other call, a write included, hits a trap and aborts. DoMessage "updates":
// it copies `post` over the nodes (the test writes what the original would
// leave). Graph (demand exemplars, group 0xC7BB4816):
//   0x1011 (census R$, region use 1): Drives 0x21010 100 %
//   0x1810: Caps 0x21010
//   0x21010 (total R$): a link target only
struct FakeOrigNode {
    void** vtbl;
    uint32_t key, ru;
    float supply, demand, ns, nd, active, econ, max, min, tax;
    float cap;  // SC4Percentage (GetDemandCap returns a pointer to it)
};

struct FakeOrigDemand {
    FakeVtable simVt, agentVt, nodeVt, cityVt, otherVt;
    FakeObject sim{}, agent{}, city{}, other{};
    FakeOrigNode nodes[3] = {};
    FakeOrigNode post[3] = {};
    int refs = 1;
    int deliveries = 0;
};

FakeOrigDemand* g_od = nullptr;

bool __fastcall OdQi(void*, void*, uint32_t iid, void** out) {  // 0 (simulator and agent)
    if (iid != 0x452294AA || out == nullptr) return false;     // cIGZMessageTarget2
    *out = &g_od->agent;
    ++g_od->refs;
    return true;
}
uint32_t __fastcall OdAddRef(void*, void*) { return static_cast<uint32_t>(++g_od->refs); }   // 1
uint32_t __fastcall OdRelease(void*, void*) { return static_cast<uint32_t>(--g_od->refs); }  // 2
void* __fastcall OdGetDemand(void*, void*, uint32_t id, uint32_t index) {                     // simulator 6
    if (index != 0) return nullptr;
    for (FakeOrigNode& n : g_od->nodes) {
        if (n.key == id) return &n;
    }
    return nullptr;
}
FakeOrigNode* On(void* self) { return static_cast<FakeOrigNode*>(self); }
float __fastcall OdSupply(void* s, void*) { return On(s)->supply; }    // 8
float __fastcall OdDemand(void* s, void*) { return On(s)->demand; }    // 9
float __fastcall OdNewSupply(void* s, void*) { return On(s)->ns; }     // 10
float __fastcall OdNewDemand(void* s, void*) { return On(s)->nd; }     // 11
float __fastcall OdActive(void* s, void*) { return On(s)->active; }    // 12
float __fastcall OdEcon(void* s, void*) { return On(s)->econ; }        // 13
float __fastcall OdMax(void* s, void*) { return On(s)->max; }          // 14
float __fastcall OdMin(void* s, void*) { return On(s)->min; }          // 15
float __fastcall OdTax(void* s, void*) { return On(s)->tax; }          // 24
void* __fastcall OdCap(void* s, void*) { return &On(s)->cap; }         // 26
uint32_t __fastcall OdRegionUse(void* s, void*) { return On(s)->ru; }  // 28
bool __fastcall OdDoMessage(void*, void*, cIGZMessage2*) {             // agent 3
    ++g_od->deliveries;
    for (int i = 0; i < 3; ++i) g_od->nodes[i] = g_od->post[i];
    return true;
}
void* __fastcall OdCityDemand(void*, void*) { return &g_od->sim; }  // city 56

void InitFakeOrigDemand(FakeOrigDemand& o) {
    g_od = &o;
    for (FakeVtable* vt : {&o.simVt, &o.agentVt}) {
        vt->slots[0] = reinterpret_cast<void*>(&OdQi);
        vt->slots[1] = reinterpret_cast<void*>(&OdAddRef);
        vt->slots[2] = reinterpret_cast<void*>(&OdRelease);
    }
    o.simVt.slots[6] = reinterpret_cast<void*>(&OdGetDemand);
    o.nodeVt.slots[8] = reinterpret_cast<void*>(&OdSupply);
    o.nodeVt.slots[9] = reinterpret_cast<void*>(&OdDemand);
    o.nodeVt.slots[10] = reinterpret_cast<void*>(&OdNewSupply);
    o.nodeVt.slots[11] = reinterpret_cast<void*>(&OdNewDemand);
    o.nodeVt.slots[12] = reinterpret_cast<void*>(&OdActive);
    o.nodeVt.slots[13] = reinterpret_cast<void*>(&OdEcon);
    o.nodeVt.slots[14] = reinterpret_cast<void*>(&OdMax);
    o.nodeVt.slots[15] = reinterpret_cast<void*>(&OdMin);
    o.nodeVt.slots[24] = reinterpret_cast<void*>(&OdTax);
    o.nodeVt.slots[26] = reinterpret_cast<void*>(&OdCap);
    o.nodeVt.slots[28] = reinterpret_cast<void*>(&OdRegionUse);
    o.agentVt.slots[3] = reinterpret_cast<void*>(&OdDoMessage);
    o.otherVt.slots[3] = reinterpret_cast<void*>(&OdDoMessage);
    o.cityVt.slots[56] = reinterpret_cast<void*>(&OdCityDemand);
    o.sim.vtbl = o.simVt.slots.data();
    o.agent.vtbl = o.agentVt.slots.data();
    o.city.vtbl = o.cityVt.slots.data();
    o.other.vtbl = o.otherVt.slots.data();
}

FakeOrigNode OdNode(const FakeOrigDemand& o, uint32_t key, uint32_t ru, float supply, float demand, float ns,
                    float active, float cap, float tax = 0.0f, float econ = 0.0f) {
    FakeOrigNode n{};
    n.vtbl = const_cast<void**>(o.nodeVt.slots.data());
    n.key = key;
    n.ru = ru;
    n.supply = supply;
    n.demand = demand;
    n.ns = ns;
    n.active = active;
    n.econ = econ;
    n.max = 6000.0f;
    n.min = -6000.0f;
    n.tax = tax;
    n.cap = cap;
    return n;
}

// The state before every NewDay case: 0x1011 has 10 pending supply.
void OdPre(FakeOrigDemand& o) {
    o.nodes[0] = OdNode(o, 0x1011, 1, 0.0f, 0.0f, 10.0f, 0.0f, 0.0f);
    o.nodes[1] = OdNode(o, 0x1810, 0, 50000.0f, 10.0f, 0.0f, -6000.0f, 0.0f);
    o.nodes[2] = OdNode(o, 0x21010, 0, 0.0f, 20.0f, 0.0f, 20.0f, 0.0f);
}

// What the original leaves after a NewDay whose census added `census` to
// 0x1011's pending supply, with 0x21010's modifiers set to tax / econ and its
// committed demand off by `skew` (the propagation, §4.4, by hand: 0x1011
// drives 10 + census into 0x21010 and folds; 0x1810 caps 0x21010 at
// 10 / 50000; 0x21010 folds; a second pass moves nothing).
void OdPost(FakeOrigDemand& o, float census, float tax, float econ, float skew) {
    const float a = 10.0f + census;
    const float d = 20.0f + a;
    const float m = tax + econ;
    const float bonus = std::fmin(6000.0f * m, d * m);
    o.post[0] = OdNode(o, 0x1011, 1, a, 0.0f, 0.0f, -a, 0.0f);
    o.post[1] = o.nodes[1];
    o.post[2] = OdNode(o, 0x21010, 0, 0.0f, d + skew, 0.0f, d + bonus + skew, 10.0f / 50000.0f, tax, econ);
}

intptr_t DemandShadowOnce(FC4TestSeamFn seam, FakeOrigDemand& o, cIGZMessageTarget2* target, uint32_t type,
                          float ordinance, float nation) {
    static const uint32_t keys[2] = {0x1011, 0x1810};
    static const FC4DemandShadowTestLink links[2] = {{0x1011, 0x47BB3F30, 0x21010, 100},
                                                     {0x1810, 0x47BB3F41, 0x21010, 0}};
    FC4DemandShadowTestInput in{reinterpret_cast<cISC4City*>(&o.city), target, keys, 2, links, 2, type,
                                2451576,  // 2000-02-01
                                -1.0f, ordinance, nation};
    return seam(kFC4SeamDemandShadow, &in, nullptr);
}

// ---- the pollution shadow (test seam op 16) ------------------------------------------------------
// An "original" pollution simulator: QueryInterface answers its agent
// (cIGZMessageTarget2) and cIGZSerializable, whose Write writes the record's
// version alone (a uint16): a record without the grid references, so the
// shadow reports the delivery INCOMPLETE and reads nothing else. Any other
// call hits a trap and aborts. The city answers GetPollutionSimulator
// (compiled slot 66) only; `noCity` returns null there.
struct FakeOrigPollution {
    FakeVtable simVt, agentVt, serVt, cityVt, noCityVt;
    FakeObject sim{}, agent{}, ser{}, city{}, noCity{}, other{};
    int refs = 1;
    int deliveries = 0;
    int writes = 0;
};

FakeOrigPollution* g_op = nullptr;

bool __fastcall OpQi(void*, void*, uint32_t iid, void** out) {  // 0 (simulator, agent, serializable)
    if (out == nullptr) return false;
    if (iid == 0x452294AA) {
        *out = &g_op->agent;  // cIGZMessageTarget2
    } else if (iid == GZIID_cIGZSerializable) {
        *out = &g_op->ser;
    } else {
        return false;  // (the reference audit asks other IIDs)
    }
    ++g_op->refs;
    return true;
}
uint32_t __fastcall OpAddRef(void*, void*) { return static_cast<uint32_t>(++g_op->refs); }   // 1
uint32_t __fastcall OpRelease(void*, void*) { return static_cast<uint32_t>(--g_op->refs); }  // 2
bool __fastcall OpWrite(void*, void*, cIGZOStream* s) {                                       // serializable 3
    ++g_op->writes;
    return s->SetUint16(4);
}
// serializable 5 (the reference audit's class lookup asks it too)
uint32_t __fastcall OpClsid(void*, void*) { return 0x8990C065; }
bool __fastcall OpDoMessage(void*, void*, cIGZMessage2*) {  // agent 3
    ++g_op->deliveries;
    return true;
}
void* __fastcall OpCityPollution(void*, void*) { return &g_op->sim; }  // city 66
void* __fastcall OpCityNoPollution(void*, void*) { return nullptr; }   // city 66

void InitFakeOrigPollution(FakeOrigPollution& o) {
    g_op = &o;
    for (FakeVtable* vt : {&o.simVt, &o.agentVt, &o.serVt}) {
        vt->slots[0] = reinterpret_cast<void*>(&OpQi);
        vt->slots[1] = reinterpret_cast<void*>(&OpAddRef);
        vt->slots[2] = reinterpret_cast<void*>(&OpRelease);
    }
    o.agentVt.slots[3] = reinterpret_cast<void*>(&OpDoMessage);
    o.serVt.slots[3] = reinterpret_cast<void*>(&OpWrite);
    o.serVt.slots[5] = reinterpret_cast<void*>(&OpClsid);
    o.cityVt.slots[66] = reinterpret_cast<void*>(&OpCityPollution);
    o.noCityVt.slots[66] = reinterpret_cast<void*>(&OpCityNoPollution);
    o.sim.vtbl = o.simVt.slots.data();
    o.agent.vtbl = o.agentVt.slots.data();
    o.ser.vtbl = o.serVt.slots.data();
    o.city.vtbl = o.cityVt.slots.data();
    o.noCity.vtbl = o.noCityVt.slots.data();
    o.other.vtbl = o.agentVt.slots.data();  // an agent, but not the simulator's
}

intptr_t PollutionShadowOnce(FC4TestSeamFn seam, FakeObject* city, cIGZMessageTarget2* target, uint32_t type) {
    FC4PollutionShadowTestInput in{reinterpret_cast<cISC4City*>(city), target, type, 2451576};  // 2000-02-01
    return seam(kFC4SeamPollutionShadow, &in, nullptr);
}

// ---- the reference audit's created-temporary release (test seam op 12) ----------------------------
// A game object (vtable in this executable) whose Release returns 0: the
// last reference. QueryInterface (the audit's class lookup) answers no.
bool __fastcall TempQi(void*, void*, uint32_t, void** out) {
    if (out != nullptr) *out = nullptr;
    return false;
}
uint32_t __fastcall TempRelease(void*, void*) { return 0; }

// ---- autotest: child processes that end with TerminateProcess -----------------------------------

// Runs this executable with `args`; returns its exit code (or -1) and the time it took.
int RunChild(const std::string& args, DWORD timeoutMs, DWORD* elapsedMs) {
    char exe[MAX_PATH] = {};
    GetModuleFileNameA(nullptr, exe, MAX_PATH);
    std::string cmd = "\"" + std::string(exe) + "\" " + args;
    STARTUPINFOA si{};
    si.cb = sizeof(si);
    PROCESS_INFORMATION pi{};
    const DWORD start = GetTickCount();
    if (!CreateProcessA(nullptr, &cmd[0], nullptr, nullptr, FALSE, 0, nullptr, nullptr, &si, &pi)) return -1;
    const DWORD w = WaitForSingleObject(pi.hProcess, timeoutMs);
    *elapsedMs = GetTickCount() - start;
    DWORD code = static_cast<DWORD>(-1);
    if (w == WAIT_OBJECT_0) {
        GetExitCodeProcess(pi.hProcess, &code);
    } else {
        TerminateProcess(pi.hProcess, 99);
        std::fprintf(stderr, "child '%s' did not end within %lu ms\n", args.c_str(), static_cast<unsigned long>(timeoutMs));
    }
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);
    return w == WAIT_OBJECT_0 ? static_cast<int>(code) : -1;
}

// A child: loads the DLL with an autotest FreeCity4.ini and plays the game's
// part through the test seam. Never returns normally (the DLL terminates it);
// exit code 3 means it did not.
int AutotestChild(const std::string& dll, const std::string& ini, const std::string& scenario) {
    if (scenario == "clean") {
        WriteTextFile(ini, "[autotest]\nautotest_seconds = 2\nautotest_unpause = 1\n");
    } else if (scenario == "corrupt") {
        WriteTextFile(ini, "[autotest]\nautotest_seconds = 60\n");
    } else if (scenario == "save") {
        WriteTextFile(ini, "[autotest]\nautotest_save_after_months = 2\nautotest_save_fast = 0\nobserve = 1\n");
    } else if (scenario == "panic") {
        WriteTextFile(ini, "[autotest]\n");
    } else {  // stalled: passthrough (the observer alone)
        WriteTextFile(ini, "[subsystems]\ndisable = all\n[autotest]\nautotest_seconds = 60\n");
    }
    HMODULE h = LoadLibraryA(dll.c_str());
    if (h == nullptr) return 4;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(h, "GZDllGetGZCOMDirector"));
    auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(h, "FC4TestSeam"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    if (director == nullptr || seam == nullptr) return 5;
    std::vector<uint32_t> classes;
    director->EnumClassObjects(&CollectClass, &classes);  // game start: configuration and autotest settings
    if (scenario == "panic") {
        seam(kFC4SeamRustPanic, nullptr, nullptr);  // release builds end here (fail-fast)
        return 3;
    }

    static FakeWorld world;
    InitFakeWorld(world);
    cISC4Simulator* sim = nullptr;
    if (scenario == "clean" || scenario == "save") {
        if (!director->GetClassObject(kCLSID_Simulator, kIID_cISC4Simulator, reinterpret_cast<void**>(&sim)) ||
            sim == nullptr) {
            return 6;
        }
        sim->Init();
        sim->Pause();  // the autotest must resume it
        world.sim = sim;
    }
    if (scenario == "save") {
        // Two month-ends, then the save at the next tick ends the process.
        seam(kFC4SeamObserverApp, &world.app, nullptr);
        seam(kFC4SeamObserverCity, &world.city, nullptr);
        seam(kFC4SeamObserverMessage, reinterpret_cast<void*>(static_cast<uintptr_t>(kMsgPostCityInit)), nullptr);
        seam(kFC4SeamObserverTick, nullptr, nullptr);  // the "after load" snapshot
        for (intptr_t julian : {2451576, 2451605}) {
            seam(kFC4SeamObserverMessage, reinterpret_cast<void*>(static_cast<uintptr_t>(kMsgSimNewMonth)),
                 reinterpret_cast<void*>(julian));
        }
        seam(kFC4SeamObserverTick, nullptr, nullptr);  // saves and terminates
        return 3;
    }
    seam(kFC4SeamObserverCity, &world.city, nullptr);
    seam(kFC4SeamObserverMessage, reinterpret_cast<void*>(static_cast<uintptr_t>(kMsgPostCityInit)), nullptr);
    seam(kFC4SeamObserverMessage, reinterpret_cast<void*>(static_cast<uintptr_t>(kMsgSimNewDay)),
         reinterpret_cast<void*>(static_cast<intptr_t>(2451600)));
    const DWORD start = GetTickCount();
    bool faulted = false;
    for (;;) {
        seam(kFC4SeamObserverTick, nullptr, nullptr);
        Sleep(20);
        const DWORD ms = GetTickCount() - start;
        if (scenario == "corrupt" && ms >= 300 && !faulted) {
            faulted = true;
            ForeignFault();  // then keep ticking until the DLL ends the process
        }
        if (scenario == "stalled" && ms >= 500) Sleep(INFINITE);  // the main thread stops ticking
        if (ms > 60000) return 3;
    }
}

// ---- popup_dialogs (FC4Popups.h, advisors §6.8.1 option 1): one child process per FreeCity4.ini ----
// A cISC4App with only slots 23/24; every other slot traps, so the switch can
// call nothing else (no QueryInterface, AddRef or Release of the borrowed app).
struct FakePopupApp {
    FakeVtable vt;
    FakeObject app{};
    bool enabled = true;
    bool ignoreSet = false;  // a setter that does nothing ("did not take effect")
    int gets = 0;
    int sets = 0;
};

FakePopupApp* g_popupApp = nullptr;

bool __fastcall AppGetPopupDialogsEnabled(void*, void*) {  // cISC4App 23 (+0x5C), pops 0
    ++g_popupApp->gets;
    return g_popupApp->enabled;
}
void __fastcall AppSetPopupDialogsEnabled(void*, void*, bool on) {  // cISC4App 24 (+0x60), one bool, pops 4
    ++g_popupApp->sets;
    if (!g_popupApp->ignoreSet) g_popupApp->enabled = on;
}

// Scenarios: off (popup_dialogs = 0), on (= 1; the parent also sets
// FC4_POPUP_DIALOGS=0, which the ini overrides), env (FC4_POPUP_DIALOGS=0
// only), bad (= maybe), unset, stuck (= 0 with a setter that does nothing).
// Messages, in order: a city load and a ConnectionsReady without an
// application (tests have no framework); then, with the fake, two
// ConnectionsReady, a second city load and its ConnectionsReady. = 0 sets
// false at the first ConnectionsReady with the app, true at the second load
// and false again at its ConnectionsReady (foundations.md 11.6); = 1 makes
// its one call at the second load. Returns 0 if the fake saw what the
// scenario expects; the parent checks the log.
int PopupsChild(const std::string& dll, const std::string& ini, const std::string& scenario) {
    if (scenario == "off" || scenario == "stuck") {
        WriteTextFile(ini, "[unattended]\npopup_dialogs = 0   ; no advisor popups\n");
    } else if (scenario == "on") {
        WriteTextFile(ini, "[unattended]\npopup_dialogs = 1\n");
    } else if (scenario == "bad") {
        WriteTextFile(ini, "[unattended]\npopup_dialogs = maybe\n");
    } else {  // env, unset
        WriteTextFile(ini, "; no popup_dialogs key\n");
    }
    HMODULE h = LoadLibraryA(dll.c_str());
    if (h == nullptr) return 4;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(h, "GZDllGetGZCOMDirector"));
    auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(h, "FC4TestSeam"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    if (director == nullptr || seam == nullptr) return 5;
    std::vector<uint32_t> classes;
    director->EnumClassObjects(&CollectClass, &classes);  // game start: popup_dialogs is read

    static FakePopupApp fake;
    g_popupApp = &fake;
    fake.vt.slots[23] = reinterpret_cast<void*>(&AppGetPopupDialogsEnabled);
    fake.vt.slots[24] = reinterpret_cast<void*>(&AppSetPopupDialogsEnabled);
    fake.app.vtbl = fake.vt.slots.data();
    fake.enabled = scenario != "on";  // "on": the game's flag starts false
    fake.ignoreSet = scenario == "stuck";
    void* const cityInit = reinterpret_cast<void*>(static_cast<uintptr_t>(kMsgPostCityInit));
    void* const connReady = reinterpret_cast<void*>(static_cast<uintptr_t>(0x6AC284F3u));  // ConnectionsReady

    seam(kFC4SeamObserverMessage, cityInit, nullptr);   // no application: = 1 skipped, logged
    seam(kFC4SeamObserverMessage, connReady, nullptr);  // no application: = 0 skipped, logged
    const int setsWithoutApp = fake.sets;
    seam(kFC4SeamObserverApp, &fake.app, nullptr);
    seam(kFC4SeamObserverMessage, connReady, nullptr);  // = 0: false
    const int setsFirst = fake.sets;
    const bool enabledFirst = fake.enabled;
    seam(kFC4SeamObserverMessage, connReady, nullptr);  // = 0: already false, nothing
    const int setsSecond = fake.sets;
    seam(kFC4SeamObserverMessage, cityInit, nullptr);  // = 0: true until ConnectionsReady; = 1: the call
    const bool enabledLoad = fake.enabled;
    seam(kFC4SeamObserverMessage, connReady, nullptr);  // = 0: false again
    seam(kFC4SeamObserverApp, nullptr, nullptr);
    FreeLibrary(h);

    const bool off = scenario == "off" || scenario == "env";
    // Sets: off/env 3 (false, true, false); stuck 1 (the failed call is not
    // retried); on 1 (at the second city load); bad/unset 0.
    const int wantSets = off ? 3 : (scenario == "stuck" || scenario == "on") ? 1 : 0;
    const int wantFirst = off || scenario == "stuck" ? 1 : 0;
    int bad = 0;
    if (setsWithoutApp != 0) bad |= 1;
    if (setsFirst != wantFirst || setsSecond != setsFirst || fake.sets != wantSets) bad |= 2;
    if (fake.gets != 2 * wantSets) bad |= 4;  // before and after each call
    if (fake.enabled != !off) bad |= 8;
    if (off && (enabledFirst || !enabledLoad)) bad |= 16;
    if (bad != 0) std::fprintf(stderr, "popups child %s: fake app check bits 0x%X\n", scenario.c_str(), bad);
    return bad == 0 ? 0 : 10 + bad;
}

int PopupsTest(const std::string& dll, const std::string& ini, const std::string& log) {
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    const std::string dllArg = "\"" + dll + "\" ";
    const auto npos = std::string::npos;
    auto run = [&](const char* scenario, const char* env) {
        SetEnvironmentVariableA("FC4_POPUP_DIALOGS", env);
        DWORD ms = 0;
        const int code = RunChild(dllArg + "--popups-child-" + scenario, 30000, &ms);
        std::printf("popups child %s: exit code %d in %lu ms\n", scenario, code, static_cast<unsigned long>(ms));
        CHECK(code == 0);
        return ReadFile(log);
    };
    const std::string skipped =
        "popup dialogs: SetPopupDialogsEnabled(false) not called at ConnectionsReady: no cISC4App (no framework); "
        "retried at the next ConnectionsReady";
    const std::string disabled =
        "popup dialogs: disabled (cISC4App::SetPopupDialogsEnabled(false); was enabled=1, now=0) at ConnectionsReady";

    // = 0: read at game start; nothing at a city load before the first false; skipped without an application;
    // false at ConnectionsReady, true again at the next city load, false again at its ConnectionsReady.
    std::string text = run("off", nullptr);
    CHECK(text.find("popup dialogs: popup_dialogs=0 (FreeCity4.ini): cISC4App::SetPopupDialogsEnabled(false) at "
                    "each ConnectionsReady 0x6AC284F3, true again at each later city load") != npos);
    CHECK(CountOf(text, skipped) == 1);
    CHECK(CountOf(text, disabled) == 2);
    CHECK(CountOf(text, "popup dialogs: enabled (cISC4App::SetPopupDialogsEnabled(true); was enabled=0, now=1) at "
                        "city load (PostCityInit), until this load's ConnectionsReady") == 1);
    CHECK(text.find("SetPopupDialogsEnabled(false) not called at city load") == npos);
    CHECK(text.find("config: WARNING") == npos);  // popup_dialogs is a known key
    CHECK(text.find("WARNING: released last reference") == npos && text.find("WARNING: over-release") == npos);

    // = 1, with FC4_POPUP_DIALOGS=0: the ini wins.
    text = run("on", "0");
    CHECK(text.find("popup dialogs: popup_dialogs=1 (FreeCity4.ini): cISC4App::SetPopupDialogsEnabled(true) at "
                    "PostAppInit") != npos);
    CHECK(CountOf(text, "popup dialogs: enabled (cISC4App::SetPopupDialogsEnabled(true); was enabled=0, now=1) at "
                        "city load (PostCityInit)") == 1);
    CHECK(CountOf(text, "popup dialogs: SetPopupDialogsEnabled(true) not called at city load (PostCityInit): no "
                        "cISC4App (no framework); retried at the next city load") == 1);
    CHECK(text.find("at ConnectionsReady") == npos);  // = 1 ignores it

    // The environment alone.
    text = run("env", "0");
    CHECK(text.find("popup dialogs: popup_dialogs=0 (FC4_POPUP_DIALOGS): cISC4App::SetPopupDialogsEnabled(false)") !=
          npos);
    CHECK(CountOf(text, disabled) == 2);

    // Not 0/1: a config warning, and nothing is called.
    text = run("bad", nullptr);
    CHECK(text.find("config: WARNING popup_dialogs='maybe' (FreeCity4.ini) is not 0/1; the game's popup dialogs are "
                    "left alone") != npos);
    CHECK(text.find("SetPopupDialogsEnabled(") == npos);

    // Not set: the game is left alone.
    text = run("unset", nullptr);
    CHECK(text.find("popup dialogs: popup_dialogs not set; the game's popup dialogs are left alone") != npos);
    CHECK(text.find("SetPopupDialogsEnabled(") == npos);

    // A setter that does not take effect: one warning, not retried.
    text = run("stuck", nullptr);
    CHECK(CountOf(text, "popup dialogs: WARNING SetPopupDialogsEnabled(false) did not take effect (was enabled=1, "
                        "now=1) at ConnectionsReady") == 1);
    CHECK(CountOf(text, "SetPopupDialogsEnabled(true)") == 0);
    CHECK(text.find("popup dialogs: disabled") == npos);

    SetEnvironmentVariableA("FC4_POPUP_DIALOGS", nullptr);
    DeleteFileA(ini.c_str());
    if (g_failures == 0) std::printf("plugin popup_dialogs test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}

}  // namespace

int main(int argc, char** argv) {
    if (argc < 2) {
        std::fprintf(stderr, "usage: %s <FreeCity4.dll>\n", argv[0]);
        return 2;
    }
    const std::string dll = argv[1];
    const std::string mode = argc >= 3 ? argv[2] : "--normal";
    const std::string log = dll.substr(0, dll.find_last_of("\\/") + 1) + "FreeCity4.log";
    const std::string ini = dll.substr(0, dll.find_last_of("\\/") + 1) + "FreeCity4.ini";
    DeleteFileA(log.c_str());
    DeleteFileA(ini.c_str());  // a leftover from an aborted run must not change this one
    SetEnvironmentVariableA("FC4_LOG_LEVEL", "2");
    SetEnvironmentVariableA("FC4_DISABLE_SIMULATOR", nullptr);
    SetEnvironmentVariableA("FC4_OBSERVE", "1");  // the same lines in release and checked builds
    SetEnvironmentVariableA("FC4_AUTOTEST_SAVE_AFTER_MONTHS", nullptr);

    const std::string childPrefix = "--autotest-child-";
    if (mode.compare(0, childPrefix.size(), childPrefix) == 0) {
        return AutotestChild(dll, ini, mode.substr(childPrefix.size()));
    }
    // popup_dialogs: the parent runs one child per FreeCity4.ini.
    const std::string popupsPrefix = "--popups-child-";
    if (mode.compare(0, popupsPrefix.size(), popupsPrefix) == 0) {
        return PopupsChild(dll, ini, mode.substr(popupsPrefix.size()));
    }
    if (mode == "--ini-popups") return PopupsTest(dll, ini, log);

    if (mode == "--autotest") {
        // Autotest: the verdict line, then TerminateProcess(exit code 0).
        SetEnvironmentVariableA("FC4_DISABLE", nullptr);
        SetEnvironmentVariableA("FC4_AUTOTEST_SECONDS", nullptr);
        SetEnvironmentVariableA("FC4_AUTOTEST_UNPAUSE", nullptr);
        DWORD ms = 0;
        const std::string dllArg = "\"" + dll + "\" ";

        // 1. Clean: verdict 2 s after the first tick; the paused simulator is resumed at speed 1.
        CHECK(RunChild(dllArg + "--autotest-child-clean", 30000, &ms) == 0);
        std::printf("autotest clean child: %lu ms\n", static_cast<unsigned long>(ms));
        CHECK(ms >= 1500 && ms < 15000);
        std::string text = ReadFile(log);
        CHECK(text.find("autotest: ON - verdict 2 s after the first tick of a loaded city, then TerminateProcess; "
                        "unpause to speed 1") != std::string::npos);
        CHECK(text.find("autotest: city loaded (PostCityInit)") != std::string::npos);
        CHECK(text.find("autotest: clock started; verdict in 2 s") != std::string::npos);
        CHECK(text.find("autotest: unpause: 1 Resume call(s), speed 1") != std::string::npos);
        CHECK(text.find("paused=0 hidden=") != std::string::npos);
        // julian: our simulator publishes its own date (2000-01-01; it is not ticked here).
        CHECK(text.find("AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, "
                        "ref_warnings=0, julian=2451545, switches_off=-, disabled=-)") != std::string::npos);
        CHECK(text.find("autotest: TerminateProcess(exit code 0) from the autotest thread") != std::string::npos);

        // 2. Corrupt: a first-chance access violation in the host; terminate ~5 s later.
        CHECK(RunChild(dllArg + "--autotest-child-corrupt", 30000, &ms) == 0);
        std::printf("autotest corrupt child: %lu ms\n", static_cast<unsigned long>(ms));
        CHECK(ms >= 4500 && ms < 20000);
        text = ReadFile(log);
        CHECK(text.find("first-chance exception OUTSIDE FreeCity4.dll in fc4_plugin_load_test.exe+0x") !=
              std::string::npos);
        CHECK(text.find("AUTOTEST VERDICT: corrupt (foreign_exceptions=1, watchdog_stalls=0, rtc=0, overruns=0, "
                        "ref_warnings=0, julian=2451600, switches_off=-, disabled=-)") != std::string::npos);
        CHECK(text.find("autotest: terminating in 5000 ms (time for stall evidence)") != std::string::npos);
        CHECK(text.find("autotest: TerminateProcess(exit code 0) from the autotest thread") != std::string::npos);
        CHECK(CountOf(text, "AUTOTEST VERDICT:") == 1);

        // 3. Stalled (passthrough: our classes off, the observer alone): no tick for > 10 s.
        CHECK(RunChild(dllArg + "--autotest-child-stalled", 40000, &ms) == 0);
        std::printf("autotest stalled child: %lu ms\n", static_cast<unsigned long>(ms));
        CHECK(ms >= 9000 && ms < 30000);
        text = ReadFile(log);
        CHECK(text.find("observer: PostCityInit (passthrough)") != std::string::npos);
        CHECK(text.find("watchdog: started for main thread") != std::string::npos);
        CHECK(text.find("AUTOTEST VERDICT: stalled (foreign_exceptions=0, watchdog_stalls=1, rtc=0, overruns=0, "
                        "ref_warnings=0, julian=2451600, switches_off=-, disabled=simulator,budget)") !=
              std::string::npos);
        CHECK(text.find("autotest: TerminateProcess(exit code 0) from the watchdog thread") != std::string::npos);
        CHECK(text.find("for 4294967") == std::string::npos);  // no unsigned underflow in any duration

        // A Rust panic in the core: reported in the log (message, location,
        // backtrace, the last interface calls) before the abort.
        {
            const int code = RunChild(dllArg + "--autotest-child-panic", 30000, &ms);
            std::printf("panic child: exit code 0x%08X\n", static_cast<unsigned>(code));
            CHECK(code == static_cast<int>(0xC0000409) || code == 3);  // abort (release) or unwound
            text = ReadFile(log);
            CHECK(text.find("fail-fast reports on:") != std::string::npos);
            CHECK(text.find("Rust PANIC in FreeCity4 core: 'fc4_debug_panic: a deliberate panic (test of the panic "
                            "report)' at ") != std::string::npos);
            CHECK(text.find("panic_log.rs:") != std::string::npos);
            CHECK(text.find("Rust backtrace:") != std::string::npos);
            CHECK(text.find("FreeCity4 interface calls (oldest first):") != std::string::npos);
        }

        // 4. Save after 2 month-ends (no autotest_seconds): snapshot, SaveCity(full), verdict, terminate.
        CHECK(RunChild(dllArg + "--autotest-child-save", 30000, &ms) == 0);
        std::printf("autotest save child: %lu ms\n", static_cast<unsigned long>(ms));
        text = ReadFile(log);
        CHECK(text.find("autotest: save ON - after 2 month-end(s) of a loaded city: snapshot, SaveCity(full) in place, "
                        "verdict, TerminateProcess") != std::string::npos);
        CHECK(text.find("observe snapshot (after load): date=2000-01-01 julian=2451545 funds=- fundings=-") !=
              std::string::npos);
        CHECK(text.find("observe month 2000-02: date=2000-02-01 julian=2451576 funds=-") != std::string::npos);
        CHECK(text.find("observe month 2000-03: date=2000-03-01 julian=2451605 funds=-") != std::string::npos);
        CHECK(text.find("autotest: 2 month-end(s) since the city load; saving at the next tick") != std::string::npos);
        CHECK(text.find("observe snapshot (before save): date=2000-01-01 julian=2451545 funds=- fundings=-") !=
              std::string::npos);
        CHECK(text.find("autotest: save after 2 month(s): SaveCity(full) returned 1 in ") != std::string::npos);
        CHECK(text.find("AUTOTEST VERDICT: clean (foreign_exceptions=0, watchdog_stalls=0, rtc=0, overruns=0, "
                        "ref_warnings=0, julian=2451605, switches_off=-, disabled=-)") != std::string::npos);
        CHECK(text.find("autotest: TerminateProcess(exit code 0) from the observer (save autotest)") !=
              std::string::npos);
        CHECK(CountOf(text, "AUTOTEST VERDICT:") == 1);
        CHECK(text.find("config: WARNING unknown key") == std::string::npos);  // observe, autotest_save_* are known
        DeleteFileA(ini.c_str());
        if (g_failures == 0) std::printf("plugin autotest test: all checks passed\n");
        return g_failures == 0 ? 0 : 1;
    }

    if (mode == "--ini-disabled") {
        // FreeCity4.ini `disable =` wins over FC4_DISABLE; switches are read at
        // game start; unknown keys are reported.
        SetEnvironmentVariableA("FC4_DISABLE", "simulator");
        WriteTextFile(ini, "; bisection test\n[subsystems]\ndisable = Budget   # the original budget\nlot_scan = off\n"
                       "bogus_key = 1\n");
        HMODULE hd = LoadLibraryA(dll.c_str());
        CHECK(hd != nullptr);
        if (hd == nullptr) return 1;
        auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
        cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
        CHECK(director != nullptr);
        if (director != nullptr) {
            std::vector<uint32_t> classes;
            director->EnumClassObjects(&CollectClass, &classes);
            CHECK(classes.size() == 1 && classes[0] == kCLSID_Simulator);
            void* obj = nullptr;
            CHECK(!director->GetClassObject(kCLSID_Budget, kIID_cISC4BudgetSimulator, &obj));
        }
        FreeLibrary(hd);
        DeleteFileA(ini.c_str());
        const std::string text = ReadFile(log);
        CHECK(text.find("subsystem 'budget': DISABLED by FreeCity4.ini (disable=)") != std::string::npos);
        CHECK(text.find("subsystem 'simulator': enabled (1 class replaced)") != std::string::npos);
        CHECK(text.find("config (game start): ") != std::string::npos);
        CHECK(text.find("FreeCity4.ini found; budget_default=1; set by ini: lot_scan=0; set by env: -") !=
              std::string::npos);
        CHECK(text.find("config (game start): budget switches OFF (1 of 72): lot_scan") != std::string::npos);
        CHECK(text.find("config: WARNING unknown key 'bogus_key' in FreeCity4.ini (ignored)") != std::string::npos);
        if (g_failures == 0) std::printf("plugin ini disable test: all checks passed\n");
        return g_failures == 0 ? 0 : 1;
    }

    if (mode == "--ini-enable" || mode == "--ini-enable-disabled") {
        // The land value subsystem is off by default; `enable = landvalue`
        // registers it (and never the game's grid classes); `disable` wins.
        const bool alsoDisabled = mode == "--ini-enable-disabled";
        SetEnvironmentVariableA("FC4_DISABLE", nullptr);
        SetEnvironmentVariableA("FC4_ENABLE", nullptr);
        WriteTextFile(ini, alsoDisabled ? "[subsystems]\nenable = LandValue\ndisable = landvalue\n"
                                        : "[subsystems]\nenable = LandValue\n");
        HMODULE hd = LoadLibraryA(dll.c_str());
        CHECK(hd != nullptr);
        if (hd == nullptr) return 1;
        auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
        cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
        CHECK(director != nullptr);
        if (director != nullptr) {
            std::vector<uint32_t> classes;
            director->EnumClassObjects(&CollectClass, &classes);
            for (uint32_t c : classes) CHECK(c < 0x49B9E600 || c > 0x49B9E60B);  // simgrid.md §6.3
            void* obj = nullptr;
            if (alsoDisabled) {
                CHECK(classes.size() == 4);
                CHECK(!director->GetClassObject(kCLSID_LandValue, kIID_cISC4LandValueSimulator, &obj));
            } else {
                CHECK(classes.size() == 5 && classes[4] == kCLSID_LandValue);
                CHECK(director->GetClassObject(kCLSID_LandValue, kIID_cISC4LandValueSimulator, &obj));
                auto* lv = static_cast<cISC4LandValueSimulator*>(obj);
                if (lv != nullptr) {
                    // §1: exactly four interfaces, one reference count.
                    void* p = nullptr;
                    CHECK(lv->QueryInterface(0x452294AA, &p) && p != nullptr);
                    if (p) static_cast<cIGZUnknown*>(p)->Release();
                    cIGZSerializable* ser = nullptr;
                    CHECK(lv->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ser)) && ser);
                    if (ser) {
                        CHECK(ser->GetGZCLSID() == kCLSID_LandValue);
                        ser->Release();
                    }
                    CHECK(lv->QueryInterface(GZIID_cIGZUnknown, &p) && p == static_cast<cIGZUnknown*>(lv));
                    if (p) static_cast<cIGZUnknown*>(p)->Release();
                    CHECK(!lv->QueryInterface(0xC9BD939A, &p));
                    CHECK(!lv->QueryInterface(0x6772BD23, &p));
                    // Before Init: no grids, zero values.
                    CHECK(lv->GetIntrinsicLandValueMap() == nullptr && lv->GetTotalLandValueMap() == nullptr &&
                          lv->GetSlopeMap() == nullptr);
                    CHECK(lv->GetLandValue(3, 4) == 0);
                    CHECK(lv->GetAverageLandValue() == 0);
                    CHECK(lv->GetModeAltitude() == 0.0f);
                    // Init without a city or GZCOM: no grids, but a clean lifecycle.
                    CHECK(lv->Init());
                    CHECK(lv->Init());
                    CHECK(lv->GetLandValueType(0, 0) == cISC4LandValueSimulator::LandValueType::Low);
                    CHECK(lv->AddRef() == 2);
                    CHECK(lv->Release() == 1);
                    CHECK(lv->Release() == 0);  // §4.3: Shutdown, destroy
                }
            }
        }
        FreeLibrary(hd);
        DeleteFileA(ini.c_str());
        const std::string text = ReadFile(log);
        if (alsoDisabled) {
            CHECK(text.find("subsystem 'landvalue': DISABLED by FreeCity4.ini (disable=)") != std::string::npos);
            CHECK(text.find("cFC4LandValueSimulator(") == std::string::npos);
        } else {
            CHECK(text.find("subsystem 'landvalue': enabled (1 class replaced) by FreeCity4.ini (enable=)") !=
                  std::string::npos);
            CHECK(text.find("landvalue Init: ok=1") != std::string::npos);
            CHECK(text.find("landvalue Shutdown (0 passes)") != std::string::npos);
            CHECK(text.find("cFC4LandValueSimulator destroyed") != std::string::npos);
        }
        CHECK(text.find("subsystem 'budget': enabled (3 classes replaced)") != std::string::npos);
        CHECK(text.find("config: WARNING unknown key") == std::string::npos);
        if (g_failures == 0) std::printf("plugin enable test (%s): all checks passed\n", mode.c_str());
        return g_failures == 0 ? 0 : 1;
    }

    if (mode == "--ini-enable-weather") {
        // The weather subsystem is off by default; `enable = weather` registers it.
        SetEnvironmentVariableA("FC4_DISABLE", nullptr);
        SetEnvironmentVariableA("FC4_ENABLE", nullptr);
        WriteTextFile(ini, "[subsystems]\nenable = weather\n");
        HMODULE hd = LoadLibraryA(dll.c_str());
        CHECK(hd != nullptr);
        if (hd == nullptr) return 1;
        auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
        cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
        CHECK(director != nullptr);
        if (director != nullptr) {
            std::vector<uint32_t> classes;
            director->EnumClassObjects(&CollectClass, &classes);
            CHECK(classes.size() == 5 && std::count(classes.begin(), classes.end(), kCLSID_Weather) == 1);
            void* obj = nullptr;
            CHECK(director->GetClassObject(kCLSID_Weather, kIID_cISC4WeatherSimulator, &obj));
            auto* w = static_cast<cFC4ISC4WeatherSimulator*>(obj);
            if (w != nullptr) {
                // weather §1: five interfaces.
                void* p = nullptr;
                for (uint32_t iid : {0x452294AAu, 0xE98B2F57u}) {
                    p = nullptr;
                    CHECK(w->QueryInterface(iid, &p) && p != nullptr);
                    if (p) static_cast<cIGZUnknown*>(p)->Release();
                }
                cIGZSerializable* ser = nullptr;
                CHECK(w->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ser)) && ser);
                if (ser) {
                    CHECK(ser->GetGZCLSID() == kCLSID_Weather);
                    ser->Release();
                }
                CHECK(w->QueryInterface(GZIID_cIGZUnknown, &p) && p == static_cast<cIGZUnknown*>(w));
                if (p) static_cast<cIGZUnknown*>(p)->Release();
                CHECK(!w->QueryInterface(0x6772BD23, &p));
                // Constructor values (§3): no grid, the code-default humidity.
                CHECK(w->CellCountX() == 0 && w->GetMoistureValue(10.0f, 10.0f) == 0);
                CHECK(w->GetHumidity(0.0f, 0.0f) == 0.75f);
                // Init without a city or GZCOM: an empty city, a clean lifecycle.
                CHECK(w->Init());
                CHECK(w->Init());
                CHECK(w->CellWidthX() == 16.0f && w->CellCountZ() == 0);
                CHECK(w->GetCurrentSeasonMonth() == 1 && w->GetCurrentSeason() == 0);
                float x = 0, z = 0;
                CHECK(w->CellToPosition(2, 3, x, z) == 48.0f && x == 32.0f && z == 48.0f);
                int32_t a = 0, b = 0;
                w->CellToStandardCityCell(3, 7, a, b);
                CHECK(a == 3 && b == 3);  // [quirk]
                CHECK(w->StandardCityCellToCell(5, 6, a, b) == 6 && a == 5 && b == 6);
                cS3DVector2 wind(0.0f, 0.0f);
                CHECK(w->GetAmbientWind(wind) == 3.0f && wind.fX == 3.0f && wind.fY == 0.0f);
                FC4ByteGridVector* grid = nullptr;
                w->GetMoistureValueMapPtr(grid);
                CHECK(grid != nullptr);
                if (grid != nullptr) {
                    void* const* v = reinterpret_cast<void* const*>(grid);
                    CHECK(v[0] == v[1]);  // no rows in an empty city
                }
                CHECK(w->Release() == 0);  // Shutdown, destroy
            }
        }
        FreeLibrary(hd);
        DeleteFileA(ini.c_str());
        const std::string text = ReadFile(log);
        CHECK(text.find("subsystem 'weather': enabled (1 class replaced) by FreeCity4.ini (enable=)") !=
              std::string::npos);
        CHECK(text.find("subsystem 'landvalue': off by default") != std::string::npos);
        CHECK(text.find("weather Init: ok=1") != std::string::npos);
        CHECK(text.find("weather Shutdown") != std::string::npos);
        CHECK(text.find("cFC4WeatherSimulator destroyed") != std::string::npos);
        CHECK(text.find("config: WARNING unknown key") == std::string::npos);
        if (g_failures == 0) std::printf("plugin enable test (%s): all checks passed\n", mode.c_str());
        return g_failures == 0 ? 0 : 1;
    }

    if (mode == "--ini-enable-ordinance") {
        // The ordinance subsystem is off by default; `enable = ordinance`
        // registers the simulator and the 21 ordinance classes.
        SetEnvironmentVariableA("FC4_DISABLE", nullptr);
        SetEnvironmentVariableA("FC4_ENABLE", nullptr);
        WriteTextFile(ini, "[subsystems]\nenable = ordinance\n");
        HMODULE hd = LoadLibraryA(dll.c_str());
        CHECK(hd != nullptr);
        if (hd == nullptr) return 1;
        auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
        cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
        CHECK(director != nullptr);
        if (director != nullptr) {
            std::vector<uint32_t> classes;
            director->EnumClassObjects(&CollectClass, &classes);
            CHECK(classes.size() == 26);
            CHECK(std::count(classes.begin(), classes.end(), kCLSID_OrdinanceSim) == 1);
            CHECK(std::count(classes.begin(), classes.end(), kCLSID_FreeClinics) == 1);
            CHECK(std::count(classes.begin(), classes.end(), 0x8BD4FC74u) == 1);
            void* obj = nullptr;
            CHECK(director->GetClassObject(kCLSID_OrdinanceSim, kIID_cISC4OrdinanceSimulator, &obj));
            auto* sim = static_cast<cISC4OrdinanceSimulator*>(obj);
            if (sim != nullptr) {
                // ordinance §1: four interfaces.
                void* p = nullptr;
                CHECK(sim->QueryInterface(0x452294AAu, &p) && p != nullptr);
                auto* target = static_cast<cIGZMessageTarget2*>(p);
                cIGZSerializable* ser = nullptr;
                CHECK(sim->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ser)) && ser);
                if (ser) {
                    CHECK(ser->GetGZCLSID() == kCLSID_OrdinanceSim);
                    ser->Release();
                }
                void* u = nullptr;
                CHECK(sim->QueryInterface(GZIID_cIGZUnknown, &u) && u == static_cast<cIGZUnknown*>(sim));
                if (u) static_cast<cIGZUnknown*>(u)->Release();
                CHECK(!sim->QueryInterface(GZIID_cISC4Ordinance, &u));
                // Init without a city or GZCOM: no exemplars, so no standard ordinances.
                CHECK(sim->Init());
                CHECK(sim->Init());
                uint32_t n = 0;
                CHECK(sim->GetOrdinanceIDArray(nullptr, n) == 0 && n == 0);

                // One of our ordinances, created through GZCOM (as the class store does).
                void* o = nullptr;
                CHECK(director->GetClassObject(kCLSID_FreeClinics, GZIID_cISC4Ordinance, &o));
                cFC4ISC4Ordinance* ord = fc4::AsGame(static_cast<cISC4Ordinance*>(o));
                if (ord != nullptr) {
                    cIGZSerializable* os = nullptr;
                    CHECK(ord->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&os)) && os);
                    if (os) {
                        CHECK(os->GetGZCLSID() == kCLSID_FreeClinics);
                        os->Release();
                    }
                    CHECK(!ord->QueryInterface(0x8BD4FC74u, &u));  // [choice]
                    // No exemplar: the zero ordinance (§3.1).
                    CHECK(ord->GetID() == 0 && !ord->IsAvailable() && !ord->IsOn());
                    CHECK(ord->GetChanceAvailability() == 0 && ord->GetEnactmentIncome() == 0);
                    CHECK(ord->GetName() != nullptr && ord->GetName()->Strlen() == 0);
                    CHECK(ord->GetMiscProperties() == nullptr);
                    CHECK(sim->AddOrdinance(*fc4::AsSdk(ord)));
                    CHECK(sim->GetOrdinanceIDArray(nullptr, n) == 1);
                    CHECK(sim->GetOrdinanceByID(0) == fc4::AsSdk(ord));
                    CHECK(ord->AddRef() == 3);  // ours, the map's, this one
                    CHECK(ord->Release() == 2);
                }

                // A foreign (DLL plugin) ordinance, available when added (§4.2).
                FakeOrdinance fake;
                CHECK(sim->AddOrdinance(*fc4::AsSdk(&fake)));
                CHECK(fake.refs == 1);
                CHECK(sim->IsOrdinanceAvailable(0x12345678) && sim->IsOrdinanceAvailableButOff(0x12345678));
                CHECK(sim->SetOrdinanceOn(0x12345678, true) && fake.on && sim->IsOrdinanceOn(0x12345678));
                CHECK(sim->GetOrdinanceOnCount() == 1 && sim->GetOrdinanceAvailableCount() == 1);
                uint32_t ids[4] = {};
                n = 4;
                CHECK(sim->GetOrdinanceIDArray(ids, n) == 2 && n == 2 && ids[0] == 0 && ids[1] == 0x12345678);
                const cRZBaseString name("FAKE ordinance");
                CHECK(sim->GetOrdinanceByName(name) == fc4::AsSdk(&fake));
                CHECK(sim->GetOrdinanceIDByName(name) == 0x12345678);
                CHECK(sim->GetCumulativeFactor(0x28ED0380, 0) == 1.0f && sim->GetCumulativeOffset(0xAA5B8407, 0) == 0);
                // Month end (§5): the enactment (-10) and the fake's -7.
                if (target != nullptr) {
                    TypeMessage month(0x66956816);
                    CHECK(target->DoMessage(&month));
                    target->Release();
                }
                CHECK(sim->GetOrdinanceMonthlyExpense() == 17 && sim->GetOrdinanceMonthlyIncome() == 0);
                CHECK(sim->RemoveOrdinance(*fc4::AsSdk(&fake)));
                CHECK(fake.refs == 0 && !fake.on && !fake.available);
                CHECK(sim->Release() == 0);  // Shutdown, destroy: the map lets go
                if (ord != nullptr) CHECK(ord->Release() == 0);
            }
        }
        FreeLibrary(hd);
        DeleteFileA(ini.c_str());
        const std::string text = ReadFile(log);
        CHECK(text.find("subsystem 'ordinance': enabled (22 classes replaced) by FreeCity4.ini (enable=)") !=
              std::string::npos);
        CHECK(text.find("subsystem 'weather': off by default") != std::string::npos);
        CHECK(text.find("ordinance Init: ok=1") != std::string::npos);
        CHECK(text.find("ordinance month: 2 ordinances (1 available, 1 on)") != std::string::npos);
        CHECK(text.find("ordinance Shutdown") != std::string::npos);
        CHECK(text.find("cFC4OrdinanceSimulator destroyed") != std::string::npos);
        CHECK(text.find("config: WARNING unknown key") == std::string::npos);
        if (g_failures == 0) std::printf("plugin enable test (%s): all checks passed\n", mode.c_str());
        return g_failures == 0 ? 0 : 1;
    }

    if (mode == "--ini-enable-demand") {
        // The demand subsystem is off by default; `enable = demand` registers
        // the simulator and the node class.
        SetEnvironmentVariableA("FC4_DISABLE", nullptr);
        SetEnvironmentVariableA("FC4_ENABLE", nullptr);
        WriteTextFile(ini, "[subsystems]\nenable = demand\n");
        HMODULE hd = LoadLibraryA(dll.c_str());
        CHECK(hd != nullptr);
        if (hd == nullptr) return 1;
        auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
        cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
        CHECK(director != nullptr);
        if (director != nullptr) {
            std::vector<uint32_t> classes;
            director->EnumClassObjects(&CollectClass, &classes);
            CHECK(classes.size() == 6);
            CHECK(std::count(classes.begin(), classes.end(), kCLSID_DemandSim) == 1);
            CHECK(std::count(classes.begin(), classes.end(), kCLSID_Demand) == 1);
            void* obj = nullptr;
            CHECK(director->GetClassObject(kCLSID_DemandSim, kIID_cISC4DemandSimulator, &obj));
            auto* sim = static_cast<cFC4ISC4DemandSimulator*>(obj);
            if (sim != nullptr) {
                // demand §1: four interfaces (not the fourth message-target IID [choice]).
                void* p = nullptr;
                CHECK(sim->QueryInterface(0x452294AAu, &p) && p != nullptr);
                auto* target = static_cast<cIGZMessageTarget2*>(p);
                cIGZSerializable* ser = nullptr;
                CHECK(sim->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ser)) && ser);
                if (ser) {
                    CHECK(ser->GetGZCLSID() == kCLSID_DemandSim);
                    ser->Release();
                }
                void* u = nullptr;
                CHECK(sim->QueryInterface(GZIID_cIGZUnknown, &u) && u == static_cast<cIGZUnknown*>(sim));
                if (u) static_cast<cIGZUnknown*>(u)->Release();
                CHECK(!sim->QueryInterface(0x452294ABu, &u));
                CHECK(!sim->QueryInterface(kIID_cISC4Demand, &u));
                CHECK(sim->GetSimulatorType() == 0x279681D7u);
                // Init without a city or GZCOM: no demand exemplars, so no nodes.
                CHECK(sim->Init());
                CHECK(sim->Init());
                CHECK(sim->GetDemand(0x1010, 0x20000) == nullptr);
                CHECK(sim->GetJobsBySensus(0x3121) == 0);
                CHECK(sim->GetNeutralTaxRate() == 0.0f);
                float jobs[3] = {1.0f, 1.0f, 1.0f};
                CHECK(sim->CalculateJobsPerUnitOfDemand(jobs, 0x3121, 0) == 0.0f);
                CHECK(jobs[0] == 0.0f && jobs[1] == 0.0f && jobs[2] == 0.0f);
                SC4Percentage full{1.0f}, half{0.5f};
                sim->UpdateOccupantEffects(0xBEEF, full, half);  // no effects: nothing happens
                std::map<uint32_t, int32_t> summary;
                sim->GetLocalPopulationSummary(summary);
                CHECK(summary.empty());  // [choice] the caller's map is not written

                // A node, created through GZCOM (as the class store does).
                void* o = nullptr;
                CHECK(director->GetClassObject(kCLSID_Demand, kIID_cISC4Demand, &o));
                cFC4ISC4Demand* node = fc4::AsGame(static_cast<cISC4Demand*>(o));
                if (node != nullptr) {
                    cIGZSerializable* ns = nullptr;
                    CHECK(node->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ns)) && ns);
                    if (ns) {
                        CHECK(ns->GetGZCLSID() == kCLSID_Demand);
                        ns->Release();
                    }
                    CHECK(node->Init());
                    // §3.1 defaults: activeMax = +8.507059e37 (0x7E7FFFFF), min its negative.
                    const float dmax = node->QueryActiveDemandMax();
                    uint32_t bits = 0;
                    std::memcpy(&bits, &dmax, sizeof(bits));
                    CHECK(bits == 0x7E7FFFFFu && node->QueryActiveDemandMin() == -dmax);
                    CHECK(node->SetId(0x1010) && node->GetId() == 0x1010);
                    CHECK(node->SetRegionUse(1) && node->GetRegionUse() == 1);
                    // §16 test 2: min/max ±6000, demand 1000, supply 400 + 10 pending.
                    CHECK(node->SetActiveDemandMin(-6000.0f) && node->SetActiveDemandMax(6000.0f));
                    CHECK(node->SetDemandValue(1000.0f) && node->SetSupplyValue(400.0f));
                    CHECK(node->AddToSupplyValue(10.0f) == 1.0f);  // [choice] 1.0 in ST0
                    CHECK(node->QuerySupplyValue() == 400.0f && node->QueryNewSupply() == 10.0f);
                    CHECK(node->EndOfCycle());
                    CHECK(!node->EndOfCycle());
                    CHECK(node->QuerySupplyValue() == 410.0f && node->QueryActiveDemandValue() == 590.0f);
                    CHECK(node->SetEconomyModifier(0.03f) && node->QueryEconomyModifier() == 0.03f);
                    CHECK(!node->EndOfCycle() && std::fabs(node->QueryActiveDemandValue() - 620.0f) < 0.01f);
                    const SC4Percentage cap{0.95f};
                    CHECK(node->SetDemandCap(cap) && node->GetDemandCap() != nullptr &&
                          node->GetDemandCap()->percentage == 0.95f);
                    node->EndOfCycle();
                    CHECK(std::fabs(node->QueryActiveDemandValue() - 300.0f) < 0.01f);
                    // §16 test 3: the minimum clamps to the maximum.
                    CHECK(node->SetActiveDemandMin(7000.0f) && node->QueryActiveDemandMin() == 6000.0f);
                    CHECK(!node->DebugIsValueLocked());
                    CHECK(node->Shutdown());
                    CHECK(node->Release() == 0);
                }
                // A month (§10.2): relief and export with no city; logs the state.
                if (target != nullptr) {
                    TypeMessage month(0x66956816);
                    CHECK(target->DoMessage(&month));
                    target->Release();
                }
                CHECK(sim->Release() == 0);  // Shutdown, destroy
            }
        }
        FreeLibrary(hd);
        DeleteFileA(ini.c_str());
        const std::string text = ReadFile(log);
        CHECK(text.find("subsystem 'demand': enabled (2 classes replaced) by FreeCity4.ini (enable=)") !=
              std::string::npos);
        CHECK(text.find("subsystem 'ordinance': off by default") != std::string::npos);
        CHECK(text.find("demand Init: ok=1") != std::string::npos);
        CHECK(text.find("demand month: 0 nodes") != std::string::npos);
        CHECK(text.find("GetLocalPopulationSummary: 0 entries not copied") != std::string::npos);
        CHECK(text.find("demand Shutdown") != std::string::npos);
        CHECK(text.find("cFC4DemandSimulator destroyed") != std::string::npos);
        CHECK(text.find("config: WARNING unknown key") == std::string::npos);
        if (g_failures == 0) std::printf("plugin enable test (%s): all checks passed\n", mode.c_str());
        return g_failures == 0 ? 0 : 1;
    }

    if (mode == "--ini-enable-plumbing") {
        // The plumbing subsystem is off by default; `enable = plumbing` registers
        // the water simulator (plumbing.md §1).
        SetEnvironmentVariableA("FC4_DISABLE", nullptr);
        SetEnvironmentVariableA("FC4_ENABLE", nullptr);
        WriteTextFile(ini, "[subsystems]\nenable = plumbing\n");
        HMODULE hd = LoadLibraryA(dll.c_str());
        CHECK(hd != nullptr);
        if (hd == nullptr) return 1;
        auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
        cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
        CHECK(director != nullptr);
        if (director != nullptr) {
            std::vector<uint32_t> classes;
            director->EnumClassObjects(&CollectClass, &classes);
            // simulator (1) + budget (3) + plumbing (1); never the game's grid
            // classes (simgrid.md §6.3).
            CHECK(classes.size() == 5);
            for (uint32_t c : classes) CHECK(c < 0x49B9E600 || c > 0x49B9E60B);
            CHECK(std::count(classes.begin(), classes.end(), kCLSID_Plumbing) == 1);
            void* obj = nullptr;
            CHECK(director->GetClassObject(kCLSID_Plumbing, kIID_cISC4PlumbingSimulator, &obj));
            auto* pl = static_cast<cFC4ISC4PlumbingSimulator*>(obj);
            if (pl != nullptr) {
                // §1: exactly four IIDs, one reference count, and the out pointer
                // is left untouched for any other IID.
                void* p = nullptr;
                CHECK(pl->QueryInterface(0x452294AAu, &p) && p != nullptr);
                if (p) static_cast<cIGZUnknown*>(p)->Release();
                cIGZSerializable* ser = nullptr;
                CHECK(pl->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ser)) && ser);
                if (ser) {
                    CHECK(ser->GetGZCLSID() == kCLSID_Plumbing);
                    ser->Release();
                }
                // cIGZUnknown is answered with the primary interface (§1).
                void* u = nullptr;
                CHECK(pl->QueryInterface(GZIID_cIGZUnknown, &u) && u == static_cast<cIGZUnknown*>(pl));
                if (u) static_cast<cIGZUnknown*>(u)->Release();
                void* untouched = reinterpret_cast<void*>(0xDEADBEEF);
                CHECK(!pl->QueryInterface(0x452294ABu, &untouched));
                CHECK(untouched == reinterpret_cast<void*>(0xDEADBEEF));
                CHECK(!pl->QueryInterface(0xE98B2F57u, &untouched));  // cIGZMessageTarget: not answered
                // Constructor values (§3): idle state 9, no city, no grids, so
                // every query reads the neutral value and nothing crashes.
                CHECK(pl->GetNumWaterSources() == 0);
                CHECK(pl->GetNumPollutedWaterSources() == 0);
                CHECK(pl->GetWaterProductionCapacity() == 0);
                CHECK(pl->GetWaterConsumed() == 0);
                CHECK(pl->GetWaterImported() == 0);
                CHECK(pl->GetWaterExported() == 0);
                CHECK(pl->GetWaterDemand() == 0);
                CHECK(pl->GetNumPipes() == 0 && pl->GetNumDistressedPipes() == 0 && pl->GetNumBurstPipes() == 0);
                CHECK(pl->GetNumWateredBuildings() == 0 && pl->GetNumUnwateredBuildings() == 0);
                CHECK(pl->GetWateredGrid() == nullptr);
                CHECK(pl->GetSupplyValue(3, 4) == 0);
                CHECK(!pl->IsCellWatered(3, 4));
                CHECK(!pl->AreWateredBuildingsInCell(3, 4));
                CHECK(pl->GetWaterProduction(nullptr) == 0);
                SC4Percentage pct{0.5f};
                CHECK(!pl->GetWaterProducerConditionPercent(nullptr, pct));
                CHECK(pct.percentage == 0.5f);  // untouched
                int64_t cost = 7;
                CHECK(!pl->GetOccupantMonthlyUpkeepCost(nullptr, cost));
                CHECK(cost == 7);  // untouched
                // Slot 24: a null producer is not a producer record, so AL = 0
                // and no output is written (§10.1).
                uint32_t curCap = 99, maxCap = 99, usedCap = 99;
                SC4Percentage eff{0.5f};
                bool polluted = true;
                CHECK(!pl->GetWaterProducerInfo(nullptr, &curCap, &maxCap, &eff, &polluted, &usedCap));
                CHECK(curCap == 99 && maxCap == 99 && usedCap == 99 && polluted && eff.percentage == 0.5f);
                cISC4SimGrid<uint8_t>* g = reinterpret_cast<cISC4SimGrid<uint8_t>*>(0x1);
                CHECK(pl->GetSupplyValueMapPtr(g) && g == nullptr);
                // Slot 28 (GetWaterProducers) is not exercised here: it fills an
                // ilist whose root node comes from the framework allocator
                // service, which is not running in this harness.
                // Init without a city or GZCOM: an empty city, a clean lifecycle.
                CHECK(pl->Init());
                CHECK(pl->Init());
                CHECK(pl->Shutdown());
                CHECK(pl->Release() == 0);
            }
        }
        FreeLibrary(hd);
        DeleteFileA(ini.c_str());
        const std::string text = ReadFile(log);
        CHECK(text.find("subsystem 'plumbing': enabled (1 class replaced) by FreeCity4.ini (enable=)") !=
              std::string::npos);
        CHECK(text.find("subsystem 'power': off by default") != std::string::npos);
        CHECK(text.find("cFC4PlumbingSimulator(") != std::string::npos);
        CHECK(text.find("plumbing Init:") != std::string::npos);
        CHECK(text.find("plumbing Shutdown") != std::string::npos);
        CHECK(text.find("cFC4PlumbingSimulator destroyed") != std::string::npos);
        CHECK(text.find("config: WARNING unknown key") == std::string::npos);
        if (g_failures == 0) std::printf("plugin enable test (%s): all checks passed\n", mode.c_str());
        return g_failures == 0 ? 0 : 1;
    }

    if (mode == "--ini-enable-pollution") {
        // The pollution subsystem is off by default; `enable = pollution`
        // registers the pollution simulator (pollution.md §1).
        SetEnvironmentVariableA("FC4_DISABLE", nullptr);
        SetEnvironmentVariableA("FC4_ENABLE", nullptr);
        WriteTextFile(ini, "[subsystems]\nenable = pollution\npollution_shadow = 1\n");
        HMODULE hd = LoadLibraryA(dll.c_str());
        CHECK(hd != nullptr);
        if (hd == nullptr) return 1;
        auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
        cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
        CHECK(director != nullptr);
        if (director != nullptr) {
            std::vector<uint32_t> classes;
            director->EnumClassObjects(&CollectClass, &classes);
            // simulator (1) + budget (3) + pollution (1); never the game's grid
            // classes (simgrid.md §6.3).
            CHECK(classes.size() == 5);
            for (uint32_t c : classes) CHECK(c < 0x49B9E600 || c > 0x49B9E60B);
            CHECK(std::count(classes.begin(), classes.end(), kCLSID_Pollution) == 1);
            void* obj = nullptr;
            CHECK(director->GetClassObject(kCLSID_Pollution, kIID_cISC4PollutionSimulator, &obj));
            auto* po = static_cast<cFC4ISC4PollutionSimulator*>(obj);
            if (po != nullptr) {
                // §1: the four interfaces and cIGZUnknown (the primary one); the
                // out pointer is left untouched for any other IID.
                void* p = nullptr;
                CHECK(po->QueryInterface(0x452294AAu, &p) && p != nullptr);
                if (p) static_cast<cIGZUnknown*>(p)->Release();
                cIGZSerializable* ser = nullptr;
                CHECK(po->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ser)) && ser);
                if (ser) {
                    CHECK(ser->GetGZCLSID() == kCLSID_Pollution);
                    ser->Release();
                }
                cFC4ISC4PropRequester* req = nullptr;
                CHECK(po->QueryInterface(kIID_cISC4PropRequester, reinterpret_cast<void**>(&req)) && req);
                if (req) {
                    // §8.4: the satisfaction id, the standard selector.
                    CHECK(req->GetRequesterSatisfactionId() == 0xE9A4300Bu);
                    CHECK(!req->UsesCustomPropSelector());
                    const SC4Rect<int32_t> cells(0, 0, 3, 3);
                    CHECK(req->GetRequestedShare(cells) == 0);  // no city, no tracts
                    req->Release();
                }
                void* u = nullptr;
                CHECK(po->QueryInterface(GZIID_cIGZUnknown, &u) && u == static_cast<cIGZUnknown*>(po));
                if (u) static_cast<cIGZUnknown*>(u)->Release();
                void* untouched = reinterpret_cast<void*>(0xDEADBEEF);
                CHECK(!po->QueryInterface(0x452294ABu, &untouched));
                CHECK(untouched == reinterpret_cast<void*>(0xDEADBEEF));
                CHECK(!po->QueryInterface(0xE98B2F57u, &untouched));  // cIGZMessageTarget: not answered
                // No city, no grids: every query reads the neutral value and
                // nothing crashes.
                for (uint32_t kind = 0; kind < 5; ++kind) CHECK(po->GetPollutionGrid(kind) == nullptr);
                CHECK(po->GetPollutionValue(0, 3, 4) == 0);
                int32_t v = 7;
                CHECK(po->GetAirValue(3, 4, v) && v == 0);
                v = 7;
                CHECK(po->GetWaterValue(3, 4, v) && v == 0);
                v = 7;
                CHECK(po->GetGarbageValue(3, 4, v) && v == 0);
                CHECK(po->GetAverageAirValue() == 0 && po->GetAverageWaterValue() == 0);
                CHECK(!po->IsAirPolluted(3, 4) && !po->IsWaterPolluted(3, 4));
                CHECK(!po->IsGarbageAccumulated(3, 4) && !po->IsRadioactive(3, 4));
                CHECK(!po->IsActiveLandfill(3, 4));
                CHECK(po->SetAirValue(3, 4, 10));  // no grid: nothing written, still true (§8.1)
                CHECK(po->GetPollutionValue(0, 3, 4) == 0);
                CHECK(po->GetIncineratorCount() == 0 && po->GetWasteToEnergyBuildingCount() == 0);
                CHECK(po->GetRecyclingCenterCount() == 0 && po->GetWaterTreatmentPlantCount() == 0);
                CHECK(po->GetGarbageProducedLastMonth() == 0 && po->GetGarbageLandfilledLastMonth() == 0);
                CHECK(po->GetBuildingGarbageCapacity(nullptr) == 0);
                SC4Percentage* effect = po->GetRecyclingCenterEffect();  // slot 33: an address
                CHECK(effect != nullptr);
                CHECK(po->GetRecyclingCenterEffect() == effect);
                // Slot 46 (GetGarbageBuildings) is not exercised here: it fills an
                // ilist whose root node comes from the framework allocator
                // service, which is not running in this harness.
                // Init without a city or GZCOM: an empty city, a clean lifecycle.
                CHECK(po->Init());
                CHECK(po->Init());
                CHECK(po->Shutdown());
                CHECK(po->Release() == 0);
            }
        }
        FreeLibrary(hd);
        DeleteFileA(ini.c_str());
        const std::string text = ReadFile(log);
        CHECK(text.find("subsystem 'pollution': enabled (1 class replaced) by FreeCity4.ini (enable=)") !=
              std::string::npos);
        CHECK(text.find("subsystem 'plumbing': off by default") != std::string::npos);
        CHECK(text.find("cFC4PollutionSimulator(") != std::string::npos);
        CHECK(text.find("pollution Init: init=") != std::string::npos);
        CHECK(text.find("pollution Shutdown (0 days, 0 months)") != std::string::npos);
        CHECK(text.find("cFC4PollutionSimulator destroyed") != std::string::npos);
        // The shadow needs the ORIGINAL pollution simulator: requested, inactive.
        CHECK(text.find("pollution shadow: requested (pollution_shadow from FreeCity4.ini) but INACTIVE") !=
              std::string::npos);
        CHECK(text.find("config: WARNING unknown key") == std::string::npos);
        if (g_failures == 0) std::printf("plugin enable test (%s): all checks passed\n", mode.c_str());
        return g_failures == 0 ? 0 : 1;
    }

    if (mode == "--disabled") {
        // FC4_DISABLE switches subsystems off: nothing is registered or created.
        SetEnvironmentVariableA("FC4_DISABLE", "Budget, Simulator");
        HMODULE hd = LoadLibraryA(dll.c_str());
        CHECK(hd != nullptr);
        if (hd == nullptr) return 1;
        auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
        cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
        CHECK(director != nullptr);
        if (director != nullptr) {
            std::vector<uint32_t> classes;
            director->EnumClassObjects(&CollectClass, &classes);
            CHECK(classes.empty());
            void* obj = nullptr;
            CHECK(!director->GetClassObject(kCLSID_Simulator, kIID_cISC4Simulator, &obj));
            CHECK(!director->GetClassObject(kCLSID_Budget, kIID_cISC4BudgetSimulator, &obj));
        }
        // The observer still runs (passthrough): the digest, read from the city's
        // simulators (here a fake city without a budget simulator).
        auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(hd, "FC4TestSeam"));
        CHECK(seam != nullptr);
        if (seam != nullptr) {
            static FakeWorld world;
            InitFakeWorld(world);
            seam(kFC4SeamObserverCity, &world.city, nullptr);
            seam(kFC4SeamObserverMessage, reinterpret_cast<void*>(static_cast<uintptr_t>(kMsgPostCityInit)), nullptr);
            seam(kFC4SeamObserverTick, nullptr, nullptr);  // the "after load" snapshot
            seam(kFC4SeamObserverMessage, reinterpret_cast<void*>(static_cast<uintptr_t>(kMsgSimNewMonth)),
                 reinterpret_cast<void*>(static_cast<intptr_t>(2451576)));
            seam(kFC4SeamObserverCity, nullptr, nullptr);
            CHECK(world.demandCalls == 28);  // digest and observe: 12 groups + 2 census nodes each
        }
        FreeLibrary(hd);
        const std::string text = ReadFile(log);
        CHECK(text.find("observer: PostCityInit (passthrough)") != std::string::npos);
        CHECK(text.find("DIGEST julian=2451576 (2000-02-01) funds=- population R/C/I=1234/480/160 demand "
                        "R$/R$$/R$$$/Cs$/Cs$$/Cs$$$/Co$$/Co$$$/I-R/I-D/I-M/I-HT=250.5/-/-/-12.0/-/-/40.0/-/7.5/-/-/0.0 "
                        "budget income/expense=-/-") != std::string::npos);
        CHECK(text.find("observe snapshot (after load): date=- julian=- funds=- fundings=-") != std::string::npos);
        CHECK(text.find("observe month 2000-02: date=2000-02-01 julian=2451576 funds=- income=- expense=- "
                        "tax[0..11]=- dept_expense=- demand R$/R$$/R$$$/Cs$/Cs$$/Cs$$$/Co$$/Co$$$/I-R/I-D/I-M/I-HT="
                        "250.5/-/-/-12.0/-/-/40.0/-/7.5/-/-/0.0 population R/C/I=1234/480/160") != std::string::npos);
        CHECK(text.find("subsystem 'simulator': DISABLED by FC4_DISABLE") != std::string::npos);
        CHECK(text.find("subsystem 'budget': DISABLED by FC4_DISABLE") != std::string::npos);
        CHECK(text.find("object created") == std::string::npos);
        if (g_failures == 0) std::printf("plugin disable test: all checks passed\n");
        return g_failures == 0 ? 0 : 1;
    }
    SetEnvironmentVariableA("FC4_DIAG_SELFTEST", "1");
    SetEnvironmentVariableA("FC4_CHECKED_SELFTEST", "1");
    SetEnvironmentVariableA("FC4_STATUS_EVERY", "5");
    SetEnvironmentVariableA("FC4_HEARTBEAT_MS", "2000");

    SetGamePrecision();  // as the game's PreFrameWorkInit does
    HMODULE h = LoadLibraryA(dll.c_str());
    CHECK(h != nullptr);
    if (h == nullptr) return 1;
    CheckCw("LoadLibrary (CRT, Rust and DllMain initialisation)");

    using GetDirectorFn = cIGZCOMDirector* (*)();
    auto getDirector = reinterpret_cast<GetDirectorFn>(GetProcAddress(h, "GZDllGetGZCOMDirector"));
    CHECK(getDirector != nullptr);
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    if (director != nullptr) {
        CHECK(director->GetDirectorID() == kDirectorID);

        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(classes.size() == 4);
        CHECK(classes.size() == 4 && classes[0] == kCLSID_Simulator && classes[1] == kCLSID_Budget &&
              classes[2] == kCLSID_Department && classes[3] == kCLSID_LineItem);

        // Another process (an editor, a tail) holding the log open must not stop the logging.
        HANDLE reader = CreateFileA(log.c_str(), GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                                    nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
        CHECK(reader != INVALID_HANDLE_VALUE);

        // Our class with an interface it does not implement: refused, object destroyed.
        void* nope = reinterpret_cast<void*>(1);
        CHECK(!director->GetClassObject(kCLSID_Simulator, 0x0BADF00D, &nope));
        CHECK(nope == nullptr);

        cISC4Simulator* sim = nullptr;
        CHECK(director->GetClassObject(kCLSID_Simulator, kIID_cISC4Simulator, reinterpret_cast<void**>(&sim)));
        CHECK(sim != nullptr);
        CHECK(!director->GetClassObject(0x12345678, kIID_cISC4Simulator, reinterpret_cast<void**>(&sim)) ||
              sim != nullptr);

        if (sim != nullptr) {
            CHECK(sim->Init());
            CHECK(sim->IsHiddenPaused());
            CHECK(sim->GetSimDateNumber() == 2451545);
            CHECK(sim->SetSimSpeed(2));
            CHECK(sim->HiddenResume());
            CHECK(!sim->IsAnyPaused());

            SlowAgent agent;
            CHECK(sim->AddAgent(&agent, cISC4Simulator::AgentTypeSimNewDay, cRZBaseString("test day agent"),
                                static_cast<cISC4Simulator::eAgentFlags>(3)));
            CHECK(agent.refs == 1);

            cIGZSystemService* service = nullptr;
            CHECK(sim->QueryInterface(GZIID_cIGZSystemService, reinterpret_cast<void**>(&service)));
            if (service != nullptr) {
                CHECK(service->GetServiceID() == 0x46956659);
                CHECK(service->GetServicePriority() == -2000000);
                // Default tuning: 100 ms per day at speed 2. Tick for ~600 ms.
                const DWORD start = GetTickCount();
                while (GetTickCount() - start < 600) service->OnTick(0);
                service->Release();
            }
            const int32_t day = sim->GetSimDateNumber();
            std::printf("simulated %d day(s) in ~600 ms; agent called %u times\n", day - 2451545, agent.calls);
            CHECK(day > 2451545 + 2);
            CHECK(agent.calls > 150);
            CHECK(agent.badMessages == 0);
            CHECK(agent.lastJulian == day);
            // Overloaded methods are only ever called through the game's slots (foundations §16).
            cIGZDate* date = reinterpret_cast<cIGZDate*>(fc4test::CallSlot(sim, 7, {}).eax);  // GetSimDate()
            CHECK(date != nullptr && date->Year() == 2000 && static_cast<int32_t>(date->DayNumber()) == day);
            if (date != nullptr) {
                // cIGZDate slots 33-36 (foundations §17.4): a uint32 day number in
                // EAX, one stack argument, nothing on the x87 stack, and the date
                // itself unchanged.
                using fc4test::CallSlot;
                const uint32_t d = static_cast<uint32_t>(day);
                cIGZDate* other = nullptr;
                CHECK((CallSlot(date, 9, {reinterpret_cast<uint32_t>(&other)}).eax & 0xFF) == 1);  // Clone
                CHECK(other != nullptr);
                if (other != nullptr) {
                    CHECK((CallSlot(other, 4, {100}).eax & 0xFF) == 1);  // Set(uint32 dayNumber)
                    const fc4test::SlotCall m1 = CallSlot(date, 33, {7});  // operator-(int32)
                    const fc4test::SlotCall m2 = CallSlot(date, 34, {reinterpret_cast<uint32_t>(other)});  // operator-(date)
                    const fc4test::SlotCall p1 = CallSlot(date, 35, {7});  // operator+(int32)
                    const fc4test::SlotCall p2 = CallSlot(date, 36, {reinterpret_cast<uint32_t>(other)});  // operator+(date)
                    std::printf("date slots 33-36: %u %u %u %u (day %u)\n", m1.eax, m2.eax, p1.eax, p2.eax, d);
                    CHECK(m1.eax == d - 7 && m2.eax == d - 100 && p1.eax == d + 7 && p2.eax == d + 100);
                    for (const fc4test::SlotCall* c : {&m1, &m2, &p1, &p2}) {
                        CHECK(c->espDelta == 0 && c->x87Pushed == 0);
                    }
                    CHECK(date->DayNumber() == d && other->DayNumber() == 100u);  // neither date modified
                    // Slots 16/17: FirstDayOfMonth(month) / FirstDayOfMonth().
                    const fc4test::SlotCall f16 = CallSlot(date, 16, {3});
                    const fc4test::SlotCall f17 = CallSlot(date, 17, {});
                    CHECK(f16.espDelta == 0 && f17.espDelta == 0);
                    CHECK(f16.eax == 2451545u + 31u + 29u);  // 2000-03-01
                    CHECK(f17.eax == 2451545u);              // the sim date is in January 2000
                    other->Release();
                }
            }
            {
                // Overloads through the game's slots (simulator §5.6), with the
                // game's argument counts: the stack must balance.
                using fc4test::CallSlot;
                uint32_t gy = 0, gm = 0, gd = 0, gdoy = 0, gwd = 0;
                const fc4test::SlotCall s6 = CallSlot(sim, 6, {reinterpret_cast<uint32_t>(&gy), reinterpret_cast<uint32_t>(&gm),
                                                         reinterpret_cast<uint32_t>(&gd), reinterpret_cast<uint32_t>(&gdoy),
                                                         reinterpret_cast<uint32_t>(&gwd)});
                std::printf("slot 6: esp %d, year %u\n", s6.espDelta, gy);
                CHECK(s6.espDelta == 0 && gy == 2000);
                const fc4test::SlotCall s7 = CallSlot(sim, 7, {});
                std::printf("slot 7: esp %d, eax %08X\n", s7.espDelta, s7.eax);
                CHECK(s7.espDelta == 0 && s7.eax != 0);
                SlowAgent other;
                const fc4test::SlotCall s20 = CallSlot(sim, 20, {reinterpret_cast<uint32_t>(static_cast<cIGZMessageTarget2*>(&other))});
                const fc4test::SlotCall s21 = CallSlot(sim, 21, {reinterpret_cast<uint32_t>(static_cast<cIGZMessageTarget2*>(&other)), 4});
                const fc4test::SlotCall s22 = CallSlot(sim, 22, {9});
                std::printf("slots 20/21/22: esp %d %d %d\n", s20.espDelta, s21.espDelta, s22.espDelta);
                CHECK(s20.espDelta == 0 && s21.espDelta == 0 && s22.espDelta == 0);
            }
            {
                // RemoveAgent(agent, level) at slot 21 removes the day registration.
                const fc4test::SlotCall r = fc4test::CallSlot(
                    sim, 21, {reinterpret_cast<uint32_t>(static_cast<cIGZMessageTarget2*>(&agent)), 4});
                CHECK(r.espDelta == 0 && (r.eax & 0xFF) == 1);
            }
            CHECK(agent.refs == 0);
            {
                // RemoveAllAgents() at slot 23.
                const fc4test::SlotCall r = fc4test::CallSlot(sim, 23, {});
                CHECK(r.espDelta == 0);
            }

            uint32_t y = 0, m = 0, d = 0, doy = 0, wd = 0;
            const fc4test::SlotCall r6 = fc4test::CallSlot(
                sim, 6,
                {reinterpret_cast<uint32_t>(&y), reinterpret_cast<uint32_t>(&m), reinterpret_cast<uint32_t>(&d),
                 reinterpret_cast<uint32_t>(&doy), reinterpret_cast<uint32_t>(&wd)});
            CHECK(r6.espDelta == 0);
            CHECK(y == 2000 && m == 1 && doy == static_cast<uint32_t>(day - 2451545 + 1));

            {
                // Watchdog: a day agent that blocks for 6.5 s inside OnTick must
                // produce a stall report (activity, call ring, main-thread sample)
                // and an "over" line; heartbeats run meanwhile (FC4_HEARTBEAT_MS=2000).
                BlockingAgent blocker;
                CHECK(sim->AddAgent(&blocker, cISC4Simulator::AgentTypeSimNewDay, cRZBaseString("blocking test agent"),
                                    static_cast<cISC4Simulator::eAgentFlags>(3)));
                cIGZSystemService* svc = nullptr;
                CHECK(sim->QueryInterface(GZIID_cIGZSystemService, reinterpret_cast<void**>(&svc)));
                if (svc != nullptr) {
                    const DWORD start = GetTickCount();
                    while (blocker.calls == 0 && GetTickCount() - start < 3000) svc->OnTick(0);
                    const DWORD after = GetTickCount();
                    while (GetTickCount() - after < 2500) svc->OnTick(0);  // let the watchdog see the end
                    svc->Release();
                }
                CHECK(blocker.calls >= 1);
                fc4test::CallSlot(sim, 20, {reinterpret_cast<uint32_t>(static_cast<cIGZMessageTarget2*>(&blocker))});
            }

            CHECK(sim->Shutdown());
            CHECK(sim->GetSimDateNumber() == 2451545);
            CHECK(sim->Release() == 0);
        }

        // The budget without a game: no city, no message server, no exemplar
        // (code defaults; difficulty 0 without a city).
        cISC4BudgetSimulator* budget = nullptr;
        CHECK(director->GetClassObject(kCLSID_Budget, kIID_cISC4BudgetSimulator, reinterpret_cast<void**>(&budget)));
        auto seam = reinterpret_cast<FC4TestSeamFn>(GetProcAddress(h, "FC4TestSeam"));
        CHECK(seam != nullptr);
        std::vector<PostedMessage> posts;  // budget §10: every message the budget posts
        if (seam != nullptr) seam(kFC4SeamBudgetPostSink, reinterpret_cast<void*>(&RecordPost), &posts);
        if (budget != nullptr) {
            CHECK(budget->Init());
            CHECK(budget->GetTotalFunds() == 500000);
            CHECK(budget->DepositFunds(100));
            CHECK(!budget->WithdrawFunds(600000));
            CHECK(budget->GetTotalFunds() == 500100);
            CHECK(budget->GetTaxRate(0) == 9.0f);

            cISC4DepartmentBudget* dept = budget->CreateDepartmentBudget(0x28F55A9F, 7);
            CHECK(dept != nullptr);
            if (dept != nullptr) {
                CHECK(dept->GetDepartmentID() == 0x28F55A9F);
                CHECK(fc4test::CallSlot(budget, 28, {0x28F55A9F}).eax == reinterpret_cast<uint32_t>(dept));
                cISC4DepartmentBudget* viaClsid = nullptr;
                CHECK(dept->QueryInterface(kCLSID_Department, reinterpret_cast<void**>(&viaClsid)));
                CHECK(viaClsid == dept);
                if (viaClsid != nullptr) viaClsid->Release();
                cISC4DepartmentBudget* viaIid = nullptr;
                CHECK(dept->QueryInterface(kIID_cISC4DepartmentBudget, reinterpret_cast<void**>(&viaIid)));
                if (viaIid != nullptr) viaIid->Release();

                cISC4LineItem* line = dept->CreateLineItem(1, false);
                CHECK(line != nullptr);
                if (line != nullptr) {
                    cISC4LineItem* viaLineClsid = nullptr;
                    CHECK(line->QueryInterface(kCLSID_LineItem, reinterpret_cast<void**>(&viaLineClsid)));
                    if (viaLineClsid != nullptr) viaLineClsid->Release();
                    CHECK(line->GetID() == 1);
                    CHECK(line->SetFullExpenses(1000));
                    SC4Percentage half{0.5f};
                    CHECK(dept->SetFundingPercentage(half, 0));
                    CHECK(dept->GetFundingPercentage()->percentage == 0.5f);
                    CHECK(line->GetCurrentExpenses() == 500);
                    CHECK(dept->GetLineItem(1) == line);
                }
                CHECK(budget->GetTotalMonthlyExpense() == 500u);  // low 32 bits of the 64-bit value
                {
                    // Overloads through the game's slots (budget "Verified slot order").
                    using fc4test::CallSlot;
                    const fc4test::SlotCall s18 = CallSlot(budget, 18, {0});  // GetTaxIncome(int32)
                    const fc4test::SlotCall s19 = CallSlot(budget, 19, {});   // GetTaxIncome()
                    const fc4test::SlotCall s34 = CallSlot(budget, 34, {0x28F55A9F});  // GetFunding(uint32)
                    const fc4test::SlotCall s35 = CallSlot(budget, 35, {reinterpret_cast<uint32_t>(dept)});
                    CHECK(s34.espDelta == 0 && s35.espDelta == 0 && s34.eax == 0 && s35.eax == 0);
                    std::printf("budget slots 18/19: esp %d %d\n", s18.espDelta, s19.espDelta);
                    CHECK(s18.espDelta == 0 && s19.espDelta == 0);
                    cRZBaseString empty("");
                    const fc4test::SlotCall s27 = CallSlot(budget, 27, {reinterpret_cast<uint32_t>(static_cast<cIGZString*>(&empty))});
                    const fc4test::SlotCall s28 = CallSlot(budget, 28, {0x28F55A9F});
                    std::printf("budget slots 27/28: esp %d %d, %08X %08X (dept %p)\n", s27.espDelta, s28.espDelta, s27.eax,
                                s28.eax, static_cast<void*>(dept));
                    CHECK(s27.espDelta == 0 && s28.espDelta == 0);
                    CHECK(s27.eax == reinterpret_cast<uint32_t>(dept));  // "" is a prefix of every name
                    CHECK(s28.eax == reinterpret_cast<uint32_t>(dept));
                }
                CHECK(budget->IssueBond(10000));
                CHECK(budget->GetTotalBorrowed() == 10000);
                CHECK(budget->GetTotalFunds() == 510100);
                CHECK(budget->GetTotalMonthlyExpense() == 500u + static_cast<uint32_t>(budget->GetMonthlyPaymentForLoan(10000)));
                {
                    // Slot 26, GetLocalFundingPercent, returns a POINTER to a float
                    // in EAX (budget erratum): the game dereferences it at once.
                    FakeHolder holder;
                    holder.Set<uint32_t>(0xEA54D283, 0x28F55A9F);  // department
                    holder.Set<uint32_t>(0xEA54D284, 2);           // line item
                    holder.Set<uint32_t>(0xEA54D285, 7);           // purpose
                    holder.Set<int64_t>(0xEA54D286, 200);          // cost
                    CHECK(dept->CreateLineItem(2, true) != nullptr);
                    CHECK(dept->AddLocallyFundedObject(holder.AsSdk(), 7));
                    CHECK(holder.refs == 2);
                    const uint32_t h = reinterpret_cast<uint32_t>(holder.AsSdk());
                    using fc4test::CallSlot;
                    const fc4test::SlotCall found = CallSlot(dept, 26, {h, 7});
                    const fc4test::SlotCall otherPurpose = CallSlot(dept, 26, {h, 8});
                    const fc4test::SlotCall noHolder = CallSlot(dept, 26, {0, 7});
                    for (const fc4test::SlotCall* c : {&found, &otherPurpose, &noHolder}) {
                        for (int i = 0; i < c->x87Pushed; ++i) fc4test::PopX87();
                    }
                    std::printf("department slot 26: found %08X (%d x87), not found %08X / %08X\n", found.eax,
                                found.x87Pushed, otherPurpose.eax, noHolder.eax);
                    CHECK(found.espDelta == 0 && otherPurpose.espDelta == 0 && noHolder.espDelta == 0);
                    CHECK(found.x87Pushed == 0 && otherPurpose.x87Pushed == 0 && noHolder.x87Pushed == 0);
                    CHECK(found.eax != 0 && otherPurpose.eax != 0 && noHolder.eax != 0);
                    if (found.eax != 0) {
                        // The item's local-funding fraction: property value / 100.
                        const float q = reinterpret_cast<const float*>(found.eax)[0];
                        CHECK(q == holder.Get32f(0xEA54D287) / 100.0f);
                        CHECK(q == 0.5f);  // AddLocallyFundedObject wrote the department's 50 %
                    }
                    if (otherPurpose.eax != 0) CHECK(reinterpret_cast<const float*>(otherPurpose.eax)[0] == 0.0f);
                    CHECK(noHolder.eax == otherPurpose.eax);  // the shared constant 0.0
                    CHECK(found.eax != otherPurpose.eax);
                    CHECK(dept->RemoveLocallyFundedObject(holder.AsSdk(), 7));
                    CHECK(holder.refs == 1);
                    const fc4test::SlotCall gone = CallSlot(dept, 26, {h, 7});
                    CHECK(gone.eax == otherPurpose.eax && gone.espDelta == 0 && gone.x87Pushed == 0);
                }
            }
            {
                // Budget §10.1/§10.2 [compat]: void1 = IGZUnknown = the sender's
                // primary interface pointer; data1 shares void1's slot.
                if (seam != nullptr) seam(kFC4SeamBudgetPostSink, nullptr, nullptr);
                std::printf("budget posted %u message(s)\n", static_cast<unsigned>(posts.size()));
                void* const b = static_cast<void*>(budget);
                void* const d = static_cast<void*>(dept);
                CHECK(CountPosts(posts, 0x0772FAD4) >= 1);  // FundsChanged (DepositFunds)
                CHECK(CountPosts(posts, 0xC772FADA) >= 1);  // BondIssued
                CHECK(CountPosts(posts, 0x08DAC3E7) >= 1);  // FundingPercentageChanged
                CHECK(CountPosts(posts, 0x08DAC3E8) >= 1);  // IdealMonthlyFundingChanged
                for (const PostedMessage& p : posts) {
                    std::printf("  post 0x%08X void1=%p unknown=%p d1=0x%08X d2=%d d3=%d\n", p.type, p.void1, p.unknown,
                                static_cast<unsigned>(p.d1), static_cast<int>(p.d2), static_cast<int>(p.d3));
                    if (p.type == 0x0772FAD4 || p.type == 0xC772FADA || p.type == 0xCA3FBDCE) {
                        CHECK(p.void1 == b && p.unknown == b);
                        CHECK(p.d1 == reinterpret_cast<intptr_t>(b));
                    }
                    if (p.type == 0x08DAC3E7 || p.type == 0x08DAC3E8) {
                        CHECK(p.void1 == d && p.unknown == d);
                        CHECK(p.d1 == reinterpret_cast<intptr_t>(d));
                        CHECK(p.d2 == 0x28F55A9F);
                    }
                    if (p.type == 0x08DAC3E8) CHECK(p.d3 == 0);
                    if (p.type == 0x0A3E7624) {  // BudgetGroupChanged: data only, no sender (§10)
                        CHECK(p.unknown == nullptr && p.d1 == 7);
                    }
                }
                bool sawPercentage = false;  // SetFundingPercentage(half, 0): data3 = the caller's argument
                for (const PostedMessage& p : posts) sawPercentage |= p.type == 0x08DAC3E7 && p.d3 == 0;
                CHECK(sawPercentage);
            }
            CHECK(budget->Shutdown());
            CHECK(fc4test::CallSlot(budget, 28, {0x28F55A9F}).eax == 0);
            CHECK(budget->GetTotalFunds() == 510100);  // kept across Shutdown
            CHECK(budget->Release() == 0);
        }

        // Bisection switches: FreeCity4.ini is re-read at city load (budget
        // Init) and wins over the environment; a switch that is off makes its
        // behaviour a no-op.
        SetEnvironmentVariableA("FC4_BUDGET_CHEATS", "0");    // overridden by the ini
        SetEnvironmentVariableA("FC4_BUDGET_DISASTER", "0");  // not in the ini: applies
        WriteTextFile(ini, "[budget]\r\nholder_refs = 0\r\nproperty_writes=off ; no writes\r\ncheats = 1\r\n");
        cISC4BudgetSimulator* budget2 = nullptr;
        CHECK(director->GetClassObject(kCLSID_Budget, kIID_cISC4BudgetSimulator, reinterpret_cast<void**>(&budget2)));
        if (budget2 != nullptr) {
            CHECK(budget2->Init());
            cISC4DepartmentBudget* dept = budget2->CreateDepartmentBudget(0x28F55A9F, 7);
            CHECK(dept != nullptr);
            if (dept != nullptr) {
                CHECK(dept->CreateLineItem(2, true) != nullptr);
                FakeHolder holder;
                holder.Set<uint32_t>(0xEA54D283, 0x28F55A9F);
                holder.Set<uint32_t>(0xEA54D284, 2);
                holder.Set<uint32_t>(0xEA54D285, 7);
                holder.Set<int64_t>(0xEA54D286, 200);
                CHECK(dept->AddLocallyFundedObject(holder.AsSdk(), 7));
                CHECK(holder.refs == 1);                             // holder_refs off: no reference held
                CHECK(holder.Get32f(0xEA54D287) == -1.0f);           // property_writes off: nothing written
                CHECK(dept->RemoveLocallyFundedObject(holder.AsSdk(), 7));
                CHECK(holder.refs == 1);
            }
            CHECK(budget2->Shutdown());
            CHECK(budget2->Release() == 0);
        }
        DeleteFileA(ini.c_str());
        SetEnvironmentVariableA("FC4_BUDGET_CHEATS", nullptr);
        SetEnvironmentVariableA("FC4_BUDGET_DISASTER", nullptr);

        // The flammability ignition path (§6.8): the layer's manager is
        // borrowed, only the QueryInterface result is released. Run 37
        // released the manager as well, once per IgniteCell and once per
        // SetCause, and freed the game's fire manager (refs would be -1).
        if (seam != nullptr) {
            static FakeFireCity fire;
            InitFakeFireCity(fire);
            CHECK(seam(kFC4SeamFlammabilityFire, &fire.city, nullptr) == 0);
            CHECK(fire.ignites == 1);
            CHECK(fire.causes == 1);
            CHECK(fire.cause == 5);
            CHECK(fire.qis == 2);
            CHECK(fire.refs == 1);
        }

        // The budget shadow next to an "original" budget: the same values
        // give no mismatch; the original counting the not-livable CS$ lot
        // gives one (type 3), attributed to that lot. Any write to the fake
        // (or any call outside its getters) would abort.
        if (seam != nullptr) {
            static FakeOrigBudget orig;
            InitFakeOrigBudget(orig);
            cIGZMessageTarget2* agent = reinterpret_cast<cIGZMessageTarget2*>(&orig.agent);
            CHECK(ShadowMonth(seam, orig, agent, 2000, 1500000) == 0);
            CHECK(orig.deliveries == 1);
            CHECK(ShadowMonth(seam, orig, agent, 2500, 1875000) == 1);
            CHECK(orig.deliveries == 2);
            // Not the budget's agent: left to the caller, nothing delivered.
            CHECK(ShadowMonth(seam, orig, reinterpret_cast<cIGZMessageTarget2*>(&orig.other), 2000, 1500000) == -1);
            CHECK(orig.deliveries == 2);
            CHECK(orig.refs == 1);  // every QueryInterface released
            CheckCw("budget shadow");
        }

        // The demand shadow next to an "original" demand simulator: FreeCity4's
        // propagation over the states read before NewDay gives the states the
        // original leaves (0 mismatches); a skewed total gives 2 (demand and
        // active); a census applied inside the delivery is inferred (0); the
        // modifiers set in a NewDay or a SimIdle are compared with §8.1; a
        // SimIdle that changes nothing compares nothing (-2). Any write to the
        // fake (or any call outside its getters) would abort.
        if (seam != nullptr) {
            static FakeOrigDemand od;
            InitFakeOrigDemand(od);
            cIGZMessageTarget2* agent = reinterpret_cast<cIGZMessageTarget2*>(&od.agent);
            OdPre(od);
            OdPost(od, 0.0f, 0.0f, 0.0f, 0.0f);
            CHECK(DemandShadowOnce(seam, od, agent, kMsgSimNewDay, 1.0f, 0.0f) == 0);
            CHECK(od.deliveries == 1);
            OdPre(od);
            OdPost(od, 0.0f, 0.0f, 0.0f, 0.5f);
            CHECK(DemandShadowOnce(seam, od, agent, kMsgSimNewDay, 1.0f, 0.0f) == 2);
            OdPre(od);
            OdPost(od, 5.0f, 0.0f, 0.0f, 0.0f);  // a census added 5 inside DoIdle(5)
            CHECK(DemandShadowOnce(seam, od, agent, kMsgSimNewDay, 1.0f, 0.0f) == 0);
            OdPre(od);
            OdPost(od, 0.0f, 0.25f, 0.25f, 0.0f);  // census state 0 inside DoIdle(5): §8.1 = 1.25 - 1, 0.25
            CHECK(DemandShadowOnce(seam, od, agent, kMsgSimNewDay, 1.25f, 0.25f) == 0);
            CHECK(od.deliveries == 4);
            // SimIdle: census state 0 sets 0x21010's tax 0.25 / economy 0.06.
            OdPre(od);
            for (int i = 0; i < 3; ++i) od.post[i] = od.nodes[i];
            od.post[2].tax = 0.25f;
            od.post[2].econ = 0.06f;
            CHECK(DemandShadowOnce(seam, od, agent, kMsgSimIdle, 1.25f, 0.06f) == 0);
            OdPre(od);
            od.post[2].tax = 0.3f;  // not what §8.1 gives
            CHECK(DemandShadowOnce(seam, od, agent, kMsgSimIdle, 1.25f, 0.06f) == 1);
            OdPre(od);
            for (int i = 0; i < 3; ++i) od.post[i] = od.nodes[i];
            CHECK(DemandShadowOnce(seam, od, agent, kMsgSimIdle, 1.25f, 0.06f) == -2);
            CHECK(od.deliveries == 7);
            // Not the demand simulator's agent: left to the caller, nothing delivered.
            CHECK(DemandShadowOnce(seam, od, reinterpret_cast<cIGZMessageTarget2*>(&od.other), kMsgSimNewDay, 1.0f,
                                   0.0f) == -1);
            CHECK(od.deliveries == 7);
            CHECK(od.refs == 1);  // every QueryInterface released
            CheckCw("demand shadow");
        }

        // The pollution shadow (op 16) on its neutral paths: no city, a city
        // without a pollution simulator and an agent that is not the
        // original's are left to the caller (-1, nothing delivered); the
        // original's agent gets its delivery exactly once, its record is
        // captured before and after (its own Write), and a record without the
        // grid references is reported INCOMPLETE (-2, nothing compared). The
        // month delivery ends with the month line. Any other call aborts.
        if (seam != nullptr) {
            static FakeOrigPollution op;
            InitFakeOrigPollution(op);
            cIGZMessageTarget2* agent = reinterpret_cast<cIGZMessageTarget2*>(&op.agent);
            CHECK(PollutionShadowOnce(seam, nullptr, agent, kMsgSimNewMonth) == -1);
            CHECK(PollutionShadowOnce(seam, &op.noCity, agent, kMsgSimNewMonth) == -1);
            CHECK(op.deliveries == 0 && op.writes == 0);
            CHECK(PollutionShadowOnce(seam, &op.city, agent, kMsgSimNewMonth) == -2);
            CHECK(op.deliveries == 1);
            CHECK(op.writes == 2);
            CHECK(PollutionShadowOnce(seam, &op.city, agent, kMsgSimNewDay) == -2);
            CHECK(op.deliveries == 2 && op.writes == 4);
            CHECK(PollutionShadowOnce(seam, &op.city, reinterpret_cast<cIGZMessageTarget2*>(&op.other),
                                      kMsgSimNewMonth) == -1);
            CHECK(op.deliveries == 2 && op.writes == 4);
            CHECK(op.refs == 1);  // every QueryInterface released
            CheckCw("pollution shadow");
        }

        // The reference audit (run 42, ForEachExemplarKey): freeing a
        // temporary the game created for us (acquired, then
        // FC4_RELEASE_CREATED) is not a warning; the same last-reference
        // release through FC4_RELEASE still is (a release of a reference the
        // audited sites never acquired still warns as an over-release; not
        // exercised here: this log must stay free of over-releases).
        if (seam != nullptr) {
            static FakeVtable tempVt;
            tempVt.slots[0] = reinterpret_cast<void*>(&TempQi);
            tempVt.slots[2] = reinterpret_cast<void*>(&TempRelease);
            static FakeObject temps[2] = {{tempVt.slots.data()}, {tempVt.slots.data()}};
            CHECK(seam(kFC4SeamRefAuditRelease, &temps[0], reinterpret_cast<void*>(intptr_t{3})) == 0);
            CHECK(seam(kFC4SeamRefAuditRelease, &temps[1], reinterpret_cast<void*>(intptr_t{1})) == 1);
        }

        // The monthly digest (observer), read through the city's public
        // interfaces: here a fake city with our budget simulator.
        cISC4BudgetSimulator* budget3 = nullptr;
        CHECK(director->GetClassObject(kCLSID_Budget, kIID_cISC4BudgetSimulator, reinterpret_cast<void**>(&budget3)));
        if (seam != nullptr && budget3 != nullptr) {
            CHECK(budget3->Init());
            // The power glue's department / line-item path on our budget
            // classes, with a department reference read back from a save
            // (run 21: the token was the department's cIGZSerializable).
            CHECK(seam(kFC4SeamPowerBudgetPath, budget3, nullptr) == 0);
            static FakeWorld world;
            InitFakeWorld(world);
            world.budget = budget3;
            seam(kFC4SeamObserverCity, &world.city, nullptr);
            seam(kFC4SeamObserverMessage, reinterpret_cast<void*>(static_cast<uintptr_t>(kMsgPostCityInit)), nullptr);
            seam(kFC4SeamObserverMessage, reinterpret_cast<void*>(static_cast<uintptr_t>(kMsgSimNewMonth)),
                 reinterpret_cast<void*>(static_cast<intptr_t>(2451576)));
            seam(kFC4SeamObserverCity, nullptr, nullptr);
            CHECK(world.demandCalls == 28);  // digest and observe: 12 groups + 2 census nodes each
            CHECK(world.demandBadIndex == 0);
            CheckCw("power budget path, observer month, our budget");

            // The always-on guard: the reference as PreAppInit takes it, a
            // quiet tick, then a 53-bit main thread is reported once.
            // (FpuTick alone: a full observer tick would start the watchdog,
            // whose thread pins the DLL.)
            void* const fpuTick = reinterpret_cast<void*>(1);
            seam(kFC4SeamFpuReference, nullptr, nullptr);
            seam(kFC4SeamFpuReference, fpuTick, nullptr);
            unsigned int unused = 0;
            _controlfp_s(&unused, _PC_53, _MCW_PC);
            seam(kFC4SeamFpuReference, fpuTick, nullptr);
            seam(kFC4SeamFpuReference, fpuTick, nullptr);
            SetGamePrecision();
            CHECK(budget3->Shutdown());
        }
        if (budget3 != nullptr) CHECK(budget3->Release() == 0);
        if (reader != INVALID_HANDLE_VALUE) CloseHandle(reader);
    }

    // The same faulting address three times: reported at most twice.
    CHECK(ForeignFault() == 1);
    CHECK(ForeignFault() == 1);
    CHECK(ForeignFault() == 1);
    const std::string text = ReadFile(log);
    CHECK(CountOf(text, "first-chance exception OUTSIDE FreeCity4.dll in fc4_plugin_load_test.exe+0x") == 2);
    CHECK(text.find("(report 2 of at most 2 for this address)") != std::string::npos);
    // Watchdog (hang diagnostics).
    CHECK(text.find("watchdog: started for main thread") != std::string::npos);
    CHECK(text.find("WATCHDOG: main thread (tid ") != std::string::npos);
    CHECK(text.find("stalled - inside FreeCity4 for ") != std::string::npos);
    CHECK(text.find("  activity: OnTick (") != std::string::npos);
    CHECK(text.find("> core fc4_sim_on_tick (") != std::string::npos);
    CHECK(text.find("| delivering msg 0x66956814 (level 4) to agent ") != std::string::npos);
    CHECK(text.find("'blocking test agent' for ") != std::string::npos);
    CHECK(text.find("  context: eip=0x") != std::string::npos);
    CHECK(text.find("] FreeCity4.dll+0x") != std::string::npos);  // a return address into our image
    CHECK(text.find("] fc4_plugin_load_test.exe+0x") != std::string::npos);  // ... and into the executable
    CHECK(text.find("  cpu: process ") != std::string::npos);
    CHECK(text.find("WATCHDOG: stall over after ") != std::string::npos);
    CHECK(text.find("heartbeat (periodic): 2000-01-") != std::string::npos);
    CHECK(text.find("watchdog: stopped") != std::string::npos);
    CHECK(text.find("SetSimSpeed(2) -> 1 (julian 2451545") != std::string::npos);
    CHECK(text.find("HiddenResume() -> 1 (pause=0 hidden=0 emergency=0; julian 2451545 speed 2)") != std::string::npos);
    CHECK(text.find("first-chance exception OUTSIDE FreeCity4.dll in fc4_plugin_load_test.exe+0x") != std::string::npos);
    CHECK(text.find("FreeCity4 interface calls (oldest first):") != std::string::npos);
    CHECK(text.find("] BudgetSimulator::Shutdown") != std::string::npos);
    CHECK(text.find("call BudgetSimulator::GetTaxIncomeOfType (#1)") != std::string::npos);
    // The budget shadow (test seam op 10): a clean month, then type 3 attributed to the not-livable lot.
    CHECK(text.find("budget shadow 2000-02-01: tax orig/ours=750000/750000,0/0,0/0,1500000/1500000,0/0,0/0,"
                    "2250000/2250000,0/0,0/0,0/0,0/0,0/0; total orig/ours=4500000/4500000; income orig/ours=4500000/-; "
                    "expense orig/ours=0/-; net orig/ours=4500000/-; lots=5; mismatches=0; months=1 clean=1; "
                    "departments unreadable; tuning: code defaults") != std::string::npos);
    CHECK(text.find("budget shadow 2000-02-01: tax orig/ours=750000/750000,0/0,0/0,1875000/1500000!,") !=
          std::string::npos);
    CHECK(text.find("; lots=5; mismatches=1 [tax 3]; months=2 clean=1;") != std::string::npos);
    CHECK(text.find("budget shadow 2000-02-01 type 3: assessed orig/ours=2500/2000 (orig-ours=500); "
                    "tax orig/ours=1875000/1500000; rate=9.00 mult=100000.000; ours: counted=1 (building wealth != "
                    "lot wealth: 0), not-livable=1 (assessed 500)") != std::string::npos);
    CHECK(text.find("budget shadow 2000-02-01 type 3 lot (2,3) 1x1: not-livable purpose=2 wealth b1/l1 value=500 lv=0 "
                    "assessed=500 tax=0.000 [assessed = |orig-ours|]") != std::string::npos);
    CHECK(text.find("budget shadow 2000-02-01 type 3 lot (1,2) 2x1: counted purpose=2 wealth b1/l1 value=2000 lv=0 "
                    "assessed=2000 tax=1500000.000") != std::string::npos);
    CHECK(CountOf(text, "budget shadow 2000-02-01 type ") == 3);  // one type line, two lot lines
    CHECK(text.find("budget shadow: the original budget ") != std::string::npos);
    CHECK(text.find("budget shadow: ON") == std::string::npos);  // off by default
    // The pollution shadow (test seam op 16): two incomplete deliveries, one month line.
    CHECK(text.find("pollution shadow: the original pollution simulator ") != std::string::npos);
    CHECK(text.find("pollution shadow: comparing with the original pollution simulator ") != std::string::npos);
    CHECK(text.find("pollution shadow 2000-02-01 month: INCOMPLETE (before: the record has no grid references); "
                    "not compared") != std::string::npos);
    CHECK(text.find("pollution shadow 2000-02-01 day: INCOMPLETE (before: the record has no grid references); "
                    "not compared") != std::string::npos);
    CHECK(text.find("pollution shadow month to 2000-02-01: deliveries=1 (days 0, months 1) clean=0 incomplete=1 "
                    "reruns=0 uncached=0; cells=0 mismatches=0; airSrc 0/0 max|d|=0, waterSrc 0/0 max|d|=0,") !=
          std::string::npos);
    CHECK(text.find("; first differing cells: none; fields: none; props compared=0 mismatched=0; deals compared=0 "
                    "mismatched=0; incomplete because: before: the record has no grid references x1") !=
          std::string::npos);
    CHECK(CountOf(text, "pollution shadow month to ") == 1);
    CHECK(text.find("pollution shadow: ON") == std::string::npos);  // off by default
    CHECK(text.find("DLL_PROCESS_ATTACH") != std::string::npos);
    CHECK(text.find("cFC4Director constructed") != std::string::npos);
    CHECK(text.find("EnumClassObjects: reporting clsid 0x2990C1E5") != std::string::npos);
    CHECK(text.find("subsystem 'landvalue': off by default") != std::string::npos);
    CHECK(text.find("cFC4LandValueSimulator(") == std::string::npos);
    CHECK(text.find("subsystem 'weather': off by default") != std::string::npos);
    CHECK(text.find("cFC4WeatherSimulator(") == std::string::npos);
    CHECK(text.find("subsystem 'ordinance': off by default") != std::string::npos);
    CHECK(text.find("cFC4OrdinanceSimulator(") == std::string::npos);
    CHECK(text.find("subsystem 'demand': off by default") != std::string::npos);
    CHECK(text.find("cFC4DemandSimulator(") == std::string::npos);
    CHECK(text.find("GetClassObject enter (clsid=0x2990C1E5, iid=0x8695664E") != std::string::npos);
    CHECK(text.find("GetClassObject exit (clsid=0x2990C1E5, iid=0x8695664E) -> 1") != std::string::npos);
    CHECK(text.find("GetClassObject exit (clsid=0x2990C1E5, iid=0x0BADF00D) -> 0") != std::string::npos);
    CHECK(text.find("GetClassObject exit (clsid=0x12345678, iid=0x8695664E) -> 0") != std::string::npos);
    CHECK(text.find("factory enter: cSC4Simulator (0x2990C1E5), factory call #1") != std::string::npos);
    CHECK(text.find("constructor enter") != std::string::npos);
    CHECK(text.find("calling fc4_sim_create") != std::string::npos);
    CHECK(text.find("fc4_sim_create returned") != std::string::npos);
    CHECK(text.find("cSC4Simulator object created") != std::string::npos);
    CHECK(text.find("CreateOurs: after QueryInterface -> 1 (clean=1)") != std::string::npos);
    CHECK(text.find("vectored handler installed") != std::string::npos);
    CHECK(text.find("first-chance exception in FreeCity4.dll: code 0xC0000005 ACCESS_VIOLATION (writing address "
                    "0x00000000) at") != std::string::npos);
    CHECK(text.find("STRUCTURED EXCEPTION caught in diagnostics self-test: code 0xC0000005") != std::string::npos);
    CHECK(text.find("diagnostics self-test: guard returned 0 (expected 0)") != std::string::npos);
    CHECK(text.find("could not be written") == std::string::npos);
    CHECK(text.find("AddAgent(") != std::string::npos);
    CHECK(text.find("deliver #1: level 4 type 0x66956814 d1=2451546 d2=0 d3=2 -> agent") != std::string::npos);
    CHECK(text.find("'test day agent' returned 0") != std::string::npos);
    CHECK(text.find("'test day agent' (level 4) has now returned false 100 times in a row") != std::string::npos);
    CHECK(text.find("status @tick 5:") != std::string::npos);
    CHECK(text.find("DAY_GATE_OPEN=") != std::string::npos);
    CHECK(text.find("  level 4: 1 agent(s), cursor") != std::string::npos);
    CHECK(text.find("post SimNewDay 0x66956814 d1=2451546 (2000-01-02) d2=0 d3=2 -> MessagePost skipped") !=
          std::string::npos);
    CHECK(text.find("post 0x4A7FB7E9 d1=2") != std::string::npos);  // SpeedChanged
    CHECK(text.find("GetSimDate() #1 -> date object") != std::string::npos);
    CHECK(text.find("::Year() -> 2000") != std::string::npos);
    CHECK(text.find("GetSimDateNumber() #1 -> 2451545") != std::string::npos);
    // Limit-timer calibration (spec 5.5): installed and published, but this test
    // process is not SimCity 4, so the compat guard must refuse to write.
    CHECK(text.find("calibration: QPC ") != std::string::npos);
    CHECK(text.find("installed=1") != std::string::npos);
    CHECK(text.find("compat: host executable ") != std::string::npos);
    CHECK(text.find("compat: WARNING unrecognised executable") != std::string::npos);
    CHECK(text.find("written to the game=yes") == std::string::npos);
    CHECK(text.find("calibration (Init): installed") != std::string::npos);
    CHECK(text.find("calibration (status): installed") != std::string::npos);
    CHECK(text.find("QueryInterface(0x8695664E) -> 1") != std::string::npos);
    CHECK(text.find("Init() called") != std::string::npos);
    CHECK(text.find("OnTick #1 ") != std::string::npos);
    CHECK(text.find("HiddenResume() -> 1 (pause=0 hidden=0") != std::string::npos);
    CHECK(text.find("cFC4Simulator destroyed") != std::string::npos);
    CHECK(text.find("subsystem 'budget': enabled (3 classes replaced)") != std::string::npos);
    CHECK(text.find("cSC4BudgetSimulator object created") != std::string::npos);
    CHECK(text.find("budget: tuning exemplar unavailable; using the code defaults") != std::string::npos);
    CHECK(text.find("budget: no simulator; agents not registered") != std::string::npos);
    CHECK(text.find("budget: game-data registry node 'budget' not published (stub)") != std::string::npos);
    CHECK(text.find("budget Init: ok=1") != std::string::npos);
    CHECK(text.find("budget post 0x0772FAD4") != std::string::npos);  // FundsChanged (skipped: no server)
    CHECK(text.find("budget IssueBond(10000) -> 1 (borrowed 10000)") != std::string::npos);
    CHECK(text.find("budget Shutdown") != std::string::npos);
    CHECK(text.find("cFC4BudgetSimulator destroyed") != std::string::npos);

    // Budget §10: the verbose post line shows what receivers read.
    CHECK(text.find("test seam: budget post sink installed") != std::string::npos);
    CHECK(text.find("budget post 0x0772FAD4 d1=0x") != std::string::npos);
    CHECK(text.find("not posted (no message server; test sink only)") != std::string::npos);
    CHECK(text.find(" void1=") != std::string::npos && text.find(" unknown=") != std::string::npos);
    // Observer (replacement mode) and the monthly digest.
    CHECK(text.find("autotest: off") != std::string::npos);
    CHECK(text.find("observer: PostCityInit (replacement)") != std::string::npos);
    CHECK(text.find("DIGEST julian=2451576 (2000-02-01) funds=500000 population R/C/I=1234/480/160 demand "
                    "R$/R$$/R$$$/Cs$/Cs$$/Cs$$$/Co$$/Co$$$/I-R/I-D/I-M/I-HT=250.5/-/-/-12.0/-/-/40.0/-/7.5/-/-/0.0 "
                    "budget income/expense=") != std::string::npos);
    CHECK(text.find("power budget path test: passed") != std::string::npos);
    // The x87 control word: logged at attach, the reference, a change seen by
    // a tick, and the guard's two warnings, each once.
    CHECK(text.find("fpu DLL attach: x87 CW=0x") != std::string::npos);
    CHECK(text.find("(PC 24-bit, RC nearest, masks 0x3F) MXCSR=0x") != std::string::npos);
    CHECK(text.find("[thread ") != std::string::npos && text.find(", main] = the reference for the guard") != std::string::npos);
    CHECK(text.find(": control word changed from 0x") != std::string::npos);
    CHECK(CountOf(text, "fpu: WARNING the main thread's control word changed (tick ") == 1);
    CHECK(CountOf(text, "fpu: WARNING the main thread is not at 24-bit x87 precision (tick ") == 1);
    CHECK(text.find("(PC 53-bit") != std::string::npos);
    CHECK(text.find("power budget path test: step") == std::string::npos);
    // The observe line (our budget; no game allocator here, so no departments).
    CHECK(text.find("observer: digest on, observe on") != std::string::npos);
    CHECK(text.find("observe month 2000-02: date=2000-02-01 julian=2451576 funds=500000 income=") != std::string::npos);
    CHECK(text.find(" dept_expense=- demand R$/R$$/R$$$/Cs$/Cs$$/Cs$$$/Co$$/Co$$$/I-R/I-D/I-M/I-HT="
                    "250.5/-/-/-12.0/-/-/40.0/-/7.5/-/-/0.0 population R/C/I=1234/480/160") != std::string::npos);
    CHECK(text.find("[INCOMPLETE") == std::string::npos);
    // Bisection configuration.
    CHECK(text.find("config (game start): all 72 budget switches ON") != std::string::npos);
    CHECK(text.find("config (city load: budget Init): all 72 budget switches ON") != std::string::npos);
    CHECK(text.find("set by ini: property_writes=0, cheats=1, holder_refs=0; set by env: disaster=0") !=
          std::string::npos);
    CHECK(text.find("config (city load: budget Init): budget switches OFF (3 of 72): property_writes, disaster, "
                    "holder_refs") != std::string::npos);
    CHECK(text.find("switch 'holder_refs' off: AddRef of a game object for the budget core skipped") !=
          std::string::npos);
    CHECK(text.find("switch 'property_writes' off: AddPropertyVariant (reported as written) skipped") !=
          std::string::npos);
    CHECK(text.find("switches (budget Shutdown): calls skipped since the last report: property_writes ") !=
          std::string::npos);
    CHECK(text.find("budget core (before budget Shutdown): holds 0 reference(s)") != std::string::npos);
    // Reference-count audit: the fake holder and the test agents are "game" objects.
    CHECK(text.find("refaudit: on (verbose line cap 2000") != std::string::npos);
    CHECK(text.find("ref +1 AddRef ") != std::string::npos);
    CHECK(text.find("ref -1 Release ") != std::string::npos);
    CHECK(text.find("[class ?, vtable fc4_plugin_load_test.exe+0x") != std::string::npos);
    CHECK(text.find("at fc4::`anonymous-namespace'::AddRefToken:") != std::string::npos);
    CHECK(text.find("refaudit (budget Shutdown): ") != std::string::npos);
    CHECK(text.find("refaudit (simulator Shutdown): ") != std::string::npos);
    CHECK(text.find("objects we still hold references to: 0") != std::string::npos);
    CHECK(text.find("WARNING: released last reference to game object ") != std::string::npos);  // test agents
    CHECK(text.find("WARNING: over-release") == std::string::npos);
    // Checked build: RTC failures and out-parameter overruns reach the log.
    if (mode == "--checked") {
        CHECK(text.find("build: CHECKED (/Od /RTC1") != std::string::npos);
        CHECK(text.find("RTC ERROR #1 (report 1 for this call site): ESP not preserved across a call") !=
              std::string::npos);
        CHECK(text.find("  call site: FreeCity4.dll+0x") != std::string::npos);
        CHECK(text.find("ProvokeEspMismatch") != std::string::npos);  // our function name (FreeCity4.pdb)
        CHECK(text.find("OUT-PARAM OVERRUN #1: the callee wrote outside the 1-byte self-test uint8 out-parameter") !=
              std::string::npos);
        CHECK(text.find("(canary C5: before C5C5C5C5C5C5C5C5C5C5C5C5C5C5C5C5 | after 332211C5") != std::string::npos);
    } else {
        CHECK(text.find("build: normal") != std::string::npos);
        CHECK(text.find("FC4_CHECKED_SELFTEST ignored: this is not a checked build") != std::string::npos);
        CHECK(text.find("RTC ERROR") == std::string::npos);
    }
    CHECK(text.find("OUT-PARAM OVERRUN #2") == std::string::npos);

    CheckCw("the whole normal run");
    FreeLibrary(h);
    CheckCw("FreeLibrary");
    CHECK(ReadFile(log).find("DLL_PROCESS_DETACH (FreeLibrary)") != std::string::npos);

    if (g_failures == 0) std::printf("plugin load test: all checks passed (log: %s)\n", log.c_str());
    return g_failures == 0 ? 0 : 1;
}
