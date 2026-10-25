// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
namespace fc4 {
// Default off. Assigned constructor fields, discoverable live headers and record
// codec compatibility. Constructor +10 is unassigned (MS22); no trip replay.
// No dispatch mutation, interception, hardware breakpoints or native Read.
void MySimDispatchShadowMonth(cISC4City* city, int32_t julian);
void MySimDispatchShadowCityLoaded(cISC4City* city);
void MySimDispatchShadowShutdown();
}
