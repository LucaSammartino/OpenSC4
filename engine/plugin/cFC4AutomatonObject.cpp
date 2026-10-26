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

// The automaton object wrapper (cFC4AutomatonObject.h; automata.md 6, 7.2).
#include "cFC4AutomatonObject.h"

#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"  // OStreamAdapter, IStreamAdapter, kFC4_IID_CoreObject

#include "cGZPersistResourceKey.h"
#include "cIGZUnknown.h"
#include "cISC4Automaton.h"
#include "cISC4Occupant.h"
#include "cS3DVector3.h"

#include <cinttypes>
#include <cstdint>

// The automaton interface IID of 1 (the read's ref check) and the write
// check's serializable IID.
constexpr uint32_t kFC4_IID_cISC4Automaton = 0xA9B40F05;

// The refs a record read holds are the host's (fc4.h): add_ref / release on
// the tokens, which are game pointers or our own wrappers' sub-objects.
namespace {
void AutomatonRefAdd(void* /*user*/, uint64_t token) {
    cIGZUnknown* o = reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(token));
    if (o != nullptr) FC4_ADDREF(o);
}
void AutomatonRefRelease(void* /*user*/, uint64_t token) {
    cIGZUnknown* o = reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(token));
    if (o != nullptr) FC4_RELEASE(o);
}
}  // namespace

uint32_t FC4AutomatonClassOfHandle(const fc4_automata_object* handle) {
    // An emergency vehicle answers the vehicle's class (12 quirk 3), so the
    // handle's own GetGZCLSID is the wrapper's class in every case.
    return fc4_automata_object_class(handle);
}

cFC4AutomatonObject::cFC4AutomatonObject(uint32_t classId, fc4_automata_object* handle)
    : classId_(classId), handle_(handle) {
    fc4::Log("cFC4AutomatonObject(%p, class 0x%08X) created, handle %p", static_cast<void*>(this), classId,
             static_cast<void*>(handle));
}

cFC4AutomatonObject::~cFC4AutomatonObject() {
    // The handle is this object's own reference; the manager's core (which
    // holds the object alive through its list) has already gone.
    if (handle_ != nullptr) fc4_automata_object_destroy(handle_);
    handle_ = nullptr;
}

bool cFC4AutomatonObject::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    *ppvObj = nullptr;
    if (riid == GZIID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else if (riid == kFC4_IID_cISC4Automaton) {
        *ppvObj = static_cast<cFC4ISC4Automaton*>(this);
    } else if (riid == kFC4_IID_CoreObject) {
        *ppvObj = static_cast<cIFC4CoreObject*>(this);
    } else if (riid == 1 || riid == classId_) {
        *ppvObj = static_cast<cIGZUnknown*>(static_cast<cFC4ISC4Automaton*>(this));
    } else {
        return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4AutomatonObject::AddRef() { return ++refCount_; }

uint32_t cFC4AutomatonObject::Release() {
    // Never deleted (cFC4AutomatonObject.h): the owning manager keeps every
    // wrapper it made alive for the whole city session, so a token the core
    // holds stays valid.
    if (refCount_ != 0) --refCount_;
    return refCount_;
}

cISC4Occupant* cFC4AutomatonObject::AsOccupant() {
    // The replacement has no occupant sub-object (cFC4AutomatonObject.h).
    // The riot's location entry then carries a null occupant ref (disasters
    // 2.7); the entries are not saved and UnregisterLocations clears them.
    fc4::Log("cFC4AutomatonObject(%p)::AsOccupant -> null", static_cast<void*>(this));
    return nullptr;
}

// The placeholder slots (0): the replacement does not animate, so each
// answers nothing.
void cFC4AutomatonObject::Slot03() {}
void cFC4AutomatonObject::Slot04() {}
void cFC4AutomatonObject::Slot06() {}
void cFC4AutomatonObject::Slot07() {}
void cFC4AutomatonObject::Slot08() {}
void cFC4AutomatonObject::Slot10() {}
void cFC4AutomatonObject::Slot11() {}
void cFC4AutomatonObject::Slot12() {}
void cFC4AutomatonObject::Slot13() {}
void cFC4AutomatonObject::Slot14() {}
void cFC4AutomatonObject::Slot15() {}
void cFC4AutomatonObject::Slot16() {}
void cFC4AutomatonObject::Slot17() {}
void cFC4AutomatonObject::Slot18() {}
void cFC4AutomatonObject::Slot19() {}
void cFC4AutomatonObject::Slot20() {}
void cFC4AutomatonObject::Slot21() {}
void cFC4AutomatonObject::Slot22() {}
void cFC4AutomatonObject::Slot23() {}
void cFC4AutomatonObject::Slot24() {}
void cFC4AutomatonObject::Slot25() {}
void cFC4AutomatonObject::Slot26() {}
void cFC4AutomatonObject::Slot27() {}
void cFC4AutomatonObject::Slot28() {}
void cFC4AutomatonObject::Slot29() {}
void cFC4AutomatonObject::Slot30() {}
void cFC4AutomatonObject::Slot31() {}
void cFC4AutomatonObject::Slot32() {}
void cFC4AutomatonObject::Slot33() {}
void cFC4AutomatonObject::Slot34() {}
void cFC4AutomatonObject::Slot35() {}
void cFC4AutomatonObject::Slot36() {}
void cFC4AutomatonObject::Slot37() {}
void cFC4AutomatonObject::Slot38() {}
void cFC4AutomatonObject::Slot39() {}
void cFC4AutomatonObject::Slot40() {}
void cFC4AutomatonObject::Slot41() {}
void cFC4AutomatonObject::Slot42() {}
void cFC4AutomatonObject::Slot43() {}
void cFC4AutomatonObject::Slot44() {}
void cFC4AutomatonObject::Slot45() {}
void cFC4AutomatonObject::Slot46() {}
void cFC4AutomatonObject::Slot47() {}
void cFC4AutomatonObject::Slot48() {}
void cFC4AutomatonObject::Slot49() {}
void cFC4AutomatonObject::Slot50() {}

void cFC4AutomatonObject::SetVisible(bool on) {
    fc4_automata_automaton_set_visible(handle_, on);
}

bool cFC4AutomatonObject::SetState(uint32_t type, uint32_t a, float b, uint32_t c, uint32_t d, uint32_t e,
                                   uint32_t f) {
    const bool changed = fc4_automata_automaton_set_state(handle_, type, a, b, c, d, e, f);
    fc4::Log("cFC4AutomatonObject(%p)::SetState(%u) -> %d", static_cast<void*>(this), type, changed ? 1 : 0);
    return changed;
}

bool cFC4AutomatonObject::Write(cIGZOStream& out) {
    fc4::OStreamAdapter a(out);
    const bool ok = handle_ != nullptr && fc4_automata_object_write(handle_, a.Get());
    fc4::Log("cFC4AutomatonObject(0x%08X) Write -> %d", classId_, ok ? 1 : 0);
    return ok;
}

bool cFC4AutomatonObject::Read(cIGZIStream& in) {
    fc4::IStreamAdapter a(in);
    // The refs read are the host's (fc4.h): add_ref / release on tokens.
    fc4_automata_ref_host h{};
    h.struct_size = sizeof(h);
    h.add_ref = &AutomatonRefAdd;
    h.release = &AutomatonRefRelease;
    const bool ok = handle_ != nullptr && fc4_automata_object_read(handle_, a.Get(), &h);
    fc4::Log("cFC4AutomatonObject(0x%08X) Read -> %d", classId_, ok ? 1 : 0);
    return ok;
}

uint32_t cFC4AutomatonObject::GetGZCLSID() { return classId_; }
