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

// The cSpanRender8888 native bridge (glue-span8888-impl4;
// docs/specs/span-render-8888.md §1.1). Default off: `enable = span8888`.
//
// - Forwarders on entries 1-5 of the one shared static vtable 0x00AC1874
//   (thiscall, `this` ignored, callee pops 16/16/20/16/24, no return value;
//   §1.1). The table is installed only if all six entries hold the published
//   originals. Slot 0 (the deleting destructor) and the 4-byte objects are
//   not touched, so the context's Shutdown / Release lifetime is unchanged.
//   Default: ours renders (engine/core span_render8888 via fc4_span8888_*),
//   and the original runs whenever the core rejects the input.
//   `span8888_shadow = 1`: the original renders, ours renders a copy of the
//   touched bytes and the two are compared.
// - A constructed direct fixture at PostAppInit (NOT a game path): the
//   original slot functions and ours called directly on identical plug-in
//   owned buffers, every byte compared. Logged "span8888 fixture [constructed
//   direct]". The native-context route (types 8 and 9 through the real
//   RenderToBuffer context, §1.1 trace) runs in FC4SpanContext.cpp and is
//   counted apart; shipped calls are counted by return address (SR8888-6).
//
// No RefCell-like state: counters are atomics (the game may draw from more
// than one thread).

#pragma once

#include <cstdint>

namespace fc4 {

void SpanRender8888Start();  // PostAppInit: fixture, forwarders
void SpanRender8888Stop();   // PreAppShutdown: original entries back, counts
// glue-span-context-final4: entries 1-5 = the originals (true) or our forwarders
// (false); false if the bridge is not installed.
bool SpanRender8888UseOriginal(bool original);
void SpanRender8888Month(int32_t julian);  // cumulative, complete method evidence

}  // namespace fc4
