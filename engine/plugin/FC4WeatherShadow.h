// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
class cIGZMessageTarget2;
class cIGZMessage2;
namespace fc4 {
void WeatherShadowCityLoaded(cISC4City* city);
void WeatherShadowMonth(cISC4City* city, int32_t julian);
void WeatherShadowCityShutdown();
bool WeatherShadowDeliver(cIGZMessageTarget2* target, const char* name, cIGZMessage2* msg,
                          uint32_t type, int64_t data1, bool* finished);
}
