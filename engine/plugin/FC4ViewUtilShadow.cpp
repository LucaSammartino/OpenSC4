// SPDX-License-Identifier: GPL-3.0-or-later
// view-utilities.md §§1,4.3,11.10: local-vector read-only differential.
#include "FC4ViewUtilShadow.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4WinInterfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4View3DWin.h"
#include "cISC43DRender.h"
#include "cRZCOMDllDirector.h"
#include "fc4.h"
#include <windows.h>
#include <array>
#include <cstring>
#include <limits>

extern "C" void fc4_viewutil_shadow_rotate(uint32_t*, int32_t);
extern "C" void fc4_viewutil_shadow_local(uint32_t, const uint32_t*, uint32_t*);
extern "C" void fc4_viewutil_shadow_cached(uint32_t, const uint32_t*, const uint32_t*, uint32_t*);

namespace {
struct Vector { float x, y, z; };
static_assert(sizeof(Vector) == 12, "spec float32 triple");
using Words = std::array<uint32_t, 3>;
bool active = false;
int32_t lastMonth = -1;
uint32_t lines = 0, clean = 0;

// VU16 section 13: private machine contracts, never declarations of game COM
// methods. Each scalar call consumes one ST(0) result through a float32 store.
template<class T> T Entry(uint32_t va) { return reinterpret_cast<T>(static_cast<uintptr_t>(va)); }
bool SupportedImage() {
    const auto base = reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr));
    if (base != 0x00400000) return false; // rebased bindings are not established
    const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(base);
    if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew <= 0 || dos->e_lfanew > 0x1000) return false;
    const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS32*>(base + dos->e_lfanew);
    if (nt->Signature != IMAGE_NT_SIGNATURE || nt->FileHeader.Machine != 0x014c ||
        nt->FileHeader.TimeDateStamp != 0x4c12bfb7 || nt->OptionalHeader.Magic != 0x010b ||
        nt->OptionalHeader.ImageBase != 0x00400000 || nt->OptionalHeader.SizeOfImage != 0x007c8000) return false;
    // Validate executable bounds for the whole family before any private call.
    const uint32_t addresses[] = {0x005d6900,0x005d6f10,0x007f6550,0x007f65b0,
        0x007f6620,0x007f6690,0x007f66a0,0x007f66c0,0x007f6710,0x007f6770,
        0x007f6910,0x007f6ab0,0x007f6af0,0x007f6b50,0x007f6ba0,0x007f6cb0,
        0x007f6cc0,0x007f6d10,0x007f6d70,0x007f6db0,0x007f6df0,0x007f7710,
        0x007f7750,0x007f7780,0x007f77d0,0x007f7850,0x007f78a0,0x007f78e0,
        0x007f7920,0x007f79b0,0x007f7a40,0x007f7e50,0x007f7f60,0x007f8300,
        0x007f8390,0x007f83f0,0x007f9080,0x007f9160,0x007f91c0,0x007f9a40,
        0x007f9b40,0x007f9ea0,0x007f9f50,0x0080ae80,0x0080aef0,0x0080b000,
        0x0080b020,0x0080b090,0x0080b320,0x0080b3b0};
    for (auto va : addresses) {
        bool executable = false;
        const auto* section = IMAGE_FIRST_SECTION(nt);
        for (unsigned i = 0; i < nt->FileHeader.NumberOfSections; ++i, ++section) {
            const auto rva = va - 0x00400000;
            if (rva >= section->VirtualAddress && rva - section->VirtualAddress < section->Misc.VirtualSize &&
                (section->Characteristics & IMAGE_SCN_MEM_EXECUTE)) executable = true;
        }
        if (!executable) return false;
    }
    return true;
}
using Packet = std::array<uint32_t, 32>;
uint32_t Bits(float f) { uint32_t u; std::memcpy(&u, &f, 4); return u; }
float Float(uint32_t u) { float f; std::memcpy(&f, &u, 4); return f; }
struct Family { uint32_t va; const char* name; uint32_t exact = 0, total = 0; };
Family localFamilies[] = {
    {0x005d6900,"bounds"}, {0x005d6f10,"cross"},
    {0x007f6550,"min"}, {0x007f65b0,"max"}, {0x007f6ab0,"ground-arithmetic"}, {0x007f6af0,"triangle-pick"},
    {0x007f6b50,"ground-predicate"}, {0x007f6cb0,"object-angle"},
    {0x007f6d10,"edge-vector"}, {0x007f6d70,"round-down"}, {0x007f6db0,"round-up"},
    {0x007f7780,"camera-angles"}, {0x007f8300,"city-width"},
    {0x0080ae80,"primitive-count"}, {0x0080aef0,"primitive-indices"},
    {0x0080b020,"vertex-copy"}, {0x0080b090,"index-decode"}
};
using Owner = std::array<uint32_t, 0x470 / 4>;
Family cachedFamilies[] = {
    {0x007f6620,"clip2"},{0x007f6690,"units"},{0x007f66a0,"zoom-rotation"},
    {0x007f66c0,"view-viewport"},{0x007f6710,"fixed"},{0x007f6770,"view-box"},
    {0x007f6ba0,"city-corner"},{0x007f7710,"clip3"},{0x007f7750,"visible-edges"},
    {0x007f77d0,"world-view"},{0x007f7850,"world-viewport"},{0x007f78a0,"depth"},
    {0x007f78e0,"world-viewport2"},{0x007f7920,"view-world"},{0x007f79b0,"viewport-world"},
    {0x007f7a40,"box-rectangle"},{0x007f7e50,"view-point"},{0x007f8390,"viewport-fixed"}
};

