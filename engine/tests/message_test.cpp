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

#include <cstdint>
#include <cstdio>
#include <cstring>

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

    // One object, one vtable: every interface pointer is the same address, so a
    // receiver may cast cIGZMessage2* to cIGZMessage2Standard* directly.
    void* p = nullptr;
    CHECK(a->QueryInterface(cFC4Message2Standard::kIID_cIGZMessage2, &p) && p != nullptr);
    CHECK(p == static_cast<void*>(static_cast<cIGZMessage2Standard*>(a)));
    CHECK(a->QueryInterface(GZIID_cIGZUnknown, &p));
    CHECK(p == static_cast<void*>(static_cast<cIGZMessage2Standard*>(a)));
    CHECK(!a->QueryInterface(0x12345678, &p) && p == nullptr);
    {
        // Read back through the base interfaces exactly as a receiving agent would.
        cIGZMessage2* base = a;
        CHECK(base->GetType() == 0x66956814);
        auto* std2 = static_cast<cIGZMessage2Standard*>(base);
        CHECK(std2->GetData1() == 1 && std2->GetData2() == 2 && std2->GetData3() == 3);
        CHECK(std2->GetHasData1() && std2->GetHasData2() && std2->GetHasData3() && !std2->GetHasData4());
    }
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

// The game's memory layout (docs/abi/layouts.md): a receiver may read the
// fields directly. Flags byte at +0x08: bit 0 type, 1-4 data1-4, 5 extra,
// 6 string, 7 unknown; type at +0x0C; data1..4 at +0x10; string +0x20,
// unknown +0x24, extra +0x28; vtable slot 37 the scalar deleting destructor.
uint8_t FlagsOf(const cFC4Message2Standard* m) { return reinterpret_cast<const uint8_t*>(m)[0x08]; }
template <class T>
T FieldAt(const cFC4Message2Standard* m, int offset) {
    T v;
    std::memcpy(&v, reinterpret_cast<const uint8_t*>(m) + offset, sizeof(v));
    return v;
}

void TestLayout() {
    cFC4Message2Standard* m = cFC4Message2Standard::New();
    m->AddRef();
    CHECK(static_cast<void*>(static_cast<cIGZMessage2Standard*>(m)) == static_cast<void*>(m));
    CHECK(FieldAt<uint32_t>(m, 0x04) == 1);  // reference count
    CHECK(FlagsOf(m) == 0x00);
    m->SetType(0x26C63341);
    CHECK(FlagsOf(m) == 0x01 && FieldAt<uint32_t>(m, 0x0C) == 0x26C63341);
    m->SetData1(11);
    CHECK(FlagsOf(m) == 0x03 && FieldAt<intptr_t>(m, 0x10) == 11);
    m->SetData2(0);  // a zero value still sets the bit
    CHECK(FlagsOf(m) == 0x07 && FieldAt<intptr_t>(m, 0x14) == 0);
    int x = 0;
    m->SetVoid3(&x);
    CHECK(FlagsOf(m) == 0x0F && FieldAt<void*>(m, 0x18) == &x);
    m->SetData4(-4);
    CHECK(FlagsOf(m) == 0x1F && FieldAt<intptr_t>(m, 0x1C) == -4);

    CountingSerializable extra;
    m->SetExtra(&extra);
    CHECK(FlagsOf(m) == 0x3F && FieldAt<void*>(m, 0x28) == static_cast<cIGZSerializable*>(&extra));
    cRZBaseString s("layout");
    s.AddRef();
    m->SetString(&s);
    CHECK(FlagsOf(m) == 0x7F && FieldAt<void*>(m, 0x20) == static_cast<cIGZString*>(&s));
    CountingSerializable unk;
    m->SetIGZUnknown(&unk);
    CHECK(FlagsOf(m) == 0xFF && FieldAt<void*>(m, 0x24) == static_cast<cIGZUnknown*>(&unk));

    // Null references and a zero type clear their bits; data bits stay set.
    m->SetString(nullptr);
    CHECK(FlagsOf(m) == 0xBF && FieldAt<void*>(m, 0x20) == nullptr);
    m->SetIGZUnknown(nullptr);
    CHECK(FlagsOf(m) == 0x3F);
    m->SetExtra(nullptr);
    CHECK(FlagsOf(m) == 0x1F && extra.refs == 0);
    m->SetType(0);
    CHECK(FlagsOf(m) == 0x1E && FieldAt<uint32_t>(m, 0x0C) == 0);
    m->SetData1(0);
    CHECK(FlagsOf(m) == 0x1E);
    m->SetType(7);
    CHECK(FlagsOf(m) == 0x1F);

    // Create: a new message with the type only.
    cFC4Message2Standard* c = nullptr;
    CHECK(m->Create(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&c)) && c != nullptr);
    if (c != nullptr) {
        CHECK(FlagsOf(c) == 0x01 && FieldAt<uint32_t>(c, 0x0C) == 7);
        c->Release();
    }
    m->Release();
    CHECK(s.Release() == 0);

#if defined(_M_IX86)
    // Slot 37: the scalar deleting destructor, __thiscall(this, flags); flags 1 frees.
    const long base = cFC4Message2Standard::LiveCount();
    cFC4Message2Standard* d = cFC4Message2Standard::New();
    CHECK(cFC4Message2Standard::LiveCount() == base + 1);
    void** vtbl = *reinterpret_cast<void***>(d);
    using DeletingDtor = void*(__thiscall*)(void*, unsigned int);
    reinterpret_cast<DeletingDtor>(vtbl[37])(d, 1);
    CHECK(cFC4Message2Standard::LiveCount() == base);
#endif
}

}  // namespace

int main() {
    TestLayout();
    TestSlots();
    TestReferences();
    TestTypeCreateCompareAndRefcount();
    if (g_failures == 0) std::printf("standard message tests: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}
