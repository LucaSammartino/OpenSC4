// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <windows.h>
#include <cstdint>

namespace fc4 {
// PV16, sections 4.8/5.2: observe the genuine calling thread's rand input
// inside the developer callback. Never call __getptd or the plugin CRT.
// This observer is scoped and refuses a busy DR0 instead of replacing it.
class PropNightCrt {
public:
    bool Begin() {
        if (active_) return false; // nesting must not overwrite a parent scope
        handler_ = AddVectoredExceptionHandler(1, &Observe);
        if (!handler_) return false;
        active_ = this;
        const ULONG_PTR operation = 1;
        RaiseException(kConfigure, 0, 1, &operation);
        return configured_;
    }
    void Finish() {
        if (active_ == this) {
            if (configured_) {
                const ULONG_PTR operation = 0;
                RaiseException(kConfigure, 0, 1, &operation);
            }
            active_ = nullptr;
        }
        if (handler_) RemoveVectoredExceptionHandler(handler_);
        handler_ = nullptr;
    }
    ~PropNightCrt() { Finish(); }
    uint32_t before = 0, after = 0, count = 0;
    bool valid = true;
private:
    static constexpr DWORD kConfigure = 0xE042FC47;
    static constexpr DWORD kRand = 0x009EFBA4;
    static LONG CALLBACK Observe(EXCEPTION_POINTERS* p) {
#if defined(_M_IX86)
        auto* s = active_;
        if (!s) return EXCEPTION_CONTINUE_SEARCH;
        auto* c = p->ContextRecord;
        const auto* e = p->ExceptionRecord;
        if (e->ExceptionCode == kConfigure && e->NumberParameters == 1) {
            c->ContextFlags |= CONTEXT_DEBUG_REGISTERS;
            if (e->ExceptionInformation[0]) {
                if (c->Dr7 & 3) { s->valid = false; return EXCEPTION_CONTINUE_EXECUTION; }
                s->dr0_ = c->Dr0; s->dr6_ = c->Dr6; s->dr7_ = c->Dr7;
                c->Dr0 = kRand; c->Dr6 &= ~1u;
                c->Dr7 = (c->Dr7 & ~0xF0000u) | 1u;
                s->configured_ = true;
            } else {
                c->Dr0 = s->dr0_; c->Dr6 = s->dr6_; c->Dr7 = s->dr7_;
                s->configured_ = false;
            }
            return EXCEPTION_CONTINUE_EXECUTION;
        }
        if (e->ExceptionCode == EXCEPTION_SINGLE_STEP && s->configured_ &&
            c->Eip == kRand && (c->Dr6 & 1)) {
            const uint32_t state = *reinterpret_cast<const uint32_t*>(c->Eax + 0x14);
            if (s->count == 0) s->before = state;
            else if (state != s->after) s->valid = false;
            s->after = state * 214013u + 2531011u;
            ++s->count;
            c->Dr6 &= ~1u;
            c->EFlags |= 0x10000;
            return EXCEPTION_CONTINUE_EXECUTION;
        }
#else
        (void)p;
#endif
        return EXCEPTION_CONTINUE_SEARCH;
    }
    inline static thread_local PropNightCrt* active_ = nullptr;
    PVOID handler_ = nullptr;
    DWORD dr0_ = 0, dr6_ = 0, dr7_ = 0;
    bool configured_ = false;
};
}
