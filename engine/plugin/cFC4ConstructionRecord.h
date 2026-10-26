// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
class cIGZUnknown;

// A record-only staging factory for the construction owner. Deliberately NOT
// in the subsystem table: this object does not expose occupant/lifecycle views.
// Registering it as cSC4ConstructionOccupant would break natural city loading.
cIGZUnknown* CreateFC4ConstructionRecord();

namespace fc4 {
// a = borrowed source writer (null selects the constructor record), b = output
// cIGZSerializable** receiving one owned reference. No city objects are changed.
intptr_t ConstructionRecordFactoryTest(cIGZUnknown* source, void* output);
}
