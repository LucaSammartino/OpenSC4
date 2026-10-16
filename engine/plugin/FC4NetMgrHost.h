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

// The network manager's and the network lot manager's host tables (fc4.h
// fc4_netmgr_host, fc4_netlot_host; docs/specs/network-manager.md §12) over
// the game's interfaces: the READ side, which the shadow
// (FC4NetMgrShadow.h) uses with every write left NULL, and which a
// replacement can extend with its writes.
//
// Tokens: a network occupant is its cISC4NetworkOccupant interface (IID
// 0xA821EF94), a lot its cISC4Lot (0xE818FCED), any other object the pointer
// the game gave. Tokens handed to the core are BORROWED unless fc4.h says
// otherwise; references the game's queries give are kept in the context until
// the outermost call into the core returns (NetMgrReleaseHeld). Main thread.
#pragma once

#include "fc4.h"

#include <cstdint>
#include <vector>

class cIGZUnknown;
class cISC4City;

namespace fc4 {

struct NetMgrContext {
    // The city (borrowed: the shadow's cores live inside one city's lifetime).
    cISC4City* city = nullptr;
    // References the queries gave (FC4_ACQUIRED), released by NetMgrReleaseHeld.
    std::vector<cIGZUnknown*> held;
};

// The read-side tables: every callback that changes the game, subscribes or
// creates objects is NULL (a missing callback reads as 0 / false / none).
fc4_netmgr_host MakeNetMgrReadOnlyHost(NetMgrContext* ctx);
fc4_netlot_host MakeNetLotReadOnlyHost(NetMgrContext* ctx);

// Drops the references the queries gave since the last call.
void NetMgrReleaseHeld(NetMgrContext* ctx);

// Holds / drops a token's object: ours with AddRef / Release, the game's
// through the reference audit (FC4_ADDREF / FC4_RELEASE).
void NetHold(uint64_t token);
void NetDrop(uint64_t token);

// A FreeCity4 stand-in object (a cIGZSerializable answering `clsid`, its
// Write empty) with one reference for the caller: the shadow's tools and
// pothole effect props, which must not reach the game.
uint64_t NewNetStandIn(uint32_t clsid);
// Whether a token is such a stand-in.
bool IsNetStandIn(uint64_t token);

}  // namespace fc4
