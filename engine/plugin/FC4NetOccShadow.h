// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
namespace fc4 {
// Record-only; netocc_shadow defaults off. Never registers replacement classes.
void NetOccShadowCityLoaded(cISC4City* city, int32_t julian);
void NetOccShadowMonth(int32_t julian);
void NetOccShadowCityShutdown();
}
