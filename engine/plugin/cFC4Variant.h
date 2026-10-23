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

// The variant we hand to the game (AddPropertyVariant): cFC4IGZVariant in the
// game's compiled slot order (foundations §17.1), with storage and
// conversions from the SDK's cRZBaseVariant (LGPL). The SDK class itself must
// not reach the game: its slots 8 and 60 have the wrong argument lists
// (callee-pop mismatch). Like cRZBaseVariant, Release never deletes an object
// on the stack; copies made by CreateCopy are heap objects and are deleted.
#pragma once

#include "FC4Interfaces.h"

#include "cIGZString.h"
#include "cIGZVariant.h"
#include "cRZBaseVariant.h"

#include <cstdint>
#include <vector>

namespace fc4 {
// The value's bytes in its own layout (scalars: the one value; arrays: the
// elements); false for pointers and objects (FC4PropertyHolder.cpp).
bool VariantValueBytes(const cFC4IGZVariant& v, std::vector<uint8_t>& out);
// cFC4Variant::GetValString's body. It goes through the core
// (fc4_variant_get_string), so it lives in FC4PropertyHolder.cpp, which sets
// this at static init; cFC4Variant.cpp alone (fc4_plugin_load_test) keeps it
// null and GetValString answers false, value unchanged.
extern bool (*gVariantGetValString)(const cFC4IGZVariant& v, cIGZString& value);
// Production hook installed with the other Rust property bridges. Standalone
// load-test objects do not link Rust; their fallback has the same raw rule.
extern uint32_t (*gVariantGetCount)(uint16_t type, uint32_t storageCount);
}  // namespace fc4

class cFC4Variant final : public cFC4IGZVariant {
public:
    cFC4Variant() = default;
    cFC4Variant(const cFC4Variant&) = delete;
    cFC4Variant& operator=(const cFC4Variant&) = delete;
    virtual ~cFC4Variant() = default;

    // A copy of a variant the game owns. CopyFrom of the SDK reads it through
    // slots whose SDK declarations §17.1 confirms (GetType, GetCount, GetValX(),
    // RefX(), RefIGZUnknown()).
    void CopyFromGame(const cFC4IGZVariant& v) { v_.CopyFrom(*fc4::AsSdk(&v)); }

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override;

    bool CopyFrom(cFC4IGZVariant const& value) override;
    bool CreateCopy(cFC4IGZVariant** value) const override;
    bool CanGetVal(uint16_t type) const override;
    bool GetValUnknownAs(uint32_t riid, void** ppvObj) override;

