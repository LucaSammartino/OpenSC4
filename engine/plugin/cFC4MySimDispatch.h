// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

// The MySim dispatch (CLSID 0xCBC14674, mysims.md §4.6.1, §4.7.1, §6.2.1,
// §7.2.1-§7.2.3) over the fc4_mysim_dispatch core (`enable = mysims`).
//
// The primary unknown interface is the nine-entry table of §7.2.3 (IID
// 0xEB8B6F9A); the message target and the serializable answer 0x452294AA and
// 0xE4FDA3D4. Everything else of the dispatch (Init, the commute requests,
// the job search, the trip getters) is a direct concrete call (§7.2.3): here
// the owner MySim's front (cFC4MySim.cpp) reaches the concrete object through
// the private IID kFC4_IID_FC4MySimDispatchConcrete.
//
// The dispatch subscribes to the minute (0x44778231) and animation
// (0x86AD10EE) messages while it has a pathfinder (between Init and
// Shutdown), and to PostCityInit (0x26D31EC1) after a Read for the deferred
// setup. Transport (automata, stops, the player vehicle) is MS18 in the core:
// no automaton is ever dispatched and no arrival is manufactured.
#pragma once

#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "fc4.h"

#include <cstdint>

constexpr uint32_t kFC4_CLSID_cSC4MySimDispatch = 0xCBC14674;
constexpr uint32_t kFC4_IID_cSC4MySimDispatch = 0xEB8B6F9A;
// Private: QueryInterface returns the concrete cFC4MySimDispatch (AddRef'd).
constexpr uint32_t kFC4_IID_FC4MySimDispatchConcrete = 0xFC4D1574;

// §7.2.3: the primary vtable (slots 0-2 are cIGZUnknown's).
class cFC4IMySimDispatch : public cIGZUnknown {
public:
    virtual ~cFC4IMySimDispatch() = default;  // 3: scalar deleting destructor(flags)
    virtual uint32_t RemoveRef() = 0;          // 4: decrement a nonzero count, never destroy
    virtual uint32_t GetRefCount() = 0;        // 5
    virtual bool CanUDriveIt() = 0;            // 6
    virtual bool EnterUDriveIt() = 0;          // 7
    virtual bool ExitUDriveIt() = 0;           // 8
};

class cFC4MySimDispatch final : public cFC4IMySimDispatch, public cIGZMessageTarget2, public cIGZSerializable {
public:
    cFC4MySimDispatch();
    ~cFC4MySimDispatch() override;
    cFC4MySimDispatch(const cFC4MySimDispatch&) = delete;
    cFC4MySimDispatch& operator=(const cFC4MySimDispatch&) = delete;

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;
    uint32_t RemoveRef() override;
    uint32_t GetRefCount() override;
    bool CanUDriveIt() override;
    bool EnterUDriveIt() override;
    bool ExitUDriveIt() override;

    bool DoMessage(cIGZMessage2* msg) override;

    bool Write(cIGZOStream& out) override;
    bool Read(cIGZIStream& in) override;
    uint32_t GetGZCLSID() override { return kFC4_CLSID_cSC4MySimDispatch; }

    // ---- direct concrete calls (the owner MySim, §7.2.3) ----------------
    // The owner is the MySim's advisor in `owners` (0 = none: no delivery, §3.3).
    void SetOwner(fc4_advisors* owners, int32_t ownerId);
    bool Init();
    void Shutdown();
    void SetHome(uint64_t homeOccupant);
    uint32_t Commute(bool hasResidence, uint8_t mysimState, int32_t hour);  // FC4_MYSIM_TRIP_*
    bool FindNewJob();
    bool ChangeJob(uint64_t buildingOccupant);
    bool JobDestination(fc4_mysim_job_destination* out) const;
    bool Snapshot(fc4_mysim_dispatch* out) const;
    bool CheckFlags(uint32_t mask) const;
    bool State(fc4_mysim_dispatch_state* out) const;

    // The concrete object behind any of its interfaces (null for another class).
    static cFC4MySimDispatch* FromUnknown(cIGZUnknown* u);

    // Host plumbing (cFC4MySimDispatch.cpp's callbacks).
    fc4_advisors* Owners() const { return owners_; }
    int32_t OwnerId() const { return ownerId_; }

private:
    void Subscribe(uint32_t type, bool on);
    void Listen(bool on);

    uint32_t refCount_ = 0;
    fc4_mysim_dispatch_host host_{};
    fc4_mysim_dispatch_obj* core_ = nullptr;
    fc4_advisors* owners_ = nullptr;
    int32_t ownerId_ = 0;
    bool listening_ = false;
    bool setupPending_ = false;
};

cIGZUnknown* CreateFC4MySimDispatch();
