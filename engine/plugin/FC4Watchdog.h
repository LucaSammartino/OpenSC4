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

// Hang diagnostics (tooling only; no spec behaviour).
//
// The simulation ("main") thread publishes cheap activity markers - no locks,
// no allocation: a stack of what it is inside (OnTick, core calls, our
// interface methods), the agent delivery in progress, and counters. A
// watchdog thread reads them every second and
//   - writes a heartbeat line every ~10 s (always, not verbose);
//   - reports a stall when the main thread has been inside our code for more
//     than 5 s, or no OnTick arrived for 15 s while a city is running: the
//     activity, the last 64 interface calls, and a sample of the main thread
//     (suspended only to copy EIP/ESP and its stack, formatted after resuming),
//     repeated every 5 s, at most 10 samples per stall.
// The watchdog logs without blocking on the log lock (LogWait), so a thread
// stuck while holding it cannot silence the report.
#pragma once

#include <cstdint>

namespace fc4 {

// Starts the watchdog (once) and records the calling thread as the main
// thread. Call from the simulation thread, never from DllMain.
void WatchdogStart();
// Stops and joins the watchdog thread (simulator destruction).
void WatchdogStop();

// True on the thread WatchdogStart was called from.
bool OnMainThread();
// That thread's ID (0 before WatchdogStart).
uint32_t MainThreadId();

// One entry of the main thread's activity stack (no-op on other threads).
// `label` must be a string literal (it is read later by the watchdog).
class ActivityScope {
public:
    explicit ActivityScope(const char* label);
    ~ActivityScope();
    ActivityScope(const ActivityScope&) = delete;
    ActivityScope& operator=(const ActivityScope&) = delete;

private:
    bool tracked_;
};

// Marks the main thread's OnTick (counts ticks; the scope is an ActivityScope).
class TickScope {
public:
    TickScope();
    ~TickScope();

private:
    ActivityScope scope_;
};

// An agent delivery in progress on the main thread (level, message type, name
// copied into a fixed buffer). End with DeliveryEnd().
void DeliveryBegin(uint32_t level, uint32_t type, const void* agent, const char* name);
void DeliveryEnd();

// Simulation state for the heartbeat and the stall rule (main thread).
void PublishSimState(int32_t julian, int32_t speed, int32_t pause, int32_t hidden, int32_t emergency);
// A city is running (between PostCityInit and Shutdown): the "no OnTick"
// rule only applies then.
void SetCityRunning(bool running);

// A verbose log line that a cap suppressed (reported in the heartbeat).
void NoteSuppressed();
// One of our interface methods was entered (FC4_TRACE; any thread).
void NoteInterfaceCall();
// A first-chance exception outside our image (vectored handler).
void NoteForeignException();

// A framework tick seen by the observer (FC4Observer; any mode): keeps the
// "no OnTick" rule meaningful when our simulator is not in use.
void NoteFrameworkTick();
// The current sim date from a calendar message (observer; 0 is ignored).
void PublishJulian(int32_t julian);

// Counters for the autotest verdict (any thread).
long PublishedJulian();
long ForeignExceptionCount();
long WatchdogStallCount();  // stall episodes reported so far
uint32_t LastTickMs();      // GetTickCount of the last OnTick / framework tick (0 = none)

// A heartbeat line now, from the main thread (month start); `why` is a literal.
void HeartbeatNow(const char* why);

// The main thread's activity stack and delivery, as in the stall report.
void DescribeCurrentActivity(char* out, unsigned cap);

// Module containing `addr` from a snapshot refreshed by the watchdog thread
// (lock-free; safe in the exception handler). Writes "name+0xOFFSET" or the
// plain address. Returns true if a module was found.
bool DescribeAddress(uintptr_t addr, char* out, unsigned cap);

// The executable's and this DLL's image ranges (computed without the loader lock).
bool InMainExe(uintptr_t addr);

// `addr` follows a call instruction (E8 rel32 or FF /2), read with
// ReadProcessMemory (no fault, no lock).
bool IsLikelyReturnAddress(uintptr_t addr);

}  // namespace fc4
