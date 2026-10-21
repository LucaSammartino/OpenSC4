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

// Tooling (no spec behaviour; glue-simgrids4): `simgrids_probe = 1` keeps a
// reference to every grid (class 0x49B9E602..0x49B9E60B) that a save hands to
// our glue through IStreamAdapter::ReadRef, and at PostCityInit captures each
// one's own Write (whoever's class it is: the game's or ours) and logs the
// FNV-1a 64 digest of the record bytes, the same digest our grid class logs
// on its read/write lines (cFC4SimGridT::RecordDigest). A save written by our
// grids and loaded by the game's grid class then shows whether the game read
// back every record as written.
#include "FC4SimGridProbe.h"

#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4SimGridLogic.h"
#include "FC4Subsystems.h"
#include "FC4WriteCapture.h"

#include "cIGZUnknown.h"

#include <cstring>
#include <mutex>
#include <vector>

namespace fc4 {
namespace {

std::mutex g_mu;
std::vector<cIGZUnknown*> g_held;  // one reference each (FC4_ADDREF)

int Enabled() {
    static const int on = SettingInt("simgrids_probe", "FC4_SIMGRIDS_PROBE", 0) != 0 ? 1 : 0;
    return on;
}

size_t ElemBytes(uint32_t clsid) {
    switch (clsid) {
        case 0x49B9E602: case 0x49B9E603: return 1;
        case 0x49B9E604: case 0x49B9E605: return 2;
        case 0x49B9E606: case 0x49B9E607: case 0x49B9E60A: return 4;
        case 0x49B9E608: case 0x49B9E609: case 0x49B9E60B: return 8;
        default: return 0;
    }
}

constexpr size_t kHeader = 2 + 1 + 4 * 10;  // §5 v1 header

}  // namespace

void SimGridProbeNoteRead(cIGZUnknown* obj, uint32_t clsid) {
    if (obj == nullptr || clsid < 0x49B9E601 || clsid > 0x49B9E60B || !Enabled()) return;
    std::lock_guard<std::mutex> lock(g_mu);
    for (cIGZUnknown* p : g_held) {
        if (p == obj) return;
    }
    FC4_ADDREF(obj);
    g_held.push_back(obj);
}

void SimGridProbeAtCityLoaded() {
    if (!Enabled()) return;
    std::vector<cIGZUnknown*> held;
    {
        std::lock_guard<std::mutex> lock(g_mu);
        held.swap(g_held);
    }
    const bool ours = FindEnabledClass(0x49B9E602) != nullptr;
    uint32_t captured = 0, failed = 0, blank = 0;
    for (cIGZUnknown* obj : held) {
        WriteCapture cap;
        const bool ok = cap.Capture(obj);
        const std::vector<uint8_t>& b = cap.Bytes();
        uint32_t clsid = 0, instance = 0;
        int32_t s = 0, cx = 0, cz = 0;
        if (b.size() >= kHeader) {
            std::memcpy(&clsid, &b[3], 4);
            std::memcpy(&instance, &b[7], 4);
            std::memcpy(&s, &b[11], 4);
            std::memcpy(&cx, &b[19], 4);
            std::memcpy(&cz, &b[23], 4);
        }
        const size_t eb = ElemBytes(clsid);
        const bool sized = ok && eb != 0 && cx >= 0 && cz >= 0 &&
                           b.size() == kHeader + static_cast<size_t>(cx) * static_cast<size_t>(cz) * eb;
        if (!sized) {
            ++failed;
            Log("simgrids-probe: WARNING grid %p: Write ok=%d bytes=%u clsid=0x%08X not a v1 record", static_cast<void*>(obj),
                ok ? 1 : 0, static_cast<unsigned>(b.size()), clsid);
            FC4_RELEASE(obj);
            continue;
        }
        uint32_t nonzero = 0;
        for (size_t off = kHeader; off < b.size(); off += eb) {
            for (size_t k = 0; k < eb; ++k) {
                if (b[off + k] != 0) {
                    ++nonzero;
                    break;
                }
            }
        }
        if (nonzero == 0) ++blank;
        ++captured;
        Log("simgrids-probe: grid clsid=0x%08X instance=0x%08X S=%d count=%dx%d digest=%016llX nonzero=%u", clsid,
            instance, s, cx, cz,
            static_cast<unsigned long long>(SimGridFnv(kSimGridFnvBasis, b.data(), b.size())), nonzero);
        FC4_RELEASE(obj);
    }
    Log("simgrids-probe: city loaded: %u grid(s) from the save, %u captured (%u all zero), %u failed; grid class %s",
        static_cast<unsigned>(held.size()), captured, blank, failed,
        ours ? "ours (enable = simgrids)" : "the game's (simgrids off)");
}

}  // namespace fc4