void NativeCached(uint32_t op, Owner& owner, const Packet& in, Packet& out) {
    Vector v[2]; std::memcpy(v, in.data(), sizeof(v));
    auto* result = reinterpret_cast<Vector*>(out.data());
    auto* self = owner.data();
    switch (op) {
    case 0x007f6620:
        std::memcpy(out.data(),in.data(),8);
        Entry<void(__thiscall*)(void*,float*)>(op)(self,reinterpret_cast<float*>(out.data())); break;
    case 0x007f6690:
        out[0]=Bits(Entry<float(__thiscall*)(void*,float)>(op)(self,v[0].x)); break;
    case 0x007f66a0: case 0x007f7750:
        Entry<void(__thiscall*)(void*,uint32_t*,uint32_t*)>(op)(self,&out[0],&out[1]); break;
    case 0x007f6770:
        out[0]=Entry<bool(__thiscall*)(void*,Vector*,Vector*)>(op)(self,&v[0],&v[1]); break;
    case 0x007f6ba0:
        Entry<void(__thiscall*)(void*,int32_t,Vector*)>(op)(self,static_cast<int32_t>(in[0]),result); break;
    case 0x007f78a0:
        out[0]=Bits(Entry<float(__thiscall*)(void*,Vector*)>(op)(self,&v[0])); break;
    case 0x007f7a40:
        Entry<void(__thiscall*)(void*,Vector*,Vector*,int32_t*,int32_t*)>(op)(self,&v[0],&v[1],
            reinterpret_cast<int32_t*>(&out[0]),reinterpret_cast<int32_t*>(&out[2])); break;
    case 0x007f7e50:
        out[0]=Entry<bool(__thiscall*)(void*,Vector*)>(op)(self,&v[0]); break;
    default:
        std::memcpy(result,&v[0],12);
        Entry<void(__thiscall*)(void*,Vector*)>(op)(self,result); break;
    }
}

