// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cISC4OrdinanceSimulator;

namespace fc4 {
// Record-only, default off. Requires our simulator and the original ordinance
// simulator. Copies the original records before its month delivery, steps the
// ordinance core with read-only host callbacks, then compares the live results.
void OrdinanceShadowStartup(bool simulatorReplaced, bool ordinanceReplaced);
bool OrdinanceShadowDeliver(cIGZMessageTarget2* target, const char* name, cIGZMessage2* msg,
                            uint32_t type, int64_t julian, bool* finished);
// Test seam: capture and step private copies only. Count or -1 on rejection.
intptr_t OrdinanceShadowTest(cISC4OrdinanceSimulator* original);
}  // namespace fc4
