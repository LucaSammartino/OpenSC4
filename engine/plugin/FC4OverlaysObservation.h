// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISTEOverlayManager;
namespace fc4 {
void OverlaysObservationReset();
void OverlaysObservationSample(cISTEOverlayManager*, uint32_t type, const char* phase, int32_t julian, bool fixtureEnabled = false);
void OverlaysObservationSummary();
}
