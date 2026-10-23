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

// An original simulator's record, read through its own Write
// (cIGZSerializable) into memory: the grid shadows read state that has no
// getter this way, without changing the simulator (tooling; no spec
// behaviour). Primitives are little-endian as the game's streams write them;
// a reference (WriteGZSerializable) is a uint32 object number (1-based, 0 =
// null) followed, when not null, by the uint32 class ID; each object is
// listed once with its cISC4SimGrid identity (IID 0xC9BD939A), borrowed: the
// simulator keeps its grids alive while the caller compares inside one
// message delivery.
#pragma once

#include "FC4Streams.h"

#include <cstddef>
#include <cstdint>
#include <unordered_map>
#include <vector>

class cIGZUnknown;

namespace fc4 {

class WriteCapture final : public cFC4ISC4DBSegmentOStream {
public:
    // Opt-in for occupant property holders, using the traffic shadow's
    // foundations §9.2 variant capture path. Other shadows keep their defaults.
    explicit WriteCapture(bool variants = false) : variants_(variants) {}
    // Runs `obj`'s Write (the cIGZSerializable reference QueryInterface gives
    // is released); false when it does not answer, Write returns false or the
    // stream saw something it cannot hold. `answered` (optional): whether it
    // answered cIGZSerializable.
    bool Capture(cIGZUnknown* obj, bool* answered = nullptr);

    const std::vector<uint8_t>& Bytes() const { return bytes_; }
    // Reference `id` (1-based as written): its cISC4SimGrid identity
    // (borrowed; nullptr if it does not answer) and class ID.
    void* Grid(uint32_t id) const;
    uint32_t ClassOf(uint32_t id) const;
    // Reference `id`'s object as written (its cIGZSerializable, borrowed;
    // nullptr for an unknown id): the zone developer shadow asks it for its lot.
    const void* Object(uint32_t id) const;
    // The same object as its cIGZSerializable (borrowed; nullptr for an
    // unknown id): the civic and parks shadows map saved occupants and
    // departments back to their objects with it.
    cIGZSerializable* SerializableAt(uint32_t id) const {
        return static_cast<cIGZSerializable*>(const_cast<void*>(Object(id)));
    }

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
    struct Ref {
        const void* object;
        void* grid;
        uint32_t classId;
    };
    bool Put(const void* p, size_t n);
    bool PutStr(const char* s, size_t n);

    bool variants_ = false;
    std::vector<uint8_t> bytes_;
    std::vector<Ref> refs_;
    std::unordered_map<const void*, size_t> index_;  // object -> refs_ index (records with many references)
    bool good_ = true;
};

// Reads a capture front to back; every read fails once the bytes run out.
class CaptureReader {
public:
    explicit CaptureReader(const std::vector<uint8_t>& b) : b_(b) {}
    bool U8(uint8_t& v) { return Get(&v, 1); }
    bool U16(uint16_t& v) { return Get(&v, 2); }
    bool U32(uint32_t& v) { return Get(&v, 4); }
    bool I32(int32_t& v) { return Get(&v, 4); }
    bool F32(float& v) { return Get(&v, 4); }
    // A reference: its object number (0 = null; the class ID is skipped).
    bool Ref(uint32_t& id);

private:
    bool Get(void* out, size_t n);
    const std::vector<uint8_t>& b_;
    size_t pos_ = 0;
};

// A capture as an fc4_istream: a shadow core's Read of the original's record
// (the civic and parks shadows). Primitives little-endian, GZStr as a uint32
// length and the bytes, a reference as its object (WriteCapture::Object)
// queried for the IID the reader asks: a host object comes back with one
// reference for the reader, one of ours without one (the host's resolve_ref
// takes it, as FC4Streams.h's IStreamAdapter does). The capture must outlive
// the stream and its objects the read.
class CaptureIStream {
public:
    // `bytes`, when given, replaces the capture's bytes (same references,
    // e.g. a record with reordered entries); it must outlive the stream.
    explicit CaptureIStream(const WriteCapture& cap, const std::vector<uint8_t>* bytes = nullptr);
    const fc4_istream* Get() const { return &api_; }
    bool good() const { return good_; }
    bool atEnd() const { return pos_ == B().size(); }

    // fc4_istream callbacks (public for the C table).
    bool Take(void* out, size_t n);
    bool Str(fc4_bytes_sink sink, void* ctx);
    bool Ref(uint32_t iid, fc4_ref* out);

private:
    const std::vector<uint8_t>& B() const { return bytes_ != nullptr ? *bytes_ : cap_.Bytes(); }
    const WriteCapture& cap_;
    const std::vector<uint8_t>* bytes_ = nullptr;
    size_t pos_ = 0;
    bool good_ = true;
    fc4_istream api_{};
};

// A shadow's line through fc4::Log (a fixed buffer): a longer line is split
// at a space, each piece after the first prefixed with `cont` (e.g.
// "landvalue shadow (cont.)").
void LogShadowLine(const char* cont, const uint8_t* data, size_t len);

}  // namespace fc4
