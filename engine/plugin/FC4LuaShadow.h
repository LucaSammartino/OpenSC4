// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
namespace fc4 {
// lua_shadow = 1: bounded collector/registration comparison, default off.
// LH6-LH7: full chunk/call replay has no documented interception contract.
void LuaShadowCityLoaded(cISC4City* city, int32_t julian);
void LuaShadowMonth(int32_t julian);
void LuaShadowCityShutdown();
}