bool CompareCached(const fc4_date& date, bool available, const Owner& snapshot) {
    bool exact = true;
    for (auto& family : cachedFamilies) {
        uint32_t fields = 0, mismatches = 0;
        for (uint32_t sample = 0; available && sample < 32; ++sample) {
            Packet in{}, native{}, replacement{};
            native.fill(0x41200000); replacement=native;
            for (unsigned j=0;j<3;++j) {
                in[j]=Bits(static_cast<float>(static_cast<int>(sample)-8)*128 + static_cast<float>(j)*16);
                in[j+3]=Bits(Float(in[j])+64);
            }
            if (family.va == 0x007f6ba0) in[0]=sample % 6 - 1;
            // R-class: copy live cached inputs; no provider pointers are used.
            Owner privateOwner=snapshot;
            NativeCached(family.va,privateOwner,in,native);
            fc4_viewutil_shadow_cached(family.va,snapshot.data(),in.data(),replacement.data());
            for (size_t j=0;j<native.size();++j) {
                ++fields;
                if (native[j] != replacement[j]) {
                    if (mismatches < 2) fc4::Log("viewutil detail family=%s sample=%u word=%u native=%08x core=%08x",family.name,sample,static_cast<unsigned>(j),native[j],replacement[j]);
                    ++mismatches;
                }
            }
            // Also enforce the R-class isolation contract on the copied owner.
            ++fields; mismatches += privateOwner != snapshot;
        }
        if (!available) ++mismatches;
        fc4::Log("viewutil-%s shadow %04d-%02d-%02d: body=%08x scope=cached-inputs capture=f32 samples=%u fields=%u resyncs=0 mismatches=%u",
            family.name,date.year,date.month,date.day_of_month,family.va,available?32u:0u,fields,mismatches);
        ++family.total; family.exact += mismatches == 0;
        exact = exact && mismatches == 0;
    }
    return exact;
}

void NativeLocal(uint32_t op, const Packet& in, Packet& out) {
    Vector v[3]; std::memcpy(v, in.data(), sizeof(v));
    auto* result = reinterpret_cast<Vector*>(out.data());
    switch (op) {
    case 0x005d6900:
        Entry<void(__cdecl*)(Vector*,int32_t,Vector*,Vector*)>(op)(v, static_cast<int32_t>(in[9]), result, result+1); break;
    case 0x005d6f10: {
        const auto destination = in[6];
        Entry<void(__cdecl*)(Vector*,Vector*,Vector*)>(op)(&v[0], &v[1], &v[destination]);
        std::memcpy(out.data(), &v[destination], 12); break;
    }
    case 0x007f6550: case 0x007f65b0:
        out[0] = Bits(Entry<float(__cdecl*)(float,float,float,float)>(op)(Float(in[0]),Float(in[1]),Float(in[2]),Float(in[3]))); break;
    case 0x007f6ab0:
        Entry<void(__cdecl*)(Vector*,Vector*,Vector*)>(op)(&v[0],&v[1],result); break;
    case 0x007f6af0: {
        std::array<uint32_t,11> pick{};
        std::memcpy(pick.data(),in.data(),24); pick[10]=in[6];
        Vector triangle[3]; std::memcpy(triangle,&in[7],36);
        out[1]=Entry<bool(__cdecl*)(void*,Vector*,Vector*,Vector*)>(op)(pick.data(),&triangle[0],&triangle[1],&triangle[2]);
        out[0]=pick[10]; break;
    }
    case 0x007f6b50:
        out[3] = Entry<bool(__stdcall*)(Vector*,Vector*,Vector*)>(op)(&v[0],&v[1],result); break;
    case 0x007f6cb0:
        out[0] = Bits(Entry<float(__cdecl*)(int32_t)>(op)(static_cast<int32_t>(in[0]))); break;
    case 0x007f6d10:
        Entry<void(__cdecl*)(Vector*,int32_t)>(op)(result,static_cast<int32_t>(in[0])); break;
    case 0x007f6d70: case 0x007f6db0: case 0x007f8300:
        out[0] = Bits(Entry<float(__cdecl*)(float,int32_t)>(op)(Float(in[0]),static_cast<int32_t>(in[1]))); break;
    case 0x007f7780:
        Entry<void(__stdcall*)(int32_t,int32_t,float*,float*,float*)>(op)(static_cast<int32_t>(in[0]),static_cast<int32_t>(in[1]),
            reinterpret_cast<float*>(&out[0]),reinterpret_cast<float*>(&out[1]),reinterpret_cast<float*>(&out[2])); break;
    case 0x0080ae80:
        out[0] = Entry<int32_t(__cdecl*)(int32_t,int32_t)>(op)(static_cast<int32_t>(in[0]),static_cast<int32_t>(in[1])); break;
    case 0x0080aef0:
        Entry<void(__cdecl*)(int32_t,int32_t,uint32_t*)>(op)(static_cast<int32_t>(in[0]),static_cast<int32_t>(in[1]),out.data()); break;
    case 0x0080b020: case 0x0080b090: {
        // VU16 private iterator; pointers reference only this local input packet.
        std::array<uint32_t,11> iterator{};
        iterator[4]=in[18]; iterator[5]=static_cast<uint32_t>(reinterpret_cast<uintptr_t>(in.data()));
        iterator[6]=in[18]; iterator[7]=iterator[5];
        Entry<void(__thiscall*)(void*,const uint32_t*,void*)>(op)(iterator.data(),&in[19],out.data()); break;
    }
    }
}

