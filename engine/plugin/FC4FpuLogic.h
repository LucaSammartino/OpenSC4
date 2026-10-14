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

// Pure decision logic of the x87 precision re-assert (foundations §18.2),
// header-only and free of Windows calls so tests/fpu_logic_test.cpp can check
// it directly. No game address is known here; FC4Fpu.cpp gets the helper
// itself from compat (the only place allowed to).
#pragma once

#include <cstddef>
#include <cstdint>
#include <cstdio>

namespace fc4 {

// The precision-control field of an x87 control word (bits 8-9): 0 = 24-bit,
// 2 = 53-bit, 3 = 64-bit, 1 = reserved.
inline unsigned PcField(uint16_t cw) { return (cw >> 8) & 3u; }

inline bool Is24Bit(uint16_t cw) { return PcField(cw) == 0u; }

// The mode the game's helper takes. It clears the PC field, then sets bit 1
// for mode 1 (53-bit) or bits 0-1 for mode 2 (64-bit), so mode 0 is 24-bit -
// the only mode FreeCity4 may pass (§18.2: the unmodified game runs the whole
// session at 24-bit).
constexpr int kFpuMode24Bit = 0;
constexpr int kFpuMode53Bit = 1;
constexpr int kFpuMode64Bit = 2;

// The control word the helper leaves behind for `mode`: PC changes, rounding
// and the exception masks do not.
inline uint16_t ApplyMode(uint16_t cw, int mode) {
    uint16_t out = static_cast<uint16_t>(cw & ~uint16_t(0x0300));
    if (mode == kFpuMode53Bit) {
        out = static_cast<uint16_t>(out | uint16_t(0x0200));  // PC = 10
    } else if (mode == kFpuMode64Bit) {
        out = static_cast<uint16_t>(out | uint16_t(0x0300));  // PC = 11
    }
    return out;
}

enum class ReassertOutcome {
    AlreadyCorrect,  // it was 24-bit and stayed so
    Restored,        // it was above 24-bit and the helper brought it back
    Failed,          // it is still above 24-bit: report this, do not trust the run
};

// How to report a re-assert from the control word before and after it.
// "Still wrong" outranks "unchanged": a helper that did nothing at 53-bit is a
// failure, not a no-op success.
inline ReassertOutcome ReassertOutcomeOf(uint16_t before, uint16_t after) {
    if (!Is24Bit(after)) return ReassertOutcome::Failed;
    return after == before ? ReassertOutcome::AlreadyCorrect : ReassertOutcome::Restored;
}

// ---- the x87 register stack (the leftover guard and the crash report) ----
//
// The game runs with every x87 exception masked, so a wrong return channel
// does not fault where it happens: a value left on the stack (a float
// declaration for a method that returns nothing) or popped off it (the
// reverse) only shifts the stack. Eight leftovers later the next load
// overflows and yields the indefinite NaN, and a NaN converted to an integer
// is a garbage size. These helpers decode the saved x87 state.

// Status word fields: TOP (bits 11-13), C1 (bit 9: with SF, 1 = overflow,
// 0 = underflow), SF (bit 6, sticky: stack fault), IE (bit 0, sticky:
// invalid operation).
inline unsigned X87TopOf(uint16_t sw) { return (sw >> 11) & 7u; }
inline bool X87StackFault(uint16_t sw) { return (sw & 0x0040u) != 0; }
inline bool X87InvalidOp(uint16_t sw) { return (sw & 0x0001u) != 0; }
inline bool X87C1(uint16_t sw) { return (sw & 0x0200u) != 0; }

// Occupied registers from the FNSAVE / FNSTENV tag word: two bits per
// physical register, 11 = empty.
inline int OccupiedFromFullTag(uint16_t tw) {
    int n = 0;
    for (unsigned r = 0; r < 8; ++r) n += ((tw >> (2 * r)) & 3u) != 3u ? 1 : 0;
    return n;
}

// Occupied registers from the FXSAVE abridged tag byte: one bit per physical
// register, 1 = valid.
inline int OccupiedFromAbridgedTag(uint8_t ftw) {
    int n = 0;
    for (unsigned r = 0; r < 8; ++r) n += (ftw >> r) & 1u;
    return n;
}

// The x87 "real indefinite" (the default NaN of a masked invalid operation)
// as 80 bits, little-endian: sign 1, exponent 0x7FFF, significand
// 0xC000000000000000, i.e. bytes 00 00 00 00 00 00 00 C0 FF FF.
inline bool IsX87Indefinite(const uint8_t* r) {
    for (unsigned i = 0; i < 7; ++i) {
        if (r[i] != 0) return false;
    }
    return r[7] == 0xC0 && r[8] == 0xFF && r[9] == 0xFF;
}

// One saved x87 state, decoded.
struct X87State {
    uint16_t cw = 0;
    uint16_t sw = 0;
    uint16_t tag = 0;      // as saved: the full tag word, or the abridged byte
    bool abridged = false;  // `tag` is FXSAVE's abridged byte
    int occupied = 0;
    uint8_t indefinite = 0;  // bit i: ST(i) holds the indefinite NaN (empty or not)
};

// FNSAVE layout, as in CONTEXT.FloatSave: control, status and tag words, then
// ST(0)..ST(7) at 10 bytes each (by stack position, not physical register).
inline X87State DecodeFnsave(uint16_t cw, uint16_t sw, uint16_t tw, const uint8_t* regs80) {
    X87State s;
    s.cw = cw;
    s.sw = sw;
    s.tag = tw;
    s.occupied = OccupiedFromFullTag(tw);
    for (unsigned i = 0; i < 8; ++i) {
        if (regs80 != nullptr && IsX87Indefinite(regs80 + 10 * i)) s.indefinite |= static_cast<uint8_t>(1u << i);
    }
    return s;
}

// FXSAVE layout (512 bytes, as in CONTEXT.ExtendedRegisters): FCW at 0, FSW at
// 2, the abridged tag at 4, ST(0)..ST(7) at 32 + 16 i.
inline X87State DecodeFxsave(const uint8_t* fx) {
    X87State s;
    s.cw = static_cast<uint16_t>(fx[0] | (fx[1] << 8));
    s.sw = static_cast<uint16_t>(fx[2] | (fx[3] << 8));
    s.tag = fx[4];
    s.abridged = true;
    s.occupied = OccupiedFromAbridgedTag(fx[4]);
    for (unsigned i = 0; i < 8; ++i) {
        if (IsX87Indefinite(fx + 32 + 16 * i)) s.indefinite |= static_cast<uint8_t>(1u << i);
    }
    return s;
}

// "ST0 ST3" (the registers holding the indefinite NaN), or "none".
inline void FormatIndefinite(char* out, size_t cap, uint8_t mask) {
    if (cap == 0) return;
    out[0] = 0;
    size_t len = 0;
    for (unsigned i = 0; i < 8; ++i) {
        if (((mask >> i) & 1u) == 0) continue;
        const int k = std::snprintf(out + len, cap - len, "%sST%u", len ? " " : "", i);
        if (k < 0 || static_cast<size_t>(k) >= cap - len) return;
        len += static_cast<size_t>(k);
    }
    if (len == 0) std::snprintf(out, cap, "none");
}

// The crash report's FPU line (FC4Diag.cpp), from CONTEXT.FloatSave and,
// when the context has them, the FXSAVE image (ExtendedRegisters):
//   fpu: CW=0x027F SW=0x0000 (TOP=0 SF=0 IE=0 C1=0) TW=0xFFFF occupied=0
//   [FXSAVE FTW=0x00 occupied=0] indefinite NaN: none
// (one line). TW is decoded as the full tag word; the FXSAVE part is the
// abridged tag, which is authoritative where both are present.
inline int FormatCrashFpu(char* out, size_t cap, const X87State& fnsave, const X87State* fxsave) {
    char nan[48];
    FormatIndefinite(nan, sizeof(nan), static_cast<uint8_t>(fnsave.indefinite | (fxsave ? fxsave->indefinite : 0)));
    char fx[64] = "";
    if (fxsave != nullptr) {
        std::snprintf(fx, sizeof(fx), " [FXSAVE FTW=0x%02X occupied=%d]", fxsave->tag & 0xFFu, fxsave->occupied);
    }
    return std::snprintf(out, cap, "fpu: CW=0x%04X SW=0x%04X (TOP=%u SF=%d IE=%d C1=%d) TW=0x%04X occupied=%d%s "
                         "indefinite NaN: %s",
                         fnsave.cw, fnsave.sw, X87TopOf(fnsave.sw), X87StackFault(fnsave.sw) ? 1 : 0,
                         X87InvalidOp(fnsave.sw) ? 1 : 0, X87C1(fnsave.sw) ? 1 : 0, fnsave.tag, fnsave.occupied, fx,
                         nan);
}

}  // namespace fc4
