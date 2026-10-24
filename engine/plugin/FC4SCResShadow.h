// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
namespace fc4 {
void SCResShadowStart();
void SCResShadowLine(const char* event, int32_t julian, cISC4City* city = nullptr);
void SCResShadowStop();
// The weekly weather operation (weather 5.2) calls no resource parser and
// only queues notifications. Lend this observer's idle DR0 for that scope.
bool SCResShadowCanLendBreakpoint(uintptr_t address);
}
