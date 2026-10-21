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

// What the city-situation, demolition and dispatch shadows share (the glue17
// lane): a byte stream that encodes a reference the way the game's
// WriteGZSerializable does, and the byte-for-byte record comparison all three
// shadows make. Tooling; no spec behaviour.
//
// The original's record is captured through its own Write (FC4WriteCapture.h),
// which numbers the objects it meets 1..n in the order they are written. Our
// shadow core writes the same record back through `ShadowOStream`, which
// numbers a reference by the capture's own object number for that token (0 =
// null). A token the capture never saw is written as 0 and counted as a
// difference: the two records cannot be the same record.
#pragma once

#include "FC4WriteCapture.h"

#include <cstddef>
#include <cstdint>
#include <string>
#include <unordered_map>
#include <vector>

class cIGZUnknown;

namespace fc4 {

// A byte stream for a shadow core's Write: primitives little-endian as the
// game writes them, a reference as its capture object number (0 = null) plus
// the class ID, matching WriteCapture's encoding.
class ShadowOStream final : public cFC4ISC4DBSegmentOStream {
public:
    ShadowOStream() { api_.struct_size = sizeof(api_); }
    // The capture whose object numbering the references follow; its map is
    // built with AddToken as the read side hands tokens out.
    void AddToken(uint64_t token, uint32_t objectId, uint32_t classId);

    const std::vector<uint8_t>& Bytes() const { return bytes_; }
    // References the stream could not place in the capture (written as 0).
    uint32_t UnknownRefs() const { return unknownRefs_; }

    const fc4_ostream* Get() { Bind(); return &api_; }

    // The fc4_ostream reference callback (public for the C table).
    bool PutRefPublic(const fc4_ref* r) { return PutRef(r); }

    // cIGZUnknown (a static object: the reference count is not kept)
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }

    // cIGZOStream
    void Flush() override {}
    bool SetSint8(int8_t v) override { return Put(&v, 1); }
    bool SetUint8(uint8_t v) override { return Put(&v, 1); }
    bool SetSint16(int16_t v) override { return Put(&v, 2); }
    bool SetUint16(uint16_t v) override { return Put(&v, 2); }
    bool SetSint32(int32_t v) override { return Put(&v, 4); }
    bool SetUint32(uint32_t v) override { return Put(&v, 4); }
    bool SetSint64(int64_t v) override { return Put(&v, 8); }
    bool SetUint64(uint64_t v) override { return Put(&v, 8); }
    bool SetFloat32(float v) override { return Put(&v, 4); }
    bool SetFloat64(double v) override { return Put(&v, 8); }
    bool SetRZCharStr(char const* s) override;
    bool SetGZStr(cIGZString const& s) override;
    bool SetGZSerializable(cIGZSerializable const& s) override;
    bool SetVoid(void const* p, uint32_t n) override;
    int32_t GetError() override { return good_ ? 0 : 1; }
    int32_t SetUserData(cIGZVariant*) override { return 0; }
    int32_t GetUserData() override { return 0; }

    // cISC4DBSegmentOStream
    bool Open(cISC4DBSegment*, cGZPersistResourceKey const&, bool) override { return false; }
    bool Close() override { return true; }
    bool IsOpen() override { return true; }
    int32_t GetRecord() override { return 0; }
    int32_t GetSegment() override { return 0; }
    bool WriteGZSerializable(cIGZSerializable const* p) override;
    bool WriteResKey(cGZPersistResourceKey const& k) override;
    bool WriteVariant(cIGZVariant const&) override;

private:
    struct Obj {
        uint32_t id;
        uint32_t classId;
    };
    void Bind();
    bool Put(const void* p, size_t n);
    bool PutRef(const fc4_ref* r);

    fc4_ostream api_{};
    std::vector<uint8_t> bytes_;
    std::unordered_map<uint64_t, Obj> objects_;
    uint32_t unknownRefs_ = 0;
    bool good_ = true;
};

// The difference list of one comparison line: the first `kMax` are spelled
// out, the rest only counted (like the other shadows' FirstList).
struct CoreDiffs {
    static constexpr unsigned kMax = 8;
    std::string text;
    unsigned listed = 0;
    unsigned n = 0;          // every difference
    unsigned unexplained = 0;  // those without a lane-report question number

    // A difference we cannot yet explain.
    void Add(const char* fmt, ...);
    // A difference already filed as a numbered question in the lane report.
    void AddQ(const char* question, const char* fmt, ...);
    const char* Str() const { return listed == 0 ? "-" : text.c_str(); }
};

// "name shadow DATE load|month: body", split at a space when it is long.
void CoreShadowLine(const char* cont, const char* name, const char* date, bool month, const std::string& body);

// A `%-style` append into a std::string (512-byte scratch).
void AppendF(std::string& s, const char* fmt, ...);

}  // namespace fc4
