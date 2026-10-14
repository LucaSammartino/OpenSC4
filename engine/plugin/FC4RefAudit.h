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

// Reference-count audit of GAME objects (tooling only; no spec behaviour).
//
// Every AddRef / Release we do on an object whose vtable is outside
// FreeCity4.dll goes through FC4_ADDREF / FC4_RELEASE, and every reference
// the game hands us (QueryInterface, out-parameters, filled lists) is noted
// with FC4_ACQUIRED. For each event the verbose log (own cap) shows the
// object, its class (cIGZSerializable::GetGZCLSID, looked up once per
// vtable), the vtable as module+offset, the count the call returned, and our
// call site. Always logged (not verbose):
//   - "WARNING: released last reference to game object ..." when a Release
//     of ours returns 0, unless the site declares the object a temporary the
//     game created for us (FC4_RELEASE_CREATED: a new object returned through
//     an out-parameter with one reference, which is ours to free) or the
//     release one the spec puts in our object's destruction
//     (FC4_RELEASE_AT_DESTRUCTION: a reference the original also keeps until
//     its destructor, so the free there is the original's too) AND the
//     audited sites acquired that reference (FC4_ACQUIRED / FC4_ADDREF) first;
//   - "WARNING: over-release ..." when we released one object more often than
//     the audited sites acquired it.
// RefSummary (city shutdown) logs the net count per class and the objects we
// still hold. FreeCity4.ini `refaudit = 0` turns the audit off (the AddRef /
// Release calls themselves are unchanged).
#pragma once

#include "cIGZUnknown.h"

#include <cstdint>

namespace fc4 {

struct RefSite {
    const char* func;
    int line;
};

// One of our classes (vtable inside FreeCity4.dll).
bool IsOurObject(const void* obj);

struct RefTarget {
    const void* obj = nullptr;
    uintptr_t vptr = 0;
    bool audited = false;  // a game object and the audit is on
};

// Before the call (reads the vtable; looks the class up while the object is alive).
RefTarget RefBefore(cIGZUnknown* obj);
// After the call. `newCount` < 0 = unknown. `how` is a string literal.
// `owned` = false for a reference handed on to the caller (not ours to release).
// `created` (a Release only) = the object is a temporary the game created and
// handed to us with its only reference: freeing it is expected, no warning,
// provided the audited sites acquired that reference.
void RefNote(const RefTarget& t, int delta, long newCount, const char* how, RefSite site, bool owned = true,
             bool created = false);

void RefSummary(const char* why);
// "released last reference" and over-release warnings so far (any thread).
long RefWarningCount();
// Re-reads `refaudit` / `refaudit_lines` (city load).
void RefAuditReload();

template <class T>
uint32_t AuditedAddRef(T* p, RefSite s) {
    if (p == nullptr) return 0;
    const RefTarget t = RefBefore(p);
    const uint32_t r = p->AddRef();
    RefNote(t, +1, static_cast<long>(r), "AddRef", s);
    return r;
}

// An AddRef for a reference we hand to the game (it releases it).
template <class T>
uint32_t AuditedAddRefForCaller(T* p, RefSite s) {
    if (p == nullptr) return 0;
    const RefTarget t = RefBefore(p);
    const uint32_t r = p->AddRef();
    RefNote(t, +1, static_cast<long>(r), "AddRef for the caller", s, false);
    return r;
}

template <class T>
uint32_t AuditedRelease(T* p, RefSite s) {
    if (p == nullptr) return 0;
    const RefTarget t = RefBefore(p);
    const uint32_t r = p->Release();
    RefNote(t, -1, static_cast<long>(r), "Release", s);
    return r;
}

// The Release of a temporary the game created for us (a new key list, ...):
// its last reference is ours, so reaching 0 is the expected free.
template <class T>
uint32_t AuditedReleaseCreated(T* p, RefSite s) {
    if (p == nullptr) return 0;
    const RefTarget t = RefBefore(p);
    const uint32_t r = p->Release();
    RefNote(t, -1, static_cast<long>(r), "Release (created for us)", s, true, true);
    return r;
}

// The Release the spec places in the destruction of one of our objects, of
// a reference our object took (the growth developer's special-building copy,
// growth-developer.md §1.3): when it is the last one, the original's
// destructor frees the object at the same point (DG28).
template <class T>
uint32_t AuditedReleaseAtDestruction(T* p, RefSite s) {
    if (p == nullptr) return 0;
    const RefTarget t = RefBefore(p);
    const uint32_t r = p->Release();
    RefNote(t, -1, static_cast<long>(r), "Release (at our destruction)", s, true, true);
    return r;
}

template <class T>
void AuditedAcquired(T* p, const char* how, RefSite s) {
    if (p == nullptr) return;
    RefNote(RefBefore(p), +1, -1, how, s);
}

}  // namespace fc4

#define FC4_SITE (::fc4::RefSite{__FUNCTION__, __LINE__})
#define FC4_ADDREF(p) ::fc4::AuditedAddRef((p), FC4_SITE)
#define FC4_ADDREF_FOR_CALLER(p) ::fc4::AuditedAddRefForCaller((p), FC4_SITE)
#define FC4_RELEASE(p) ::fc4::AuditedRelease((p), FC4_SITE)
#define FC4_RELEASE_CREATED(p) ::fc4::AuditedReleaseCreated((p), FC4_SITE)
#define FC4_RELEASE_AT_DESTRUCTION(p) ::fc4::AuditedReleaseAtDestruction((p), FC4_SITE)
#define FC4_ACQUIRED(p, how) ::fc4::AuditedAcquired((p), (how), FC4_SITE)
