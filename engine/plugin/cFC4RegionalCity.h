// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4Interfaces.h"
#include "FC4RegionalCityState.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include <atomic>
#include <string>

class cFC4RegionalCity final : public cFC4ISC4RegionalCity,
    public cIGZMessageTarget2, public cIGZSerializable {
public:
    cFC4RegionalCity();
    virtual ~cFC4RegionalCity(); // regional-city §1: deleting destructor, slot 73
    bool QueryInterface(uint32_t, void**) override;
    uint32_t AddRef() override;
    uint32_t Release() override;
    bool Init() override;
    bool Shutdown() override;
    void GetPosition(int32_t& x, int32_t& z) const override { x = Int(0); z = Int(1); }
    void SetPosition(int32_t x, int32_t z, bool move) override { fc4_regionalcity_owner_position(owner_, x, z, move); }
    void GetCitySize(int32_t& x, int32_t& z) const override { x = Int(2); z = Int(3); }
    void SetCitySize(int32_t x, int32_t z) override { fc4_regionalcity_owner_size(owner_, x, z); }
    int32_t GetPopulationOfType(uint32_t t) const override { return Int(41, t); }
    int32_t GetPopulation() const override { return Int(4); }
    int32_t GetCommercialJobs() const override { return Int(5); }
    int32_t GetIndustrialJobs() const override { return Int(6); }
    uint32_t GetWorkforcePercentage() const override;
    int8_t GetMayorRating() const override { return static_cast<int8_t>(Int(8)); }
    cISC4City::DifficultyLevel GetDifficultyLevel() const override { return static_cast<cISC4City::DifficultyLevel>(Int(9)); }
    float GetTaxRate(uint32_t t) const override { return static_cast<float>(Get(40, t)); }
    int32_t GetExtrapolatedPopulation(uint32_t t) const override { return Int(42, t); }
    int32_t GetAllowableExtrapolation(uint32_t t) const override { return Int(43, t); }
    void ExtrapolateGrowth(uint32_t t, float n) override { Set(42, n, t); }
    cISC4RegionalCity* FindConnection(int32_t t, int32_t x, int32_t z) const override;
    void GetAllConnections(SC4List<cISC4NeighborConnection*>&) const override;
    void ChangeSymmetricConnection(cISC4NeighborConnection*, bool) override;
    void SetupPreferences(SC4NewCityPreferences*) override;
    void UpdateCityCacheFromCity(cISC4City*) override;
    void UpdateCityCacheFromPreferences(SC4NewCityPreferences*) override;
    void SetupCity(cISC4City*) override;
    uint32_t GetCitySerialNumber() const override { return static_cast<uint32_t>(Get(10)); }
    void SetCitySerialNumber(uint32_t n) override { Set(10, n); }
    bool GetOriginalLanguageAndCountry(int32_t& l, int32_t& c) const override { l = Int(21); c = Int(22); return true; }
    bool GetLastLanguageAndCountry(int32_t& l, int32_t& c) override { l = Int(23); c = Int(24); return true; }
    void GetCitySaveFilePath(cIGZString& s) const override { Text(34, s); }
    void SetCitySaveFilePath(const cIGZString& s) override { Text(34, s); }
    void GetCityName(cIGZString& s) const override { Text(30, s); }
    void SetCityName(const cIGZString& s) override { Text(30, s); }
    void GetMayorName(cIGZString& s) const override { Text(31, s); }
    void SetMayorName(const cIGZString& s) override { Text(31, s); }
    void GetUtilityAdvisorName(cIGZString& s) const override { Text(32, s); }
    void SetUtilityAdvisorName(const cIGZString& s) override { Text(32, s); }
    void GetCityDescription(cIGZString& s) const override { Text(33, s); }
    void SetCityDescription(const cIGZString& s) override { Text(33, s); }
    uint32_t GetBirthDate() const override { return static_cast<uint32_t>(Get(11)); }
    void SetBirthDate(uint32_t n) override { Set(11, n); }
    bool GetEstablished() const override { return Int(12) != 0; }
    void SetEstablished(bool b) override { Set(12, b); }
    void GetWorldPosition(float& x, float& z) const override { x = static_cast<float>(Get(13)); z = static_cast<float>(Get(14)); }
    void SetWorldPosition(float x, float z) override { Set(13, x); Set(14, z); }
    float GetWorldBaseElevation() const override { return static_cast<float>(Get(15)); }
    void SetWorldBaseElevation(float h) override { Set(15, h); }
    cISC4City::WorldHemisphere GetWorldHemisphere() const override { return static_cast<cISC4City::WorldHemisphere>(Int(16)); }
    double GetBudget() const override { return Get(17); }
    void SetBudget(double n) override { Set(17, n); }
    double GetIncome() const override { return Get(18); }
    void SetIncome(double n) override { Set(18, n); }
    double GetExported(int32_t t) const override { return Get(52, t); }
    void SetExported(int32_t t, double n) override { Set(52, n, t); }
    double GetImported(int32_t t) const override { return Get(53, t); }
    void SetImported(int32_t t, double n) override { Set(53, n, t); }
    double GetProduced(int32_t t) const override { return Get(50, t); }
    void SetProduced(int32_t t, double n) override { Set(50, n, t); }
    double GetDemanded(int32_t t) const override { return Get(51, t); }
    void SetDemanded(int32_t t, double n) override { Set(51, n, t); }
    double GetCostPerUnit(int32_t t) const override { return Get(54, t); }
    void SetCostPerUnit(int32_t t, double n) override { Set(54, n, t); }
    double GetCommodityBalance(int32_t t) const override { return Get(55, t); }
    uint32_t GetTutorialGUID() const override { return static_cast<uint32_t>(Int(19)); }
    void SetTutorialGUID(uint32_t n) override { Set(19, static_cast<int32_t>(n)); }
    bool IsTutorial() const override { return Int(20) != 0; }
    bool UpdateLocalDeals() override;
    bool SetLocalDeals(SC4List<cISC4NeighborDeal*>&) override;
    bool GetLocalDeals(SC4List<cISC4NeighborDeal*>&) override;
    bool UpdateImportExport() override;
    void GetPointsOfInterest(uint32_t, fc4::GameVector<FC4PointOfInterest>&) override;
    bool DoMessage(cIGZMessage2*) override;
    bool Write(cIGZOStream&) override;
    bool Read(cIGZIStream&) override;
    uint32_t GetGZCLSID() override { return 0x6A037C24; }
    cISC4RegionalCity* Primary() { return reinterpret_cast<cISC4RegionalCity*>(static_cast<cFC4ISC4RegionalCity*>(this)); }
    FC4RegionalCityOwner* Owner() const { return owner_; }
private:
    double Get(uint32_t f, uint32_t t = 0) const { double n = 0; fc4_regionalcity_owner_get(owner_, f, t, &n); return n; }
    int32_t Int(uint32_t f, uint32_t t = 0) const { return static_cast<int32_t>(Get(f, t)); }
    void Set(uint32_t f, double n, uint32_t t = 0) { fc4_regionalcity_owner_set(owner_, f, t, n); }
    std::string Text(uint32_t) const;
    void Text(uint32_t, cIGZString&) const;
    void Text(uint32_t, const cIGZString&);
    bool Deals(uint32_t, SC4List<cISC4NeighborDeal*>*);
    FC4RegionalCityOwner* owner_;
    std::atomic<uint32_t> references_{0};
    bool initialized_ = false;
    bool destroying_ = false;
};
cIGZUnknown* CreateFC4RegionalCity();

namespace fc4::regionalcity {
void Reconcile(cFC4RegionalCity&, cISC4City*, bool);
void RepairLiveConnection(void*, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);
void FinishRead(cFC4RegionalCity&, cIGZIStream&, uint8_t);
}
