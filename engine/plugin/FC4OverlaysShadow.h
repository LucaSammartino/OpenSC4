// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
namespace fc4 {
// Default-off getter, live-record and optional controlled-suffix probes.
// SPEC-GAP: no factory registration contract; section 3.8.1 specifies direct
// construction/Init, so these observations cannot enable an owner replacement.
void OverlaysShadowCityLoaded(cISC4City* city);
void OverlaysShadowOnTick(cISC4City* city);
void OverlaysShadowMonth(cISC4City* city, int32_t julian);
void OverlaysShadowCityShutdown();
}
