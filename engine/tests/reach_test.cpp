// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4Reach.h"
#include <windows.h>
#include <algorithm>
#include <atomic>
#include <cstdio>
#include <fstream>
#include <sstream>
#include <thread>

namespace {
int failures = 0;
#define CHECK(x) do { if (!(x)) { std::printf("FAIL line %d: %s\n", __LINE__, #x); ++failures; } } while (0)
using IntFn = int(__cdecl*)(int);
using FaultFn = int(__fastcall*)(const int*);
using FlagsFn = unsigned(__cdecl*)();
volatile LONG foreignExceptions = 0;
volatile LONG translatedBreakpoints = 0, translatedSteps = 0;

// Exercise actual INT3/TF delivery and re-arming using the WoW64 status codes,
// even on a host that delivers only the usual Win32 exception codes.
LONG CALLBACK AsWx86(EXCEPTION_POINTERS* ep) {
    auto& code = ep->ExceptionRecord->ExceptionCode;
    if (code == EXCEPTION_BREAKPOINT) {
        code = 0x4000001f;
        InterlockedIncrement(&translatedBreakpoints);
    } else if (code == EXCEPTION_SINGLE_STEP) {
        code = 0x4000001e;
        InterlockedIncrement(&translatedSteps);
    }
    return EXCEPTION_CONTINUE_SEARCH;
}

LONG CALLBACK Foreign(EXCEPTION_POINTERS* ep) {
    if (ep->ExceptionRecord->ExceptionCode == 0xe0424242 ||
        ep->ExceptionRecord->ExceptionCode == EXCEPTION_SINGLE_STEP ||
        ep->ExceptionRecord->ExceptionCode == EXCEPTION_BREAKPOINT ||
        ep->ExceptionRecord->ExceptionCode == 0x4000001e ||
        ep->ExceptionRecord->ExceptionCode == 0x4000001f) {
        InterlockedIncrement(&foreignExceptions);
        return EXCEPTION_CONTINUE_EXECUTION;
    }
    return EXCEPTION_CONTINUE_SEARCH;
}

bool FaultIsCaught(FaultFn fn) {
    __try { fn(nullptr); }
    __except(GetExceptionCode() == EXCEPTION_ACCESS_VIOLATION ? EXCEPTION_EXECUTE_HANDLER : EXCEPTION_CONTINUE_SEARCH) {
        return true;
    }
    return false;
}

unsigned WithCarry(FlagsFn fn, bool carry) {
    unsigned result;
    if (carry) {
        __asm {
            stc
            call fn
            mov result, eax
        }
    } else {
        __asm {
            clc
            call fn
            mov result, eax
        }
    }
    return result;
}

fc4::reach::Row RowAt(const fc4::reach::Tracer& t, FARPROC fn) {
    for (const auto& r : t.Rows()) if (r.address == reinterpret_cast<uint32_t>(fn)) return r;
    CHECK(false);
    return {};
}
}

