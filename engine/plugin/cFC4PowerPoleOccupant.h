// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "cIGZUnknown.h"
// The Rust-backed cSC4PowerPoleOccupant (enable = netocc).
cIGZUnknown* CreateFC4PowerPoleOccupant();
// first.RemoveConnection(second, quiet) (network-occupants.md §4.6) when
// `first` is a FreeCity4 pole; false (nothing done) for any other object.
bool FC4NetOccRemovePoleConnection(cIGZUnknown* first, cIGZUnknown* second, bool quiet);
// Direct-call adapters: first/second are borrowed primary pole views. False
// means first is foreign (no operation); these do not install native hooks.
bool FC4NetOccEnablePoleConnection(cIGZUnknown* first, cIGZUnknown* second, bool enable);
bool FC4NetOccAddPoleConnection(cIGZUnknown* first, cIGZUnknown* second, const void* wires);
bool FC4NetOccRebuildPoleWires(cIGZUnknown* first, cIGZUnknown* second);
bool FC4NetOccPoleAttachmentPoints(cIGZUnknown* pole, uint32_t orientation, float* out);

namespace fc4::netpole {
// §4.6.3: tool-owned header/buffer and references, never consumed by AddConnection.
struct Wire {
    int32_t x, z;
    cIGZUnknown* line;
    uint8_t pivot, padding[3];
};
struct WireVector { Wire* begin; Wire* end; Wire* capacity; };
static_assert(sizeof(void*) != 4 || sizeof(Wire) == 16, "native wire stride");
static_assert(sizeof(void*) != 4 || sizeof(WireVector) == 12, "native wire header");
}
