// SPDX-License-Identifier: GPL-3.0-or-later
// Bounded IDirect3DDevice7 call capture. No device queries or COM ownership.
#pragma once
#include <array>
#include <cstdint>
#include <cstring>

namespace fc4::gdriver_trace {
constexpr unsigned Limit = 128, DeviceSlots = 49;
struct Record {
    uint32_t target{}, slot{}, argsLen{}, dataLen{}, outLen{}, result{};
    uint32_t args[6]{}, data[16]{}, out[6]{};
};
static_assert(sizeof(Record) == 136, "Rust static-link transcript layout");
struct Trace {
    std::array<Record, Limit> records{};
    unsigned count = 0;
    bool refused = false;
};
inline thread_local Trace trace;
inline thread_local Trace* active = nullptr;
inline void* original[DeviceSlots]{};

inline bool Begin() {
    if (active) return false; // Nested driver methods belong to the outer call.
    trace.count = 0; trace.refused = false; active = &trace;
    return true;
}
inline void End() { active = nullptr; }

// Normalization is entirely at the public DX7 interface boundary. Pointers to
// input records are copied before native executes; output records afterwards.
inline void Input(Record& r, unsigned slot, const uint32_t* a) {
    r.slot = slot;
    switch (slot) {
    case 5: case 6: break;
    case 10: // Clear: rectangle variants require an unbounded payload.
        if (a[0] || a[1]) { r.slot = UINT32_MAX; break; }
        r.argsLen = 4; std::memcpy(r.args, a + 2, 16); break;
    case 11:
        if (!a[1]) { r.slot = UINT32_MAX; break; }
        r.argsLen = 1; r.args[0] = a[0]; r.dataLen = 64;
        std::memcpy(r.data, reinterpret_cast<const void*>(static_cast<uintptr_t>(a[1])), 64); break;
    case 13:
        if (!a[0]) { r.slot = UINT32_MAX; break; }
        r.argsLen = 6; std::memcpy(r.args, reinterpret_cast<const void*>(static_cast<uintptr_t>(a[0])), 24); break;
    case 15: r.outLen = 6; break;
    case 20: case 35: r.argsLen = 2; std::memcpy(r.args, a, 8); break;
    case 21: r.argsLen = 1; r.args[0] = a[0]; r.outLen = 1; break;
    case 37: r.argsLen = 3; std::memcpy(r.args, a, 12); break;
    default: break; // Kept in the stream, rejected by the core as unmodeled.
    }
}
inline void Output(Record& r, const uint32_t* a, uint32_t result) {
    r.result = result;
    const uint32_t pointer = r.slot == 15 ? a[0] : r.slot == 21 ? a[1] : 0;
    if (r.outLen) {
        // Failed queries can leave native stack storage undefined. Do not read it.
        if (!pointer || static_cast<int32_t>(result) < 0) { r.slot = UINT32_MAX; return; }
        std::memcpy(r.out, reinterpret_cast<const void*>(static_cast<uintptr_t>(pointer)), r.outLen * 4);
    }
}

template<unsigned Slot, typename... A> uint32_t __stdcall Hook(void* self, A... args) {
    const std::array<uint32_t, sizeof...(A)> words{args...};
    Trace* const capture = active;
    Record* record = nullptr;
    if (capture) {
        if (capture->count == Limit) capture->refused = true;
        else {
            record = &capture->records[capture->count++]; *record = {};
            record->target = static_cast<uint32_t>(reinterpret_cast<uintptr_t>(self));
            Input(*record, Slot, words.data());
        }
    }
    const uint32_t result = reinterpret_cast<uint32_t(__stdcall*)(void*, A...)>(original[Slot])(self, args...);
    if (record) Output(*record, words.data(), result);
    return result;
}

inline std::array<void*, DeviceSlots> Hooks() {
    using U = uint32_t;
    return {{
        reinterpret_cast<void*>(&Hook<0,U,U>), reinterpret_cast<void*>(&Hook<1>), reinterpret_cast<void*>(&Hook<2>),
        reinterpret_cast<void*>(&Hook<3,U>), reinterpret_cast<void*>(&Hook<4,U,U>), reinterpret_cast<void*>(&Hook<5>),
        reinterpret_cast<void*>(&Hook<6>), reinterpret_cast<void*>(&Hook<7,U>), reinterpret_cast<void*>(&Hook<8,U,U>),
        reinterpret_cast<void*>(&Hook<9,U>), reinterpret_cast<void*>(&Hook<10,U,U,U,U,U,U>), reinterpret_cast<void*>(&Hook<11,U,U>),
        reinterpret_cast<void*>(&Hook<12,U,U>), reinterpret_cast<void*>(&Hook<13,U>), reinterpret_cast<void*>(&Hook<14,U,U>),
        reinterpret_cast<void*>(&Hook<15,U>), reinterpret_cast<void*>(&Hook<16,U>), reinterpret_cast<void*>(&Hook<17,U>),
        reinterpret_cast<void*>(&Hook<18,U,U>), reinterpret_cast<void*>(&Hook<19,U,U>), reinterpret_cast<void*>(&Hook<20,U,U>),
        reinterpret_cast<void*>(&Hook<21,U,U>), reinterpret_cast<void*>(&Hook<22>), reinterpret_cast<void*>(&Hook<23,U>),
        reinterpret_cast<void*>(&Hook<24,U>), reinterpret_cast<void*>(&Hook<25,U,U,U,U,U>), reinterpret_cast<void*>(&Hook<26,U,U,U,U,U,U,U>),
        reinterpret_cast<void*>(&Hook<27,U>), reinterpret_cast<void*>(&Hook<28,U>), reinterpret_cast<void*>(&Hook<29,U,U,U,U,U>),
        reinterpret_cast<void*>(&Hook<30,U,U,U,U,U,U,U>), reinterpret_cast<void*>(&Hook<31,U,U,U,U,U>), reinterpret_cast<void*>(&Hook<32,U,U,U,U,U,U,U>),
        reinterpret_cast<void*>(&Hook<33,U,U,U,U,U>), reinterpret_cast<void*>(&Hook<34,U,U>), reinterpret_cast<void*>(&Hook<35,U,U>),
        reinterpret_cast<void*>(&Hook<36,U,U,U>), reinterpret_cast<void*>(&Hook<37,U,U,U>), reinterpret_cast<void*>(&Hook<38,U>),
        reinterpret_cast<void*>(&Hook<39,U>), reinterpret_cast<void*>(&Hook<40,U>), reinterpret_cast<void*>(&Hook<41,U>),
        reinterpret_cast<void*>(&Hook<42,U,U>), reinterpret_cast<void*>(&Hook<43,U,U,U,U,U>), reinterpret_cast<void*>(&Hook<44,U,U>),
        reinterpret_cast<void*>(&Hook<45,U,U>), reinterpret_cast<void*>(&Hook<46,U,U>), reinterpret_cast<void*>(&Hook<47,U,U>),
        reinterpret_cast<void*>(&Hook<48,U,U,U>)
    }};
}
}
