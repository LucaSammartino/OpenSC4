// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
namespace fc4 {
// app_shadow=1 only. Main-thread observer callbacks; never replaces the app.
void AppShadowStart();
void AppShadowMessage(uint32_t kind);
void AppShadowObserve(int32_t julian, bool load);
void AppShadowTick(int32_t julian);
void AppShadowStop();
}
