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

// The cSpanRender1555 native bridge (glue-span1555-impl4; docs/specs/span-render-1555.md
// §1.1). With `enable = span1555` (off by default) entries 1-5 of the renderer's static
// vtable 0x00AC182C (compat::Span1555SlotEntry) are replaced by thiscall forwarders with
// the published argument lists and callee pops. Each call snapshots the destination bytes
// its spans cover, runs the original method, keeps its result, restores the snapshot, runs
// ours (fc4_span1555_*) on the same memory and compares the two byte for byte. Ours stays
// in place (the replacement); if ours refuses the input (a non-zero code) the original's
// pixels are put back. Calls with n = 0 or a span off its row go to the original alone.
//
// At the first city load a CONSTRUCTED fixture (not a game path; §1.2: no shipped
// producer of a type-4/6 RenderToBuffer target is known, SR1555-6) creates cGZBuffer
// objects through GZCOM (CLSID 0xC470D325), Inits destinations of colour types 4 and 6
// and sources of types 7, 6 and 9, and calls the five patched entries through the vtable
// as the native caller would. Lines: "span1555: fixture ..." per case and a total;
// "span1555: game call" for the first call from any other caller of each slot.
#pragma once

namespace fc4 {
// FC4Subsystems.cpp, at plug-in startup: installs the forwarders when enabled.
void Span1555Startup(bool enabled);
// FC4Observer.cpp, after each city load: runs the constructed fixture once per session.
void Span1555CityLoaded();
}  // namespace fc4
