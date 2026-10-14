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

// The standard message (docs/specs/foundations.md §5.1), implemented from the
// spec. The gzcom-dll SDK's cRZMessage2Standard must not be used: it copies
// strings, lacks Get/SetExtra, and its GetHasData2..4 are wrong.
#pragma once

#include "cIGZMessage2Standard.h"

#include <cstdint>

class cIGZSerializable;
class cIGZString;

// Not final: a message that owns what its data points at derives from it
// (cFC4ZoningChangeMessage, cFC4ZoneManager.h: zones.md §5.3, §13.1).
class cFC4Message2Standard : public cIGZMessage2Standard {
public:
    static constexpr uint32_t kIID_cIGZMessage2 = 0xA52294B4;  // foundations §5.1 [inferred]

    // A new message has reference count 0; the creator AddRefs it.
    static cFC4Message2Standard* New();

    // Number of live instances (for tests).
    static long LiveCount();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cIGZMessage2
    bool Create(uint32_t dwTypeIID, void** ppUnknown) const override;
    uint32_t GetType() const override;
    void SetType(uint32_t dwType) override;
    bool operator==(cIGZMessage2 const& msgOther) const override;
    bool operator<(cIGZMessage2 const& msgOther) const override;

    // cIGZMessage2Standard
    bool GetHasData1() const override;
    intptr_t GetData1() const override;
    void* GetVoid1() const override;
    void SetData1(intptr_t v) override;
    void SetVoid1(void* p) override;
    bool GetHasData2() const override;
    intptr_t GetData2() const override;
    void* GetVoid2() const override;
    void SetData2(intptr_t v) override;
    void SetVoid2(void* p) override;
    bool GetHasData3() const override;
    intptr_t GetData3() const override;
    void* GetVoid3() const override;
    void SetData3(intptr_t v) override;
    void SetVoid3(void* p) override;
    bool GetHasData4() const override;
    intptr_t GetData4() const override;
    void* GetVoid4() const override;
    void SetData4(intptr_t v) override;
    void SetVoid4(void* p) override;
    bool GetHasString() const override;
    // The first parameter is an interface ID (the SDK calls it dwMaxLen).
    bool GetString(uint32_t riid, void** ppStringOut) const override;
    void SetString(cIGZString* pString) override;
    bool GetHasIGZUnknown() const override;
    cIGZUnknown* GetIGZUnknown() const override;
    void SetIGZUnknown(cIGZUnknown* pObj) override;
    bool GetHasExtra() const override;
    // The first parameter is an interface ID; returns 1 on success, 0 otherwise.
    uint32_t GetExtra(uint32_t riid, void** ppOut) const override;
    void SetExtra(cIGZSerializable* pSerializable) override;

    // The flags byte at +0x08 (docs/abi/layouts.md: the game's layout).
    enum Flag : uint8_t {
        kFlagType = 1u << 0,
        kFlagData1 = 1u << 1,  // data1..data4: bits 1..4
        kFlagExtra = 1u << 5,
        kFlagString = 1u << 6,
        kFlagUnknown = 1u << 7,
    };

    // The memory layout matches the game's standard message, because game code
    // may read the fields of a message it receives directly (layouts.md):
    //   +0x00 vtable (37 interface slots, slot 37 the scalar deleting destructor)
    //   +0x04 reference count
    //   +0x08 flags byte (Flag), +0x0C type
    //   +0x10..+0x1C data1..data4, +0x20 string, +0x24 unknown, +0x28 extra
    // (static_asserts in the constructor; message_test checks the flags byte).

protected:
    cFC4Message2Standard();
    // Virtual, so that it occupies vtable slot 37 like the game's (layouts.md).
    virtual ~cFC4Message2Standard();

private:
    cFC4Message2Standard(const cFC4Message2Standard&) = delete;
    cFC4Message2Standard& operator=(const cFC4Message2Standard&) = delete;

    void SetSlot(int i, intptr_t v);
    void SetFlag(uint8_t bit, bool on);

    uint32_t refCount_ = 0;
    uint8_t flags_ = 0;  // Flag bits; data bits are never cleared
    uint32_t type_ = 0;
    intptr_t data_[4] = {0, 0, 0, 0};
    cIGZString* string_ = nullptr;
    cIGZUnknown* unknown_ = nullptr;
    cIGZSerializable* extra_ = nullptr;
};

// "message (type T, data1 = a, data2 = b, data3 = c)" (foundations §5.1):
// a fresh standard message, already AddRef'd once. Release it when done.
cFC4Message2Standard* FC4MakeMessage(uint32_t type, intptr_t data1, intptr_t data2, intptr_t data3);
