// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "cFC4Message2Standard.h"

namespace fc4 {
// advisors.md §10: object and integer payloads occupy distinct message fields.
// The returned reference belongs to the caller; the message retains its object.
inline cFC4Message2Standard* MakeAdvisorMessage(uint32_t type, uint32_t data1, uint32_t data2,
                                               cIGZUnknown* object) {
    auto* msg = cFC4Message2Standard::New();
    msg->AddRef();
    msg->SetType(type);
    msg->SetData1(static_cast<intptr_t>(data1));
    msg->SetData2(static_cast<intptr_t>(data2));
    if (object != nullptr) msg->SetIGZUnknown(object);
    return msg;
}
}  // namespace fc4
