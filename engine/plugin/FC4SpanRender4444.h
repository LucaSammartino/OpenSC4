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

// The cSpanRender4444 native bridge (glue-span4444-impl4;
// docs/specs/span-render-4444.md §1.1). Default off: `enable = span4444`.
//
// - Forwarders on entries 1-5 of the one shared static vtable 0x00AC185C
//   (thiscall, `this` ignored, callee pops 16/16/20/16/24, no return value).
//   Slot 0 (the deleting destructor) and the 4-byte objects are not touched,
//   so the context's Shutdown / Release lifetime (§1.1) is unchanged.
//   Default: ours renders (engine/core span_render4444 via fc4_span4444_*),
//   and the original runs whenever the core rejects the input.
//   `span4444_shadow = 1`: the original renders, ours renders a copy of the
//   touched bytes and the two are compared.
// - A constructed fixture at PostAppInit (NOT a game path): the original
//   slot functions and ours on identical plug-in owned buffers, every byte
//   compared. Logged as "span4444 fixture [constructed]".
// - A COM probe: one cGZBuffer made through GZCOM (CLSID 0xC470D325, IID
//   0x20732180, persistence.md §6.2), released at once. Initialising it and
//   drawing into it need contracts no spec publishes (question SP5).
//
// No RefCell-like state: counters are atomics (the game may draw from more
// than one thread).

#pragma once

namespace fc4 {

void SpanRender4444Start();  // PostAppInit: fixture, COM probe, forwarders
void SpanRender4444Stop();   // PreAppShutdown: original entries back, counts

}  // namespace fc4