    // Slots that only forward to the SDK implementation (generated).
    uint16_t GetType() const override { return v_.GetType(); }
    uint32_t GetCount() const override;
    bool Erase() override { return v_.Erase(); }
    bool GetValBool() const override { return v_.GetValBool(); }
    bool GetValBoolTo(bool& value) const override { return v_.GetValBool(value); }
    void SetValBool(bool value) override { v_.SetValBool(value); }
    uint8_t GetValUint8() const override { return v_.GetValUint8(); }
    bool GetValUint8To(uint8_t& value) const override { return v_.GetValUint8(value); }
    void SetValUint8(uint8_t value) override { v_.SetValUint8(value); }
    int8_t GetValSint8() const override { return v_.GetValSint8(); }
    bool GetValSint8To(int8_t& value) const override { return v_.GetValSint8(value); }
    void SetValSint8(int8_t value) override { v_.SetValSint8(value); }
    uint16_t GetValUint16() const override { return v_.GetValUint16(); }
    bool GetValUint16To(uint16_t& value) const override { return v_.GetValUint16(value); }
    void SetValUint16(uint16_t value) override { v_.SetValUint16(value); }
    int16_t GetValSint16() const override { return v_.GetValSint16(); }
    bool GetValSint16To(int16_t& value) const override { return v_.GetValSint16(value); }
    void SetValSint16(int16_t value) override { v_.SetValSint16(value); }
    uint32_t GetValUint32() const override { return v_.GetValUint32(); }
    bool GetValUint32To(uint32_t& value) const override { return v_.GetValUint32(value); }
    void SetValUint32(uint32_t value) override { v_.SetValUint32(value); }
    int32_t GetValSint32() const override { return v_.GetValSint32(); }
    bool GetValSint32To(int32_t& value) const override { return v_.GetValSint32(value); }
    void SetValSint32(int32_t value) override { v_.SetValSint32(value); }
    uint64_t GetValUint64() const override { return v_.GetValUint64(); }
    bool GetValUint64To(uint64_t& value) const override { return v_.GetValUint64(value); }
    void SetValUint64(uint64_t value) override { v_.SetValUint64(value); }
    int64_t GetValSint64() const override { return v_.GetValSint64(); }
    bool GetValSint64To(int64_t& value) const override { return v_.GetValSint64(value); }
    void SetValSint64(int64_t value) override { v_.SetValSint64(value); }
    float GetValFloat32() const override { return v_.GetValFloat32(); }
    bool GetValFloat32To(float& value) const override { return v_.GetValFloat32(value); }
    void SetValFloat32(float value) override { v_.SetValFloat32(value); }
    double GetValFloat64() const override { return v_.GetValFloat64(); }
    bool GetValFloat64To(double& value) const override { return v_.GetValFloat64(value); }
    void SetValFloat64(double value) override { v_.SetValFloat64(value); }  // nothing in ST0 (FC4Interfaces.h)
    char GetValChar() const override { return v_.GetValChar(); }
    bool GetValCharTo(char& value) const override { return v_.GetValChar(value); }
    void SetValChar(char value) override { v_.SetValChar(value); }
    uint16_t GetValRZUnicodeChar() const override { return v_.GetValRZUnicodeChar(); }
    bool GetValRZUnicodeCharTo(uint16_t& value) const override { return v_.GetValRZUnicodeChar(value); }
    void SetValRZUnicodeChar(uint16_t value) override { v_.SetValRZUnicodeChar(value); }
    char GetValRZChar() const override { return v_.GetValRZChar(); }
    bool GetValRZCharTo(char& value) const override { return v_.GetValRZChar(value); }
    void SetValRZChar(char value) override { v_.SetValRZChar(value); }
    void* GetValVoidPtr() const override { return v_.GetValVoidPtr(); }
    bool GetValVoidPtrTo(void** value) const override { return v_.GetValVoidPtr(value); }
    void SetValVoidPtr(void* value) override { v_.SetValVoidPtr(value); }
    bool CreateValString(cIGZString** value) const override { return v_.CreateValString(value); }
    // Slot 55 (data-formats §5.1): not the SDK's (it answers false and writes
    // nothing); see cFC4Variant.cpp.
    bool GetValString(cIGZString& value) const override;
    void SetValString(cIGZString const& value) override { v_.SetValString(value); }
    // Slots 57 / 58, the raw bytes (type 0x8000): not the SDK's (it answers
    // false and stores nothing); see cFC4Variant.cpp (NG37, data-formats §5.2).
    bool GetValVoid(void* value, uint32_t& length) const override;
    void SetValVoid(void* value, uint32_t length) override;
    cIGZUnknown* GetValUnknown() override { return v_.GetValUnknown(); }
    void SetValUnknown(cIGZUnknown* value) override { v_.SetValUnknown(value); }
    bool AsBool() const override { return static_cast<const cRZBaseVariant&>(v_).AsBool(); }
    bool& AsBoolRef() override { return v_.AsBool(); }
    uint8_t AsUint8() const override { return static_cast<const cRZBaseVariant&>(v_).AsUint8(); }
    uint8_t& AsUint8Ref() override { return v_.AsUint8(); }
    int8_t AsSint8() const override { return static_cast<const cRZBaseVariant&>(v_).AsSint8(); }
    int8_t& AsSint8Ref() override { return v_.AsSint8(); }
    uint16_t AsUint16() const override { return static_cast<const cRZBaseVariant&>(v_).AsUint16(); }
    uint16_t& AsUint16Ref() override { return v_.AsUint16(); }
    int16_t AsSint16() const override { return static_cast<const cRZBaseVariant&>(v_).AsSint16(); }
    int16_t& AsSint16Ref() override { return v_.AsSint16(); }
    uint32_t AsUint32() const override { return static_cast<const cRZBaseVariant&>(v_).AsUint32(); }
    uint32_t& AsUint32Ref() override { return v_.AsUint32(); }
    int32_t AsSint32() const override { return static_cast<const cRZBaseVariant&>(v_).AsSint32(); }
    int32_t& AsSint32Ref() override { return v_.AsSint32(); }
    uint64_t AsUint64() const override { return static_cast<const cRZBaseVariant&>(v_).AsUint64(); }
    uint64_t& AsUint64Ref() override { return v_.AsUint64(); }
    int64_t AsSint64() const override { return static_cast<const cRZBaseVariant&>(v_).AsSint64(); }
    int64_t& AsSint64Ref() override { return v_.AsSint64(); }
    float AsFloat32() const override { return static_cast<const cRZBaseVariant&>(v_).AsFloat32(); }
    float& AsFloat32Ref() override { return v_.AsFloat32(); }
    double AsFloat64() const override { return static_cast<const cRZBaseVariant&>(v_).AsFloat64(); }
    double& AsFloat64Ref() override { return v_.AsFloat64(); }
    char AsChar() const override { return static_cast<const cRZBaseVariant&>(v_).AsChar(); }
    char& AsCharRef() override { return v_.AsChar(); }
    uint16_t AsRZUnicodeChar() const override { return static_cast<const cRZBaseVariant&>(v_).AsRZUnicodeChar(); }
    uint16_t& AsRZUnicodeCharRef() override { return v_.AsRZUnicodeChar(); }
    char AsRZChar() const override { return static_cast<const cRZBaseVariant&>(v_).AsRZChar(); }
    char& AsRZCharRef() override { return v_.AsRZChar(); }
    void* AsVoidPtr() const override { return static_cast<const cRZBaseVariant&>(v_).AsVoidPtr(); }
    void*& AsVoidPtrRef() override { return v_.AsVoidPtr(); }
    void SetRefBool(bool* value, uint32_t length) override { v_.RefBool(value, length); }
    bool* RefBool() const override { return v_.RefBool(); }
    void SetRefUint8(uint8_t* value, uint32_t length) override { v_.RefUint8(value, length); }
    uint8_t* RefUint8() const override { return v_.RefUint8(); }
    void SetRefSint8(int8_t* value, uint32_t length) override { v_.RefSint8(value, length); }
    int8_t* RefSint8() const override { return v_.RefSint8(); }
    void SetRefUint16(uint16_t* value, uint32_t length) override { v_.RefUint16(value, length); }
    uint16_t* RefUint16() const override { return v_.RefUint16(); }
    void SetRefSint16(int16_t* value, uint32_t length) override { v_.RefSint16(value, length); }
    int16_t* RefSint16() const override { return v_.RefSint16(); }
    void SetRefUint32(uint32_t* value, uint32_t length) override { v_.RefUint32(value, length); }
    uint32_t* RefUint32() const override { return v_.RefUint32(); }
    void SetRefSint32(int32_t* value, uint32_t length) override { v_.RefSint32(value, length); }
    int32_t* RefSint32() const override { return v_.RefSint32(); }
    void SetRefUint64(uint64_t* value, uint32_t length) override { v_.RefUint64(value, length); }
    uint64_t* RefUint64() const override { return v_.RefUint64(); }
    void SetRefSint64(int64_t* value, uint32_t length) override { v_.RefSint64(value, length); }
    int64_t* RefSint64() const override { return v_.RefSint64(); }
    void SetRefFloat32(float* value, uint32_t length) override { v_.RefFloat32(value, length); }
    float* RefFloat32() const override { return v_.RefFloat32(); }
    void SetRefFloat64(double* value, uint32_t length) override { v_.RefFloat64(value, length); }
    double* RefFloat64() const override { return v_.RefFloat64(); }
    void SetRefChar(char* value, uint32_t length) override { v_.RefChar(value, length); }
    char* RefChar() const override { return v_.RefChar(); }
    void SetRefRZUnicodeChar(uint16_t* value, uint32_t length) override { v_.RefRZUnicodeChar(value, length); }
    uint16_t* RefRZUnicodeChar() const override { return v_.RefRZUnicodeChar(); }
    void SetRefRZChar(char* value, uint32_t length) override { v_.RefRZChar(value, length); }
    char* RefRZChar() const override { return v_.RefRZChar(); }
    void SetRefVoidPtr(void** value, uint32_t length) override { v_.RefVoidPtr(value, length); }
    void** RefVoidPtr() const override { return v_.RefVoidPtr(); }
    void SetRefVoid(void* value, uint32_t length) override { v_.RefVoid(value, length); }
    void* RefVoid() const override { return v_.RefVoid(); }
    void SetRefIGZUnknown(cIGZUnknown* value) override { v_.RefIGZUnknown(value); }
    bool RefIGZUnknownAs(uint32_t riid, void** ppvObj) override { return v_.RefIGZUnknown(riid, ppvObj); }
    cIGZUnknown* RefIGZUnknown() const override { return v_.RefIGZUnknown(); }

private:
    cRZBaseVariant v_;
    uint32_t refs_ = 0;
    bool heap_ = false;
};
