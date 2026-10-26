// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4ConfigRulesSystem.h"
#include <Windows.h>
#include <intrin.h>
#include <algorithm>
#include <cstdio>
#include <cstring>
#include <vector>

namespace fc4::cfgrules {
namespace {
bool HasCPUID() {
    uint32_t before, after;
    __asm {
        pushfd
        pop eax
        mov before, eax
        xor eax, 200000h
        push eax
        popfd
        pushfd
        pop eax
        mov after, eax
        mov eax, before
        push eax
        popfd
    }
    return ((before ^ after) & 0x200000) != 0;
}
bool Speed(uint32_t features, uint32_t family, uint32_t& mhz) {
    mhz = 0;
    LARGE_INTEGER frequency{};
    if (!QueryPerformanceFrequency(&frequency)) return true;
    if (features & 16) {
        uint32_t trials[3]{};
        for (auto& trial : trials) {
            const int saved = GetThreadPriority(GetCurrentThread());
            SetThreadPriority(GetCurrentThread(), 15);
            Sleep(0);
            LARGE_INTEGER q0{}, q1{}, q2{};
            bool ok = QueryPerformanceCounter(&q0) && QueryPerformanceCounter(&q1);
            const auto c0 = __rdtsc();
            if (ok) do {
                ok = QueryPerformanceCounter(&q2) != FALSE;
            } while (ok && q2.QuadPart < q1.QuadPart + frequency.QuadPart / 20);
            const auto c1 = __rdtsc();
            SetThreadPriority(GetCurrentThread(), saved);
            if (!ok) return false;
            const int64_t duration = (q2.QuadPart - q1.QuadPart) - ((q1.QuadPart - q0.QuadPart) >> 1);
            if (duration > 0) trial = static_cast<uint32_t>(
                (static_cast<long double>(static_cast<int64_t>(c1 - c0)) * frequency.QuadPart) /
                (static_cast<long double>(duration) * 1000000.));
        }
        std::sort(std::begin(trials), std::end(trials));
        mhz = trials[1];
        return true;
    }
    uint32_t minimum = UINT32_MAX;
    for (int i = 0; i < 10; ++i) {
        LARGE_INTEGER a{}, b{};
        if (!QueryPerformanceCounter(&a)) return false;
        __asm {
            mov ecx, 4000
            mov edx, 80000000h
        again:
            bsf eax, edx
            dec cx
            jnz again
        }
        if (!QueryPerformanceCounter(&b)) return false;
        minimum = (std::min)(minimum, static_cast<uint32_t>(b.QuadPart - a.QuadPart));
    }
    const uint32_t f = static_cast<uint32_t>(frequency.QuadPart);
    const uint32_t divisor = f / 10;
    if (!divisor) return false;
    uint32_t m = (minimum * 100000u) / divisor;
    if (m % f > (f >> 1)) ++m;
    if (!m) return false;
    const uint32_t multipliers[] = {0, 0, 0, 115, 47, 43, 38, 38, 38, 38, 38, 38};
    const uint32_t u = multipliers[(std::min)(family, 11u)] * 4000;
    if (!u) return false;
    mhz = u / m;
    if (u % m > (m >> 1)) ++mhz;
    return true;
}
bool Version(std::array<std::string, 6>& strings) {
    char path[MAX_PATH]{};
    wchar_t wide[MAX_PATH]{};
    if (GetModuleFileNameW(nullptr, wide, MAX_PATH)) {
        if (!WideCharToMultiByte(CP_ACP, 0, wide, -1, path, MAX_PATH, nullptr, nullptr)) return false;
    } else if (!GetModuleFileNameA(nullptr, path, MAX_PATH)) return false;
    DWORD unused = 0;
    const DWORD size = GetFileVersionInfoSizeA(path, &unused);
    if (!size) return true; // Missing resource preserves previous fields (2.5).
    std::vector<uint8_t> resource(size);
    if (!GetFileVersionInfoA(path, 0, size, resource.data())) return false;
    void* translation = nullptr;
    UINT length = 0;
    if (!VerQueryValueA(resource.data(), "\\VarFileInfo\\Translation", &translation, &length) || length < 4)
        return false;
    const auto* pair = static_cast<const uint16_t*>(translation);
    for (unsigned index : {1u, 2u}) {
        char key[96];
        sprintf_s(key, "\\StringFileInfo\\%04lx%04lx\\%s", static_cast<unsigned long>(pair[0]),
                  static_cast<unsigned long>(pair[1]), index == 1 ? "FileDescription" : "FileVersion");
        void* value = nullptr;
        strings[index] = VerQueryValueA(resource.data(), key, &value, &length) && value
            ? static_cast<const char*>(value) : "";
    }
    return true;
}
}
bool MeasureSystem(FC4ConfigRulesMeasurement& m, std::array<std::string, 6>& strings) {
    char buffer[512]{};
    DWORD size = sizeof(buffer);
    strings[0] = GetUserNameA(buffer, &size) ? buffer : "?";
    if (!Version(strings)) return false;
#pragma warning(push)
#pragma warning(disable: 4996)
    const DWORD version = GetVersion();
#pragma warning(pop)
    sprintf_s(buffer, version & 0x80000000 ? "Windows %u.%u" : "Windows NT %u.%u",
              version & 255, (version >> 8) & 255);
    strings[3] = buffer;
    uint32_t family = 4;
    strings[5].clear();
    m.cpuFeatures = 0;
    if (HasCPUID()) {
        int cpu[4]{};
        __cpuid(cpu, 0);
        char vendor[13]{};
        memcpy(vendor, &cpu[1], 4); memcpy(vendor + 4, &cpu[3], 4); memcpy(vendor + 8, &cpu[2], 4);
        strings[5] = vendor;
        if (cpu[0] >= 1) { __cpuid(cpu, 1); m.cpuFeatures = cpu[3]; family = (cpu[0] >> 8) & 15; }
    } else {
        uint32_t before, after;
        __asm {
            pushfd
            pop eax
            mov before, eax
            xor eax, 40000h
            push eax
            popfd
            pushfd
            pop eax
            mov after, eax
            mov eax, before
            push eax
            popfd
        }
        family = ((before ^ after) & 0x40000) ? 4 : 3;
    }
    size = sizeof(buffer);
    strings[4] = GetComputerNameA(buffer, &size) ? buffer : "localhost";
    MEMORYSTATUS memory{};
    memory.dwLength = 32;
    GlobalMemoryStatus(&memory);
    m.physicalBytes = memory.dwTotalPhys;
    m.availableBytes = memory.dwAvailPhys;
    if (!Speed(m.cpuFeatures, family, m.cpuMHz)) return false;
    for (size_t i = 0; i < strings.size(); ++i)
        m.osStrings[i] = {reinterpret_cast<const uint8_t*>(strings[i].data()), strings[i].size()};
    return true;
}
}
