// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
class cIGZUnknown;
namespace fc4 {
void AdvisorsShadowStartup();
void AdvisorsShadowCityLoaded(cISC4City* city, int32_t julian);
void AdvisorsShadowMonth(int32_t julian);
void AdvisorsShadowCityShutdown();
// Existing test-seam dispatcher; returns mismatches after AD19 Read fixups.
uint32_t AdvisorsShadowTest(cIGZUnknown* original);
}
