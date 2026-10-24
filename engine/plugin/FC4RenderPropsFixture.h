// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4RenderProperties;
namespace fc4 {
// Executes only on private native storage, never the borrowed application/view.
// Returns false if the supported original-owner binding is unavailable.
bool RenderPropsFixture(cISC4RenderProperties* application, uint32_t& fields, uint32_t& mismatches);
}
