// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
class cISC4City;
namespace fc4 {
// Constructed original versus independent Rust only. IN3 remains excluded.
void IndustrialShadowStartup(bool replaced);
void IndustrialShadowProbe(cISC4City* city, const char* phase);
}
