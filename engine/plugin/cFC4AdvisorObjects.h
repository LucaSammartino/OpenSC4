// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.
//
// The GZCOM fronts of core advisors (cISC4Advisor, advisors.md §1.1, §12.2)
// and advices (cISC4Advice, §12.3), built in object_created kinds 2 and 3.
// The core holds the state; every slot forwards to the fc4_adv_advisor_* /
// fc4_adv_advice_* calls on the front's handle. Lifetime: fc4::AdvFront.
// The interfaces are declared here with the game's slot order (§12.2/§12.3
// list the SDK's errors), not taken from the SDK headers.

#pragma once

#include "FC4AdvLife.h"
#include "FC4AdvisorMaps.h"
#include "FC4GameList.h"  // fc4::StlNodeAllocator
#include "FC4Streams.h"  // cIFC4CoreObject
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4AdviceSubject.h"
#include "cRZBaseString.h"

#include <cstdint>

constexpr uint32_t kFC4_IID_cISC4Advisor = 0xCA09F544;
constexpr uint32_t kFC4_IID_cISC4Advice = 0x8A09F5F5;
constexpr uint32_t kFC4_IID_cIGZMessageTarget2 = 0x452294AA;  // §1.1

// §12.3, vtable 0x00A808B8, 31 slots.
class cFC4ISC4Advice : public cIGZUnknown {
public:
    virtual bool Init(int32_t tableIndex) = 0;                   // 3
    virtual bool Shutdown() = 0;                                 // 4
    virtual bool Clone(cFC4ISC4Advice** out) = 0;                // 5
    virtual int32_t GetMood() = 0;                               // 6
    virtual int32_t GetPriority() = 0;                           // 7
    virtual bool GetTrigger() = 0;                               // 8 (AL)
    virtual cIGZString* GetTriggerString() = 0;                  // 9 (string object)
    virtual uint32_t GetGUID() = 0;                              // 10
    virtual int32_t GetAdvisorID() = 0;                          // 11
    virtual cFC4ISC4Advice* SetAdvisorID(int32_t id) = 0;        // 12 (§12.3: return this)
    virtual int32_t GetSessionID() = 0;                          // 13
    virtual bool SetFlags(uint32_t mask, bool on) = 0;           // 14
    virtual bool CheckFlags(uint32_t mask) = 0;                  // 15
    virtual int32_t GetFrequencyInDays() = 0;                    // 16
    virtual int32_t GetTimeOutInDays() = 0;                      // 17
    virtual cIGZString* GetTextTitle() = 0;                      // 18
    virtual uint32_t GetTextTitleGUID() = 0;                     // 19
    virtual cIGZString* GetTextMessage() = 0;                    // 20
    virtual uint32_t GetTextMessageGUID() = 0;                   // 21
    virtual bool DoLink(const cIGZString& link) = 0;             // 22
    virtual void NotifyGlobalEffectTriggerChanged() = 0;         // 23 (void)
    virtual bool SetEventSubject(cISC4AdviceSubject* s) = 0;     // 24
    virtual cISC4AdviceSubject* GetEventSubject() = 0;           // 25
    virtual int32_t GetEventID() = 0;                            // 26
    virtual int32_t GetCommandID() = 0;                          // 27
    virtual bool UpdateText() = 0;                               // 28 (tail call to 30)
    virtual ~cFC4ISC4Advice() = default;                         // 29 destructor thunk
    virtual bool RebuildText() = 0;                              // 30 class text rebuild
};

// §12.2, iface vtable 0x00A811B0, 21 slots.
class cFC4ISC4Advisor : public cIGZUnknown {
public:
    virtual bool Init(int32_t tableIndex) = 0;                          // 3
    virtual bool Shutdown() = 0;                                        // 4
    virtual int32_t GetID() = 0;                                        // 5
    virtual cIGZString* GetName() = 0;                                  // 6 caption object
    virtual bool SetName(const cIGZString& s) = 0;                      // 7
    virtual cIGZString* GetPersonalName() = 0;                          // 8
    virtual bool SetPersonalName(const cIGZString& s) = 0;              // 9
    virtual void* GetCurrentAdvices() = 0;                              // 10 game-layout map&
    virtual void* GetCurrentAdvicesConst() = 0;                         // 11 (one body)
    virtual cFC4ISC4Advice* GetAdviceByGUID(uint32_t guid) = 0;         // 12
    virtual cFC4ISC4Advice* GetAdviceBySessionID(int32_t id) = 0;       // 13
    virtual bool Simulate(int32_t kind) = 0;                            // 14
    virtual int32_t GetHeadAnimCount(bool b) = 0;                       // 15
    virtual bool SetHeadAnim(bool b, int32_t i) = 0;                    // 16
    virtual int32_t GetHeadAnimID(int32_t i) = 0;                       // 17
    virtual bool PushEvent(int32_t event, cISC4AdviceSubject* s) = 0;   // 18
    virtual bool CommandWouldTrigger(uint32_t command) = 0;             // 19
    virtual bool RetireAdvice(cFC4ISC4Advice* advice) = 0;              // 20
};

