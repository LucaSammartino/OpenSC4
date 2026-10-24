// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
namespace fc4 {
// simgrid_shadow=1 only. SG3 owner/getter scope, record reads, no mutations.
void SimGridRecordShadowObserve(cISC4City* city, int32_t julian, bool loaded);
}