bool CompareLocal(const fc4_date& date, bool binding) {
    bool exact = true;
    for (auto& family : localFamilies) {
        uint32_t mismatches = 0, fields = 0;
        for (uint32_t sample = 0; binding && sample < 32; ++sample) {
            Packet in{}, native{}, replacement{};
            native.fill(0x41200000); replacement = native;
            for (unsigned j = 0; j < 9; ++j) in[j] = Bits(static_cast<float>(static_cast<int>(sample * (j+1)) - 60));
            switch (family.va) {
            case 0x005d6900: in[9] = sample % 5 - 1; break;
            case 0x005d6f10: in[6] = sample % 3; break;
            case 0x007f6550: case 0x007f65b0: {
                const uint32_t special[] = {0,0x80000000,0x7f800000,0xff800000,0x7fc12345,0x7f7fffff,0xff7fffff,0x3f800000};
                for (unsigned j = 0; j < 4; ++j) in[j] = special[(sample+j) % 8];
                if (sample >= 8) in[sample % 4] = Bits(static_cast<float>(sample)-16.5f);
                break;
            }
            case 0x007f6ab0: case 0x007f6b50:
                // Exact dyadic rays, including parallel and coplanar predicate cases.
                in[0]=Bits(4); in[1]=Bits(sample % 4 == 0 ? 0.0f : 8.0f); in[2]=Bits(-4);
                in[3]=Bits(2); in[4]=Bits(sample % 3 == 0 && family.va == 0x007f6b50 ? 0.0f : -2.0f); in[5]=Bits(1); break;
            case 0x007f6af0:
                in[0]=Bits(sample % 4 == 0 ? 8.0f : 1.0f); in[1]=Bits(sample % 2 == 0 ? -2.0f : 2.0f); in[2]=Bits(1);
                in[3]=0; in[4]=Bits(-2); in[5]=0; in[6]=Bits(sample % 3 == 0 ? 1.0f : 10.0f);
                { const Vector tri[]={{0,0,0},{4,0,0},{0,0,4}}; std::memcpy(&in[7],tri,36); }
                break;
            case 0x007f6cb0: case 0x007f6d10: in[0]=sample-16; break;
            case 0x007f6d70: case 0x007f6db0:
                in[0]=Bits(static_cast<float>(static_cast<int>(sample)-16)*64+0.75f); in[1]=64;
                if (sample == 31) in[0]=Bits(2147483648.0f);
                break;
            case 0x007f7780: in[0]=sample % 4; in[1]=sample % 5; break;
            case 0x007f8300: in[0]=Bits(static_cast<float>(sample)*64); in[1]=sample % 5; break;
            case 0x0080ae80: case 0x0080aef0: in[0]=sample % 10; in[1]=sample-4; break;
            case 0x0080b020: case 0x0080b090:
                for (unsigned j=0;j<16;++j) in[j]=Bits(static_cast<float>(j)-8.0f);
                in[18]=family.va == 0x0080b020 ? (sample % 4 + 1)*4 : sample % 9;
                in[19]=sample % 3; in[20]=(sample+1) % 3; in[21]=(sample+2) % 3;
                if (family.va == 0x0080b090 && in[18] == 7) {
                    const double numbers[]={-2.5,0.0,3.75}; std::memcpy(in.data(),numbers,sizeof(numbers));
                }
                break;
            }
            NativeLocal(family.va, in, native);
            fc4_viewutil_shadow_local(family.va, in.data(), replacement.data());
            for (size_t j = 0; j < native.size(); ++j) {
                ++fields;
                if (native[j] != replacement[j]) {
                    if (mismatches < 2) fc4::Log("viewutil detail family=%s sample=%u word=%u native=%08x core=%08x", family.name,sample,static_cast<unsigned>(j),native[j],replacement[j]);
                    ++mismatches;
                }
            }
        }
        if (!binding) ++mismatches;
        fc4::Log("viewutil-%s shadow %04d-%02d-%02d: body=%08x capture=f32 samples=%u fields=%u resyncs=0 mismatches=%u",
            family.name,date.year,date.month,date.day_of_month,family.va,binding?32u:0u,fields,mismatches);
        ++family.total; family.exact += mismatches == 0;
        exact = exact && mismatches == 0;
    }
    return exact;
}