class cFC4Advice final : public cFC4ISC4Advice, public cIGZSerializable, public cIFC4CoreObject, public fc4::AdvFront {
public:
    explicit cFC4Advice(fc4_adv_obj* handle);

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override { return AddRefFront(); }
    uint32_t Release() override { return ReleaseFront(); }  // may delete this

    bool Init(int32_t tableIndex) override;
    bool Shutdown() override;
    bool Clone(cFC4ISC4Advice** out) override;
    int32_t GetMood() override;
    int32_t GetPriority() override;
    bool GetTrigger() override;
    cIGZString* GetTriggerString() override;
    uint32_t GetGUID() override;
    int32_t GetAdvisorID() override;
    cFC4ISC4Advice* SetAdvisorID(int32_t id) override;
    int32_t GetSessionID() override;
    bool SetFlags(uint32_t mask, bool on) override;
    bool CheckFlags(uint32_t mask) override;
    int32_t GetFrequencyInDays() override;
    int32_t GetTimeOutInDays() override;
    cIGZString* GetTextTitle() override;
    uint32_t GetTextTitleGUID() override;
    cIGZString* GetTextMessage() override;
    uint32_t GetTextMessageGUID() override;
    bool DoLink(const cIGZString& link) override;
    void NotifyGlobalEffectTriggerChanged() override;
    bool SetEventSubject(cISC4AdviceSubject* s) override;
    cISC4AdviceSubject* GetEventSubject() override;
    int32_t GetEventID() override;
    int32_t GetCommandID() override;
    bool UpdateText() override { return RebuildText(); }
    bool RebuildText() override;

    // rebuild_text (§6.9): the strings slots 18 and 20 return.
    void SetText(const char* title, size_t tn, const char* message, size_t mn);
    int32_t TableIndex() {
        fc4_advice_state s{};
        return State(&s) ? s.table_index : 0;
    }

    bool Write(cIGZOStream& out) override;
    bool Read(cIGZIStream& in) override;
    uint32_t GetGZCLSID() override;

    void* CoreHandle() override { return StrongHandle(); }
    uint32_t CoreClass() override { return GetGZCLSID(); }

    uint64_t Token() { return fc4::TokenOf(static_cast<cIGZUnknown*>(static_cast<cFC4ISC4Advice*>(this))); }

private:
    bool State(struct fc4_advice_state* out);
    cRZBaseString trigger_, title_, message_;
};

class cFC4Advisor : public cFC4ISC4Advisor,
                          public cIGZSerializable,
                          public cIGZMessageTarget2,
                          public cIFC4CoreObject,
                          public fc4::AdvFront {
public:
    explicit cFC4Advisor(fc4_adv_obj* handle);

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override { return AddRefFront(); }
    uint32_t Release() override { return ReleaseFront(); }  // may delete this

    bool Init(int32_t tableIndex) override;
    bool Shutdown() override;
    int32_t GetID() override;
    cIGZString* GetName() override;
    bool SetName(const cIGZString& s) override;
    cIGZString* GetPersonalName() override;
    bool SetPersonalName(const cIGZString& s) override;
    void* GetCurrentAdvices() override;
    void* GetCurrentAdvicesConst() override { return GetCurrentAdvices(); }
    cFC4ISC4Advice* GetAdviceByGUID(uint32_t guid) override;
    cFC4ISC4Advice* GetAdviceBySessionID(int32_t id) override;
    bool Simulate(int32_t kind) override;
    int32_t GetHeadAnimCount(bool b) override;
    bool SetHeadAnim(bool b, int32_t i) override;
    int32_t GetHeadAnimID(int32_t i) override;
    bool PushEvent(int32_t event, cISC4AdviceSubject* s) override;
    bool CommandWouldTrigger(uint32_t command) override;
    bool RetireAdvice(cFC4ISC4Advice* advice) override;

    bool DoMessage(cIGZMessage2* m) override;

    bool Write(cIGZOStream& out) override;
    bool Read(cIGZIStream& in) override;
    uint32_t GetGZCLSID() override;

    void* CoreHandle() override { return StrongHandle(); }
    uint32_t CoreClass() override { return GetGZCLSID(); }

    uint64_t Token() { return fc4::TokenOf(static_cast<cIGZUnknown*>(static_cast<cFC4ISC4Advisor*>(this))); }

private:
    cRZBaseString name_, personal_;
    fc4::AdvisorGameMap<fc4::StlNodeAllocator> active_{fc4::AdvisorMapOrder::UnsignedKey};  // slots 10/11 mirror
};

// The front behind a token the core handed out (null for none / another kind).
cFC4ISC4Advice* FC4AdviceFront(uint64_t token);
