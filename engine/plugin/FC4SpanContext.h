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

// The native draw-context route for the span bridges (glue-span-context-final4;
// docs/specs/span-render-4444.md §1.3, fixture steps 1-5).
//
// - Caller classes, counted per renderer and slot by the bridges' forwarders
//   (by return address, §1.3 "Counting and labels"): "constructed (native
//   context)" = from a DrawPreclippedSpanList site while this fixture's slot-22
//   call is active on the calling thread; "shipped" = from those sites at any
//   other time; "constructed (direct)" = a fixture calling an entry itself;
//   "other" = none of these.
// - The fixture (once, at the first city load, when span1555 and/or span4444 is
//   on): real cGZBuffer destinations (types 4, 6, 7) and textures through GZCOM,
//   the draw system's own RenderToBuffer context, slot 22 with the original
//   renderer entries and again with ours, full arenas (padding included)
//   compared byte for byte; Lock / Unlock / Shutdown / Release audited.

#pragma once

#include <cstdint>

namespace fc4 {

enum SpanRendererId : unsigned { kSpanR1555 = 0, kSpanR4444 = 1, kSpanRendererCount = 2 };

// From a bridge forwarder: slot 1-5, the forwarder's return address, and
// whether that bridge's own constructed fixture is making a direct call.
void SpanNoteCall(unsigned renderer, unsigned slot, uintptr_t ret, bool fixtureDirect);
// From a bridge: ours refused a call and the original's pixels were kept.
void SpanNoteRefused(unsigned renderer);
// 4444-only observation of the real native texture descriptor.
void Span4444NoteTexture(uint32_t format, uint8_t keyed, uint32_t key, uint32_t mode);
// True on the thread whose fixture slot-22 call is active.
bool SpanContextFixtureThread();

void SpanContextCityLoaded();  // runs the fixture once
void SpanContextShutdown();    // logs the caller counts

}  // namespace fc4
