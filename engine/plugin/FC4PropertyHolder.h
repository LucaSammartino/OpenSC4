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

// A plain property holder we hand to the game (foundations §10.2, §17.2),
// embedded in another object (the neighbour connection's AsISCPropertyHolder,
// neighbors.md §4.2): cFC4ISCPropertyHolder in the game's compiled slot order,
// with cFC4Property (cISCProperty, foundations §10.1) entries holding a
// cFC4Variant (the game's variant slot order, never the SDK variant).
//
// Behaviour (foundations §10.2): the properties are kept sorted by ascending
// unsigned ID, at most one per ID; adding never fails; the bool arguments have
// no effect.
//
// Ownership (foundations §10.2 "Ownership", NG1 answered): the holder keeps
// OBJECTS, not copies. AddProperty(cISCProperty*) (slot 15, AddPropertyRecord)
// stores the caller's object itself with a reference of ours; an entry of the
// same ID is replaced and released. GetProperty (slot 9) returns the stored
// object borrowed, and the callers write its variant in place (the array data
// getters, slots 99 / 103, and the copying setter, slot 102): every read here
// (the getters, ToCore for the record) goes through the stored object's
// variant at the point of use, so those writes are seen. An entry is either
// one of ours (cFC4Property) or the caller's object (the game's cSCProperty);
// both are used through cISCProperty and the game's variant slot order only.
// The value adders (slots 10-14) and CopyAddProperty (16) write the value of
// the entry for that ID in place (a new cFC4Property if there is none)
// [choice]. AddPropertyData stores raw bytes (type 0x8000, count = length;
// neighbors.md §15.4 NG37), which GetPropertyData reads back (raw bytes only,
// no size query, NG23); GetPropertyList is not supported (false) [choice].
//
// Lifetime: AddRef / Release go to the owner (the object the holder is part
// of). The entries are released at removal, replacement and Clear (holder
// destruction); the holder may hold an entry's last reference (the caller
// released its own after AddProperty), so that release is the expected free
// (FC4_RELEASE_CREATED). Thread safety: a recursive lock per holder around the
// map; callbacks of EnumProperties run on a snapshot (each entry AddRef'd),
// outside the lock.
#pragma once

#include "FC4Interfaces.h"
#include "cFC4Variant.h"
#include "cISCProperty.h"
#include "fc4.h"

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <map>
#include <mutex>

class cIGZIStream;
class cIGZOStream;

static const uint32_t kFC4_IID_cISCPropertyHolder = 0x25216283;

// One property: an ID and a value (foundations §10.1). Reference counted;
// deleted at 0.
class cFC4Property final : public cISCProperty {
public:
    explicit cFC4Property(uint32_t id) : id_(id) {}
    cFC4Property(const cFC4Property&) = delete;
    cFC4Property& operator=(const cFC4Property&) = delete;

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override;

    // cISCProperty. The two GetPropertyValue overloads take nothing and
    // return the same pointer, so their compiled order does not matter.
    uint32_t GetPropertyID() const override { return id_; }
    void SetPropertyID(uint32_t value) override { id_ = value; }
    cIGZVariant* GetPropertyValue() override;
    const cIGZVariant* GetPropertyValue() const override;
    bool SetPropertyValue(const cIGZVariant& value) override;
    // §10.1: uint32 propertyID, then the variant (§9.2) through the segment
    // stream's WriteVariant / ReadVariant; "the stream is not in the error
    // state". False without the segment interface.
    bool Write(cIGZOStream& stream) const override;
    bool Read(cIGZIStream& stream) override;

    cFC4Variant& Value() { return value_; }
    const cFC4Variant& Value() const { return value_; }

private:
    ~cFC4Property() = default;

    std::atomic<uint32_t> refs_{0};
    uint32_t id_;
    cFC4Variant value_;
};

class cFC4PropertyHolder final : public cFC4ISCPropertyHolder {
public:
    // `owner`: the object this holder is part of (AddRef / Release and the
    // QueryInterface of every other IID go to it).
    explicit cFC4PropertyHolder(cIGZUnknown* owner) : owner_(owner) {}
    ~cFC4PropertyHolder();
    cFC4PropertyHolder(const cFC4PropertyHolder&) = delete;
    cFC4PropertyHolder& operator=(const cFC4PropertyHolder&) = delete;

