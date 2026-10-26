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
// Hold a balanced hidden pause from sample capture until pending reads drain.
// Tick releases it when the queue empties; Shutdown releases it on interruption.
bool DbpfShadowWait();
// Invalidate readers before native save/re-registration; harness saves wait
// for Pending() to clear at ordinary framework ticks before invoking SaveCity.
void DbpfShadowBeforeSave();
// Observed public slot-15 entry and actual caller continuation (EX58).
void DbpfShadowSaveEnter(void* app, void* caller);
void DbpfShadowSaveReturned(void* app, bool result);
void DbpfShadowShutdown();
}
