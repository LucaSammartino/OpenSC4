// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "FC4RegionalCityState.h"

class cIGZIStream;
class cIGZOStream;
class cFC4ISC4RegionalCity;
namespace fc4 { class WriteCapture; }

namespace fc4::regionalcity {
// The enclosing record and the elements consume the same native stream. The
// context is used only during a Read/Write; retained elements copy their calls.
struct RecordContext { cIGZIStream* input; cIGZOStream* output; };
using Repair = void (*)(void*, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);
// repair is supplied by the owner, which presents the specified geometry to
// connection slot 10. All other callbacks use only the public live interfaces.
FC4RegionalCityConnectionCalls ConnectionCalls(Repair repair);
const FC4RegionalCityDealCalls& DealCalls();
FC4RegionalCityRecordCalls RecordCalls(RecordContext*, Repair repair);
// Differential of the production element path, including the record's native
// live connections/local deals. Does not mutate the record or its elements.
bool CheckLiveRecord(const WriteCapture&, cFC4ISC4RegionalCity*);
} // namespace fc4::regionalcity
