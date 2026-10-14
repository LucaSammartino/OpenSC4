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

#include "FC4HistClock.h"

#include "FC4Fpu.h"
#include "FC4Watchdog.h"

#include <utility>

namespace fc4 {
namespace {

bool g_hooksOn = false;
FC4HistClockHooks g_hooks{};

}  // namespace

const FC4HistClockHooks* HistClockHooks() { return g_hooksOn ? &g_hooks : nullptr; }

CoreLock::CoreLock() : mainTid_(GetCurrentThreadId()) { InitializeCriticalSection(&cs_); }

CoreLock::~CoreLock() { DeleteCriticalSection(&cs_); }

void CoreLock::Defer(std::function<void()> op) {
    // depth_ is only non-zero for the thread inside the section (owner check
    // through the critical section's own owner field would be private API).
    if (TryEnterCriticalSection(&cs_)) {
        const bool held = depth_ > 0;  // re-entered by its holder: this thread is inside a section
        if (held) pending_.push_back(std::move(op));
        LeaveCriticalSection(&cs_);
        if (held) return;
    }
    op();
}

CoreLock::Section::Section(CoreLock& lock, const char* what) : l_(lock) {
    if (GetCurrentThreadId() != l_.mainTid_ && l_.mainTid_ != 0) {
        ++l_.offMain_;
        NoteOffMainThreadCall(what);
    }
    EnterCriticalSection(&l_.cs_);
    ++l_.depth_;
}

CoreLock::Section::~Section() {
    std::vector<std::function<void()>> ops;
    if (--l_.depth_ == 0) ops.swap(l_.pending_);
    LeaveCriticalSection(&l_.cs_);
    for (auto& op : ops) op();
}

uint32_t CallerPrecisionBits() { return ((X87ControlWord() >> 8) & 3u) == 0 ? 24u : 53u; }

}  // namespace fc4

intptr_t HistClockTestSeam(const FC4HistClockTestInput* in) {
    if (in == nullptr) return -1;
    switch (in->op) {
        case 0:
            if (in->hooks != nullptr && in->hooks->struct_size >= sizeof(FC4HistClockHooks)) {
                fc4::g_hooks = *in->hooks;
                fc4::g_hooksOn = true;
            } else {
                fc4::g_hooks = FC4HistClockHooks{};
                fc4::g_hooksOn = false;
            }
            return 1;
        default:
            return -1;
    }
}
