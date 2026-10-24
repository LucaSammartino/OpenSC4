// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
namespace fc4 {
// Opt-in, once per process, no retained or serialized probe state.
void DbpfNativeCityLoaded();
// Capture all resource-manager packed files at load, then a deterministic
// monthly rotation covering every file/record within twelve months. Drain on
// framework ticks; summaries report subsets and completed coverage.
// QI references keep each selected file alive until its comparison finishes.
void DbpfShadowObserve(int32_t julian);
void DbpfShadowTick();
// Main-thread only: let an autotest quit drain the final dated sample first.
bool DbpfShadowPending();
void DbpfShadowShutdown();
}
