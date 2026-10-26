// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4DrawStreamCapture.h"
#ifndef FC4_DRAW_STREAM_TEST
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4HarnessLogic.h"
#include "compat/FC4Compat.h"
#include <array>
#include <atomic>
#include <windows.h>
#endif

namespace fc4 {
namespace drawstream {
namespace {
thread_local Capture* current = nullptr;
#ifndef FC4_DRAW_STREAM_TEST
thread_local unsigned forwarding = 0;
uintptr_t original[48]{};
struct Forwarding { Forwarding() { ++forwarding; } ~Forwarding() { --forwarding; } };

template<unsigned Slot, class... Args>
uint32_t __fastcall Tap(void* self, void*, Args... args) {
    auto* capture = forwarding ? nullptr : Capture::Current(self);
    if (capture) {
        try { capture->Before(Slot, {static_cast<uint32_t>(args)...}); }
        catch (...) { capture->Refuse(); }
    }
    uint32_t result;
    {
        Forwarding guard;
        // Integer-register transport preserves EAX without assigning a meaning
        // to it for void methods. These slots have no x87 return values.
        using Fn = uint32_t(__fastcall*)(void*, void*, Args...);
        result = reinterpret_cast<Fn>(original[Slot])(self, nullptr, args...);
    }
    if (capture) {
        try { capture->After(Slot, {static_cast<uint32_t>(args)...}, result); }
        catch (...) { capture->Refuse(); }
    }
    return result;
}

template<unsigned Slot, class... Args>
void Bind(std::array<void*, 48>& hooks) { hooks[Slot] = reinterpret_cast<void*>(&Tap<Slot, Args...>); }
#endif
}  // namespace

Capture::Capture(void* context, Stream& stream, Objects& objects)
    : context_(context), stream_(stream), objects_(objects), previous_(current) {
    current = this;
    // This adapter has no font/surface or complete owner-output binding yet.
    // Context-only traces must never produce complete method credit.
    stream_.Refuse("context-only capture: font/surface/owner binding required");
}
Capture::~Capture() { current = previous_; }
Capture* Capture::Current(void* context) {
    return current && current->context_ == context ? current : nullptr;
}

void Capture::Before(uint32_t slot, std::initializer_list<uint32_t> arguments) {
    Event event{Domain::Context, slot, objects_.Borrow(context_), arguments, {}};
    auto copy = [&](uint32_t address, size_t size) {
        if (size > 1024 * 1024 || (size && !address)) { stream_.Refuse("invalid context payload"); return; }
        if (size) {
            const auto* bytes = reinterpret_cast<const uint8_t*>(address);
            event.bytes.insert(event.bytes.end(), bytes, bytes + size);
        }
    };
    switch (slot) {
    case 5: case 6: case 31: case 35:
        copy(event.words[0], 16); event.words.clear(); break;
    case 7: case 8:
        copy(event.words[0], 8); event.words.clear(); break;
    case 12: case 13: case 14:
        event.words.clear(); break; // output pointers, copied after forwarding
    case 29:
        event.words[0] = objects_.Borrow(reinterpret_cast<const void*>(event.words[0]));
        stream_.Refuse("SetBlt image key not bound"); break;
    case 38: case 39:
        copy(event.words[1], 16); copy(event.words[2], 16);
        event.words[0] = objects_.Borrow(reinterpret_cast<const void*>(event.words[0]));
        event.words[1] = event.words[2] = 0;
        stream_.Refuse("Blt image key not bound"); break;
    case 44: case 45:
        if (event.words[2] > 1024 * 1024 / 8) stream_.Refuse("context point capacity");
        else copy(event.words[1], static_cast<size_t>(event.words[2]) * 8);
        event.words[1] = 0; break;
    default: break;
    }
    stream_.Append(std::move(event));
}
void Capture::After(uint32_t slot, std::initializer_list<uint32_t> arguments, uint32_t result) {
    const auto id = objects_.Borrow(context_);
    if (slot == 12 || slot == 13 || slot == 14) {
        stream_.Data(Domain::Return, slot, id, reinterpret_cast<const void*>(*arguments.begin()), slot == 14 ? 8 : 16);
    } else if (slot == 15 || slot == 16 || slot == 18 || slot == 41) {
        if (slot == 18) result = objects_.Borrow(reinterpret_cast<const void*>(result));
        if (slot == 16 || slot == 41) result &= 0xff;
        stream_.Call(Domain::Return, slot, id, {result});
    }
}
}  // namespace drawstream

#ifndef FC4_DRAW_STREAM_TEST
namespace {
uintptr_t paintOriginal[2]{};
struct Counts { std::atomic<uint64_t> draws{0}, events{0}, incomplete{0}; } counts[2];
template<unsigned Owner>
bool __fastcall Paint(void* self, void*) noexcept {
    using Fn = bool(__fastcall*)(void*, void*);
    const auto native = reinterpret_cast<Fn>(paintOriginal[Owner]);
    bool called = false, result = false;
    size_t events = 0;
    try {
        drawstream::Stream stream;
        drawstream::Objects objects;
        void* context = *reinterpret_cast<void**>(static_cast<uint8_t*>(self) + 0x6c); // widget-toolkit 14.1
        {
            drawstream::Capture capture(context, stream, objects);
            called = true;
            result = native(self, nullptr);
        }
        stream.Call(drawstream::Domain::Return, 88, objects.Borrow(self), {result ? 1u : 0u});
        events = stream.Events().size();
    } catch (...) {
        // Allocation/serialization failure cannot skip or repeat the native draw.
        if (!called) { try { result = native(self, nullptr); } catch (...) {} }
    }
    ++counts[Owner].draws;
    counts[Owner].events += events;
    ++counts[Owner].incomplete; // context-only adapter, including capture failures
    return result;
}
bool Patch(void** table, const void* const* replacements, size_t length) {
    DWORD protection;
    if (!VirtualProtect(table, length * sizeof(void*), PAGE_READWRITE, &protection)) return false;
    for (size_t i = 0; i < length; ++i)
        if (replacements[i]) InterlockedExchangePointer(table + i, const_cast<void*>(replacements[i]));
    DWORD ignored;
    VirtualProtect(table, length * sizeof(void*), protection, &ignored);
    return true;
}
}  // namespace

void DrawStreamStartup() {
    if (SettingInt("drawstream_shadow", nullptr, 0) != 1) return;
    void** context = nullptr;
    void** tree = nullptr;
    void** list = nullptr;
    if (!compat::DrawStreamTables(&context, &tree, &list)) {
        Log("drawstream capture unavailable: unsupported context/window tables");
        return;
    }
    // Native tables retain these callbacks until process exit.
    HMODULE pinned = nullptr;
    if (!GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN,
                           reinterpret_cast<LPCSTR>(&DrawStreamStartup), &pinned)) {
        Log("drawstream capture unavailable: DLL pin failed");
        return;
    }
    std::array<void*, 48> hooks{};
    using namespace drawstream;
    Bind<3>(hooks); Bind<4>(hooks); Bind<5, uint32_t>(hooks); Bind<6, uint32_t>(hooks);
    Bind<7, uint32_t>(hooks); Bind<8, uint32_t>(hooks); Bind<9>(hooks); Bind<10>(hooks); Bind<11>(hooks);
    Bind<12, uint32_t>(hooks); Bind<13, uint32_t>(hooks); Bind<14, uint32_t>(hooks);
    Bind<15>(hooks); Bind<16>(hooks); Bind<18>(hooks); Bind<19>(hooks);
    Bind<20, uint32_t, uint32_t>(hooks); Bind<21, uint32_t>(hooks); Bind<22, uint32_t>(hooks);
    Bind<23, uint32_t>(hooks); Bind<24, uint32_t>(hooks); Bind<25, uint32_t>(hooks);
    Bind<26>(hooks); Bind<28>(hooks); Bind<29, uint32_t, uint32_t, uint32_t, uint32_t, uint32_t>(hooks);
    Bind<30>(hooks); Bind<31, uint32_t>(hooks); Bind<32>(hooks); Bind<33, uint32_t, uint32_t>(hooks);
    Bind<34, uint32_t, uint32_t, uint32_t, uint32_t>(hooks); Bind<35, uint32_t>(hooks);
    Bind<36, uint32_t, uint32_t, uint32_t, uint32_t>(hooks); Bind<37, uint32_t, uint32_t, uint32_t, uint32_t>(hooks);
    Bind<38, uint32_t, uint32_t, uint32_t>(hooks);
    Bind<39, uint32_t, uint32_t, uint32_t, uint32_t, uint32_t>(hooks);
    Bind<40>(hooks); Bind<41>(hooks); Bind<42>(hooks); Bind<43>(hooks);
    Bind<44, uint32_t, uint32_t, uint32_t>(hooks); Bind<45, uint32_t, uint32_t, uint32_t>(hooks);
    for (unsigned i = 0; i < 48; ++i) original[i] = reinterpret_cast<uintptr_t>(context[i]);
    if (!Patch(context, hooks.data(), hooks.size())) return;
    paintOriginal[0] = reinterpret_cast<uintptr_t>(tree[88]);
    paintOriginal[1] = reinterpret_cast<uintptr_t>(list[88]);
    const void* treeHook = reinterpret_cast<void*>(&Paint<0>);
    const void* listHook = reinterpret_cast<void*>(&Paint<1>);
    const bool treeOn = Patch(tree + 88, &treeHook, 1), listOn = Patch(list + 88, &listHook, 1);
    size_t tapped = 0;
    for (const auto hook : hooks) if (hook) ++tapped;
    Log("drawstream capture installed tree=%u list=%u context_slots=%u compare=0 complete=0", treeOn, listOn,
        static_cast<unsigned>(tapped));
}
void DrawStreamMonth(int32_t julian) {
    if (SettingInt("drawstream_shadow", nullptr, 0) != 1) return;
    const auto date = harness::CivilFromJulian(julian);
    const char* owners[] = {"cGZWinTreeView", "cGZWinListBox"};
    for (unsigned i = 0; i < 2; ++i)
        Log("drawstream capture %s (month) %04d-%02d-%02d: draws=%llu events=%llu incomplete=%llu compare=0 complete=0",
            owners[i], date.year, date.month, date.day, counts[i].draws.load(), counts[i].events.load(), counts[i].incomplete.load());
}
#endif
}  // namespace fc4
