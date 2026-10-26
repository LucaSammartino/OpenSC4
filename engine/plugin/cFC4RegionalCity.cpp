// SPDX-License-Identifier: GPL-3.0-or-later
#include "cFC4RegionalCity.h"
#include "FC4RegionalCityElements.h"
#include "FC4RegionalCityHost.h"
#include "FC4Streams.h"
#include "FC4Log.h"
#include "FC4Diag.h"
#include "cFC4Neighbors.h"
#include "GZServPtrs.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessage2Standard.h"
#include "cISC4AuraSimulator.h"
#include "cISC4AirportDeveloper.h"
#include "cISC4SeaportDeveloper.h"
#include "cISC4TutorialSystem.h"
#include "cISC4Region.h"
#include "cISC4RegionalCity.h"
#include "cISC4Lot.h"
#include "cRZBaseString.h"
#include "cRZAutoRefCount.h"
#include "SC4Preferences.h"
#include "SC4List.h"
#include "SC4Vector.h"
#include <algorithm>
#include <cstring>
#include <map>
#include <vector>

namespace rc = fc4::regionalcity;
namespace {
constexpr uint32_t messages[] = {0x26C63342, 0x26C63343, 0x26D31EC1, 0x26D31EC3, 0xCA63D517};
uint8_t Difficulty() {
    rc::App app;
    auto* p = app.p ? app.p->GetPreferences() : nullptr;
    return p ? static_cast<uint8_t>(p->newCityPreferences.difficultyLevel) : 0;
}
uint64_t CityAt(void* raw, int32_t x, int32_t z) {
    auto* r = static_cast<cISC4Region*>(raw);
    auto* p = r ? r->GetCity(static_cast<uint32_t>(x), static_cast<uint32_t>(z)) : nullptr;
    return p ? fc4::TokenOf(static_cast<cISC4RegionalCity*>(*p)) : 0;
}
template<class T> void Emit(void* raw, void* p) {
    auto* object = static_cast<T*>(p);
    FC4_ADDREF_FOR_CALLER(object); // the receiving SC4List owns this copied reference
    static_cast<SC4List<T*>*>(raw)->push_back(object);
}
}
cFC4RegionalCity::cFC4RegionalCity() : owner_(fc4_regionalcity_owner_create(Difficulty())) {}
cFC4RegionalCity::~cFC4RegionalCity() {
    Shutdown();
    fc4_regionalcity_owner_destroy(owner_);
    fc4::NeighborsCollectReleasedElements();
}
bool cFC4RegionalCity::QueryInterface(uint32_t id, void** out) {
    if (!out) return false;
    *out = nullptr;
    if (id == 1 || id == 0x69D2BE1D) *out = static_cast<cFC4ISC4RegionalCity*>(this);
    else if (id == 0x452294AA) *out = static_cast<cIGZMessageTarget2*>(this);
    else if (id == 0xE4FDA3D4) *out = static_cast<cIGZSerializable*>(this);
    if (!*out) return false;
    AddRef();
    return true;
}
uint32_t cFC4RegionalCity::AddRef() { return ++references_; }
uint32_t cFC4RegionalCity::Release() {
    if (destroying_) return 0;
    const auto n = references_.fetch_sub(1);
    if (n > 1) return n - 1;
    destroying_ = true;
    delete this;
    return 0;
}
bool cFC4RegionalCity::Init() {
    FC4_TRACE("RegionalCity::Init");
    if (!initialized_) {
        initialized_ = true;
        cIGZMessageServer2Ptr ms;
        if (ms) for (auto m : messages) ms->AddNotification(this, m);
    }
    return true;
}
bool cFC4RegionalCity::Shutdown() {
    FC4_TRACE("RegionalCity::Shutdown");
    if (initialized_) {
        initialized_ = false;
        cIGZMessageServer2Ptr ms;
        if (ms) for (auto m : messages) ms->RemoveNotification(this, m);
        fc4_regionalcity_owner_event(owner_, 2);
        fc4::NeighborsCollectReleasedElements();
    }
    return true;
}
uint32_t cFC4RegionalCity::GetWorkforcePercentage() const {
    // regional-city-records §5.4: slot 13 returns an address. A per-thread
    // snapshot avoids racing the writer while preserving the pointer channel.
    static thread_local float value;
    value = static_cast<float>(Get(7));
    return static_cast<uint32_t>(reinterpret_cast<uintptr_t>(&value));
}
std::string cFC4RegionalCity::Text(uint32_t f) const {
    const auto n = fc4_regionalcity_owner_text(owner_, f, nullptr, 0);
    if (n == UINT32_MAX) return {};
    std::string s(n, '\0');
    fc4_regionalcity_owner_text(owner_, f, reinterpret_cast<uint8_t*>(s.data()), n);
    return s;
}
void cFC4RegionalCity::Text(uint32_t f, cIGZString& out) const {
    const auto s = Text(f);
    out.FromChar(s.data(), static_cast<uint32_t>(s.size()));
}
void cFC4RegionalCity::Text(uint32_t f, const cIGZString& s) {
    fc4_regionalcity_owner_set_text(owner_, f, reinterpret_cast<const uint8_t*>(s.ToChar()), s.Strlen());
}
cISC4RegionalCity* cFC4RegionalCity::FindConnection(int32_t t, int32_t x, int32_t z) const {
    return reinterpret_cast<cISC4RegionalCity*>(fc4_regionalcity_owner_find(owner_, static_cast<uint32_t>(t), x, z));
}
void cFC4RegionalCity::GetAllConnections(SC4List<cISC4NeighborConnection*>& out) const {
    while (!out.empty()) out.erase(out.begin());
    fc4_regionalcity_owner_list(owner_, 0, &out, &Emit<cISC4NeighborConnection>);
}
void cFC4RegionalCity::ChangeSymmetricConnection(cISC4NeighborConnection* c, bool add) {
    rc::App app;
    auto* current = app.p ? fc4::AsGame(app.p->GetRegionalCity()) : nullptr;
    if (!current || !c) return;
    int32_t x = 0, z = 0, sx = 0, sz = 0;
    current->GetPosition(x, z);
    current->GetCitySize(sx, sz);
    const auto calls = rc::ConnectionCalls(rc::RepairLiveConnection);
    fc4_regionalcity_owner_change_connection(owner_, c, &calls, x, z, sx, sz, add);
}
bool cFC4RegionalCity::Deals(uint32_t action, SC4List<cISC4NeighborDeal*>* list) {
    rc::App app;
    FC4RegionalCityRegionCalls region{app.p ? app.p->GetRegion() : nullptr,
        fc4::TokenOf(Primary()), app.p ? fc4::TokenOf(app.p->GetRegionalCity()) : 0, CityAt};
    std::vector<void*> objects;
    if (list) for (auto* p : *list) objects.push_back(p);
    return fc4_regionalcity_owner_deal_action(owner_, action, &region, objects.data(),
        static_cast<uint32_t>(objects.size()), &rc::DealCalls());
}
bool cFC4RegionalCity::UpdateLocalDeals() { return Deals(0, nullptr); }
bool cFC4RegionalCity::SetLocalDeals(SC4List<cISC4NeighborDeal*>& list) { return Deals(1, &list); }
bool cFC4RegionalCity::GetLocalDeals(SC4List<cISC4NeighborDeal*>& out) {
    while (!out.empty()) out.erase(out.begin());
    return fc4_regionalcity_owner_list(owner_, 1, &out, &Emit<cISC4NeighborDeal>);
}
bool cFC4RegionalCity::UpdateImportExport() { return Deals(2, nullptr); }
void cFC4RegionalCity::SetupPreferences(SC4NewCityPreferences* p) {
    if (!p) return;
    const auto city = Text(30), mayor = Text(31);
    strncpy_s(p->cityName, city.c_str(), _TRUNCATE);
    strncpy_s(p->mayorName, mayor.c_str(), _TRUNCATE);
    p->difficultyLevel = static_cast<uint32_t>(Int(9));
}
void cFC4RegionalCity::UpdateCityCacheFromPreferences(SC4NewCityPreferences* p) {
    if (!p) return;
    fc4_regionalcity_owner_preferences(owner_, reinterpret_cast<const uint8_t*>(p->cityName),
        static_cast<uint32_t>(strnlen(p->cityName, 512)), reinterpret_cast<const uint8_t*>(p->mayorName),
        static_cast<uint32_t>(strnlen(p->mayorName, 512)), static_cast<uint8_t>(p->difficultyLevel));
}
void cFC4RegionalCity::UpdateCityCacheFromCity(cISC4City* city) {
    FC4_TRACE("RegionalCity::UpdateCityCache(city)");
    if (!city) return;
    cRZBaseString s;
    city->GetMayorName(s); SetMayorName(s);
    city->GetCityName(s); SetCityName(s);
    FC4RegionalCityCache cache{};
    cache.difficulty = static_cast<uint8_t>(city->GetDifficultyLevel());
    city->GetCitySaveFilePath(s); SetCitySaveFilePath(s);
    SetCitySerialNumber(city->GetCitySerialNumber()); cache.serial = GetCitySerialNumber();
    SetEstablished(city->GetEstablished()); cache.established = GetEstablished();
    auto* aura = city->GetAuraSimulator();
    cache.mayorRating = aura ? aura->GetMayorRating() : 0;
    auto* budget = fc4::AsGame(city->GetBudgetSimulator());
    for (uint32_t i = 0; i < 12; ++i) cache.tax[i] = budget ? budget->GetTaxRate(i) : 7.0f;
    auto* neighbors = fc4::AsGame(city->GetNeighborsSimulator());
    if (neighbors) {
        SC4List<cISC4NeighborConnection*> list;
        neighbors->GetAllConnections(list);
        std::vector<void*> objects;
        for (auto* c : list) objects.push_back(c);
        const auto calls = rc::ConnectionCalls(rc::RepairLiveConnection);
        fc4_regionalcity_owner_connections(owner_, objects.data(), static_cast<uint32_t>(objects.size()), &calls);
        neighbors->UpdateCommodityStats(city);
    }
    auto* tutorial = city->GetTutorialSystem();
    cache.tutorial = tutorial ? tutorial->GetTutorialGUID() : -1;
    auto* residential = fc4::AsGame(city->GetResidentialSimulator());
    cache.residential = residential != nullptr;
    if (residential) {
        cache.population = residential->GetPopulation();
        cache.workforce = residential->GetGlobalWorkforcePercent().percentage;
    }
    std::map<uint32_t, int32_t> summary;
    std::vector<FC4RegionalCityPopulation> populations;
    auto* demand = fc4::AsGame(city->GetDemandSimulator());
    if (demand) {
        demand->GetLocalPopulationSummary(summary);
        for (const auto& entry : summary) {
            FC4RegionalCityPopulation p{}; p.type = entry.first; p.count = entry.second;
            demand->CalculateJobsPerUnitOfDemand(p.jobs, p.type, 0);
            populations.push_back(p);
        }
    }
    cache.populations = populations.data(); cache.populationCount = static_cast<uint32_t>(populations.size());
    std::vector<FC4RegionalCityPoint> points;
    auto ports = [&](cISC4PortDeveloper* dev, uint32_t type) {
        if (!dev) return;
        SC4Vector<cISC4Lot*> lots;
        dev->GetAllPortLots(lots);
        for (auto* lot : lots) if (lot && lot->IsInLivableState()) {
            FC4RegionalCityPoint point{}; point.type = type; point.livable = true;
            lot->GetCenter(point.position); points.push_back(point);
        }
    };
    ports(city->GetAirportDeveloper(), 0); ports(city->GetSeaportDeveloper(), 1);
    cache.points = points.data(); cache.pointCount = static_cast<uint32_t>(points.size());
    fc4_regionalcity_owner_cache(owner_, &cache);
}
void cFC4RegionalCity::GetPointsOfInterest(uint32_t type, fc4::GameVector<FC4PointOfInterest>& list) {
    fc4_regionalcity_owner_points(owner_, type, &list, [](void* raw, const float (*p)[3], uint32_t n) {
        auto& out = *static_cast<fc4::GameVector<FC4PointOfInterest>*>(raw);
        out.clear();
        for (uint32_t i = 0; i < n; ++i) {
            FC4PointOfInterest point; std::memcpy(&point, p[i], sizeof(point)); out.push_back(point);
        }
    });
}
bool cFC4RegionalCity::DoMessage(cIGZMessage2* message) {
    FC4_TRACE("RegionalCity::DoMessage");
    if (!message) return true;
    const auto type = message->GetType();
    if (type == 0x26C63342) { fc4_regionalcity_owner_event(owner_, 0); return true; }
    rc::App app;
    if (!app.p || app.p->GetRegionalCity() != Primary()) return true;
    auto* city = app.p->GetCity();
    if (type == 0x26D31EC3) fc4_regionalcity_owner_event(owner_, 0);
    else if (city && (type == 0x26D31EC1 || type == 0x26C63343)) {
        UpdateCityCacheFromCity(city);
        if (type == 0x26C63343) fc4_regionalcity_owner_event(owner_, 1);
    } else if (city && type == 0xCA63D517) {
        auto* standard = static_cast<cIGZMessage2Standard*>(message);
        rc::Reconcile(*this, city, standard->GetData3() != 0);
    }
    return true;
}
bool cFC4RegionalCity::Write(cIGZOStream& output) {
    FC4_TRACE("RegionalCity::Write");
    fc4::OStreamAdapter stream(output);
    rc::RecordContext context{nullptr, &output};
    const auto calls = rc::RecordCalls(&context, rc::RepairLiveConnection);
    return fc4_regionalcity_owner_write(owner_, stream.Get(), &calls);
}
bool cFC4RegionalCity::Read(cIGZIStream& input) {
    FC4_TRACE("RegionalCity::Read");
    fc4::IStreamAdapter stream(input);
    rc::RecordContext context{&input, nullptr};
    const auto calls = rc::RecordCalls(&context, rc::RepairLiveConnection);
    if (!fc4_regionalcity_owner_decode(owner_, stream.Get(), Difficulty(), &calls)) return false;
    rc::FinishRead(*this, input, Difficulty());
    return Init();
}
cIGZUnknown* CreateFC4RegionalCity() {
    return static_cast<cFC4ISC4RegionalCity*>(new cFC4RegionalCity());
}
