// SPDX-License-Identifier: GPL-3.0-or-later
// renderer-driver §§5,8.6,9.5,10,12.2. Native calls always execute unchanged.
#include "FC4GDriverShadow.h"
#include "FC4GDriverTrace.h"
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
#include <type_traits>

extern "C" uint32_t fc4_gdriver_shadow_stream(uint32_t, const uint32_t*, const uint32_t*, const uint32_t*, const fc4::gdriver_trace::Record*, uint32_t);
namespace fc4 {
namespace {
using Words = std::array<uint32_t, 72>;
using Args = std::array<uint32_t, 20>;
std::recursive_mutex mutex;
cIGZGDriver* driver = nullptr; // Explicitly acquired for this city.
bool selectionLogged = false;
void** table = nullptr;
void** deviceTable = nullptr;
std::array<void*, gdriver_trace::DeviceSlots> deviceHooks{};
void* original[83]{};
void* hooks[83]{};
uint64_t frame = 0, sampled[83]{}, bySlot[83]{};
uint64_t compared = 0, differences = 0, refused = 0, lines = 0, clean = 0;
uint32_t details = 0;
uint64_t methodCalls[83]{}, complete[83]{}, methodDifferences[83]{}, methodRefused[83]{};
struct Method { unsigned slot; const char* address; const char* name; };
const Method methods[] = {
    {11, "0x00882C80", "nSGLDX7::cGDriver::Clear"},
    {26, "0x00881FC0", "nSGLDX7::cGDriver::PixelStore"},
    {27, "0x008830A0", "nSGLDX7::cGDriver::TexEnv"},
    {28, "0x00884960", "nSGLDX7::cGDriver::TexEnv"},
    {40, "0x00881EA0", "nSGLDX7::cGDriver::MatrixMode"},
    {41, "0x00882AC0", "nSGLDX7::cGDriver::LoadMatrix"},
    {42, "0x00881EB0", "nSGLDX7::cGDriver::LoadIdentity"},
    {43, "0x00884BA0", "nSGLDX7::cGDriver::Flush"},
    {48, "0x00881FE0", "nSGLDX7::cGDriver::GetInteger"},
    {50, "0x00882000", "nSGLDX7::cGDriver::GetError"},
    {51, "0x00882020", "nSGLDX7::cGDriver::TexStage"},
    {52, "0x00883550", "nSGLDX7::cGDriver::TexStageCoord"},
    {53, "0x00883600", "nSGLDX7::cGDriver::TexStageMatrix"},
    {54, "0x008852D0", "nSGLDX7::cGDriver::TexStageCombine"},
    {55, "0x00885220", "nSGLDX7::cGDriver::TexStageCombine"},
    {56, "0x00885190", "nSGLDX7::cGDriver::TexStageCombine"},
    {57, "0x00885130", "nSGLDX7::cGDriver::TexStageCombine"},
    {62, "0x00884A70", "nSGLDX7::cGDriver::SetCombiner"},
    {67, "0x00883710", "nSGLDX7::cGDriver::PolygonOffset"},
    {74, "0x008837E0", "nSGLDX7::cGDriver::SetViewport"},
    {75, "0x00883760", "nSGLDX7::cGDriver::SetViewport"},
    {76, "0x008838D0", "nSGLDX7::cGDriver::GetViewport"},
    {78, "0x00886E00", "nSGLDX7::cGDriver::GetGZCLSID"},
    {81, "0x00886380", "nSGLDX7::cGDriver::IsDeviceReady"},
};

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
    s[5] = Word(owner, 0x120);
    s[6] = Word(owner, 0x124); s[7] = owner[0x209]; s[8] = owner[0x261];
    std::memcpy(s.data() + 9, owner + 0x150, 16 * sizeof(uint32_t));
    std::memcpy(s.data() + 50, owner + 0x238, 6 * sizeof(uint32_t));
    s[63] = selected < 0 ? s[5] : static_cast<uint32_t>(selected);
    if (s[6] > 8 || s[5] >= s[6] || s[63] >= s[6]) return false;
    const auto* stages = reinterpret_cast<const uint8_t*>(static_cast<uintptr_t>(Word(owner, 0x128)));
    if (!stages) return false;
    const auto* stage = stages + s[63] * 0x80;
    s[25] = stage[1]; s[26] = Word(stage, 0x1C);
    for (unsigned i = 0; i < 16; ++i) s[27 + i] = Word(stage, 0x24 + i * 4);
    s[43] = Word(owner, 0x11C); s[44] = owner[0x134];
    s[45] = stage[0]; s[46] = Word(stage, 0x20);
    s[47] = owner[0x201]; s[48] = Word(owner, 0x224);
    s[60] = stage[2]; s[61] = Word(stage, 4);
    for (unsigned i = 0; i < 4; ++i) s[64 + i] = Word(stage, 0x0C + i * 4);
    s[68] = Word(stage, 8);
    return true;
}
bool Sample(void* self, unsigned slot, bool supported = true) {
    if (self != driver || !driver) return false;
    ++methodCalls[slot];
    if (!supported || gdriver_trace::active) return false;
    // At most one transition per slot in each 32-frame bucket. No pixel copies,
    // device queries, allocation-sized readbacks or logging on the hot path.
    const uint64_t bucket = frame / 32 + 1;
    if (sampled[slot] == bucket) return false;
    sampled[slot] = bucket;
    const auto* owner = reinterpret_cast<const uint8_t*>(driver) - 12;
    const auto* device = reinterpret_cast<void***>(static_cast<uintptr_t>(Word(owner, 0x224)));
    if (!device || *device != deviceTable) { ++refused; ++methodRefused[slot]; return false; }
    return gdriver_trace::Begin();
}
void Compare(unsigned slot, const Args& args, const Words& before, Words& after, bool captured) {
    gdriver_trace::End();
    const auto& trace = gdriver_trace::trace;
    if (!captured || trace.refused) { ++refused; ++methodRefused[slot]; return; }
    const uint32_t diff = fc4_gdriver_shadow_stream(slot, args.data(), before.data(), after.data(), trace.records.data(), trace.count);
    if (diff == UINT32_MAX) { ++refused; ++methodRefused[slot]; return; }
    ++compared; ++bySlot[slot]; differences += diff;
    methodDifferences[slot] += diff;
    // Queue prestate and texture/surface calls are still unmeasured. Only
    // methods with no such effects can receive complete credit in this stage.
    const bool covered = slot == 26 || slot == 28 || slot == 40 || slot == 46 || slot == 48 || slot == 50
        || slot == 51 || (slot >= 54 && slot <= 57) || slot == 78 || slot == 81 || slot == 82
        || ((slot == 44 || slot == 45) && (args[0] == 5 || args[0] == 7));
    if (covered && !diff) ++complete[slot];
    if (diff && details++ < 8) Log("gdriver detail: slot=%u differing_words=%u frame=%llu", slot, diff, frame);
}

template<unsigned Slot> void __fastcall One(void* self, void* edx, uint32_t a) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, Slot, (Slot != 44 && Slot != 45) || a <= 5 || a == 7);
    const bool captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t)>(original[Slot])(self, edx, a);
    lock.lock();
    if (take) {
        Args args{}; args[0] = a;
        const bool ok = captured && Capture(after, static_cast<int>(before[63]));
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
    if (take) { Args args{}; args[0] = a; args[1] = b; Compare(Slot, args, before, after, captured && Capture(after, static_cast<int>(before[63]))); }
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
        Compare(12, args, before, after, captured && Capture(after, static_cast<int>(before[63])));
    }
}
void __fastcall ClearDepth(void* self, void* edx, double value) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, 13), captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, double)>(original[13])(self, edx, value);
    lock.lock();
    if (take) { Args args{}; std::memcpy(args.data(), &value, sizeof(value)); Compare(13, args, before, after, captured && Capture(after, static_cast<int>(before[63]))); }
}
void __fastcall Matrix(void* self, void* edx, const float* value) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    Args args{};
    const bool take = Sample(self, 41, value != nullptr), captured = take && Capture(before);
    if (take) std::memcpy(args.data(), value, 16 * sizeof(float)); // Copy before native can touch aliased storage.
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, const float*)>(original[41])(self, edx, value);
    lock.lock();
    if (take) Compare(41, args, before, after, captured && Capture(after, static_cast<int>(before[63])));
}
void __fastcall Environment(void* self, void* edx, uint32_t target, uint32_t pname, uint32_t mode) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, 28), captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t, uint32_t, uint32_t)>(original[28])(self, edx, target, pname, mode);
    lock.lock();
    if (take) { Args args{}; args[2] = mode; Compare(28, args, before, after, captured && Capture(after, static_cast<int>(before[63]))); }
}
void __fastcall Combiner(void* self, void* edx, const uint8_t* bytes, uint32_t stage) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{}; Args args{};
    const bool take = Sample(self, 62, bytes && stage < 8);
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
    const bool take = Sample(self, 46, cap <= 7), captured = take && Capture(before);
    lock.unlock();
    const bool value = reinterpret_cast<bool(__fastcall*)(void*, void*, uint32_t)>(original[46])(self, edx, cap);
    lock.lock();
    if (take) { Args args{}; args[0] = cap; const bool ok = captured && Capture(after, static_cast<int>(before[63])); after[49] = value; Compare(46, args, before, after, ok); }
    return value;
}
void __fastcall Integer(void* self, void* edx, uint32_t pname, uint32_t* output) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, 48, output != nullptr), captured = take && Capture(before);
    if (take && pname != 0 && pname != 7 && pname != 8) before[49] = *output;
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t, uint32_t*)>(original[48])(self, edx, pname, output);
    lock.lock();
    if (take) { Args args{}; args[0] = pname; const bool ok = captured && Capture(after, static_cast<int>(before[63])); after[49] = *output; Compare(48, args, before, after, ok); }
}
void __fastcall Flush(void* self, void* edx) {
    reinterpret_cast<void(__fastcall*)(void*, void*)>(original[43])(self, edx);
    std::lock_guard<std::recursive_mutex> lock(mutex);
    if (self == driver) { ++frame; ++methodCalls[43]; }
}
template<unsigned Slot> void __fastcall Three(void* self, void* edx, uint32_t a, uint32_t b, uint32_t c) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, Slot), captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t, uint32_t, uint32_t)>(original[Slot])(self, edx, a, b, c);
    lock.lock();
    if (take) { Args args{}; args[0] = a; args[1] = b; args[2] = c; Compare(Slot, args, before, after, captured && Capture(after, static_cast<int>(before[63]))); }
}
template<unsigned Slot> void __fastcall NoArgs(void* self, void* edx) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, Slot), captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*)>(original[Slot])(self, edx);
    lock.lock();
    if (take) Compare(Slot, Args{}, before, after, captured && Capture(after, static_cast<int>(before[63])));
}
template<unsigned Slot> std::conditional_t<Slot == 81, bool, uint32_t> __fastcall Query(void* self, void* edx) {
    using Return = std::conditional_t<Slot == 81, bool, uint32_t>;
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, Slot), captured = take && Capture(before);
    lock.unlock();
    const Return result = reinterpret_cast<Return(__fastcall*)(void*, void*)>(original[Slot])(self, edx);
    lock.lock();
    if (take) { const bool ok = captured && Capture(after, static_cast<int>(before[63])); after[49] = result; Compare(Slot, Args{}, before, after, ok); }
    return result;
}
void __fastcall EnvironmentColor(void* self, void* edx, uint32_t target, uint32_t pname, const float* rgba) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{}; Args args{};
    const bool take = Sample(self, 27, rgba != nullptr), captured = take && Capture(before);
    if (take) std::memcpy(args.data(), rgba, 16);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t, uint32_t, const float*)>(original[27])(self, edx, target, pname, rgba);
    lock.lock();
    if (take) Compare(27, args, before, after, captured && Capture(after, static_cast<int>(before[63])));
}
void __fastcall StageMatrix(void* self, void* edx, const float* matrix, uint32_t count, uint32_t unused, uint32_t flags) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{}; Args args{};
    const bool take = Sample(self, 53), captured = take && Capture(before);
    if (take) { if (matrix) std::memcpy(args.data(), matrix, 64); args[16] = matrix != nullptr; args[17] = count; args[18] = unused; args[19] = flags; }
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, const float*, uint32_t, uint32_t, uint32_t)>(original[53])(self, edx, matrix, count, unused, flags);
    lock.lock();
    if (take) Compare(53, args, before, after, captured && Capture(after, static_cast<int>(before[63])));
}
void __fastcall GetViewport(void* self, void* edx, uint32_t* out) {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{}, after{};
    const bool take = Sample(self, 76, out != nullptr), captured = take && Capture(before);
    lock.unlock();
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t*)>(original[76])(self, edx, out);
    lock.lock();
    if (take) { const bool ok = captured && Capture(after, static_cast<int>(before[63])); std::memcpy(after.data() + 56, out, 16); Compare(76, Args{}, before, after, ok); }
}
template<unsigned Slot> void ViewportCompare(void* self, const Args& args, const Words& before, bool captured) {
    Words after{};
    std::lock_guard<std::recursive_mutex> lock(mutex);
    const bool ok = captured && Capture(after, static_cast<int>(before[63]));
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
bool InstallDevice(DWORD& error) {
    error = ERROR_SUCCESS;
    const auto* owner = reinterpret_cast<const uint8_t*>(driver) - 12;
    const auto* device = reinterpret_cast<void***>(static_cast<uintptr_t>(Word(owner, 0x224)));
    if (!device) return false;
    deviceTable = *device;
    deviceHooks = gdriver_trace::Hooks();
    DWORD old = 0;
    if (!VirtualProtect(deviceTable, deviceHooks.size() * sizeof(void*), PAGE_READWRITE, &old)) { error = GetLastError(); deviceTable = nullptr; return false; }
    for (unsigned i = 0; i < deviceHooks.size(); ++i) gdriver_trace::original[i] = deviceTable[i];
    for (unsigned i = 0; i < deviceHooks.size(); ++i) InterlockedExchangePointer(deviceTable + i, deviceHooks[i]);
    DWORD ignored = 0;
    VirtualProtect(deviceTable, deviceHooks.size() * sizeof(void*), old, &ignored);
    return true;
}
void Restore() {
    if (!table) return;
    for (unsigned i = 0; i < 83; ++i) if (hooks[i] && table[i] == hooks[i]) Patch(i, original[i]);
    table = nullptr;
    if (deviceTable) {
        DWORD old = 0;
        if (VirtualProtect(deviceTable, deviceHooks.size() * sizeof(void*), PAGE_READWRITE, &old)) {
            for (unsigned i = 0; i < deviceHooks.size(); ++i)
                if (deviceTable[i] == deviceHooks[i]) InterlockedExchangePointer(deviceTable + i, gdriver_trace::original[i]);
            DWORD ignored = 0;
            VirtualProtect(deviceTable, deviceHooks.size() * sizeof(void*), old, &ignored);
        }
        deviceTable = nullptr;
    }
}
}
void GDriverShadowCityLoaded() {
    std::lock_guard<std::recursive_mutex> lock(mutex);
    if (driver || !SettingInt("gdriver_shadow", nullptr, 0)) return;
    const auto* version = compat::IdentifiedVersion();
    if (!version || version->imageBase != 0x00400000 || reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr)) != 0x00400000) return;
    cIGZGraphicSystem2Ptr graphics;
    cIGZGDriver* candidate = graphics ? graphics->GetGDriver() : nullptr;
    const uintptr_t currentTable = candidate ? *reinterpret_cast<uintptr_t*>(candidate) : 0;
    // SPEC-GAP: driver-selection timing is unspecified; use bounded public enumeration and monthly retries, requiring the documented DX7 identity throughout.
    if (graphics && currentTable != 0x00AC4660) {
        const uint32_t count = graphics->GetGDriverCount();
        for (uint32_t i = 0; i < count && i < 16; ++i) {
            cIGZGDriver* const entry = graphics->GetGDriverByIndex(i);
            if (entry && *reinterpret_cast<uintptr_t*>(entry) == 0x00AC4660) { candidate = entry; break; }
        }
    }
    if (!candidate || *reinterpret_cast<uintptr_t*>(candidate) != 0x00AC4660) {
        if (!selectionLogged) Log("gdriver: unavailable: native DX7 interface required; current_table=0x%08X; retry=monthly", static_cast<unsigned>(currentTable));
        selectionLogged = true;
        return;
    }
    driver = candidate; FC4_ADDREF(driver);
    table = *reinterpret_cast<void***>(driver);
    std::memset(sampled, 0, sizeof(sampled)); std::memset(bySlot, 0, sizeof(bySlot));
    frame = compared = differences = refused = lines = clean = details = 0;
    std::memset(methodCalls, 0, sizeof(methodCalls)); std::memset(complete, 0, sizeof(complete));
    std::memset(methodDifferences, 0, sizeof(methodDifferences)); std::memset(methodRefused, 0, sizeof(methodRefused));
    DWORD deviceError = ERROR_SUCCESS;
    if (!InstallDevice(deviceError)) {
        const auto* owner = reinterpret_cast<const uint8_t*>(driver) - 12;
        if (!selectionLogged) Log("gdriver: unavailable: device capture installation failed; current_table=0x%08X selected_active=%u device=0x%08X stages=%u winerror=%lu; retry=monthly",
            static_cast<unsigned>(currentTable), candidate == (graphics ? graphics->GetGDriver() : nullptr), Word(owner, 0x224), Word(owner, 0x124), deviceError);
        selectionLogged = true;
        table = nullptr; FC4_RELEASE(driver); driver = nullptr;
        return;
    }
    // Explicit spec slots: the SDK reverses several overloads. No SDK overload
    // dispatch is used for those methods (renderer-driver §§10.2 and 11).
    hooks[12] = reinterpret_cast<void*>(&ClearColor); hooks[13] = reinterpret_cast<void*>(&ClearDepth);
    hooks[11] = reinterpret_cast<void*>(&One<11>); hooks[15] = reinterpret_cast<void*>(&One<15>);
    hooks[16] = reinterpret_cast<void*>(&One<16>); hooks[17] = reinterpret_cast<void*>(&One<17>);
    hooks[18] = reinterpret_cast<void*>(&Three<18>); hooks[19] = reinterpret_cast<void*>(&One<19>);
    hooks[20] = reinterpret_cast<void*>(&Three<20>); hooks[21] = reinterpret_cast<void*>(&Two<21>);
    hooks[22] = reinterpret_cast<void*>(&Two<22>); hooks[23] = reinterpret_cast<void*>(&One<23>);
    hooks[27] = reinterpret_cast<void*>(&EnvironmentColor); hooks[42] = reinterpret_cast<void*>(&NoArgs<42>);
    hooks[14] = reinterpret_cast<void*>(&One<14>); hooks[26] = reinterpret_cast<void*>(&Two<26>);
    hooks[28] = reinterpret_cast<void*>(&Environment); hooks[40] = reinterpret_cast<void*>(&One<40>);
    hooks[41] = reinterpret_cast<void*>(&Matrix); hooks[43] = reinterpret_cast<void*>(&Flush);
    hooks[44] = reinterpret_cast<void*>(&One<44>); hooks[45] = reinterpret_cast<void*>(&One<45>);
    hooks[46] = reinterpret_cast<void*>(&Enabled); hooks[48] = reinterpret_cast<void*>(&Integer);
    hooks[51] = reinterpret_cast<void*>(&One<51>); hooks[54] = reinterpret_cast<void*>(&Two<54>);
    hooks[50] = reinterpret_cast<void*>(&Query<50>); hooks[52] = reinterpret_cast<void*>(&One<52>);
    hooks[53] = reinterpret_cast<void*>(&StageMatrix); hooks[67] = reinterpret_cast<void*>(&One<67>);
    hooks[76] = reinterpret_cast<void*>(&GetViewport); hooks[78] = reinterpret_cast<void*>(&Query<78>);
    hooks[81] = reinterpret_cast<void*>(&Query<81>);
    hooks[55] = reinterpret_cast<void*>(&Two<55>); hooks[56] = reinterpret_cast<void*>(&Two<56>);
    hooks[57] = reinterpret_cast<void*>(&Two<57>); hooks[62] = reinterpret_cast<void*>(&Combiner);
    hooks[74] = reinterpret_cast<void*>(&Viewport); hooks[75] = reinterpret_cast<void*>(&FullViewport);
    for (unsigned i = 0; i < 83; ++i) if (hooks[i]) original[i] = table[i];
    for (unsigned i = 0; i < 83; ++i) if (hooks[i] && !Patch(i, hooks[i])) {
        Restore(); FC4_RELEASE(driver); driver = nullptr; Log("gdriver: unavailable: observer installation failed"); return;
    }
    Log("gdriver: observing native transitions; sample=one-per-slot-per-32-frames event_limit=128 scope=device-stream-partial prestate=per-transition texture_formats=unmeasured primitive_queue=unmeasured");
}
void GDriverShadowMonth(int32_t julian) {
    std::lock_guard<std::recursive_mutex> lock(mutex);
    if (!driver) GDriverShadowCityLoaded();
    if (!driver) return;
    fc4_date date{}; fc4_date_from_julian(julian, &date);
    Log("gdriver transitions %04d-%02d-%02d: samples=%llu mismatches=%llu refused=%llu frames=%llu scope=device-stream-partial", date.year, date.month, date.day_of_month, compared, differences, refused, frame);
    for (unsigned slot = 0; slot < 83; ++slot) if (hooks[slot]) {
        const Method* method = nullptr;
        for (const auto& item : methods) if (item.slot == slot) { method = &item; break; }
        if (method) Log("gdriver method %s %s (month) %04d-%02d-%02d: calls=%llu complete=%llu mismatches=%llu; resyncs=0",
            method->address, method->name, date.year, date.month, date.day_of_month, methodCalls[slot], complete[slot], methodDifferences[slot]);
        else Log("gdriver method - cIGZGDriver::slot%u (month) %04d-%02d-%02d: calls=%llu complete=%llu mismatches=%llu; resyncs=0",
            slot, date.year, date.month, date.day_of_month, methodCalls[slot], complete[slot], methodDifferences[slot]);
    }
    ++lines; if (compared && !differences && !refused) ++clean;
}
std::string GDriverShadowProbe() {
    std::unique_lock<std::recursive_mutex> lock(mutex);
    Words before{};
    if (!driver || !table || !Capture(before)) return "skipped: DX7 shadow unavailable";
    cIGZGDriver* const target = driver;
    void** const methods = table;
    lock.unlock();
    // Only public interface calls. Setters reassign the original values: they
    // exercise the specified invalidation without changing the visual state.
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t)>(methods[14])(target, nullptr, before[2]);
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t, uint32_t)>(methods[26])(target, nullptr, 0, before[3]);
    for (uint32_t cap : {0u, 5u, 7u}) reinterpret_cast<bool(__fastcall*)(void*, void*, uint32_t)>(methods[46])(target, nullptr, cap);
    uint32_t output = 0xC105E029;
    reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t, uint32_t*)>(methods[48])(target, nullptr, 0, &output);
    if (before[7]) {
        for (const auto& item : {std::array<uint32_t, 2>{54, before[41]}, {55, before[35]}, {56, before[29]}, {57, before[27]}})
            reinterpret_cast<void(__fastcall*)(void*, void*, uint32_t, uint32_t)>(methods[item[0]])(target, nullptr, 0, item[1]);
    }
    return "slots=14,26,46,48,54-57 combine_cap=" + std::to_string(before[7]) + " row_length=" + std::to_string(output);
}
void GDriverShadowCityShutdown() {
    std::lock_guard<std::recursive_mutex> lock(mutex);
    selectionLogged = false;
    if (!driver) return;
    Restore();
    Log("gdriver: shutdown: %llu/%llu exact; samples=%llu mismatches=%llu refused=%llu scope=device-stream-partial prestate=per-transition", clean, lines, compared, differences, refused);
    for (unsigned i = 0; i < 83; ++i) Log("gdriver coverage: slot=%u observed=%u calls=%llu samples=%llu complete=%llu mismatches=%llu refused=%llu", i, hooks[i] != nullptr, methodCalls[i], bySlot[i], complete[i], methodDifferences[i], methodRefused[i]);
    FC4_RELEASE(driver); driver = nullptr;
}
}
