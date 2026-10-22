// SPDX-License-Identifier: GPL-3.0-or-later
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

// `enable = span565` (glue-span565-impl4): the five methods of the native
// 565 span renderer vtable 0x00AC1844 (docs/specs/span-render-565.md §1.1)
// are redirected at PostAppInit to thiscall forwarders that draw with the
// fc4_span565_* core (engine/capi/src/span565.rs); a call the core declines
// goes to the saved native method. Entries are written only when all six hold
// the published originals; PreAppShutdown writes the originals back.
//
// Counters are split by origin: "constructed" = calls made on the thread of
// the span565_trace fixture while it runs, "shipped" = every other call.
//
// `span565_trace = 1` (needs the bridge): at PostAppInit a constructed buffer
// (cGZBuffer 0xC470D325, Init(5, 3, 5, 16), stride 12) gets a native
// RenderToBuffer context from the draw system (slot 4), Init, then every
// untextured DrawPreclippedSpanList path is drawn twice, the native method
// forced and ours, and every byte of the buffer (padding included) compared;
// RenderBlt is compared by direct slot calls on a second constructed buffer
// (no texture setter is published). Log lines start "span565".
#pragma once

namespace fc4 {

void Span565SetEnabled(bool on);  // FC4Subsystems: enable = span565
void Span565Install();            // PostAppInit: entries, then the trace if asked
void Span565Restore();            // PreAppShutdown: originals back, counter line

}  // namespace fc4
