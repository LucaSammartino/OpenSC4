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

// Helpers shared by the prop developer (cFC4PropDeveloper.h,
// prop-developer.md) and the city detail manager (cFC4CityDetail.h,
// city-detail.md) glue and their shadows: game references held for the
// duration of one core call, configuration objects as the cores read them,
// the lot manager's lots in list order, the 3D view, and a stream wrapper
// that records the references a core's Read takes.
#pragma once

#include "fc4.h"

#include "SC4Vector.h"

#include <cstdint>
#include <vector>

class cIGZUnknown;
class cISC4City;
class cISC4Lot;
class cISC4LotManager;
class cISC4View3DWin;
class cISCPropertyHolder;
class cSC4LotConfigurationObject;

namespace fc4 {
namespace propcity {

// The application's city (borrowed), or null.
cISC4City* AppCity();

// AddRef / Release of any object: ours plainly, a game object through the
// reference audit (FC4RefAudit.h). DropHeld is the release of a reference
// a core kept after the game may have let the object go (the prop lists of
// prop-developer.md §2: FC4_RELEASE_LAST_HOLDER).
void Hold(cIGZUnknown* o);
void Drop(cIGZUnknown* o);
void DropHeld(cIGZUnknown* o);

// References kept alive for one core call (list entries, QueryInterface
// results): each is released when the pool is drained (or destroyed).
class CallPool {
public:
    CallPool() = default;
    CallPool(const CallPool&) = delete;
    CallPool& operator=(const CallPool&) = delete;
    ~CallPool() { Drain(); }
    // Takes over one reference the caller owns.
    void Adopt(cIGZUnknown* o) {
        if (o != nullptr) held_.push_back(o);
    }
    void Drain();
    size_t Size() const { return held_.size(); }

private:
    std::vector<cIGZUnknown*> held_;
};

// QueryInterface `iid` of `u`; the reference goes to `pool`. Null when it does
// not answer.
void* QueryInto(CallPool& pool, cIGZUnknown* u, uint32_t iid);

// A configuration object array (SDK cSC4LotConfigurationObject, 0x34 bytes)
// as the cores read it, through `sink` in array order (one call). The ID
// lists are the objects' own vectors (families not expanded).
void SinkCfgObjects(SC4Vector<cSC4LotConfigurationObject>* objects, fc4_lot_cfg_objects_sink sink, void* ctx);

// Every lot of `lm` in its list order (lot manager slot 15, the safe lot
// iterator, as budget.md §5.2 walks it), borrowed from the master list; when
// that list cannot be walked (executable not identified, a lot manager that
// is neither the original nor FreeCity4's), the lots of GetLotsInCellRect over
// the whole city (one reference each, kept in `pool`; `*ordered` = false).
std::vector<cISC4Lot*> AllLots(cISC4City* city, cISC4LotManager* lm, CallPool& pool, bool* ordered);

// The city view's cISC4View3DWin (the app's main window, child 0x6104489A,
// GetChildAs(0x9A47B417, 0xFA47B3F9)) with one reference for the caller
// (release with Drop), or null without a city view.
cISC4View3DWin* View3D();

// A uint32 property: the first element (false when absent) / set in place
// when present, else added (holder slot 13).
bool U32Property(const cISCPropertyHolder* h, uint32_t id, uint32_t* out);
void SetU32Property(cISCPropertyHolder* h, uint32_t id, uint32_t value);

// An fc4_istream that forwards to another and records every reference its
// get_ref returns (iid, token). With `addRefOurs`, a reference to one of our
// objects (which a CaptureIStream hands over without a reference) gets one,
// so the reader always owns one reference per non-null token.
class RecordingIStream {
public:
    struct Taken {
        uint32_t iid;
        uint64_t token;
        uint32_t classId;
    };
    RecordingIStream(const fc4_istream* inner, bool addRefOurs);
    const fc4_istream* Get() const { return &api_; }
    const std::vector<Taken>& Refs() const { return refs_; }

private:
    static bool GetRef(void* user, uint32_t iid, fc4_ref* out);
    const fc4_istream* inner_;
    bool addRefOurs_;
    std::vector<Taken> refs_;
    fc4_istream api_{};
};

// Drops the references a failed Read left behind (every non-null token of
// `refs`, once each).
void DropRefs(const std::vector<RecordingIStream::Taken>& refs);

// "YYYY-MM-DD" of a julian day number.
const char* DateText(int32_t julian, char (&buf)[16]);

}  // namespace propcity
}  // namespace fc4
