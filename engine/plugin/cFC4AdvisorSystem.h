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

// The advisor system replacement (docs/specs/advisors.md §1.1, §12.1): the
// GZCOM object over the Rust core (fc4_advisors_*), its fc4_advisor_host over
// the app's Lua context (cISCLua), the message server, the cheat manager and
// the clock, slot 8 through AdvisorGameMap (FC4AdvisorMaps.h) and slots 23-28
// through AdvisorWindowSlot (FC4AdvisorWindow.h).
//
// Not registered yet (no `enable = advisors` row): the advisor, advice and
// subject classes the core's objects need (object_created for kinds 2-4) are
// the next round; until they exist this object is only built by tests.
#pragma once

#include "FC4AdvisorApi.h"
#include "FC4AdvisorMaps.h"
#include "FC4AdvisorWindow.h"
#include "FC4GameList.h"
#include "FC4GameMap.h"
#include "FC4HistClock.h"
#include "cIGZMessageTarget.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <cstdint>

class cIGZString;
class cISC4Advice;
class cISC4AdviceSubject;
class cISC4Advisor;
class cISCLua;

static const uint32_t kFC4_CLSID_cSC4AdvisorSystem = 0x4A0DD616;
static const uint32_t kFC4_IID_cISC4AdvisorSystem = 0x4A0DD617;

// §12.1, vtable 0x00A82AC8, 29 slots, in the exe's order. The SDK header
// differs (§12.1 "SDK errors"): slots 12/13 are reversed there, slot 18
// returns nothing, slots 23-28 are missing; distinct names keep MSVC from
// regrouping the overload pair.
class cFC4ISC4AdvisorSystem : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                             // 3
    virtual bool Shutdown() = 0;                                                         // 4
    virtual bool AddAdvisor(cISC4Advisor* a) = 0;                                        // 5
    virtual bool RemoveAdvisor(int32_t id) = 0;                                          // 6
    virtual cISC4Advisor* GetAdvisor(int32_t id) = 0;                                    // 7 (no AddRef)
    virtual fc4::GameMapObject* GetAdvisors() = 0;                                            // 8 (a reference in the exe)
    virtual void* GetRandom() = 0;                                                       // 9
    virtual cISCLua* GetScriptingContext() = 0;                                          // 10
    virtual bool DoLink(const cIGZString& link) = 0;                                     // 11
    virtual bool SetGlobalValueString(const char* name, cIGZString* value) = 0;          // 12
    virtual bool SetGlobalValueNumber(const char* name, double value) = 0;               // 13
    virtual bool SetLocalValue(const char* name, double value, cISC4AdviceSubject* s) = 0;  // 14
    virtual bool PushEvent(int32_t eventId, cISC4AdviceSubject* s) = 0;                  // 15
    virtual bool SubscribeToEvent(int32_t eventId, cISC4Advisor* a) = 0;                 // 16
    virtual bool UnsubscribeFromEvent(int32_t eventId, cISC4Advisor* a) = 0;             // 17
    virtual void PushContextAdvice(cISC4Advice* advice) = 0;                             // 18
    virtual cISC4Advice* PopContextAdvice() = 0;                                         // 19
    virtual cISC4Advice* GetContextAdvice() = 0;                                         // 20
    virtual int32_t GetAdvicePopupPriorityThreshold() = 0;                               // 21
    virtual int32_t GetDataTag(int32_t i) = 0;                                           // 22
    virtual void* GetAdvisorWinProc() = 0;                                               // 23
    virtual void StartAdvisorWindow() = 0;                                               // 24
    virtual void ShowAdvisorWindow() = 0;                                                // 25
    virtual void HideAdvisorWindow() = 0;                                                // 26
    virtual bool IsAdvisorWindowVisible() = 0;                                           // 27
    virtual void RemoveWinProcs() = 0;                                                   // 28
};

class cFC4AdvisorSystem;

