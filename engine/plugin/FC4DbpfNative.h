// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
namespace fc4 {
// Opt-in, once per process, no retained or serialized probe state.
void DbpfNativeCityLoaded();
// Resource-manager packed files at load and each month; no retained game refs.
void DbpfShadowObserve(int32_t julian);
}
