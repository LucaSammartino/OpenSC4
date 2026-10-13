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

// Adapters from the game's streams to the core codec (fc4_ostream /
// fc4_istream, foundations §2), and the GZCOM object-reference helpers.
#pragma once

#include "cIGZIStream.h"
#include "cIGZOStream.h"
#include "cIGZUnknown.h"
#include "fc4.h"

#include <cstdint>

class cGZPersistResourceKey;
class cIGZSerializable;
class cIGZVariant;
class cISC4DBSegment;

// ---- segment streams, declared as the game implements them -----------------------------------
//
// foundations §2.7 [compat]: the game's ReadGZSerializable takes TWO arguments
// (cIGZSerializable**, bool loadNow); the gzcom-dll header declares one, and
// calling through it corrupts the stack (__thiscall: the callee pops its
// arguments). These declarations replace the SDK's for our calls.

static const uint32_t kFC4_IID_cISC4DBSegmentIStream = 0x4999A8F5;
static const uint32_t kFC4_IID_cISC4DBSegmentOStream = 0x6999A8FB;

class cFC4ISC4DBSegmentIStream : public cIGZIStream {
public:
    virtual bool Open(cISC4DBSegment* pSegment, cGZPersistResourceKey const& sKey, bool bUnknown) = 0;
    virtual bool Close() = 0;
    virtual bool IsOpen() = 0;
    virtual int32_t GetRecord() = 0;
    virtual int32_t GetSegment() = 0;
    virtual bool ReadGZSerializable(cIGZSerializable** ppOut, bool loadNow) = 0;
    virtual bool ReadResKey(cGZPersistResourceKey& sKeyOut) = 0;
    virtual bool ReadVariant(cIGZVariant& sVariantOut) = 0;
};

class cFC4ISC4DBSegmentOStream : public cIGZOStream {
public:
    virtual bool Open(cISC4DBSegment* pSegment, cGZPersistResourceKey const& sKey, bool bUnknown) = 0;
    virtual bool Close() = 0;
    virtual bool IsOpen() = 0;
    virtual int32_t GetRecord() = 0;
    virtual int32_t GetSegment() = 0;
    virtual bool WriteGZSerializable(cIGZSerializable const* pObject) = 0;
    virtual bool WriteResKey(cGZPersistResourceKey const& sKey) = 0;
    virtual bool WriteVariant(cIGZVariant const& sVariant) = 0;
};

// ---- our objects ----------------------------------------------------------------------------------
//
// Every GZCOM object that fronts a core object answers QueryInterface for this
// private interface, so references read from a save can be mapped back to the
// core object they stand for.

static const uint32_t kFC4_IID_CoreObject = 0x7F4C0B1E;

class cIFC4CoreObject : public cIGZUnknown {
public:
    // The core handle (e.g. an fc4_department*) and the class ID it belongs to.
    virtual void* CoreHandle() = 0;
    virtual uint32_t CoreClass() = 0;
};

namespace fc4 {

// The token a core object uses for a GZCOM object: the cIGZUnknown* of the
// interface the object is referred to by.
inline uint64_t TokenOf(const void* p) { return static_cast<uint64_t>(reinterpret_cast<uintptr_t>(p)); }
inline cIGZUnknown* FromToken(uint64_t t) { return reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(t)); }

// Writes through a game cIGZOStream. References need the segment stream.
class OStreamAdapter {
public:
    explicit OStreamAdapter(cIGZOStream& s);
    ~OStreamAdapter();
    OStreamAdapter(const OStreamAdapter&) = delete;
    OStreamAdapter& operator=(const OStreamAdapter&) = delete;

    const fc4_ostream* Get() const { return &api_; }
    cIGZOStream& Stream() { return s_; }

    // WriteGZSerializable on the object behind `token` (QueryInterfaced to
    // cIGZSerializable); token 0 writes a null reference.
    bool WriteRef(uint64_t token);

private:
    cIGZOStream& s_;
    cFC4ISC4DBSegmentOStream* seg_ = nullptr;
    fc4_ostream api_{};
};

// A reference read from a stream (`obj`, with the reference ReadGZSerializable
// returned) when it is one of our objects (cIFC4CoreObject): fills `out` with
// the core handle and, as for host objects, the token of the interface `iid`
// (0 = the primary cIGZUnknown), and drops the reference (ours: the core keeps
// the object alive; a consumer that keeps the token as a host object must
// AddRef it). Returns false, touching nothing, for objects that are not ours.
bool OurObjectRef(cIGZSerializable* obj, uint32_t iid, fc4_ref* out);

// Reads through a game cIGZIStream. References need the segment stream.
class IStreamAdapter {
public:
    explicit IStreamAdapter(cIGZIStream& s);
    ~IStreamAdapter();
    IStreamAdapter(const IStreamAdapter&) = delete;
    IStreamAdapter& operator=(const IStreamAdapter&) = delete;

    const fc4_istream* Get() const { return &api_; }
    cIGZIStream& Stream() { return s_; }

    // ReadGZSerializable (with loadNow = false), then the "usual helper
    // semantics" of foundations §2.7: QueryInterface to `iid` (0 = keep the
    // cIGZUnknown). For our own objects the core handle is filled in and no
    // reference is kept; for host objects `out.token` carries one reference
    // owned by the caller.
    bool ReadRef(uint32_t iid, fc4_ref* out);

private:
    cIGZIStream& s_;
    cFC4ISC4DBSegmentIStream* seg_ = nullptr;
    fc4_istream api_{};
};

}  // namespace fc4
