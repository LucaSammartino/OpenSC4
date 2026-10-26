// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
namespace fc4 {
// PreAppInit: observe app-target deliveries and native slot-15 save returns.
// Default off; enabled by app_shadow or either DBPF shadow. Never replaces the app.
void AppShadowStart();
void AppShadowMessage(uint32_t kind);
void AppShadowObserve(int32_t julian, bool load);
void AppShadowTick(int32_t julian);
void AppShadowStop();
}
