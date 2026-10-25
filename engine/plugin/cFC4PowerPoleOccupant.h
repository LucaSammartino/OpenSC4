// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "cIGZUnknown.h"
// The Rust-backed cSC4PowerPoleOccupant (enable = netocc).
cIGZUnknown* CreateFC4PowerPoleOccupant();
// first.RemoveConnection(second, quiet) (network-occupants.md §4.6) when
// `first` is a FreeCity4 pole; false (nothing done) for any other object.
bool FC4NetOccRemovePoleConnection(cIGZUnknown* first, cIGZUnknown* second, bool quiet);
