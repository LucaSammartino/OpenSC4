// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cISC4City;
namespace fc4 {
// Private native tools: draw-only keys and draft planning keys. Draft
// powerline previews are explicitly undone before releasing each tool.
void NetToolDragShadowLoaded(cISC4City* city);
void NetToolDragShadowMonth(cISC4City* city, int32_t julian);
void NetToolDragShadowShutdown();
}
