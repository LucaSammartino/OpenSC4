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
// Exact published context CALL return addresses distinguish constructed native
// context, constructed direct, natural native context, and other calls.
//
// `span565_trace = 1` (needs the bridge): at PostAppInit a constructed buffer
// (cGZBuffer 0xC470D325, Init(17, 12, 5, 16), stride 36) gets a native
// RenderToBuffer context from draw-system slot 4. Public Init/setters/slot 22
// produce all four untextured paths and 156 reachable texture row/kind pairs.
// The original draws once on the live native target; independent Rust receives
// guarded copies of the complete padded preimage and captured source/records.
// No native result is restored or resynchronized. Public reference/lock probes
// check source clearing, Suspend/Resume, Shutdown and final releases. These
// constructed fixtures establish no natural usage or replacement-owner credit.
// Log lines start "span565". SR565-6's unresolved failure exits are unchanged.
#pragma once

namespace fc4 {

void Span565SetEnabled(bool on);  // FC4Subsystems: enable = span565
void Span565Install();            // PostAppInit: entries, then the trace if asked
void Span565Restore();            // PreAppShutdown: originals back, counter line
// Explicit suite workload, outside monthly ReachProbeScope. Uses only public
// buffer/context calls; constructed buffers still do not prove shipped usage.
const char* Span565ScriptProbe();

}  // namespace fc4
