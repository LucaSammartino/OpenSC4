// SPDX-License-Identifier: GPL-3.0-or-later
// renderer-driver §§5,8.6,9.5,10,12.2. Native calls always execute unchanged.
#include "FC4GDriverShadow.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "compat/FC4Compat.h"
#include "GZServPtrs.h"
#include "cIGZGraphicSystem2.h"
#include "cIGZGDriver.h"
#include "fc4.h"
#include <windows.h>
#include <array>
#include <cstring>
#include <mutex>

extern "C" uint32_t fc4_gdriver_shadow_compare(uint32_t, const uint32_t*, const uint32_t*, const uint32_t*);
namespace fc4 {
namespace {
using Words = std::array<uint32_t, 64>;
using Args = std::array<uint32_t, 20>;
std::recursive_mutex mutex;
cIGZGDriver* driver = nullptr; // Explicitly acquired for this city.
void** table = nullptr;
void* original[83]{};
void* hooks[83]{};
uint64_t frame = 0, sampled[83]{}, bySlot[83]{};
uint64_t compared = 0, differences = 0, refused = 0, lines = 0, clean = 0;
uint32_t details = 0;

uint32_t Word(const uint8_t* p, size_t offset) {
    uint32_t result;
    std::memcpy(&result, p + offset, sizeof(result));
    return result;
}
bool Capture(Words& s, int selected = -1) {
    if (!driver) return false;
    const auto* owner = reinterpret_cast<const uint8_t*>(driver) - 12;
    s[0] = Word(owner, 0x190); s[1] = Word(owner, 0x194); s[2] = Word(owner, 0x198);
    s[3] = Word(owner, 0x140); s[4] = Word(owner, 0x214);
    s[5] = selected < 0 ? Word(owner, 0x120) : static_cast<uint32_t>(selected);
    s[6] = Word(owner, 0x124); s[7] = owner[0x209]; s[8] = owner[0x261];
    std::memcpy(s.data() + 9, owner + 0x150, 16 * sizeof(uint32_t));
    std::memcpy(s.data() + 50, owner + 0x238, 6 * sizeof(uint32_t));
    if (s[6] > 8 || s[5] >= s[6]) return false;
    const auto* stages = reinterpret_cast<const uint8_t*>(static_cast<uintptr_t>(Word(owner, 0x128)));
    if (!stages) return false;
    const auto* stage = stages + s[5] * 0x80;
    s[25] = stage[1]; s[26] = Word(stage, 0x1C);
    for (unsigned i = 0; i < 16; ++i) s[27 + i] = Word(stage, 0x24 + i * 4);
    return true;
}
bool Sample(void* self, unsigned slot) {
    if (self != driver || !driver) return false;
    // At most one transition per slot in each 32-frame bucket. No pixel copies,
    // device queries, allocation-sized readbacks or logging on the hot path.
    const uint64_t bucket = frame / 32 + 1;
    if (sampled[slot] == bucket) return false;
    sampled[slot] = bucket;
    return true;
}
void Compare(unsigned slot, const Args& args, const Words& before, Words& after, bool captured) {
    if (!captured) { ++refused; return; }
    const uint32_t diff = fc4_gdriver_shadow_compare(slot, args.data(), before.data(), after.data());
    if (diff == UINT32_MAX) { ++refused; return; }
    ++compared; ++bySlot[slot]; differences += diff;
    if (diff && details++ < 8) Log("gdriver detail: slot=%u differing_words=%u frame=%llu", slot, diff, frame);
}

template<unsigned Slot> void __fastcall One(void* self, void* edx, uint32_t a) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = ((Slot != 44 && Slot != 45) || a <= 5 || a == 7) && Sample(self, Slot);
    const bool captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t)>(original[Slot])(self, edx, a);
    lock.lock();
    if (take) {
        Args args{}; args[0] = a;
        const bool ok = captured && Capture(after);
        if ((Slot == 44 || Slot == 45) && a != 5) {
            lock.unlock();
            after[49] = reinterpret_cast<bool(__fastcall*)(void*, void*, uint32_t)>(original[46])(self, nullptr, a);
            lock.lock();
        }
        Compare(Slot, args, before, after, ok);
    }
}
template<unsigned Slot> void __fastcall Two(void* self, void* edx, uint32_t a, uint32_t b) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, Slot);
    const bool captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t, uint32_t)>(original[Slot])(self, edx, a, b);
    lock.lock();
    if (take) { Args args{}; args[0] = a; args[1] = b; Compare(Slot, args, before, after, captured && Capture(after)); }
}
void __fastcall ClearColor(void* self, void* edx, float r, float g, float b, float a) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, 12), captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, float, float, float, float)>(original[12])(self, edx, r, g, b, a);
    lock.lock();
    if (take) {
        Args args{}; const float values[4] = {r,g,b,a}; std::memcpy(args.data(), values, sizeof(values));
        Compare(12, args, before, after, captured && Capture(after));
    }
}
void __fastcall ClearDepth(void* self, void* edx, double value) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, 13), captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, double)>(original[13])(self, edx, value);
    lock.lock();
    if (take) { Args args{}; std::memcpy(args.data(), &value, sizeof(value)); Compare(13, args, before, after, captured && Capture(after)); }
}
void __fastcall Matrix(void* self, void* edx, const float* value) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    Args args{};
    const bool take = value && Sample(self, 41), captured = take && Capture(before);
    if (take) std::memcpy(args.data(), value, 16 * sizeof(float)); // Copy before native can touch aliased storage.
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, const float*)>(original[41])(self, edx, value);
    lock.lock();
    if (take) Compare(41, args, before, after, captured && Capture(after));
}
void __fastcall Environment(void* self, void* edx, uint32_t target, uint32_t pname, uint32_t mode) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, 28), captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t, uint32_t, uint32_t)>(original[28])(self, edx, target, pname, mode);
    lock.lock();
    if (take) { Args args{}; args[2] = mode; Compare(28, args, before, after, captured && Capture(after)); }
}
void __fastcall Combiner(void* self, void* edx, const uint8_t* bytes, uint32_t stage) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{}; Args args{};
    const bool take = bytes && stage < 8 && Sample(self, 62);
    const bool captured = take && Capture(before, static_cast<int>(stage));
    if (take) for (unsigned i = 0; i < 10; ++i) args[i] = bytes[i];
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, const uint8_t*, uint32_t)>(original[62])(self, edx, bytes, stage);
    lock.lock();
    if (take) Compare(62, args, before, after, captured && Capture(after, static_cast<int>(stage)));
}
bool __fastcall Enabled(void* self, void* edx, uint32_t cap) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = (cap == 5 || cap == 7) && Sample(self, 46), captured = take && Capture(before);
    lock.unlock();
    const bool value = reinterpret_cast<bool(__fastcall*)(void*, void*, uint32_t)>(original[46])(self, edx, cap);
    lock.lock();
    if (take) { Args args{}; args[0] = cap; const bool ok = captured && Capture(after); after[49] = value; Compare(46, args, before, after, ok); }
    return value;
}
void __fastcall Integer(void* self, void* edx, uint32_t pname, uint32_t* output) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = output && Sample(self, 48), captured = take && Capture(before);
    if (take) before[49] = *output;
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t, uint32_t*)>(original[48])(self, edx, pname, output);
    lock.lock();
    if (take) { Args args{}; args[0] = pname; const bool ok = captured && Capture(after); after[49] = *output; Compare(48, args, before, after, ok); }
}
void __fastcall Flush(void* self, void* edx) {
    reinterpret_cast<void(__fastcall*)(void*, void*)>(original[43])(self, edx);
    std::lock_guard<std::recursive_mutex> lock(mutex);
    if (self == driver) ++frame;
}
template<unsigned Slot> void ViewportCompare(void* self, const Args& args, const Words& before, bool captured) {
    Words after{};
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t*)>(original[76])(self, nullptr, after.data() + 56);
    std::lock_guard<std::recursive_mutex> lock(mutex);
    const bool ok = captured && Capture(after);
    Compare(Slot, args, before, after, ok);
}
void __fastcall Viewport(void* self, void* edx, uint32_t x, uint32_t y, uint32_t w, uint32_t h) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{};
    const bool take = Sample(self, 74), captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t, uint32_t, uint32_t, uint32_t)>(original[74])(self, edx, x, y, w, h);
    if (take) { Args args{}; args[0] = x; args[1] = y; args[2] = w; args[3] = h; ViewportCompare<74>(self, args, before, captured); }
}
void __fastcall FullViewport(void* self, void* edx) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{};
    const bool take = Sample(self, 75), captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*)>(original[75])(self, edx);
    if (take) ViewportCompare<75>(self, Args{}, before, captured);
}
bool Patch(unsigned slot, void* value) {
    DWORD old = 0;
    if (!VirtualProtect(table + slot, sizeof(void*), PAGE_READWRITE, &old)) return false;
    InterlockedExchangePointer(table + slot, value);
    DWORD ignored = 0;
    VirtualProtect(table + slot, sizeof(void*), old, &ignored);
    return true;
}
void Restore() {
    if (!table) return;
    for (unsigned i = 0; i < 83; ++i) if (hooks[i] && table[i] == hooks[i]) Patch(i, original[i]);
    table = nullptr;
}
}
void GDriverShadowCityLoaded() {
    std::lock_guard<std::recursive_mutex> lock(mutex);
    if (driver || !SettingInt("gdriver_shadow", nullptr, 0)) return;
    const auto* version = compat::IdentifiedVersion();
    if (!version || version->imageBase != 0x00400000 || reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr)) != 0x00400000) return;
    cIGZGraphicSystem2Ptr graphics;
    cIGZGDriver* candidate = graphics ? graphics->GetGDriver() : nullptr;
    if (!candidate || *reinterpret_cast<uintptr_t*>(candidate) != 0x00AC4660) {
        Log("gdriver: unavailable: native DX7 interface required"); return;
    }
    driver = candidate; FC4_ADDREF(driver);
    table = *reinterpret_cast<void***>(driver);
    std::memset(sampled, 0, sizeof(sampled)); std::memset(bySlot, 0, sizeof(bySlot));
    frame = compared = differences = refused = lines = clean = details = 0;
    // Explicit spec slots: the SDK reverses several overloads. No SDK overload
    // dispatch is used for those methods (renderer-driver §§10.2 and 11).
    hooks[12] = reinterpret_cast<void*>(&ClearColor); hooks[13] = reinterpret_cast<void*>(&ClearDepth);
    hooks[14] = reinterpret_cast<void*>(&One<14>); hooks[26] = reinterpret_cast<void*>(&Two<26>);
    hooks[28] = reinterpret_cast<void*>(&Environment); hooks[40] = reinterpret_cast<void*>(&One<40>);
    hooks[41] = reinterpret_cast<void*>(&Matrix); hooks[43] = reinterpret_cast<void*>(&Flush);
    hooks[44] = reinterpret_cast<void*>(&One<44>); hooks[45] = reinterpret_cast<void*>(&One<45>);
    hooks[46] = reinterpret_cast<void*>(&Enabled); hooks[48] = reinterpret_cast<void*>(&Integer);
    hooks[51] = reinterpret_cast<void*>(&One<51>); hooks[54] = reinterpret_cast<void*>(&Two<54>);
    hooks[55] = reinterpret_cast<void*>(&Two<55>); hooks[56] = reinterpret_cast<void*>(&Two<56>);
    hooks[57] = reinterpret_cast<void*>(&Two<57>); hooks[62] = reinterpret_cast<void*>(&Combiner);
    hooks[74] = reinterpret_cast<void*>(&Viewport); hooks[75] = reinterpret_cast<void*>(&FullViewport);
    original[76] = table[76]; // Read-only public rectangle query, never patched.
    for (unsigned i = 0; i < 83; ++i) if (hooks[i]) original[i] = table[i];
    for (unsigned i = 0; i < 83; ++i) if (hooks[i] && !Patch(i, hooks[i])) {
        Restore(); FC4_RELEASE(driver); driver = nullptr; Log("gdriver: unavailable: observer installation failed"); return;
    }
    Log("gdriver: observing native transitions; sample=one-per-slot-per-32-frames scope=partial prestate=per-transition texture_formats=unmeasured transform_outputs=unmeasured");
}
void GDriverShadowMonth(int32_t julian) {
    std::lock_guard<std::recursive_mutex> lock(mutex);
    if (!driver) return;
    fc4_date date{}; fc4_date_from_julian(julian, &date);
    Log("gdriver transitions %04d-%02d-%02d: samples=%llu mismatches=%llu refused=%llu frames=%llu scope=cached-transitions", date.year, date.month, date.day_of_month, compared, differences, refused, frame);
    ++lines; if (compared && !differences && !refused) ++clean;
}
void GDriverShadowCityShutdown() {
    std::lock_guard<std::recursive_mutex> lock(mutex);
    if (!driver) return;
    Restore();
    Log("gdriver: shutdown: %llu/%llu exact; samples=%llu mismatches=%llu refused=%llu scope=cached-transitions prestate=per-transition", clean, lines, compared, differences, refused);
    for (unsigned i = 0; i < 83; ++i) if (bySlot[i]) Log("gdriver coverage: slot=%u samples=%llu", i, bySlot[i]);
    FC4_RELEASE(driver); driver = nullptr;
}
}