// §1.1: cIGZMessageTarget at +8 (the cheat notifications).
class cFC4AdvisorCheatHandler final : public cIGZMessageTarget {
public:
    explicit cFC4AdvisorCheatHandler(cFC4AdvisorSystem* owner) : owner_(owner) {}
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;
    bool DoMessage(cGZMessage& msg) override;

private:
    cFC4AdvisorSystem* owner_;
};

class cFC4AdvisorSystem final : public cFC4ISC4AdvisorSystem, public cIGZMessageTarget2, public cIGZSerializable {
public:
    cFC4AdvisorSystem();
    ~cFC4AdvisorSystem();

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    bool Init() override;
    bool Shutdown() override;
    bool AddAdvisor(cISC4Advisor* a) override;
    bool RemoveAdvisor(int32_t id) override;
    cISC4Advisor* GetAdvisor(int32_t id) override;
    fc4::GameMapObject* GetAdvisors() override;
    void* GetRandom() override;
    cISCLua* GetScriptingContext() override;
    bool DoLink(const cIGZString& link) override;
    bool SetGlobalValueString(const char* name, cIGZString* value) override;
    bool SetGlobalValueNumber(const char* name, double value) override;
    bool SetLocalValue(const char* name, double value, cISC4AdviceSubject* s) override;
    bool PushEvent(int32_t eventId, cISC4AdviceSubject* s) override;
    bool SubscribeToEvent(int32_t eventId, cISC4Advisor* a) override;
    bool UnsubscribeFromEvent(int32_t eventId, cISC4Advisor* a) override;
    void PushContextAdvice(cISC4Advice* advice) override;
    cISC4Advice* PopContextAdvice() override;
    cISC4Advice* GetContextAdvice() override;
    int32_t GetAdvicePopupPriorityThreshold() override;
    int32_t GetDataTag(int32_t i) override;
    void* GetAdvisorWinProc() override;
    void StartAdvisorWindow() override;
    void ShowAdvisorWindow() override;
    void HideAdvisorWindow() override;
    bool IsAdvisorWindowVisible() override;
    void RemoveWinProcs() override;

    bool DoMessage(cIGZMessage2* pMessage) override;

    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    cIGZUnknown* Primary() { return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4AdvisorSystem*>(this)); }
    cIGZMessageTarget2* Target() { return static_cast<cIGZMessageTarget2*>(this); }
    cIGZMessageTarget* CheatTarget() { return &cheat_; }
    fc4_advisors* Core() const { return core_; }
    fc4_advisors_state State() const;

    // host plumbing (cFC4AdvisorSystem.cpp's callbacks)
    cISCLua* Lua() const { return lua_; }
    void AcquireLua();
    void ReleaseLua();
    void Subscribe(uint32_t msgType, bool on);
    void Cheat(uint32_t id, const char* name, bool on);
    void WindowCallback(bool start);
    void ReloadScripts();  // §4.2 steps 1-3 (AD10)

private:
    uint32_t refCount_ = 0;
    bool destroying_ = false;
    bool initialised_ = false;  // Init without a matching Shutdown (the last Release shuts down first)
    fc4_advisor_host host_{};
    fc4_mysim_host mysimHost_{};  // `enable = mysims` only
    fc4_advisors* core_ = nullptr;
    fc4_adv_obj* self_ = nullptr;  // the core's system object (token = Primary())
    cISCLua* lua_ = nullptr;      // the app's Lua context, one reference held between Init and Shutdown
    cFC4AdvisorCheatHandler cheat_;
    fc4::AdvisorWindowSlot window_;
    fc4::CoreLock lock_;
    fc4::AdvisorGameMap<fc4::StlNodeAllocator> map_{fc4::AdvisorMapOrder::SignedKey};  // slot 8 mirror
    fc4::AdvisorSystemGlobalSlot global_{nullptr};
    fc4::advisors::RZRandomShape random_{};  // §9 / §12.1 slot 9 (AD8, FC4AdvisorApi.h)
};

cIGZUnknown* CreateFC4AdvisorSystem();
