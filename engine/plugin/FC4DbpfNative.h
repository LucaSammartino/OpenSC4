// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
namespace fc4 {
// Opt-in, once per process, no retained or serialized probe state.
void DbpfNativeCityLoaded();
// Capture resource-manager packed files at load/month; drain on framework ticks.
// QI references keep each selected file alive until its comparison finishes.
void DbpfShadowObserve(int32_t julian);
void DbpfShadowTick();
void DbpfShadowShutdown();
}