bool LiveInputs(std::array<Words, 5>& vectors, int32_t& rotation, Owner& snapshot) {
    auto* fw = RZGetFrameWork();
    auto* app = fw ? fw->Application() : nullptr; // borrowed
    cISC4App* sc4 = nullptr;
    if (!app || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || !sc4) return false;
    FC4_ACQUIRED(sc4, "QueryInterface");
    auto* main = reinterpret_cast<cFC4IGZWin*>(sc4->GetMainWindow()); // borrowed
    auto* parent = main ? main->GetChildWindowFromID(0x6104489A) : nullptr; // borrowed
    cISC4View3DWin* view = nullptr;
    bool available = false;
    if (parent && parent->GetChildAs(0x9A47B417, 0xFA47B3F9, reinterpret_cast<void**>(&view)) && view) {
        FC4_ACQUIRED(view, "GetChildAs");
        auto* render = view->GetRenderer(); // borrowed
        const auto* utilities = render ? reinterpret_cast<const unsigned char*>(render->GetViewUtilities()) : nullptr;
        if (utilities) {
            std::memcpy(snapshot.data(), utilities, sizeof(snapshot));
            // §1 published cached inputs: camera position, look-at, forward,
            // right, up. Read only; never bootstrap a projection expectation
            // from native projection output. Only the local rotation is tested.
            std::memcpy(&rotation, utilities + 0x10, sizeof(rotation));
            for (size_t i = 0; i < vectors.size(); ++i)
                std::memcpy(vectors[i].data(), utilities + 0x3c + i * 12, 12);
            available = true;
        }
        FC4_RELEASE(view);
    }
    FC4_RELEASE(sc4);
    return available;
}

