// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4ConfigRulesAbi.h"
#include <array>
#include <string>
namespace fc4::cfgrules {
bool MeasureSystem(FC4ConfigRulesMeasurement&, std::array<std::string, 6>&);
}
