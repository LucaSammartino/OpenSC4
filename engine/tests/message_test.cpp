// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

// Conformance tests for the plugin's standard message class
// (docs/specs/foundations.md §5.1 and §7 item 12).

#include "cFC4Message2Standard.h"

#include "cIGZSerializable.h"
#include "cIGZString.h"
#include "cRZBaseString.h"

#include <cstdio>

namespace {

int g_failures = 0;

#define CHECK(cond)                                                                       \
    do {                                                                                  \
        if (!(cond)) {                                                                    \
            std::fprintf(stderr, "%s:%d: CHECK failed: %s\n", __FILE__, __LINE__, #cond); \
            ++g_failures;                                                                 \
        }                                                                                 \
    } while (0)

// A cIGZSerializable that only counts references.
class CountingSerializable final : public cIGZSerializable {
public:
    uint32_t refs = 0;
    bool QueryInterface(uint32_t riid, void** pp) override {
        if (riid == GZIID_cIGZSerializable || riid == GZIID_cIGZUnknown) {
            *pp = static_cast<cIGZSerializable*>(this);
            AddRef();
            return true;
        }
        return false;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return refs > 0 ? --refs : 0; }
    bool Write(cIGZOStream&) override { return true; }
    bool Read(cIGZIStream&) override { return true; }
    uint32_t GetGZCLSID() override { return 0x12345678; }
};

void TestSlots() {
    cFC4Message2Standard* m = cFC4Message2Standard::New();
    m->AddRef();
    CHECK(m->GetType() == 0);
    CHECK(!m->GetHasData1() && !m->GetHasData2() && !m->GetHasData3() && !m->GetHasData4());
    int x = 0;
    m->SetVoid1(&x);
    CHECK(m->GetHasData1());
    CHECK(m->GetData1() == reinterpret_cast<intptr_t>(&x));
    CHECK(m->GetVoid1() == &x);
    CHECK(!m->GetHasData2() && !m->GetHasData3() && !m->GetHasData4());
    m->SetData2(0);  // setting a slot to 0 still marks it as set
    CHECK(m->GetHasData2());
    m->SetData3(-7);
    CHECK(m->GetHasData3() && m->GetData3() == -7);
    m->SetVoid4(&x);
    CHECK(m->GetHasData4() && m->GetData4() == reinterpret_cast<intptr_t>(&x));
    m->Release();
}

void TestReferences() {
    cFC4Message2Standard* m = cFC4Message2Standard::New();
    m->AddRef();

    cRZBaseString s("Fire Dept.");
    s.AddRef();  // our own reference
    CHECK(!m->GetHasString());
    m->SetString(&s);
    CHECK(m->GetHasString());
    CHECK(s.AddRef() == 3);  // ours + the message's + this probe
    s.Release();
    cIGZString* out = nullptr;
    CHECK(m->GetString(GZIID_cIGZString, reinterpret_cast<void**>(&out)));
    CHECK(out == &s);
    out->Release();
    m->SetString(nullptr);  // clears and releases
    CHECK(!m->GetHasString());
    CHECK(!m->GetString(GZIID_cIGZString, reinterpret_cast<void**>(&out)));
    CHECK(s.Release() == 0);

    CountingSerializable extra;
    m->SetExtra(&extra);
    CHECK(m->GetHasExtra() && extra.refs == 1);
    void* e = nullptr;
    CHECK(m->GetExtra(GZIID_cIGZSerializable, &e) == 1);
    CHECK(e == static_cast<cIGZSerializable*>(&extra) && extra.refs == 2);
    extra.Release();
    CHECK(m->GetExtra(0xDEADBEEF, &e) == 0);

    CountingSerializable unk;
    m->SetIGZUnknown(&unk);
    CHECK(m->GetHasIGZUnknown() && m->GetIGZUnknown() == static_cast<cIGZUnknown*>(&unk) && unk.refs == 1);

    m->Release();  // destruction releases every stored reference
    CHECK(extra.refs == 0);
    CHECK(unk.refs == 0);
}

void TestTypeCreateCompareAndRefcount() {
    const long base = cFC4Message2Standard::LiveCount();
    cFC4Message2Standard* a = FC4MakeMessage(0x66956814, 1, 2, 3);
    CHECK(cFC4Message2Standard::LiveCount() == base + 1);
    CHECK(a->GetType() == 0x66956814 && a->GetData1() == 1 && a->GetData2() == 2 && a->GetData3() == 3);
    CHECK(!a->GetHasData4());

    cIGZMessage2Standard* b = nullptr;
    CHECK(a->Create(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&b)));
    CHECK(b != nullptr && b != a);
    CHECK(b->GetType() == 0x66956814);
    CHECK(!b->GetHasData1() && b->GetData1() == 0);  // Create copies the type only
    CHECK(*a == *b);
    b->SetType(0x66956815);
    CHECK(!(*a == *b));
    CHECK(*a < *b && !(*b < *a));
    b->SetType(0xFFFFFFFF);
    CHECK(*a < *b);  // unsigned comparison
    CHECK(cFC4Message2Standard::LiveCount() == base + 2);
    b->Release();
    CHECK(cFC4Message2Standard::LiveCount() == base + 1);

    void* p = nullptr;
    CHECK(a->QueryInterface(cFC4Message2Standard::kIID_cIGZMessage2, &p) && p != nullptr);
    CHECK(a->QueryInterface(GZIID_cIGZUnknown, &p));
    CHECK(!a->QueryInterface(0x12345678, &p) && p == nullptr);
    CHECK(a->Release() == 2);
    CHECK(a->Release() == 1);
    CHECK(cFC4Message2Standard::LiveCount() == base + 1);
    CHECK(a->Release() == 0);  // last reference frees
    CHECK(cFC4Message2Standard::LiveCount() == base);

    cIGZUnknown* failed = nullptr;
    cFC4Message2Standard* c = FC4MakeMessage(1, 0, 0, 0);
    CHECK(!c->Create(0x12345678, reinterpret_cast<void**>(&failed)));
    CHECK(cFC4Message2Standard::LiveCount() == base + 1);
    c->Release();
    CHECK(cFC4Message2Standard::LiveCount() == base);
}

}  // namespace

int main() {
    TestSlots();
    TestReferences();
    TestTypeCreateCompareAndRefcount();
    if (g_failures == 0) std::printf("standard message tests: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}
