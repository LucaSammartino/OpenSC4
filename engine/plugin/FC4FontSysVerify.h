// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
namespace fc4 {
// Opt-in constructed verification; never acquires or shuts down a global service.
void FontSysVerify(const char* phase);
}
