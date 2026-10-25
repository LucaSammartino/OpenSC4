// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
namespace fc4 {
// Default-off regionalcity_shadow. Observe both the application's current
// record and every distinct region record (including the second current-city
// object; regional-city-records §5.1). Never update the native cache or save.
void RegionalCityShadowObserve(int32_t julian, bool load);
}
