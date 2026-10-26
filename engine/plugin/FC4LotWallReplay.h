// SPDX-License-Identifier: GPL-3.0-or-later
// A detached native wall reads the identical LR15 payload as the Rust wall.
// No Init, city insertion, recursive Read or live-owner mutation is performed.
#pragma once
#include "FC4WriteCapture.h"
#include "FC4RefAudit.h"
#include "cIGZSerializable.h"
#include "cGZPersistResourceKey.h"
#include <cstring>

namespace fc4 {
class LotWallInput final : public cFC4ISC4DBSegmentIStream {
public:
    LotWallInput(const WriteCapture& capture, const std::vector<uint8_t>& bytes)
        : capture_(capture), bytes_(bytes) {}
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out || (iid != 1 && iid != kFC4_IID_cISC4DBSegmentIStream)) return false;
        *out = this; return true;
    }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    bool Skip(uint32_t n) override {
        if (n > bytes_.size() - at_) return Fail();
        at_ += n; return true;
    }
#define WALL_READ(Name, T) bool Name(T& v) override { return GetVoid(&v, sizeof(v)); }
    WALL_READ(GetSint8, int8_t) WALL_READ(GetUint8, uint8_t)
    WALL_READ(GetSint16, int16_t) WALL_READ(GetUint16, uint16_t)
    WALL_READ(GetSint32, int32_t) WALL_READ(GetUint32, uint32_t)
    WALL_READ(GetSint64, int64_t) WALL_READ(GetUint64, uint64_t)
    WALL_READ(GetFloat32, float) WALL_READ(GetFloat64, double)
#undef WALL_READ
    bool GetVoid(void* out, uint32_t n) override {
        if (!good_ || n > bytes_.size() - at_) return Fail();
        if (n) std::memcpy(out, bytes_.data() + at_, n);
        at_ += n; return true;
    }
    bool GetRZCharStr(char*, uint32_t) override { return Fail(); }
    bool GetGZStr(cIGZString&) override { return Fail(); }
    bool GetGZSerializable(cIGZSerializable&) override { return Fail(); }
    int32_t GetError() override { return good_ ? 0 : 1; }
    int32_t SetUserData(cIGZVariant*) override { return 0; }
    int32_t GetUserData() override { return 0; }
    bool Open(cISC4DBSegment*, const cGZPersistResourceKey&, bool) override { return Fail(); }
    bool Close() override { return true; }
    bool IsOpen() override { return true; }
    int32_t GetRecord() override { return 0; }
    int32_t GetSegment() override { return 0; }
    bool ReadGZSerializable(cIGZSerializable** out, bool loadNow) override {
        if (!out) return Fail();
        *out = nullptr;
        uint32_t id = 0, cls = 0;
        if (loadNow || !GetUint32(id)) return Fail();
        if (!id) return true;
        if (!GetUint32(cls) || capture_.ClassOf(id) != cls) return Fail();
        auto* object = static_cast<cIGZSerializable*>(const_cast<void*>(capture_.Object(id)));
        if (!object) return Fail();
        FC4_ADDREF_FOR_CALLER(object); // SC4ReadGZSerializable consumes this reference
        *out = object;
        return true;
    }
    bool ReadResKey(cGZPersistResourceKey& key) override {
        return GetUint32(key.type) && GetUint32(key.group) && GetUint32(key.instance);
    }
    bool ReadVariant(cIGZVariant&) override { return Fail(); }
    bool AtEnd() const { return good_ && at_ == bytes_.size(); }
private:
    bool Fail() { good_ = false; return false; }
    const WriteCapture& capture_;
    const std::vector<uint8_t>& bytes_;
    size_t at_ = 0;
    bool good_ = true;
};
} // namespace fc4