    // The holder as the SDK type the game's callers declare.
    cISCPropertyHolder* AsSdk() { return reinterpret_cast<cISCPropertyHolder*>(static_cast<cFC4ISCPropertyHolder*>(this)); }

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISCPropertyHolder (foundations §17.2, compiled order)
    bool HasProperty(uint32_t id) const override;
    bool GetPropertyList(cIGZUnknownList** list) const override;
    bool GetPropertyData(uint32_t id, void* buffer, uint32_t& length) const override;
    bool GetPropertyInterface(uint32_t id, uint32_t riid, void** ppv) const override;
    bool GetPropertyString(uint32_t id, cIGZString& value) const override;
    bool GetPropertyU32(uint32_t id, uint32_t& value) const override;
    cISCProperty* GetProperty(uint32_t id) const override;
    bool AddPropertyData(uint32_t id, void* data, uint32_t length, bool b) override;
    bool AddPropertyObject(uint32_t id, cIGZUnknown* value, bool b) override;
    bool AddPropertyString(uint32_t id, cIGZString const& value, bool b) override;
    bool AddPropertyU32(uint32_t id, uint32_t value, bool b) override;
    bool AddPropertyVariant(uint32_t id, cFC4IGZVariant const* value, bool b) override;
    bool AddPropertyRecord(cISCProperty* property, bool b) override;
    bool CopyAddProperty(cISCProperty* property, bool b) override;
    bool RemoveProperty(uint32_t id, bool b) override;
    bool RemoveAllProperties(bool b) override;
    bool EnumPropertiesFiltered(FilterFn filter, EnumFn fn, void* data) const override;
    bool EnumProperties(EnumFn fn, void* data) const override;
    bool CompactProperties() override;

    // ---- the core's copy (fc4_props) ----
    // A NEW fc4_props with every property in the holder's order, with its
    // exact variant type and shape (fc4_props_set_typed); empty, pointer and
    // object values are left out (the core does not keep them). NG2
    // (neighbors.md §15.4, matches): the original drops an object-valued
    // property on load, so leaving it out of the core and the save is
    // invisible; this holder still keeps it while the city runs. The caller
    // destroys it.
    fc4_props* ToCore() const;
    // Replaces the holder's contents (the properties the core read from a
    // record; fc4_neighbor_connection_props delivers them through this).
    void Clear();
    static void FromCoreProperty(void* ctx, uint32_t id, uint16_t memType, bool isArray, const void* data,
                                 size_t count);
    size_t Count() const;
    // The exemplar parent (cSCExemplarPropertyHolder, occupant-manager §13.1):
    // the getters fall back to it for a property the holder does not have.
    // Borrowed; the owner keeps it alive and clears it first. Null by default.
    void SetParent(const cISCPropertyHolder* parent);

private:
    // The entry for `id`, a new (empty) cFC4Property if absent; the lock must be held.
    cISCProperty* Slot(uint32_t id);
    cISCProperty* Find(uint32_t id) const;
    // Find, then the parent's slot 9 (getters only); the lock must be held.
    cISCProperty* Lookup(uint32_t id) const;
    // The entry's variant (borrowed, the game's slot order); null if it has none.
    static cFC4IGZVariant* ValueOf(cISCProperty* p);
    void Drop(cISCProperty* p) const;  // releases one entry reference of ours

    cIGZUnknown* owner_;
    mutable std::recursive_mutex lock_;
    // Ascending ID; one reference of ours each: a cFC4Property, or the
    // caller's object stored by AddProperty(cISCProperty*).
    std::map<uint32_t, cISCProperty*> props_;
    const cISCPropertyHolder* parent_ = nullptr;
};

namespace fc4 {
// The value of a variant as uint32: a numeric scalar, or the first element of
// a numeric array (bools as 0 / 1). False for empty, string, pointer and
// object values and for empty arrays.
bool VariantU32(const cFC4IGZVariant& v, uint32_t* out);
// Whether a variant reads as true: a non-zero number (scalar or first
// element). False otherwise.
bool VariantTrue(const cFC4IGZVariant& v);
// Copies a game variant into a core property holder (fc4_props_set_typed),
// with its exact type and shape. False for empty, pointer and object values.
bool VariantToCore(const cFC4IGZVariant& v, uint32_t id, fc4_props* out);
}  // namespace fc4
