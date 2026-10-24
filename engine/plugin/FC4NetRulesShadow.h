// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
namespace fc4 {
// Default-off, record-only. Hooks remain for process lifetime: original static
// cleanup can run after PreAppShutdown. No native pointers survive an epoch.
void NetRulesShadowStart();
void NetRulesShadowMonth(int32_t julian);
void NetRulesShadowCityLoaded();
void NetRulesShadowCityShutdown();
}
