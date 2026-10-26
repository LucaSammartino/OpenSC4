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

// The main thread's floating-point control state (foundations §18.2). The
// game sets 24-bit x87 precision at the start of PreFrameWorkInit and the
// whole simulation depends on it - every x87 operation in the original
// subsystems rounds by PC alone, so a higher precision silently changes what
// they compute. Our DLL's mere presence raises the main thread to 53-bit
// somewhere between PreAppInit and PostAppInit, and nothing in the game can
// restore 24-bit afterwards, so FreeCity4 re-asserts it itself at PostAppInit
// through the game's own helper (FpuReassert24Bit). Simulation entries retain
// live precision, including Lua/modal re-entry (§18.4).
//
// It is logged:
//   fpu <where>: x87 CW=0x007F (PC 24-bit, RC nearest) MXCSR=0x00001F80 [thread …]
// at DLL attach, at every framework hook, at city load, at each observe month
// line, inside our budget settlement, and on any tick where it changed - so a
// reader of any log can see the precision at every stage and that we restored
// it. Always on: the first time the main thread's control word differs from
// the reference (the value seen at PreAppInit, after the game's
// PreFrameWorkInit), or is not at 24-bit precision once the reference is
// taken, one WARNING line; a successful re-assert re-arms both, so a later
// change warns again rather than passing silently.
#pragma once

#include <cstdint>

namespace fc4 {

uint16_t X87ControlWord();  // fnstcw (0 on non-x86 builds)
uint32_t SseMxcsr();        // _mm_getcsr

// Logs the state at `where` and runs the guard. `hook` marks the calling
// thread as the main thread (framework hooks run there).
void FpuLog(const char* where, bool hook = false);
// DllMain (process attach): the control word as the loader left it.
void FpuAttach();
// PreAppInit: the reference for the guard (logged).
void FpuSetReference(const char* where);
// PostAppInit: restore the 24-bit precision the game asserted at start-up and
// our DLL's presence lost (foundations §18.2). Goes through the game's own
// helper, so it inherits the game's notion of the 24-bit word rather than a
// hardcoded one. Logs the call and its effect; no-op off the main thread.
void FpuReassert24Bit(const char* where);
// foundations §18.4: diagnose simulation entry without changing live PC.
void FpuSimulationEntry(const char* where);
// Every framework tick (observer): logs a change of the control word once,
// with the tick number, and runs the guard.
void FpuTick();
// The guard alone (no line unless it fires).
void FpuCheck(const char* where);

// ---- the x87 leftover guard -------------------------------------------------
//
// At every point where the main thread's x87 register stack must be empty
// (MSVC code keeps nothing on it across calls), FXSAVE into a stack buffer
// and count the occupied registers in the abridged tag. A non-empty stack
// means some call left a value there (a float declaration where the callee
// returns nothing) or the game handed it to us that way; with every x87
// exception masked, the eighth leftover turns the next load into the
// indefinite NaN. Diagnostic only: nothing is popped or cleared.
//
// Sites: the entry and end of our OnTick, each framework tick (observer),
// before and after every message we deliver to a game agent, the entry and
// exit of every DoMessage of ours, after our synchronous MessageSend, and
// every FpuLog (which includes the monthly observation). Cheap when clean:
// one FXSAVE, no allocation, no log line. Each finding is counted per site;
// the first per site per game month is logged:
//   x87 leftovers at '<site>' (msg 0x<type>, '<agent>'): N register(s) occupied
//   (FSW=0x.... TOP=t, FTW=0x..), indefinite NaN: <ST list|none>; previous
//   check '<site>' had M; game month YYYY-MM, K at this site, T in all
// and the heartbeat carries `x87 leftovers: T` (FC4Watchdog.cpp). The first
// time the sticky stack-fault flag (SF) is seen set, one line says so
// (C1 = 1: overflow, 0: underflow). Main thread only (any thread before the
// main thread is known, as in the host tests).
struct X87Site {
    const char* name;
    volatile long count;        // findings at this site
    volatile long loggedMonth;  // the game month last logged (0 = never)
    volatile long registered;   // 1 once linked into the registry
    X87Site* volatile next;     // registry link (sites with findings only)
};

// The guard at `site`. `msg` (0 = none) and `who` (null = none) only go into
// the log line. Returns the occupied registers (0 = clean).
int X87Check(X87Site& site, uint32_t msg = 0, const char* who = nullptr);
// The occupied registers now (no counting, no log line).
int X87Occupied();
// Findings at every site since the DLL was loaded.
long X87LeftoverTotal();
// "x87 leftovers: T" plus, when T > 0, the (at most) three sites with the
// most findings: "x87 leftovers: 12 (crime DoMessage exit 8, ...)".
void X87DescribeLeftovers(char* out, size_t cap);

// Checks `entry` now and `exit` when the scope ends (a DoMessage, an OnTick).
class X87Scope {
public:
    X87Scope(X87Site& entry, X87Site& exit, uint32_t msg) : exit_(exit), msg_(msg) { X87Check(entry, msg); }
    ~X87Scope() { X87Check(exit_, msg_); }
    X87Scope(const X87Scope&) = delete;
    X87Scope& operator=(const X87Scope&) = delete;

private:
    X87Site& exit_;
    uint32_t msg_;
};

}  // namespace fc4

// One guard site (a string literal name), checked here.
#define FC4_X87_CHECK(name)                                        do {                                                               static ::fc4::X87Site fc4_x87_site_ = {name, 0, 0, 0, nullptr};         ::fc4::X87Check(fc4_x87_site_);                            } while (0)
// Two sites, "<name> entry" now and "<name> exit" at the end of the enclosing
// scope; `msg` is the message type for the log line (0 = none).
#define FC4_X87_SCOPE(name, msg)                                                             static ::fc4::X87Site fc4_x87_entry_ = {name " entry", 0, 0, 0, nullptr};                static ::fc4::X87Site fc4_x87_exit_ = {name " exit", 0, 0, 0, nullptr};                  ::fc4::X87Scope fc4_x87_scope_(fc4_x87_entry_, fc4_x87_exit_, (msg))
