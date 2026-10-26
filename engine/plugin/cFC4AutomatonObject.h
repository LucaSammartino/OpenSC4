// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR
// PURPOSE. See the GNU General Public License for more details. You should have
// received a copy of the GNU General Public License along with this program; if
// not, see <https://www.gnu.org/licenses/>.

// The automaton object wrapper (docs/specs/automata.md 6, 7.2): the GZCOM
// object over one fc4_automata_object handle that cFC4AutomataManager's
// CreateAutomaton (slot 15) returns, and that a record's `ref` automaton
// (0xA9B40F05) resolves to when the object is one of ours. The token the core
// writes for it is this object's cFC4ISC4Automaton sub-object pointer
// (fc4_automata_object_set_token), so a re-save writes the same game object
// the original would.
//
// Scope is 0's: the records and the hooks. The interface is the sparse
// cFC4ISC4Automaton of FC4AutomataInterfaces.h (2 Release, 5 AsOccupant,
// 7/8 Get/SetPosition, 9 SetVisible, 14 SetOrientation, 30 SetAutomataFlag,
// 51 SetState) plus cIGZSerializable (Write/Read of the
// object's own record, 6.4) and cIFC4CoreObject (the handle for
// OurObjectRef). Movement and animation are not implemented; AsOccupant
// returns null (the location entry then carries no occupant, disasters.md
// 2.7 - the location list is not saved and the instance teardown clears it).
//
// Lifetime: created with reference count 0 (the GZCOM factory convention,
// 7.2: the returned pointer holds no reference for the caller). Release
// never deletes the object and never touches the core handle: the owning
// manager keeps every wrapper it created alive until the city closes, so a
// token the core holds stays valid for the whole city session.
#pragma once

#include "FC4AutomataInterfaces.h"

#include "FC4Streams.h"  // cIFC4CoreObject, TokenOf

#include "cIGZSerializable.h"
#include "cS3DVector3.h"

#include <cstdint>
#include <mutex>

class cIGZOStream;
class cIGZIStream;
struct fc4_automata_object;

// The classes an automaton object answers its CLSID with (6.4; the emergency
// vehicle answers the vehicle's, 12 quirk 3).
uint32_t FC4AutomatonClassOfHandle(const fc4_automata_object* handle);

class cFC4AutomatonObject final : public cFC4ISC4Automaton,
                                  public cIGZSerializable,
                                  public cIFC4CoreObject {
public:
    // `handle` is a handle this object takes over (one reference; destroyed
    // with the wrapper, after the manager core has gone).
    cFC4AutomatonObject(uint32_t classId, fc4_automata_object* handle);
    ~cFC4AutomatonObject();

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // Unused slots retain their indices; construction setters still store
    // their inputs even though this replacement does not animate models.
    void Slot03() override;
    void Slot04() override;
    void Slot06() override;
    cS3DVector3* GetPosition(cS3DVector3& position) override;
    void SetPosition(cS3DVector3 const& position) override;
    void Slot10() override;
    void Slot11() override;
    void Slot12() override;
    void Slot13() override;
    void SetOrientation(cS3DVector3 const& orientation) override;
    void Slot15() override;
    void Slot16() override;
    void Slot17() override;
    void Slot18() override;
    void Slot19() override;
    void Slot20() override;
    void Slot21() override;
    void Slot22() override;
    void Slot23() override;
    void Slot24() override;
    void Slot25() override;
    void Slot26() override;
    void Slot27() override;
    void Slot28() override;
    void Slot29() override;
    void SetAutomataFlag(uint32_t mask, bool on) override;
    void Slot31() override;
    void Slot32() override;
    void Slot33() override;
    void Slot34() override;
    void Slot35() override;
    void Slot36() override;
    void Slot37() override;
    void Slot38() override;
    void Slot39() override;
    void Slot40() override;
    void Slot41() override;
    void Slot42() override;
    void Slot43() override;
    void Slot44() override;
    void Slot45() override;
    void Slot46() override;
    void Slot47() override;
    void Slot48() override;
    void Slot49() override;
    void Slot50() override;

    // cFC4ISC4Automaton: the sparse contract (5, 9, 51).
    cISC4Occupant* AsOccupant() override;  // 5
    void SetVisible(bool on) override;  // 9
    bool SetState(uint32_t type, uint32_t a, float b, uint32_t c, uint32_t d, uint32_t e, uint32_t f) override;  // 51

    // cIGZSerializable: the object's own record (6.4).
    bool Write(cIGZOStream& out) override;
    bool Read(cIGZIStream& in) override;
    uint32_t GetGZCLSID() override;

    // cIFC4CoreObject (FC4Streams.h): the handle OurObjectRef hands the core.
    void* CoreHandle() override { return handle_; }
    uint32_t CoreClass() override { return classId_; }

private:
    void RefreshPosition();
    // The getter can run on the game's second thread; never read Rust's
    // RefCell-backed record there. Setters and Read publish copied values.
    std::mutex positionMutex_;
    cS3DVector3 position_;
    uint32_t refCount_ = 0;
    uint32_t classId_ = 0;
    fc4_automata_object* handle_ = nullptr;
};
