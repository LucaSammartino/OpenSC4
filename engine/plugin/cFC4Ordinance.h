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

// The ordinance simulator and the standard ordinance classes
// (docs/specs/ordinance.md §1, §3.6). The model lives in the Rust core behind
// fc4.h; these classes adapt the SDK interfaces to it.
//
// Ordinance §10.2: the simulator and the ordinance classes are replaced
// together (the original simulator would call a private method of the
// original ordinance classes). Foreign cISC4Ordinance objects (DLL plugins)
// are accepted and called through the interface only.
//
// The subsystem is OFF by default: FreeCity4.ini `enable = ordinance` turns it
// on (FC4Subsystems.h); `disable` still wins.
#pragma once

#include "FC4Interfaces.h"
#include "FC4Properties.h"
#include "FC4Streams.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4OrdinanceSimulator.h"
#include "cRZBaseString.h"
#include "fc4.h"

#include <cstdint>
#include <unordered_map>

class cISC4Simulator;

static const uint32_t kFC4_CLSID_cSC4OrdinanceSimulator = 0xA990C08E;
static const uint32_t kFC4_IID_cISC4OrdinanceSimulator = 0x41193C3A;

// The standard ordinance classes (§3.6): the base class first, then the 20
// subclasses. The class ID is the ordinance ID and the exemplar instance.
extern const uint32_t kFC4OrdinanceClasses[21];
bool FC4IsOrdinanceClass(uint32_t clsid);

class cFC4OrdinanceSimulator final : public cISC4OrdinanceSimulator,
                                     public cIGZMessageTarget2,
                                     public cIGZSerializable {
public:
    cFC4OrdinanceSimulator();
    ~cFC4OrdinanceSimulator();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4OrdinanceSimulator (§4.2)
    bool Init() override;
    bool Shutdown() override;
    bool AddOrdinance(cISC4Ordinance& ordinance) override;
    bool RemoveOrdinance(cISC4Ordinance& ordinance) override;
    bool IsOrdinanceAvailable(uint32_t id) override;
    bool IsOrdinanceOn(uint32_t id) override;
    bool IsOrdinanceAvailableButOff(uint32_t id) override;
    bool SetOrdinanceAvailable(uint32_t id, bool available) override;
    uint32_t GetOrdinanceAvailableCount() override;
    uint32_t GetOrdinanceAvailableArray(uint32_t* out, uint32_t& count) override;
    bool SetOrdinanceOn(uint32_t id, bool on) override;
    uint32_t GetOrdinanceOnCount() override;
    uint32_t GetOrdinanceOnArray(uint32_t* out, uint32_t& count) override;
    bool IsOrdinanceEnabled(uint32_t id) override;
    bool SetOrdinanceEnabled(uint32_t id, bool enabled) override;
    uint32_t GetOrdinanceIDArray(uint32_t* out, uint32_t& count) override;
    uint32_t GetOrdinanceIDByName(cIGZString const& name) override;
    cISC4Ordinance* GetOrdinanceByID(uint32_t id) override;
    cISC4Ordinance* GetOrdinanceByName(cIGZString const& name) override;
    int64_t GetOrdinanceMonthlyExpense() override;
    int64_t GetOrdinanceMonthlyIncome() override;
    int64_t GetCumulativeOffset(uint32_t prop, uint32_t index) override;
    float GetCumulativeFactor(uint32_t prop, uint32_t index) override;

    // cIGZMessageTarget2 (§5)
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable (§9.1)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // void1 of the posted messages (§7.1): the IGZUnknown.
    cIGZUnknown* AsUnknown() { return static_cast<cIGZUnknown*>(static_cast<cISC4OrdinanceSimulator*>(this)); }

    // Foreign ordinances read from a save (the host's foreign_ordinance): the
    // core holds the reference the save handed over, which is ours, so its
    // last Release is an expected free (FC4_RELEASE_CREATED), like the land
    // value grids after a save (in-game run 88). An AddOrdinance reference
    // (our AddRef) keeps FC4_RELEASE.
    void NoteSavedReference(uint64_t token) { ++savedRefs_[token]; }
    // True (and one fewer noted) when `token` still has a saved reference.
    bool TakeSavedReference(uint64_t token);

private:
    void RegisterAgent(bool add);
    void LogState(const char* what);

    uint32_t refCount_ = 0;
    bool destroying_ = false;
    fc4_ordinance_host host_{};
    fc4_ordinance_sim* core_ = nullptr;
    cISC4Simulator* simulator_ = nullptr;
    bool agentRegistered_ = false;
    std::unordered_map<uint64_t, uint32_t> savedRefs_;  // token -> saved references the core holds
};

// A standard ordinance of one of the classes of §3.6.
class cFC4Ordinance final : public cFC4ISC4Ordinance, public cIGZSerializable, public cIFC4CoreObject {
public:
    explicit cFC4Ordinance(uint32_t classId);
    ~cFC4Ordinance();

    // The token the core uses (the cISC4Ordinance*) and the core object.
    uint64_t Token() { return fc4::TokenOf(static_cast<cFC4ISC4Ordinance*>(this)); }
    fc4_ordinance* Core() const { return core_; }

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4Ordinance (§3.5)
    bool Init() override;
    bool Shutdown() override;
    int64_t GetCurrentMonthlyIncome() override;
    uint32_t GetID() const override;
    cIGZString* GetName() override;
    cIGZString* GetDescription() override;
    uint32_t GetYearFirstAvailable() override;
    uint32_t GetChanceAvailability() override;
    int64_t GetEnactmentIncome() override;
    int64_t GetRetracmentIncome() override;
    int64_t GetMonthlyConstantIncome() override;
    float GetMonthlyIncomeFactor() override;
    uint32_t GetAdvisorID() override;
    cISCPropertyHolder* GetMiscProperties() override;
    bool IsAvailable() override;
    bool IsOn() override;
    bool IsEnabled() override;
    int64_t GetMonthlyAdjustedIncome() override;
    bool CheckConditions() override;
    bool IsIncomeOrdinance() override;
    bool Simulate() override;
    bool SetAvailable(bool isAvailable) override;
    bool SetOn(bool isOn) override;
    bool SetEnabled(bool isEnabled) override;
    bool ForceAvailable(bool isAvailable) override;
    bool ForceOn(bool isOn) override;
    bool ForceMonthlyAdjustedIncome(int64_t monthlyAdjustedIncome) override;

    // cIGZSerializable (§9.2)
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // cIFC4CoreObject
    void* CoreHandle() override { return core_; }
    uint32_t CoreClass() override { return classId_; }

private:
    int64_t Call(uint32_t op, int64_t arg = 0) const;

    uint32_t refCount_ = 0;
    uint32_t classId_;
    fc4_ordinance* core_ = nullptr;
    cRZBaseString name_;
    cRZBaseString description_;
    // GetMiscProperties [choice]: the parent exemplar, loaded on first use.
    fc4::props::Exemplar misc_;
    bool miscLoaded_ = false;
    uint32_t miscKey_[3] = {0, 0, 0};
};

// The host table of the ordinance cores (one table, user = the simulator for
// the simulator's core, nullptr for the ordinances').
fc4_ordinance_host FC4MakeOrdinanceHost(cFC4OrdinanceSimulator* sim);

// Factories for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4OrdinanceSimulator();
template <uint32_t ClassId>
cIGZUnknown* CreateFC4Ordinance() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4Ordinance*>(new cFC4Ordinance(ClassId)));
}
