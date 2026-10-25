// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4AdvisorMessage.h"
#include <cstdio>
#include <initializer_list>

class Object : public cIGZUnknown {
public:
    uint32_t refs = 1;
    bool QueryInterface(uint32_t iid, void** out) override {
        if (iid != 1 || out == nullptr) return false;
        *out = this;
        AddRef();
        return true;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }
};

int main() {
    int failures = 0;
    auto check = [&failures](bool ok) { if (!ok) ++failures; };
    Object object;
    for (uint32_t type : {0x8A09FA64u, 0x8A09FA67u, 0x8A09FA6Au, 0x8A09FA6Cu}) {
        auto* msg = fc4::MakeAdvisorMessage(type, 0x12345678u, 19, &object);
        // Model a queued delivery: the server retains the message after the caller drops it.
        msg->AddRef();
        msg->Release();
        check(msg->GetType() == type && msg->GetData1() == 0x12345678 && msg->GetData2() == 19);
        check(msg->GetHasIGZUnknown() && msg->GetIGZUnknown() == &object && object.refs == 2);
        msg->Release();
        check(object.refs == 1);
    }
    auto* empty = fc4::MakeAdvisorMessage(0x8A09FA6Bu, 7, 8, nullptr);
    check(!empty->GetHasIGZUnknown() && empty->GetData1() == 7 && empty->GetData2() == 8);
    empty->Release();
    check(cFC4Message2Standard::LiveCount() == 0);
    std::printf("advisor_message_test: %d failures\n", failures);
    return failures ? 1 : 0;
}
