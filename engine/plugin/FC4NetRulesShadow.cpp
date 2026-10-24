// SPDX-License-Identifier: GPL-3.0-or-later
// Clean-room record-only NR16/NR17 boundary. See network-rules.md section 2.5.
#include "FC4NetRulesShadow.h"
#include "FC4NetRulesObservation.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "fc4.h"
#include "GZServPtrs.h"
#include "cGZPersistResourceKey.h"
#include "cIGZPersistDBRecord.h"
#include "cIGZPersistResourceManager.h"

#include <windows.h>
#include <tlhelp32.h>
#include <bcrypt.h>
#include <intrin.h>
#include <algorithm>
#include <array>
#include <cstring>
#include <map>
#include <memory>
#include <stdexcept>
#include <vector>
#pragma comment(lib, "bcrypt.lib")

namespace fc4 {
namespace {
using namespace netrules;
constexpr uint32_t kLimit = 1000000;
bool g_enabled = false, g_installed = false;
DWORD g_thread = 0;
volatile LONG g_foreignBoundary = 0;
Epoch g_epoch;
fc4_network_rules* g_rules = nullptr;
struct Identity { uint32_t type, ordinal; };
std::map<uint32_t, Identity> g_ids;
uint64_t g_evaluations = 0, g_mismatches = 0, g_incomplete = 0, g_unavailable = 0;
uint64_t g_tableMismatches = 0, g_tables = 0, g_lines = 0, g_clean = 0;
uint64_t g_lastEvaluations = 0;
void* g_trampolines[3]{};

// SEH stays in a leaf without C++ destructors. Observations fail closed on bad
// borrowed storage; the original operation remains outside this boundary.
bool Copy(void* to, uintptr_t from, size_t n) {
    __try { std::memcpy(to, reinterpret_cast<const void*>(from), n); return true; }
    __except (EXCEPTION_EXECUTE_HANDLER) { return false; }
}
uint32_t Word(uintptr_t p) {
    uint32_t v = 0;
    if (!Copy(&v, p, 4)) throw std::runtime_error("unreadable word");
    return v;
}
uint8_t Byte(uintptr_t p) {
    uint8_t v = 0;
    if (!Copy(&v, p, 1)) throw std::runtime_error("unreadable byte");
    return v;
}
uint32_t VectorCount(uintptr_t p, uint32_t stride) {
    const auto begin = Word(p), end = Word(p + 4), capacity = Word(p + 8);
    if (end < begin || capacity < end || (end - begin) % stride || (end - begin) / stride > kLimit ||
        (!begin && (end || capacity))) throw std::runtime_error("invalid vector");
    return (end - begin) / stride;
}
struct FxState {
    alignas(16) unsigned char bytes[512];
    FxState() { _fxsave(bytes); }
    ~FxState() { _fxrstor(bytes); }
};

std::vector<uint32_t> NativeRules(uint32_t evaluator) {
    if (Word(evaluator) != 0x00AA8338) throw std::runtime_error("evaluator identity");
    const auto h = Word(evaluator + 4), count = Word(evaluator + 8);
    if (count > kLimit) throw std::runtime_error("tree size");
    const auto root = Word(h + 4);
    if (!count) {
        if (root || Word(h + 8) != h || Word(h + 12) != h) throw std::runtime_error("empty header");
        return {};
    }
    if (!root || Word(root + 4) != h) throw std::runtime_error("root parent");
    std::vector<uint32_t> result, stack;
    uint32_t node = root, seen = 0, descended = 0, previous = 0, first = 0, last = 0;
    while (node || !stack.empty()) {
        while (node) {
            if (node == h || ++descended > count) throw std::runtime_error("tree cycle");
            stack.push_back(node);
            const auto child = Word(node + 8);
            if (child && Word(child + 4) != node) throw std::runtime_error("left parent");
            node = child;
        }
        node = stack.back(); stack.pop_back();
        const auto key = Word(node + 0x10);
        if ((seen && key < previous) || key != Word(node + 0x14)) throw std::runtime_error("rule key");
        if (!seen) first = node;
        last = node; previous = key; ++seen;
        const auto conditions = VectorCount(node + 0x18, 12), outputs = VectorCount(node + 0x24, 8);
        result.insert(result.end(), {key, Byte(node + 0x30), conditions, outputs});
        for (uint32_t j = 0, p = Word(node + 0x18); j < conditions; ++j, p += 12)
            result.insert(result.end(), {Byte(p), Word(p + 4), Word(p + 8)});
        for (uint32_t j = 0, p = Word(node + 0x24); j < outputs; ++j, p += 8)
            result.insert(result.end(), {Word(p), Byte(p + 4), Byte(p + 5)});
        const auto child = Word(node + 12);
        if (child && Word(child + 4) != node) throw std::runtime_error("right parent");
        node = child;
    }
    if (seen != count || first != Word(h + 8) || last != Word(h + 12)) throw std::runtime_error("tree count");
    return result;
}

// OpenDBRecord supplies effective, decompressed resource bytes through the
// game's resource manager. Never scan game installation paths or own files.
void LoadTile(uint32_t instance) {
    cIGZPersistResourceManagerPtr rm;
    if (!rm) throw std::runtime_error("resource manager unavailable");
    const cGZPersistResourceKey key(0x0A5BCF4B, 0xAA5BCF57, instance);
    cIGZPersistDBRecord* record = nullptr;
    // [abi-ret] slot 37's EAX is not a usable success channel. As in the
    // network-tool host, test its out pointer. The manager owns this reference
    // until CloseDBRecord; this observer neither AddRefs nor Releases it.
    (void)rm->OpenDBRecord(key, &record, false);
    if (!record) {
        if (!fc4_network_rules_load(g_rules, instance, nullptr, 0, nullptr)) throw std::runtime_error("missing tile load");
        return;
    }
    bool ok = false;
    try {
        const uint32_t size = record->GetSize();
        if (size > 64 * 1024 * 1024) throw std::runtime_error("resource size");
        std::vector<uint8_t> data(size + 1);
        ok = (size == 0 || record->GetFieldVoid(data.data(), size)) &&
             fc4_network_rules_load(g_rules, instance, data.data(), size, nullptr);
    } catch (...) {
        rm->CloseDBRecord(key, &record); throw;
    }
    rm->CloseDBRecord(key, &record);
    if (!ok) throw std::runtime_error("resource read/load");
}
void Invalidate() {
    g_epoch.Invalidate(); g_ids.clear();
    fc4_network_rules_destroy(g_rules); g_rules = nullptr;
}
void Snapshot() {
    g_rules = fc4_network_rules_create();
    if (!g_rules) throw std::runtime_error("core allocation");
    for (uint32_t t = 0; t < 13; ++t) {
        uint32_t instances[2]{};
        const auto n = fc4_network_rules_tile_instances(t, instances, 2);
        for (uint32_t i = 0; i < n; ++i) LoadTile(instances[i]);
        const auto record = 0x00B452C8 + t * 0x114;
        const auto nativeCount = VectorCount(record + 0x24, 4);
        uint32_t actual = 0;
        for (; actual < nativeCount; ++actual) if (!Word(Word(record + 0x24) + actual * 4)) break;
        if (actual != fc4_network_rules_evaluator_count(g_rules, t)) ++g_tableMismatches;
        for (uint32_t i = 0; i < actual; ++i) {
            const auto evaluator = Word(Word(record + 0x24) + i * 4);
            if (!g_ids.emplace(evaluator, Identity{t, i}).second) throw std::runtime_error("duplicate evaluator");
            const auto native = NativeRules(evaluator);
            const auto size = fc4_network_rules_snapshot(g_rules, t, i, nullptr, 0);
            std::vector<uint32_t> ours(size);
            fc4_network_rules_snapshot(g_rules, t, i, ours.data(), size);
            ++g_tables;
            if (ours != native) {
                ++g_tableMismatches;
                const size_t end = std::min(ours.size(), native.size());
                size_t at = 0; while (at < end && ours[at] == native[at]) ++at;
                Log("netrules shadow table difference: type=%u ordinal=%u words=%u/%u first=%u ours=%08X original=%08X",
                    t, i, size, static_cast<unsigned>(native.size()), static_cast<unsigned>(at),
                    at < ours.size() ? ours[at] : 0, at < native.size() ? native[at] : 0);
            }
        }
    }
    g_epoch.Complete();
    Log("netrules shadow: reload epoch=%u ready=1 tables=%llu table_mismatches=%llu intersections=unobserved",
        g_epoch.number, g_tables, g_tableMismatches);
}

struct Query { uint8_t kind, m; uint32_t value; };
struct Frame {
    Frame* previous = nullptr;
    uintptr_t returnTo = 0;
    uint32_t evaluator = 0, neighbourhood = 0, resultSlot = 0, originalVtable = 0;
    uint32_t type = 0, ordinal = 0, mask = 0, epoch = 0;
    int32_t x = 0, z = 0;
    bool available = false, conflict = false;
    void* vtable[2]{};
    fc4_network_rules_capture capture{};
    std::vector<Query> queries;
};
__declspec(thread) Frame* g_frame = nullptr;
Frame* Find(void* n) {
    for (auto* f = g_frame; f; f = f->previous)
        if (f->neighbourhood == reinterpret_cast<uintptr_t>(n)) return f;
    return nullptr;
}
void CaptureEdge(Frame& f, uint8_t m, uint32_t word) {
    if (f.capture.edge_known[m] && f.capture.edges[m] != word) f.conflict = true;
    f.capture.edge_known[m] = 1; f.capture.edges[m] = word;
}
uint32_t __fastcall Valid(void* n, void*, uint32_t meta) {
    auto* f = Find(n);
    using Fn = uint32_t (__thiscall*)(void*, uint32_t);
    const auto result = reinterpret_cast<Fn>(static_cast<uintptr_t>(Word(f->originalVtable)))(n, meta);
    FxState state;
    try {
        const auto m = static_cast<uint8_t>(meta), value = static_cast<uint8_t>(result ? 2 : 1);
        if (f->capture.validity[m] && f->capture.validity[m] != value) f->conflict = true;
        f->capture.validity[m] = value;
        f->queries.push_back({0, m, result});
    } catch (...) { f->conflict = true; }
    return result;
}
void __fastcall Edges(void* n, void*, uint32_t meta, uint32_t* out) {
    auto* f = Find(n);
    using Fn = void (__thiscall*)(void*, uint32_t, uint32_t*);
    reinterpret_cast<Fn>(static_cast<uintptr_t>(Word(f->originalVtable + 4)))(n, meta, out);
    FxState state;
    try {
        const auto word = Word(reinterpret_cast<uintptr_t>(out));
        CaptureEdge(*f, static_cast<uint8_t>(meta), word);
        f->queries.push_back({1, static_cast<uint8_t>(meta), word});
    } catch (...) { f->conflict = true; }
}
void EvalAfter();
void __cdecl EvalBegin(uint32_t* saved) noexcept {
    // The contract permits only synchronous owner-thread traversal. A foreign
    // boundary permanently disables observation without touching borrowed data.
    if (GetCurrentThreadId() != g_thread) { InterlockedExchange(&g_foreignBoundary, 1); return; }
    try {
        // pushad layout: EDI,ESI,EBP,ESP,EBX,EDX,ECX,EAX,flags,ret,args.
        std::unique_ptr<Frame> f(new Frame);
        f->returnTo = saved[9]; f->evaluator = saved[6];
        f->neighbourhood = saved[10]; f->resultSlot = saved[11];
        f->previous = g_frame; f->epoch = g_epoch.number;
        const auto id = g_ids.find(f->evaluator);
        if (!InterlockedCompareExchange(&g_foreignBoundary, 0, 0) && g_epoch.ready && id != g_ids.end()) {
            const auto n = f->neighbourhood;
            f->originalVtable = Word(n);
            if (f->originalVtable != 0x00AA8700) throw std::runtime_error("neighbourhood vtable");
            if (Word(f->originalVtable) != 0x00623F40 || Word(f->originalVtable + 4) != 0x00623EE0)
                throw std::runtime_error("neighbourhood slots");
            f->capture.centre = Word(n + 4);
            f->type = Word(n + 0x20); f->mask = Word(n + 0x24);
            f->x = static_cast<int32_t>(Word(n + 0x14)); f->z = static_cast<int32_t>(Word(n + 0x18));
            if (f->type != id->second.type) throw std::runtime_error("type identity");
            f->ordinal = id->second.ordinal;
            const auto count = VectorCount(n + 8, 12);
            for (uint32_t i = 0, p = Word(n + 8); i < count; ++i, p += 12) CaptureEdge(*f, Byte(p), Word(p + 4));
            f->vtable[0] = reinterpret_cast<void*>(&Valid); f->vtable[1] = reinterpret_cast<void*>(&Edges);
            *reinterpret_cast<void***>(n) = f->vtable;
            f->available = true;
        }
        g_frame = f.release(); saved[9] = reinterpret_cast<uintptr_t>(&EvalAfter);
    } catch (...) { ++g_unavailable; }
}
uintptr_t __cdecl EvalEnd(uint32_t* saved) noexcept {
    std::unique_ptr<Frame> f(g_frame);
    g_frame = f->previous;
    if (f->available) *reinterpret_cast<uint32_t*>(f->neighbourhood) = f->originalVtable;
    try {
        ++g_evaluations;
        if (!f->available || f->epoch != g_epoch.number || !g_epoch.ready) { ++g_unavailable; return f->returnTo; }
        if (f->conflict) { ++g_incomplete; return f->returnTo; }
        const bool success = (saved[7] & 0xFF) != 0;
        const auto descriptor = Word(f->resultSlot);
        if (success != (descriptor != 0)) { ++g_mismatches; return f->returnTo; }
        const auto count = descriptor ? VectorCount(descriptor, 8) : 0;
        std::vector<fc4_network_rules_output> original(count), ours(count);
        for (uint32_t i = 0, p = descriptor ? Word(descriptor) : 0; i < count; ++i, p += 8)
            original[i] = {Word(p), Byte(p + 4), Byte(p + 5), {0, 0}};
        const auto result = fc4_network_rules_replay(g_rules, f->type, f->ordinal, &f->capture, ours.data(), count);
        if (result == -2) { ++g_incomplete; return f->returnTo; }
        bool same = result >= -1 && success == (result >= 0) && (!success || result == static_cast<int32_t>(count));
        if (same && success) for (size_t i = 0; i < count; ++i)
            same = same && original[i].id == ours[i].id && original[i].m == ours[i].m &&
                   original[i].orientation == ours[i].orientation;
        if (!same) {
            if (++g_mismatches <= 8) Log("netrules shadow evaluation difference: type=%u ordinal=%u centre=%08X x=%d z=%d mask=%08X original=%d/%u ours=%d queries=%u",
                f->type, f->ordinal, f->capture.centre, f->x, f->z, f->mask, success, count, result,
                static_cast<unsigned>(f->queries.size()));
        }
    } catch (...) { ++g_unavailable; }
    return f->returnTo;
}

__declspec(thread) uintptr_t g_reloadReturn = 0;
void ReloadAfter();
void __cdecl ReloadBegin(uint32_t* saved) noexcept {
    // Recursive/concurrent reload is not a supported observation lifetime.
    if (GetCurrentThreadId() != g_thread) { InterlockedExchange(&g_foreignBoundary, 1); return; }
    if (g_reloadReturn) { ++g_unavailable; return; }
    Invalidate(); g_reloadReturn = saved[9]; saved[9] = reinterpret_cast<uintptr_t>(&ReloadAfter);
}
uintptr_t __cdecl ReloadEnd(uint32_t*) noexcept {
    const auto target = g_reloadReturn; g_reloadReturn = 0;
    try { Snapshot(); } catch (const std::exception& e) {
        ++g_unavailable; Invalidate(); Log("netrules shadow: snapshot unavailable (%s)", e.what());
    } catch (...) { ++g_unavailable; Invalidate(); }
    return target;
}
void __cdecl CleanupBegin(uint32_t*) noexcept {
    if (GetCurrentThreadId() != g_thread) { InterlockedExchange(&g_foreignBoundary, 1); return; }
    Invalidate();
}

// Integer/flags and full x87/SSE state are restored around every observer.
// Return interception leaves the original argument slots and ECX untouched;
// the original executes once through its displaced whole-instruction span.
#define NR_ENTRY(name, callback, index) \
    __declspec(naked) void name() { \
        __asm { pushfd } \
        __asm { pushad } \
        __asm { mov ebp, esp } \
        __asm { sub esp, 528 } \
        __asm { and esp, -16 } \
        __asm { fxsave [esp] } \
        __asm { push ebp } \
        __asm { call callback } \
        __asm { add esp, 4 } \
        __asm { fxrstor [esp] } \
        __asm { mov esp, ebp } \
        __asm { popad } \
        __asm { popfd } \
        __asm { jmp dword ptr [g_trampolines + index * 4] } \
    }
#define NR_AFTER(name, callback) \
    __declspec(naked) void name() { \
        __asm { push 0 } \
        __asm { pushfd } \
        __asm { pushad } \
        __asm { mov ebp, esp } \
        __asm { sub esp, 528 } \
        __asm { and esp, -16 } \
        __asm { fxsave [esp] } \
        __asm { push ebp } \
        __asm { call callback } \
        __asm { add esp, 4 } \
        __asm { mov [ebp + 36], eax } \
        __asm { fxrstor [esp] } \
        __asm { mov esp, ebp } \
        __asm { popad } \
        __asm { popfd } \
        __asm { ret } \
    }
NR_ENTRY(EvalEntry, EvalBegin, 0)
NR_ENTRY(ReloadEntry, ReloadBegin, 1)
NR_ENTRY(CleanupEntry, CleanupBegin, 2)
NR_AFTER(EvalAfter, EvalEnd)
NR_AFTER(ReloadAfter, ReloadEnd)
#undef NR_ENTRY
#undef NR_AFTER

bool Fingerprint() {
    // Only the digest is observed; no executable data is logged or exported.
    wchar_t path[MAX_PATH]{};
    if (!GetModuleFileNameW(nullptr, path, MAX_PATH)) return false;
    HANDLE file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING, 0, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    BCRYPT_ALG_HANDLE alg = nullptr; BCRYPT_HASH_HANDLE hash = nullptr;
    bool ok = BCryptOpenAlgorithmProvider(&alg, BCRYPT_SHA256_ALGORITHM, nullptr, 0) >= 0;
    if (ok) ok = BCryptCreateHash(alg, &hash, nullptr, 0, nullptr, 0, 0) >= 0;
    uint8_t buffer[16384], digest[32]{}; DWORD got = 0;
    while (ok) {
        if (!ReadFile(file, buffer, sizeof(buffer), &got, nullptr)) { ok = false; break; }
        if (!got) break;
        ok = BCryptHashData(hash, buffer, got, 0) >= 0;
    }
    if (ok) ok = BCryptFinishHash(hash, digest, sizeof(digest), 0) >= 0;
    if (hash) BCryptDestroyHash(hash);
    if (alg) BCryptCloseAlgorithmProvider(alg, 0);
    CloseHandle(file);
    constexpr uint8_t expected[] = {0x3b,0xc5,0xc7,0xfe,0x80,0x7f,0xe5,0xaa,0x24,0xa3,0xab,0xb0,0x6e,0x40,0xf1,0x34,
        0x86,0x67,0x0b,0xc1,0x15,0x06,0x12,0xc0,0x9b,0xaf,0x4e,0xce,0xda,0xfe,0x1a,0x4b};
    return ok && std::memcmp(digest, expected, 32) == 0;
}
bool SitesMatch() {
    uint8_t spans[3][6]{};
    for (size_t i = 0; i < 3; ++i) {
        MEMORY_BASIC_INFORMATION info{};
        if (!VirtualQuery(reinterpret_cast<void*>(kSites[i]), &info, sizeof(info)) ||
            info.State != MEM_COMMIT || (info.Protect & (PAGE_GUARD | PAGE_NOACCESS)) ||
            !(info.Protect & (PAGE_EXECUTE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY)) ||
            info.AllocationBase != reinterpret_cast<void*>(kBase) ||
            !Copy(spans[i], kSites[i], kLengths[i])) return false;
    }
    return Reference(reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr)), spans);
}
void Jump(uint8_t* to, uintptr_t from, uintptr_t target) {
    to[0] = 0xE9;
    const uint32_t displacement = static_cast<uint32_t>(target - from - 5);
    std::memcpy(to + 1, &displacement, 4);
}
bool Install() {
    if (!Fingerprint() || !SitesMatch()) return false;
    // Allocate all storage and acquire handles before stopping threads: never
    // allocate or acquire the CRT heap lock while another thread is suspended.
    std::vector<HANDLE> threads;
    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
    if (snapshot == INVALID_HANDLE_VALUE) return false;
    THREADENTRY32 entry{}; entry.dwSize = sizeof(entry);
    bool ok = Thread32First(snapshot, &entry) != 0;
    if (ok) do {
        if (entry.th32OwnerProcessID != GetCurrentProcessId() || entry.th32ThreadID == GetCurrentThreadId()) continue;
        HANDLE h = OpenThread(THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT | THREAD_QUERY_INFORMATION, FALSE, entry.th32ThreadID);
        if (!h) { ok = false; break; }
        threads.push_back(h);
    } while (Thread32Next(snapshot, &entry));
    CloseHandle(snapshot);
    for (size_t i = 0; i < 3 && ok; ++i) {
        auto* memory = static_cast<uint8_t*>(VirtualAlloc(nullptr, 16, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE));
        if (!memory) { ok = false; break; }
        g_trampolines[i] = memory;
        std::memcpy(memory, kBytes[i], kLengths[i]); // absolute cleanup operand unchanged
        Jump(memory + kLengths[i], reinterpret_cast<uintptr_t>(memory) + kLengths[i], kSites[i] + kLengths[i]);
        DWORD old = 0;
        ok = VirtualProtect(memory, 16, PAGE_EXECUTE_READ, &old) != 0;
        FlushInstructionCache(GetCurrentProcess(), memory, 16);
    }
    size_t suspended = 0;
    for (; suspended < threads.size() && ok; ++suspended) {
        if (SuspendThread(threads[suspended]) == static_cast<DWORD>(-1)) { ok = false; break; }
        CONTEXT context{}; context.ContextFlags = CONTEXT_CONTROL;
        if (!GetThreadContext(threads[suspended], &context)) ok = false;
        for (size_t i = 0; i < 3; ++i)
            if (context.Eip >= kSites[i] && context.Eip < kSites[i] + kLengths[i]) ok = false;
    }
    // Exact second validation under the same serialization as installation.
    if (ok) ok = SitesMatch();
    DWORD protections[3]{};
    size_t writable = 0;
    for (; writable < 3 && ok; ++writable)
        if (!VirtualProtect(reinterpret_cast<void*>(kSites[writable]), kLengths[writable], PAGE_EXECUTE_READWRITE,
                            &protections[writable])) { ok = false; break; }
    if (ok) {
        const uintptr_t targets[] = {reinterpret_cast<uintptr_t>(&EvalEntry), reinterpret_cast<uintptr_t>(&ReloadEntry),
                                     reinterpret_cast<uintptr_t>(&CleanupEntry)};
        for (size_t i = 0; i < 3; ++i) {
            uint8_t patch[6] = {0x90,0x90,0x90,0x90,0x90,0x90};
            Jump(patch, kSites[i], targets[i]);
            std::memcpy(reinterpret_cast<void*>(kSites[i]), patch, kLengths[i]);
            FlushInstructionCache(GetCurrentProcess(), reinterpret_cast<void*>(kSites[i]), kLengths[i]);
        }
    }
    for (size_t i = 0; i < writable; ++i) {
        DWORD ignored = 0;
        VirtualProtect(reinterpret_cast<void*>(kSites[i]), kLengths[i], protections[i], &ignored);
    }
    for (size_t i = 0; i < suspended; ++i) ResumeThread(threads[i]);
    for (HANDLE h : threads) CloseHandle(h);
    if (!ok) for (void*& p : g_trampolines) { if (p) VirtualFree(p, 0, MEM_RELEASE); p = nullptr; }
    return ok;
}
void Report(const char* stage, int32_t julian) {
    if (!g_enabled) return;
    fc4_date d{}; fc4_date_from_julian(julian, &d);
    const uint64_t unavailable = g_unavailable + (InterlockedCompareExchange(&g_foreignBoundary, 0, 0) ? 1 : 0);
    const bool ready = g_epoch.ready && !InterlockedCompareExchange(&g_foreignBoundary, 0, 0);
    // Before the first ready snapshot there is no comparison to report. Once
    // reporting starts, retain every result, including any later loss of readiness.
    if (!ready && !g_lines) {
        Log("netrules status (%s): awaiting comparison readiness; epoch=%u evaluations=%llu",
            stage, g_epoch.number, g_evaluations);
        return;
    }
    const bool exact = g_installed && ready && g_tables && g_evaluations && !g_tableMismatches &&
                       !g_mismatches && !g_incomplete && !unavailable;
    ++g_lines; if (exact) ++g_clean;
    Log("netrules shadow (%s) %04d-%02d-%02d: mismatches=%llu table_mismatches=%llu incomplete=%llu unavailable=%llu evaluations=%llu new_evaluations=%llu tables=%llu epoch=%u ready=%u exact=%u intersections=unobserved",
        stage, d.year, d.month, d.day_of_month, g_mismatches, g_tableMismatches, g_incomplete,
        unavailable, g_evaluations, g_evaluations - g_lastEvaluations, g_tables, g_epoch.number,
        ready, exact);
    g_lastEvaluations = g_evaluations;
}
} // namespace

void NetRulesShadowStart() {
    g_enabled = SettingInt("netrules_shadow", "FC4_NETRULES_SHADOW", 0) != 0;
    if (!g_enabled || g_installed) return;
    g_thread = GetCurrentThreadId();
    try { g_installed = Install(); } catch (...) { g_installed = false; }
    Log("netrules shadow: %s (NR16/NR17 record-only; intersections=unobserved)",
        g_installed ? "ON; three sites validated" : "requested but INACTIVE; observation set unavailable");
    if (!g_installed) ++g_unavailable;
}
void NetRulesShadowMonth(int32_t julian) { Report("month", julian); }
void NetRulesShadowCityLoaded() { Report("load", 0); }
void NetRulesShadowCityShutdown() {
    if (!g_enabled) return;
    Report("shutdown", 0);
    Log("netrules shadow: city shutdown: %llu lines, %llu exact; evaluations=%llu intersections=unobserved",
        g_lines, g_clean, g_evaluations);
}
} // namespace fc4