void Compare(int32_t julian, bool load) {
    if (!active) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    const int32_t month = date.year * 12 + date.month;
    if (month == lastMonth) return;
    lastMonth = month;
    uint32_t fields = 0, mismatches = 0, samples = 0;
    std::array<Words, 5> live{};
    Owner snapshot{};
    int32_t liveRotation = 0;
    const bool available = LiveInputs(live, liveRotation, snapshot);
    const auto base = SupportedImage() ? reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr)) : 0;
    // §4.3 explicitly documents free cdecl, in-place float32 triple and signed
    // rotation. Literal void return, §12. Addresses are for supported 1.1.641.
    using Rotate = void(__cdecl*)(Vector*, int32_t);
    auto native = reinterpret_cast<Rotate>(base + (0x007F6CC0 - 0x00400000));
    auto probe = [&](Words input, int32_t rotation) {
        Words expected = input, actual{};
        Vector v;
        std::memcpy(&v, input.data(), sizeof(v));
        native(&v, rotation);
        std::memcpy(actual.data(), &v, sizeof(v));
        fc4_viewutil_shadow_rotate(expected.data(), rotation);
        ++samples;
        for (size_t i = 0; i < actual.size(); ++i) {
            ++fields;
            if (actual[i] != expected[i]) {
                if (mismatches < 8) fc4::Log("viewutil diff: sample=%u rotation=%d word=%u native=%08x core=%08x",
                    samples, rotation, static_cast<unsigned>(i), actual[i], expected[i]);
                ++mismatches;
            }
        }
    };
    // §11.10 plus §4.3 signed remainder, infinities, quiet NaN word copies,
    // signed zero and finite extrema. Signaling NaNs require a numeric-mode
    // contract and are deliberately excluded (VU12); y is still bit-compared.
    const Words fixed[] = {
        {0x40000000, 0x40e00000, 0x41300000}, // (2,7,11)
        {0, 0x80000000, 0x80000000},
        {0x80000000, 0x7fc12345, 0},
        {0x7f800000, 0xff800000, 0xff800000},
        {0x7f7fffff, 0x3f800000, 0xff7fffff},
        {0x7fc23456, 0x80000000, 0xffc34567},
        {1, 0x3f800000, 0x80000001},
    };
    const int32_t rotations[] = {0, 1, 2, 3, -1, -2, -3, -4, 4, 5,
        std::numeric_limits<int32_t>::min(), std::numeric_limits<int32_t>::max()};
    if (base) {
        for (const auto& v : fixed) {
            for (auto r : rotations) probe(v, r);
            if (available) probe(v, liveRotation);
        }
        if (available) for (const auto& v : live) {
            for (auto r : rotations) probe(v, r);
            probe(v, liveRotation);
        }
    }
    // Missing live state or binding is a failed observation, never vacuous exact.
    ++fields;
    mismatches += !available || !base;
    mismatches += !CompareLocal(date, base != 0);
    mismatches += !CompareCached(date, available && base != 0, snapshot);
    fc4::Log("viewutil shadow %04d-%02d-%02d: phase=%s scope=local-and-cached bodies=36/50 live=%u rotation=%d samples=%u fields=%u resyncs=0 mismatches=%u",
        date.year, date.month, date.day_of_month, load ? "load" : "month", available ? 1u : 0u,
        liveRotation, samples, fields, mismatches);
    ++lines;
    clean += mismatches == 0;
}
}

namespace fc4 {
void ViewUtilShadowCityLoaded(cISC4City* city, int32_t julian) {
    active = city && SettingInt("viewutil_shadow", nullptr, 0) != 0;
    lines = clean = 0;
    for (auto& family : localFamilies) family.exact = family.total = 0;
    for (auto& family : cachedFamilies) family.exact = family.total = 0;
    lastMonth = -1;
    if (!active) return;
    Log("viewutil: ON; VU15/VU16 local and cached arithmetic (36/50 bodies); scalar capture=f32; isolated storage; default-off");
    if (auto* sim = AsGame(city->GetSimulator())) julian = sim->GetSimDateNumber();
    Compare(julian, true);
}
void ViewUtilShadowMonth(int32_t julian) { Compare(julian, false); }
void ViewUtilShadowCityShutdown() {
    if (active) {
        Log("viewutil: %u/%u exact (36/50 bodies), resyncs=0", clean, lines);
        for (const auto& family : localFamilies) Log("viewutil-%s: %u/%u exact, resyncs=0",family.name,family.exact,family.total);
        for (const auto& family : cachedFamilies) Log("viewutil-%s: %u/%u exact, resyncs=0",family.name,family.exact,family.total);
    }
    active = false;
}
}
