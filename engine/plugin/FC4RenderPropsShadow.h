// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
namespace fc4 {
void RenderPropsShadowCityLoaded(cISC4City* city, int32_t julian);
void RenderPropsShadowMonth(int32_t julian);
void RenderPropsShadowCityShutdown();
}
