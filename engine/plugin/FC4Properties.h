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

// Exemplar and property helpers shared by every replaced subsystem: load a
// tuning exemplar through the resource manager, read numeric properties with
// type conversion, and write numeric arrays back as instance overrides.
#pragma once

#include <cstdint>
#include <vector>

class cISCPropertyHolder;
class cISCResExemplar;
class cIGZVariant;
struct fc4_props;

namespace fc4 {
namespace props {

// A loaded exemplar resource (released on destruction).
class Exemplar {
public:
    Exemplar() = default;
    ~Exemplar();
    Exemplar(const Exemplar&) = delete;
    Exemplar& operator=(const Exemplar&) = delete;

    // Loads type/group/instance through the resource manager. Logs on failure.
    bool Load(uint32_t type, uint32_t group, uint32_t instance);

    // The exemplar as a property holder (nullptr if not loaded).
    const cISCPropertyHolder* Holder() const;
    // The resource itself (nullptr if not loaded; no AddRef): what
    // cISC4NeighborsSimulator::GetTuningExemplar hands out (neighbors.md §6.2).
    cISCResExemplar* Resource() const { return res_; }

private:
    cISCResExemplar* res_ = nullptr;
};

// Numeric reads. Integer and float variant types are converted to the
// requested type; scalars read as one-element arrays. Return false if the
// property is absent or not numeric (the output is then left empty).
bool GetU32s(const cISCPropertyHolder* holder, uint32_t id, std::vector<uint32_t>& out);
bool GetI64s(const cISCPropertyHolder* holder, uint32_t id, std::vector<int64_t>& out);
bool GetF32s(const cISCPropertyHolder* holder, uint32_t id, std::vector<float>& out);
bool Has(const cISCPropertyHolder* holder, uint32_t id);

// The variant type of a property (cIGZVariant::Type), 0 if absent.
uint16_t TypeOf(const cISCPropertyHolder* holder, uint32_t id);

// The raw storage of an ARRAY property, without conversion: the bytes the
// variant's data pointer (RefUint8, slot 95) points at, count x element size
// (bool, void and 8-bit chars 1, 16-bit 2, 32-bit and pointers 4, 64-bit 8),
// as the crime effect is read (crime.md §6.10). False if the property is
// absent, a scalar, an object array or of unknown type (out left empty).
bool GetRawArrayBytes(const cISCPropertyHolder* holder, uint32_t id, std::vector<uint8_t>& out);

// Numeric writes: replace the property on `holder` (an instance override)
// with an array of the given type.
bool SetU32s(cISCPropertyHolder* holder, uint32_t id, const std::vector<uint32_t>& values);
bool SetI64s(cISCPropertyHolder* holder, uint32_t id, const std::vector<int64_t>& values);
bool SetF32s(cISCPropertyHolder* holder, uint32_t id, const std::vector<float>& values);

// Writes property `id` exactly as a core variant holds it (the inverse of
// CopyToCore): `memType` is the element type (cIGZVariant type 0x01-0x0E,
// without the array bit), `data` holds `count` elements in that type's native
// layout (bool: one byte). A scalar (`isArray` false) needs `count` 1. False
// for other types or a bad count.
bool SetTyped(cISCPropertyHolder* holder, uint32_t id, uint16_t memType, bool isArray, const void* data, size_t count);

// Copies a property (whatever its type) from `src` onto `dst`; false if absent.
bool Copy(const cISCPropertyHolder* src, cISCPropertyHolder* dst, uint32_t id);

// The IDs of the holder's own properties (EnumProperties, compiled slot 20),
// in the holder's order. Empty for nullptr.
std::vector<uint32_t> PropertyIds(const cISCPropertyHolder* holder);

// Copies property `id` into a core property holder with its exact variant type
// and shape (fc4_props_set_typed). False if absent or of a type the core does
// not keep (empty, pointers, objects).
bool CopyToCore(const cISCPropertyHolder* holder, uint32_t id, fc4_props* out);

}  // namespace props
}  // namespace fc4