int main(int argc, char** argv) {
    if (argc != 2 && argc != 3) return 2;
    std::string error;
    std::vector<fc4::reach::Contract> parsed;
    std::istringstream good("# metadata\naddress\tsize\tname\tconvention\r\n00401000\t8\tA\tcdecl\n");
    CHECK(fc4::reach::ReadContracts(good, parsed, error));
    CHECK(parsed.size() == 1 && parsed[0].address == 0x401000 && parsed[0].size == 8);
    std::istringstream duplicate("address\tsize\tname\n00401000\t1\tA\n00401000\t1\tB\n");
    CHECK(!fc4::reach::ReadContracts(duplicate, parsed, error));
    std::istringstream malformed("address\tsize\tname\n00401000\t-1\tA\n");
    CHECK(!fc4::reach::ReadContracts(malformed, parsed, error));
    std::istringstream unknownSize("address\tsize\tname\n00401000\t0\t\n");
    CHECK(fc4::reach::ReadContracts(unknownSize, parsed, error));
    CHECK(parsed.size() == 1 && parsed[0].size == 0 && parsed[0].name.empty());
    if (argc == 3) {
        std::ifstream actual(argv[2]);
        CHECK(fc4::reach::ReadContracts(actual, parsed, error));
        std::printf("reach ABI roster parsed rows=%zu error=%s\n", parsed.size(), error.c_str());
    }

    HMODULE dll = LoadLibraryA(argv[1]);
    CHECK(dll != nullptr);
    if (!dll) return 2;
    const char* names[] = {"ReachFixtureA", "ReachFixtureB", "ReachFixtureFloat", "ReachFixtureFlags",
        "ReachFixtureRet", "ReachFixtureJump", "ReachFixtureHook", "@ReachFixtureFault@4", "ReachFixturePushFlags"};
    FARPROC fn[9]{};
    std::vector<fc4::reach::Contract> contracts;
    for (size_t i = 0; i < 9; ++i) {
        fn[i] = GetProcAddress(dll, names[i]);
        CHECK(fn[i] != nullptr);
        if (!fn[i]) return 2;
        contracts.push_back({reinterpret_cast<uint32_t>(fn[i]), i == 4 ? 0u : 4u, names[i]});
    }
    std::sort(contracts.begin(), contracts.end(), [](const auto& a, const auto& b) { return a.address < b.address; });
    const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(dll);
    const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS*>(reinterpret_cast<const uint8_t*>(dll) + dos->e_lfanew);
    auto* counts = reinterpret_cast<volatile LONG*>(GetProcAddress(dll, "reach_fixture_counts"));
    CHECK(counts != nullptr);
    if (!counts) return 2;

    auto a = reinterpret_cast<IntFn>(fn[0]);
    auto b = reinterpret_cast<IntFn>(fn[1]);
    auto floating = reinterpret_cast<float(__cdecl*)(float)>(fn[2]);
    auto flags = reinterpret_cast<FlagsFn>(fn[3]);
    auto ret = reinterpret_cast<void(__cdecl*)()>(fn[4]);
    auto fault = reinterpret_cast<FaultFn>(fn[7]);
    void* foreign = AddVectoredExceptionHandler(0, Foreign);
    {
        fc4::reach::Tracer tracer;
        CHECK(tracer.Prepare(contracts, reinterpret_cast<uintptr_t>(dll), reinterpret_cast<uintptr_t>(dll),
                             nt->OptionalHeader.SizeOfImage));
        // Simulate a hook that kept byte 0 but changed another entry byte.
        auto* hook = reinterpret_cast<uint8_t*>(fn[6]);
        DWORD old = 0, unused = 0;
        CHECK(VirtualProtect(hook, 4, PAGE_EXECUTE_READWRITE, &old) != 0);
        const uint8_t saved = hook[1];
        hook[1] ^= 1;
        CHECK(tracer.Start(32));
        CHECK(RowAt(tracer, fn[6]).skipped == "entry-changed-by-hook");
        hook[1] = saved;
        VirtualProtect(hook, 4, old, &unused);
        CHECK(RowAt(tracer, fn[5]).skipped == "preexisting-jump-or-hook");
        CHECK(RowAt(tracer, fn[8]).skipped == "trap-unsafe-prefix");
        // Restoring the hook page's protection could remove RWX used by the
        // tracer on the same page; leave it writable until tracer teardown.
        VirtualProtect(hook, 4, PAGE_EXECUTE_READWRITE, &unused);

        for (int i = 0; i < 10; ++i) CHECK(a(i) == i + 7);
        for (int i = 0; i < 40; ++i) CHECK(b(i) == i * 3);
        CHECK(RowAt(tracer, fn[0]).entries == static_cast<uint32_t>(counts[0]));
        CHECK(RowAt(tracer, fn[1]).entries == 32 && counts[1] == 40);
        CHECK(RowAt(tracer, fn[1]).capped);
        tracer.NextWindow();
        for (int i = 0; i < 7; ++i) CHECK(a(i) == i + 7);
        void* ignore = tracer.IgnoreThread();
        for (int i = 0; i < 5; ++i) CHECK(a(i) == i + 7);
        tracer.UnignoreThread(ignore);
        tracer.NextWindow();
        CHECK(a(0) == 7);
        auto row = RowAt(tracer, fn[0]);
        CHECK(row.entries == 18 && counts[0] == 23 && row.windowsHit == 3);
        CHECK(row.firstWindow == 1 && row.lastWindow == 3 && !row.capped);

        // Actual concurrent game threads, distinct entries: exact counts.
        std::thread t1([&] { for (int i = 0; i < 10; ++i) a(i); });
        std::thread t2([&] { for (int i = 0; i < 12; ++i) b(i); });
        t1.join(); t2.join();
        CHECK(RowAt(tracer, fn[0]).entries == 28 && counts[0] == 33);
        CHECK(RowAt(tracer, fn[1]).entries == 44 && counts[1] == 52);
        CHECK(floating(-1.25f) == -1.25f);
        CHECK(floating(3.5f) == 3.5f);
        CHECK(RowAt(tracer, fn[2]).entries == static_cast<uint32_t>(counts[2]));
        CHECK(WithCarry(flags, true) == 1 && WithCarry(flags, false) == 0);
        const auto retBefore = RowAt(tracer, fn[4]).entries;
        CHECK(retBefore == 0);
        ret(); ret();
        CHECK(RowAt(tracer, fn[4]).entries == retBefore + 2);
        CHECK(FaultIsCaught(fault));
        const int value = 77;
        CHECK(fault(&value) == 77);
        CHECK(RowAt(tracer, fn[7]).entries == 2);
        CHECK(tracer.Suspend());
        CHECK(a(0) == 7);  // no breakpoint during a constructed probe pass
        CHECK(RowAt(tracer, fn[0]).entries == 28);
        tracer.Resume();
        CHECK(a(0) == 7);
        CHECK(RowAt(tracer, fn[0]).entries == 29);
        RaiseException(0xe0424242, 0, 0, nullptr);
        RaiseException(EXCEPTION_SINGLE_STEP, 0, 0, nullptr);
        RaiseException(EXCEPTION_BREAKPOINT, 0, 0, nullptr);
        RaiseException(0x4000001e, 0, 0, nullptr);
        RaiseException(0x4000001f, 0, 0, nullptr);
        CHECK(foreignExceptions == 5);
        const auto s = tracer.GetSummary();
        CHECK(s.ignored == 5 && s.abortedSteps == 1 && s.slotExhaustions == 0);
        tracer.Stop();
        CHECK(a(0) == 7);
        CHECK(RowAt(tracer, fn[0]).entries == 29);
        CHECK(tracer.Write("reach-synthetic.tsv"));
        std::printf("reach synthetic exact A=29/29 B=44/44 float=2/2 flags=2/2 ret=2/2 fault=2/2 windows=3 cap=32 ignored=5 skipped=3 suspend_resume=1 traps=%u aborted=%u\n", s.traps, s.abortedSteps);
    }
    {
        fc4::reach::Tracer tracer;
        CHECK(tracer.Prepare(contracts, reinterpret_cast<uintptr_t>(dll), reinterpret_cast<uintptr_t>(dll),
                             nt->OptionalHeader.SizeOfImage));
        CHECK(tracer.Start(4));
        void* translator = AddVectoredExceptionHandler(1, AsWx86);
        CHECK(translator != nullptr);
        const LONG before = counts[0];
        for (int i = 0; i < 7; ++i) CHECK(a(i) == i + 7);
        CHECK(RowAt(tracer, fn[0]).entries == 4);
        CHECK(*reinterpret_cast<const uint8_t*>(fn[0]) == 0x90);
        tracer.NextWindow();
        CHECK(*reinterpret_cast<const uint8_t*>(fn[0]) == 0xcc);
        for (int i = 0; i < 3; ++i) CHECK(a(i) == i + 7);
        const auto row = RowAt(tracer, fn[0]);
        const auto s = tracer.GetSummary();
        CHECK(row.entries == 7 && row.windowsHit == 2 && row.capped);
        CHECK(counts[0] - before == 10);
        CHECK(s.wxBreakpoints == 7 && s.wxSteps == 7 && s.traps == 14 && s.abortedSteps == 0);
        CHECK(translatedBreakpoints == 7 && translatedSteps == 7);
        CHECK(*reinterpret_cast<const uint8_t*>(fn[0]) == 0xcc);
        tracer.Stop();
        CHECK(*reinterpret_cast<const uint8_t*>(fn[0]) == 0x90);
        CHECK(RemoveVectoredExceptionHandler(translator) != 0);
        std::printf("reach WoW64 exact breakpoints=%u steps=%u entries=7/7 body=10/10 windows=2 rearmed=1\n",
                    s.wxBreakpoints, s.wxSteps);
    }
    {
        // Same-entry concurrency is safe, but byte-restoration necessarily
        // makes counts a lower bound. Quantify the loss rather than hiding it.
        fc4::reach::Tracer tracer;
        CHECK(tracer.Prepare(contracts, reinterpret_cast<uintptr_t>(dll), reinterpret_cast<uintptr_t>(dll),
                             nt->OptionalHeader.SizeOfImage));
        const LONG before = counts[0];
        CHECK(tracer.Start(1000000));
        std::atomic<bool> go{false};
        std::thread t1([&] { while (!go.load()) YieldProcessor(); for (int i = 0; i < 5000; ++i) a(i); });
        std::thread t2([&] { while (!go.load()) YieldProcessor(); for (int i = 0; i < 5000; ++i) a(i); });
        go.store(true);
        t1.join(); t2.join();
        tracer.Stop();
        const auto reached = RowAt(tracer, fn[0]).entries;
        CHECK(counts[0] - before == 10000);
        CHECK(reached > 0 && reached <= 10000);
        std::printf("reach concurrent observed=%u independent=10000 missed=%u (restore/step lower bound)\n", reached, 10000 - reached);
    }
    RemoveVectoredExceptionHandler(foreign);
    FreeLibrary(dll);
    std::printf("reach tests failures=%d\n", failures);
    return failures ? 1 : 0;
}
