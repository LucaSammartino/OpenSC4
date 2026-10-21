// SPDX-License-Identifier: GPL-3.0-or-later
// FreeCity4 - advisor window slots 23-28 (FC4AdvisorWindow.h, advisors.md §12.1.1).

#include "FC4AdvisorWindow.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

namespace {

int g_failures = 0;
std::string g_log;
alignas(8) unsigned char g_block[fc4::kAdvisorWinProcSize];
int g_refs = 0;

#define CHECK(cond)                                                       \
    do {                                                                  \
        if (!(cond)) {                                                    \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);   \
            ++g_failures;                                                 \
        }                                                                 \
    } while (0)

void* Allocate(uint32_t size) {
    g_log += "alloc" + std::to_string(size) + ";";
    std::memset(g_block, 0xCD, sizeof(g_block));
    return g_block;
}
void* Construct(void* b) {
    g_log += "ctor;";
    static_cast<unsigned char*>(b)[fc4::kAdvisorWinProcVisibleOffset] = 0;
    g_refs = 0;  // AD4 (a): the constructor leaves the count at 0
    return b;
}
void Init(void*) { g_log += "init;"; }
void Show(void* p) {
    g_log += "show;";
    static_cast<unsigned char*>(p)[fc4::kAdvisorWinProcVisibleOffset] = 1;
}
void Hide(void* p) {
    g_log += "hide;";
    static_cast<unsigned char*>(p)[fc4::kAdvisorWinProcVisibleOffset] = 0;
}
void Teardown(void*) { g_log += "teardown;"; }
void AddRef(void*) {
    g_log += "addref;";
    ++g_refs;
}
void Release(void*) {
    g_log += "release;";
    --g_refs;
}

fc4::AdvisorWinProcOps Ops() {
    fc4::AdvisorWinProcOps o{};
    o.allocate = &Allocate;
    o.construct = &Construct;
    o.init = &Init;
    o.show = &Show;
    o.hide = &Hide;
    o.teardown = &Teardown;
    o.addRef = &AddRef;
    o.release = &Release;
    return o;
}

}  // namespace

int main() {
    // No window yet: 23 null, 25/26/28 no-ops, 27 false.
    {
        fc4::AdvisorWindowSlot w(Ops());
        CHECK(w.Get() == nullptr);
        w.Show();
        w.Hide();
        w.Remove();
        CHECK(!w.IsVisible());
        CHECK(g_log.empty());
    }
    // Slot 24: allocate 0x70, construct, AddRef (count 1), Init; a second call does nothing.
    {
        g_log.clear();
        fc4::AdvisorWindowSlot w(Ops());
        CHECK(w.Start());
        CHECK(g_log == "alloc112;ctor;addref;init;");
        CHECK(w.Get() == g_block);
        CHECK(g_refs == 1);
        g_log.clear();
        CHECK(w.Start());
        CHECK(g_log.empty());
        // 25 / 27 / 26
        w.Show();
        CHECK(w.IsVisible());
        w.Hide();
        CHECK(!w.IsVisible());
        CHECK(g_log == "show;hide;");
        // 28: teardown, null, Release.
        g_log.clear();
        w.Remove();
        CHECK(g_log == "teardown;release;");
        CHECK(w.Get() == nullptr);
        CHECK(g_refs == 0);
        CHECK(!w.IsVisible());
        // Start again after Remove builds a new window.
        g_log.clear();
        CHECK(w.Start());
        CHECK(g_log == "alloc112;ctor;addref;init;");
        w.Remove();
    }
    // Without operations slot 24 refuses and nothing is called.
    {
        g_log.clear();
        fc4::AdvisorWindowSlot w;
        CHECK(!w.Start());
        CHECK(w.Get() == nullptr);
        CHECK(g_log.empty());
    }
    // The private global: publish, withdraw (unconditional null), unavailable slot refused.
    {
        void* global = nullptr;
        int system = 0;
        fc4::AdvisorSystemGlobalSlot g(&global);
        CHECK(g.Available());
        CHECK(g.Publish(&system));
        CHECK(global == &system && g.Current() == &system);
        g.Withdraw();
        CHECK(global == nullptr);
        fc4::AdvisorSystemGlobalSlot none(nullptr);
        CHECK(!none.Available());
        CHECK(!none.Publish(&system));
        CHECK(none.Current() == nullptr);
    }
    if (g_failures == 0) std::printf("advisor_window_test: all checks passed\n");
    return g_failures == 0 ? EXIT_SUCCESS : EXIT_FAILURE;
}
