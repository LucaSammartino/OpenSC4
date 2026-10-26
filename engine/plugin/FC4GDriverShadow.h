// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
#include <string>
namespace fc4 {
// Opt-in gdriver_shadow_select: public selection before city resources load.
void GDriverShadowStart();
void GDriverShadowCityLoaded();
void GDriverShadowMonth(int32_t julian);
void GDriverShadowCityShutdown();
std::string GDriverShadowProbe(bool select = false);
}
