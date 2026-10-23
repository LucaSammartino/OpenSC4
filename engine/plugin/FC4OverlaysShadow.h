// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
namespace fc4 {
// Default-off, read-only getter probes. Live record walking is blocked by OV12.
void OverlaysShadowCityLoaded(cISC4City* city);
void OverlaysShadowMonth(cISC4City* city, int32_t julian);
void OverlaysShadowCityShutdown();
}
