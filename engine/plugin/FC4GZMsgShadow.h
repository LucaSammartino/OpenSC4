// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
namespace fc4 {
// gzmsg_shadow = 1, default off; GZ11 bounded lookup scope, no full-state credit.
void GZMsgShadowCityLoaded(cISC4City* city, int32_t julian);
void GZMsgShadowMonth(int32_t julian);
void GZMsgShadowCityShutdown();
}
