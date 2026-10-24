// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
namespace fc4 {
// Default off. Constructor fixtures plus discoverable live record headers only.
// No dispatch mutation, interception, hardware breakpoints or native Read.
void MySimDispatchShadowMonth(cISC4City* city, int32_t julian);
void MySimDispatchShadowCityLoaded(cISC4City* city);
void MySimDispatchShadowShutdown();
}
