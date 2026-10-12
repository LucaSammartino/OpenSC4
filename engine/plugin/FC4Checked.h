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

// Checked build support (tooling only; no spec behaviour).
//
// Configure with -DFC4_CHECKED=ON: the plugin is compiled with /Od /RTC1, so
// MSVC checks ESP after every call through a function pointer or vtable (a
// callee-pop / argument-count mismatch on a game interface shows up at the
// call) and the stack frames of our functions for overruns. RTC failures go
// to FreeCity4.log (no dialog): kind, message, the call site as module+offset
// with our function name (DbgHelp, from FreeCity4.pdb next to the DLL), our
// activity stack and the last interface calls. Execution continues.
//
// OutParam<T> wraps a small buffer or scalar we pass to a game method by
// pointer or reference. In checked builds it is surrounded by 16-byte
// canaries; if the game wrote past it, FreeCity4.log gets an
// "OUT-PARAM OVERRUN" line with the call site. In normal builds it is just T.
//
// FC4_CHECKED_SELFTEST=1 (checked builds) provokes one ESP mismatch through a
// wrong vtable signature and one out-parameter overrun at start-up, to prove
// that both reports work.
#pragma once

#include "FC4RefAudit.h"  // RefSite / FC4_SITE

#include <cstdint>
#include <cstring>

#ifndef FC4_CHECKED
#define FC4_CHECKED 0
#endif

namespace fc4 {

constexpr bool kCheckedBuild = FC4_CHECKED != 0;

// DllMain attach: installs the RTC error handler (checked builds; just a pointer store).
void CheckedAttach();
// First non-loader-lock opportunity (class enumeration): self-tests.
void CheckedStartup();
long RtcErrorCount();
long OutOverrunCount();

void ReportOutOverrun(const char* what, RefSite site, unsigned size, const uint8_t* pre, const uint8_t* post,
                      unsigned canarySize);

template <class T>
class OutParam {
public:
    OutParam(const char* what, RefSite site) : what_(what), site_(site) {
#if FC4_CHECKED
        std::memset(pre_, kCanary, sizeof(pre_));
        std::memset(post_, kCanary, sizeof(post_));
#endif
    }
    OutParam(const OutParam&) = delete;
    OutParam& operator=(const OutParam&) = delete;
    ~OutParam() { Check(); }

    T& get() { return value_; }
    T* ptr() { return &value_; }

    // Call right after the game call (also runs from the destructor).
    void Check() {
#if FC4_CHECKED
        if (checked_) return;
        checked_ = true;
        bool ok = true;
        for (unsigned i = 0; i < sizeof(pre_); ++i) ok = ok && pre_[i] == kCanary && post_[i] == kCanary;
        if (!ok) ReportOutOverrun(what_, site_, static_cast<unsigned>(sizeof(T)), pre_, post_, sizeof(pre_));
#endif
    }

private:
    static constexpr uint8_t kCanary = 0xC5;
    const char* what_;
    RefSite site_;
    bool checked_ = false;
#if FC4_CHECKED
    alignas(16) uint8_t pre_[16];
#endif
    T value_{};
#if FC4_CHECKED
    uint8_t post_[16];  // directly after value_ (byte alignment)
#endif
};

}  // namespace fc4

// FC4_OUT(uint8_t, sx, "cISC4Lot::GetSize x"); lot->GetSize(sx.get(), ...); sx.Check();
#define FC4_OUT(T, name, what) ::fc4::OutParam<T> name((what), FC4_SITE)
