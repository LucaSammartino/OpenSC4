// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4Reach.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <intrin.h>

#include <algorithm>
#include <atomic>
#include <cstring>
#include <cstdio>
#include <fstream>
#include <iomanip>
#include <limits>
#include <sstream>

#ifndef FC4_REACH_TEST
#include "FC4Config.h"
#include "FC4HarnessLogic.h"
#include "FC4Log.h"
#endif

namespace fc4 {
namespace reach {
namespace {
constexpr uint32_t kTrapFlag = 0x100;
constexpr uint8_t kBreakpoint = 0xcc;
constexpr DWORD kWx86SingleStep = 0x4000001e;
constexpr DWORD kWx86Breakpoint = 0x4000001f;
constexpr size_t kThreadSlots = 4096;
static_assert(std::atomic<uint32_t>::is_always_lock_free, "VEH counters must be lock free");

bool Number(const std::string& text, int base, uint32_t& out) {
    if (text.empty()) return false;
    uint64_t n = 0;
    for (char c : text) {
        unsigned d = c >= '0' && c <= '9' ? c - '0' : c >= 'a' && c <= 'f' ? c - 'a' + 10 : 255;
        if (d >= static_cast<unsigned>(base)) return false;
        n = n * base + d;
        if (n > UINT32_MAX) return false;
    }
    out = static_cast<uint32_t>(n);
    return true;
}

bool Hazardous(const std::string& name) {
    // Small conservative deny-list: exception dispatch, CRT lock/loader
    // callbacks and stack probes can run recursively inside the VEH itself.
    const char* const deny[] = {"Exception", "exception", "except_handler", "CxxFrameHandler",
        "CxxThrowException", "XcptFilter", "DllMain", "dllmain", "DllEntry", "CRTStartup",
        "_initterm", "_onexit", "atexit", "_lock", "_unlock", "chkstk", "alloca_probe",
        "setjmp", "longjmp", "security_check_cookie", "report_gsfailure"};
    for (const char* s : deny) if (name.find(s) != std::string::npos) return true;
    return false;
}

const char* NeverArm(uint32_t address, uintptr_t preferredBase) {
    if (preferredBase != 0x00400000) return "";
    // 1.1.641 ABI roster artifacts, verified with re_query.py. These addresses
    // are operands, not instruction boundaries. INT3 corrupts the containing
    // instruction without delivering a breakpoint. See docs/ops/reach-tracing.md.
    switch (address) {
    case 0x00420036: return "not-entry:IID-immediate-at-00420034";
    case 0x004f4442: return "not-entry:call-displacement-at-004f4440";
    case 0x00505553: return "not-entry:ModRM-at-00505552";
    case 0x00544547: return "not-entry:ModRM-stack-store-at-00544546";
    case 0x005c3a63: return "not-entry:ModRM-stack-load-at-005c3a62";
    case 0x005f5a52: return "not-entry:displacement-at-005f5a50";
    case 0x00656c64: return "not-entry:absolute-address-at-00656c61";
    case 0x00682632: return "not-entry:stack-displacement-at-0068262f";
    case 0x00736559: return "not-entry:ModRM-stack-test-at-00736558";
    case 0x00786f62: return "not-entry:SIB-stack-store-at-00786f60";
    default: return "";
    }
}

const char* UnsafeByte(uint8_t b) {
    if (b == 0xcc) return "preexisting-breakpoint";
    // Conservatively omit preexisting detours (including genuine jump thunks).
    if (b == 0xe9 || b == 0xeb) return "preexisting-jump-or-hook";
    // POPF/IRET can clear TF; SS loads delay the debug trap. Prefixes could
    // hide these instructions, so omit prefixed entries rather than decode.
    switch (b) {
    case 0x17: case 0x8e: case 0x9c: case 0x9d: case 0xcf: case 0xcd: case 0xce: case 0xf1:
    case 0x26: case 0x2e: case 0x36: case 0x3e: case 0x64: case 0x65:
    case 0x66: case 0x67: case 0xf0: case 0xf2: case 0xf3:
        return "trap-unsafe-prefix";
    default: return "";
    }
}

bool Executable(const void* p, size_t bytes) {
    MEMORY_BASIC_INFORMATION m{};
    if (!VirtualQuery(p, &m, sizeof(m)) || m.State != MEM_COMMIT || (m.Protect & PAGE_GUARD)) return false;
    const DWORD protection = m.Protect & 0xff;
    if (protection != PAGE_EXECUTE_READ && protection != PAGE_EXECUTE_READWRITE &&
        protection != PAGE_EXECUTE_WRITECOPY) return false;
    return reinterpret_cast<uintptr_t>(p) + bytes <= reinterpret_cast<uintptr_t>(m.BaseAddress) + m.RegionSize;
}
}  // namespace

bool ReadContracts(std::istream& input, std::vector<Contract>& out, std::string& error) {
    out.clear();
    error.clear();
    std::string line;
    bool header = false;
    unsigned lineNumber = 0;
    while (std::getline(input, line)) {
        ++lineNumber;
        if (!line.empty() && line.back() == '\r') line.pop_back();
        if (line.empty() || line[0] == '#') continue;
        if (!header) {
            if (line.compare(0, 18, "address\tsize\tname\t") != 0 && line != "address\tsize\tname") {
                error = "expected address/size/name header";
                return false;
            }
            header = true;
            continue;
        }
        const auto a = line.find('\t');
        const auto b = a == std::string::npos ? a : line.find('\t', a + 1);
        const auto c = b == std::string::npos ? b : line.find('\t', b + 1);
        Contract row{};
        if (a != 8 || b == std::string::npos || !Number(line.substr(0, a), 16, row.address) ||
            !Number(line.substr(a + 1, b - a - 1), 10, row.size)) {
            error = "invalid contract row " + std::to_string(lineNumber);
            out.clear();
            return false;
        }
        row.name = line.substr(b + 1, c == std::string::npos ? c : c - b - 1);
        out.push_back(std::move(row));
    }
    std::sort(out.begin(), out.end(), [](const Contract& a, const Contract& b) { return a.address < b.address; });
    for (size_t i = 1; i < out.size(); ++i) {
        if (out[i - 1].address == out[i].address) {
            error = "duplicate contract address";
            out.clear();
            return false;
        }
    }
    if (!header || out.empty() || input.bad()) {
        error = "empty or unreadable contracts";
        out.clear();
        return false;
    }
    return true;
}

struct Tracer::State {
    struct Entry {
        uint32_t address = 0;
        volatile uint8_t* code = nullptr;
        uint8_t expected[16]{};
        size_t expectedSize = 0;
        std::string skipped;
        // Only this tiny, allocation-free critical section touches the entry
        // byte, pending count and metrics. No Win32/game/CRT call while held.
        std::atomic_flag gate = ATOMIC_FLAG_INIT;
        uint32_t pending = 0, window = 0, count = 0;
        uint32_t entries = 0, windowsHit = 0, first = 0, last = 0;
        bool capped = false;
        std::atomic<bool> everArmed{false}, writable{false};
        DWORD oldProtection = 0;
        bool conflict = false, exhausted = false;
        void Lock() { while (gate.test_and_set(std::memory_order_acquire)) YieldProcessor(); }
        void Unlock() { gate.clear(std::memory_order_release); }
    };
    struct Thread {
        struct Event {
            uint32_t entry, code, eip, esp, eax, ecx, edx, ebx, esi, edi, ebp, flags, dr6;
        } history[32]{};
        std::atomic<uint32_t> id{0};
        Entry* entry = nullptr;
        bool previousTF = false;
        uint32_t ignored = 0;
        uint32_t events = 0;
    };
    std::unique_ptr<Entry[]> entries;
    size_t count = 0;
    Thread threads[kThreadSlots];
    std::atomic<uint32_t> window{0}, traps{0}, ignored{0}, aborted{0}, exhausted{0};
    std::atomic<uint32_t> wxBreakpoints{0}, wxSteps{0};
    std::atomic<bool> active{false};
    uint32_t cap = 0;
    PVOID handler = nullptr;
    HANDLE diagnostic = INVALID_HANDLE_VALUE;
    std::atomic<uint32_t> reports{0};
    static std::atomic<State*> current;

    void Record(Thread* t, Entry* e, DWORD code, const CONTEXT& c) {
#if defined(_M_IX86)
        if (diagnostic == INVALID_HANDLE_VALUE || !t) return;
        t->history[t->events++ % 32] = {e ? e->address : 0, code, c.Eip, c.Esp,
            c.Eax, c.Ecx, c.Edx, c.Ebx, c.Esi, c.Edi, c.Ebp, c.EFlags, c.Dr6};
#endif
    }

    void Report(Thread* t, const EXCEPTION_RECORD& r, const CONTEXT& c) {
#if defined(_M_IX86)
        if (diagnostic == INVALID_HANDLE_VALUE || reports.fetch_add(1) >= 16) return;
        char buffer[768];
        auto write = [&](int n) {
            DWORD done = 0;
            if (n > 0) WriteFile(diagnostic, buffer, static_cast<DWORD>((std::min)(n, 767)), &done, nullptr);
        };
        write(std::snprintf(buffer, sizeof(buffer),
            "exception tid=%lu code=%08lx at=%p info=%08lx/%08lx pending=%08x previousTF=%u\n",
            GetCurrentThreadId(), r.ExceptionCode, r.ExceptionAddress,
            r.NumberParameters > 0 ? r.ExceptionInformation[0] : 0,
            r.NumberParameters > 1 ? r.ExceptionInformation[1] : 0,
            t && t->entry ? t->entry->address : 0, t && t->previousTF ? 1 : 0));
        Record(t, t ? t->entry : nullptr, r.ExceptionCode, c);
        if (t) for (uint32_t i = t->events > 32 ? t->events - 32 : 0; i < t->events; ++i) {
            const auto& e = t->history[i % 32];
            write(std::snprintf(buffer, sizeof(buffer),
                "event=%u entry=%08x code=%08x eip=%08x esp=%08x eax=%08x ecx=%08x edx=%08x ebx=%08x esi=%08x edi=%08x ebp=%08x flags=%08x dr6=%08x\n",
                i, e.entry, e.code, e.eip, e.esp, e.eax, e.ecx, e.edx, e.ebx, e.esi, e.edi, e.ebp, e.flags, e.dr6));
        }
        FlushFileBuffers(diagnostic);
#endif
    }

    Thread* Slot(bool create) {
        const uint32_t tid = GetCurrentThreadId();
        size_t slot = (tid >> 2) % kThreadSlots;
        for (size_t n = 0; n < kThreadSlots; ++n, slot = (slot + 1) % kThreadSlots) {
            uint32_t id = threads[slot].id.load(std::memory_order_relaxed);
            if (id == tid) return &threads[slot];
            if (id == 0) {
                if (!create) return nullptr;
                if (threads[slot].id.compare_exchange_strong(id, tid, std::memory_order_relaxed)) return &threads[slot];
                if (id == tid) return &threads[slot];
            }
        }
        return nullptr;
    }

    Entry* Find(uintptr_t address) {
        size_t lo = 0, hi = count;
        while (lo < hi) {
            const size_t mid = lo + (hi - lo) / 2;
            const uintptr_t value = reinterpret_cast<uintptr_t>(entries[mid].code);
            if (value < address) lo = mid + 1;
            else hi = mid;
        }
        return lo < count && reinterpret_cast<uintptr_t>(entries[lo].code) == address ? &entries[lo] : nullptr;
    }

    static void Flush(Entry& e) {
        FlushInstructionCache(GetCurrentProcess(), const_cast<const uint8_t*>(e.code), 1);
    }

    bool Eligible(Entry& e) const {
        return active.load(std::memory_order_relaxed) && !e.conflict && !e.exhausted &&
            (e.window != window.load(std::memory_order_relaxed) || e.count < cap);
    }

    void FinishStep(Thread& t) {
        Entry& e = *t.entry;
        t.entry = nullptr;
        e.Lock();
        --e.pending;
        if (!e.pending && Eligible(e)) {
            if (*e.code == e.expected[0]) *e.code = kBreakpoint;
            else if (*e.code != kBreakpoint) e.conflict = true;
        }
        e.Unlock();
        Flush(e);
    }

    static LONG CALLBACK Handler(EXCEPTION_POINTERS* ep) {
#if defined(_M_IX86)
        State* s = current.load(std::memory_order_acquire);
        if (!s || !ep || !ep->ExceptionRecord || !ep->ContextRecord) return EXCEPTION_CONTINUE_SEARCH;
        const DWORD code = ep->ExceptionRecord->ExceptionCode;
        CONTEXT& ctx = *ep->ContextRecord;
        if (code == EXCEPTION_BREAKPOINT || code == kWx86Breakpoint) {
            Entry* e = s->Find(reinterpret_cast<uintptr_t>(ep->ExceptionRecord->ExceptionAddress));
            if (!e || !e->everArmed || !e->writable) return EXCEPTION_CONTINUE_SEARCH;
            s->traps.fetch_add(1, std::memory_order_relaxed);
            if (code == kWx86Breakpoint) s->wxBreakpoints.fetch_add(1, std::memory_order_relaxed);
            Thread* t = s->Slot(true);
            s->Record(t, e, code, ctx);
            // An unexpected nested breakpoint/aborted first instruction must
            // not leak a pending step or consume somebody else's TF.
            bool previousTF = (ctx.EFlags & kTrapFlag) != 0;
            if (t && t->entry) {
                previousTF = t->previousTF;
                s->FinishStep(*t);
                s->aborted.fetch_add(1, std::memory_order_relaxed);
            }
            e->Lock();
            const uint32_t w = s->window.load(std::memory_order_relaxed);
            if (e->window != w) { e->window = w; e->count = 0; }
            if (s->active.load(std::memory_order_relaxed) && e->count < s->cap) {
                if (t && t->ignored) s->ignored.fetch_add(1, std::memory_order_relaxed);
                else {
                    if (e->count++ == 0) {
                        ++e->windowsHit;
                        if (!e->first) e->first = w;
                        e->last = w;
                    }
                    ++e->entries;
                    if (e->count == s->cap) e->capped = true;
                }
            }
            if (*e->code == kBreakpoint) *e->code = e->expected[0];
            else if (*e->code != e->expected[0]) e->conflict = true;
            if (t) {
                ++e->pending;
                t->entry = e;
                t->previousTF = previousTF;
                ctx.EFlags |= kTrapFlag;
            } else {
                e->exhausted = true;
                s->exhausted.fetch_add(1, std::memory_order_relaxed);
            }
            e->Unlock();
            Flush(*e);
            ctx.Eip = reinterpret_cast<DWORD>(e->code);
            return EXCEPTION_CONTINUE_EXECUTION;
        }
        Thread* t = s->Slot(false);
        if (code != EXCEPTION_SINGLE_STEP && code != kWx86SingleStep) s->Report(t, *ep->ExceptionRecord, ctx);
        if (t && t->entry) {
            s->Record(t, t->entry, code, ctx);
            const bool foreignTF = t->previousTF;
            s->FinishStep(*t);
            if (!foreignTF) ctx.EFlags &= ~kTrapFlag;
            if (code == EXCEPTION_SINGLE_STEP || code == kWx86SingleStep) {
                s->traps.fetch_add(1, std::memory_order_relaxed);
                if (code == kWx86SingleStep) s->wxSteps.fetch_add(1, std::memory_order_relaxed);
                if (!foreignTF && !(ctx.Dr6 & 0xf)) return EXCEPTION_CONTINUE_EXECUTION;
            } else s->aborted.fetch_add(1, std::memory_order_relaxed);
        }
#else
        (void)ep;
#endif
        return EXCEPTION_CONTINUE_SEARCH;
    }
};

std::atomic<Tracer::State*> Tracer::State::current{nullptr};

Tracer::Tracer() : state_(new State) {}
Tracer::~Tracer() {
    Stop();
    if (state_->handler) {
        RemoveVectoredExceptionHandler(state_->handler);
        State::current.store(nullptr, std::memory_order_release);
    }
    if (state_->diagnostic != INVALID_HANDLE_VALUE) CloseHandle(state_->diagnostic);
    for (size_t i = state_->count; i > 0; --i) {
        auto& e = state_->entries[i - 1];
        if (e.writable) {
            DWORD unused = 0;
            VirtualProtect(const_cast<uint8_t*>(e.code), 1, e.oldProtection, &unused);
        }
    }
}

bool Tracer::Diagnostics(const std::string& path) {
    if (state_->handler || state_->diagnostic != INVALID_HANDLE_VALUE) return false;
    state_->diagnostic = CreateFileA(path.c_str(), GENERIC_WRITE, FILE_SHARE_READ,
                                    nullptr, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    return state_->diagnostic != INVALID_HANDLE_VALUE;
}

bool Tracer::Prepare(const std::vector<Contract>& contracts, uintptr_t preferredBase,
                     uintptr_t actualBase, size_t imageSize) {
    State& s = *state_;
    if (s.entries || contracts.empty()) return false;
    s.count = contracts.size();
    s.entries.reset(new State::Entry[s.count]);
    uint32_t last = 0;
    for (size_t i = 0; i < s.count; ++i) {
        const auto& c = contracts[i];
        auto& e = s.entries[i];
        e.address = c.address;
        e.code = reinterpret_cast<uint8_t*>(actualBase + (static_cast<uintptr_t>(c.address) - preferredBase));
        if ((i && c.address <= last) || c.address < preferredBase ||
            static_cast<uintptr_t>(c.address) - preferredBase >= imageSize ||
            c.size > imageSize - (static_cast<uintptr_t>(c.address) - preferredBase)) e.skipped = "outside-image-or-unordered";
        else if (*NeverArm(c.address, preferredBase)) e.skipped = NeverArm(c.address, preferredBase);
        else if (Hazardous(c.name)) e.skipped = "exception-or-loader-runtime";
        else {
            // SPEC-GAP: a vtable-only contract can have unknown size (0);
            // snapshot only its entry byte and credit no bytes for that row.
            e.expectedSize = (std::max)(size_t{1}, (std::min)(sizeof(e.expected), static_cast<size_t>(c.size)));
            if (!Executable(const_cast<const uint8_t*>(e.code), e.expectedSize)) e.skipped = "not-readable-executable";
            else {
                // SPEC-GAP: contracts have no entry bytes; use an in-memory
                // snapshot taken before any FreeCity4 entry patches.
                std::memcpy(e.expected, const_cast<const uint8_t*>(e.code), e.expectedSize);
                e.skipped = UnsafeByte(e.expected[0]);
            }
        }
        last = c.address;
    }
    return true;
}

bool Tracer::Start(uint32_t cap) {
#if !defined(_M_IX86)
    (void)cap;
    return false;
#else
    State& s = *state_;
    if (!s.entries || !cap || s.handler) return false;
    State* none = nullptr;
    if (!State::current.compare_exchange_strong(none, &s, std::memory_order_release)) return false;
    s.cap = cap;
    s.window.store(1, std::memory_order_relaxed);
    s.handler = AddVectoredExceptionHandler(1, &State::Handler);
    if (!s.handler) { State::current.store(nullptr, std::memory_order_release); return false; }
    s.active.store(true, std::memory_order_release);
    for (size_t i = 0; i < s.count; ++i) {
        auto& e = s.entries[i];
        if (!e.skipped.empty()) continue;
        if (std::memcmp(e.expected, const_cast<const uint8_t*>(e.code), e.expectedSize)) e.skipped = "entry-changed-by-hook";
        else if (!VirtualProtect(const_cast<uint8_t*>(e.code), 1, PAGE_EXECUTE_READWRITE, &e.oldProtection)) e.skipped = "protect-failed";
        else {
            e.writable = true;
            e.Lock();
            e.everArmed = true;
            *e.code = kBreakpoint;
            e.Unlock();
            State::Flush(e);
        }
    }
    return true;
#endif
}

void Tracer::NextWindow() {
    State& s = *state_;
    if (!s.active.load(std::memory_order_acquire)) return;
    s.window.fetch_add(1, std::memory_order_relaxed);
    for (size_t i = 0; i < s.count; ++i) {
        auto& e = s.entries[i];
        if (!e.everArmed) continue;
        e.Lock();
        if (!e.pending && s.Eligible(e)) {
            if (*e.code == e.expected[0]) *e.code = kBreakpoint;
            else if (*e.code != kBreakpoint) e.conflict = true;
        }
        e.Unlock();
        State::Flush(e);
    }
}

void Tracer::Stop() {
    Suspend();
}

bool Tracer::Suspend() {
    State& s = *state_;
    if (!s.active.exchange(false, std::memory_order_acq_rel)) return false;
    for (size_t i = 0; i < s.count; ++i) {
        auto& e = s.entries[i];
        if (!e.everArmed) continue;
        e.Lock();
        if (*e.code == kBreakpoint) *e.code = e.expected[0];
        else if (*e.code != e.expected[0]) e.conflict = true;
        e.Unlock();
        State::Flush(e);
    }
    return true;
}

void Tracer::Resume() {
    State& s = *state_;
    if (!s.handler || s.active.exchange(true, std::memory_order_acq_rel)) return;
    for (size_t i = 0; i < s.count; ++i) {
        auto& e = s.entries[i];
        if (!e.everArmed) continue;
        e.Lock();
        if (!e.pending && s.Eligible(e)) {
            if (*e.code == e.expected[0]) *e.code = kBreakpoint;
            else if (*e.code != kBreakpoint) e.conflict = true;
        }
        e.Unlock();
        State::Flush(e);
    }
}

std::vector<Row> Tracer::Rows() const {
    std::vector<Row> rows;
    for (size_t i = 0; i < state_->count; ++i) {
        auto& e = state_->entries[i];
        Row r;
        r.address = e.address;
        r.skipped = e.skipped;
        e.Lock();
        r.entries = e.entries; r.windowsHit = e.windowsHit;
        r.firstWindow = e.first; r.lastWindow = e.last; r.capped = e.capped;
        if (e.conflict) r.skipped = "late-entry-conflict";
        if (e.exhausted) r.skipped = "thread-slot-exhaustion";
        e.Unlock();
        rows.push_back(std::move(r));
    }
    return rows;
}

Summary Tracer::GetSummary() const {
    Summary out;
    for (const auto& r : Rows()) {
        ++out.functions;
        if (!r.skipped.empty()) ++out.skipped;
        if (r.entries) ++out.reached;
    }
    for (size_t i = 0; i < state_->count; ++i) if (state_->entries[i].everArmed) ++out.armed;
    out.windows = state_->window.load(); out.traps = state_->traps.load();
    out.ignored = state_->ignored.load(); out.abortedSteps = state_->aborted.load();
    out.slotExhaustions = state_->exhausted.load();
    out.wxBreakpoints = state_->wxBreakpoints.load(); out.wxSteps = state_->wxSteps.load();
    return out;
}

std::vector<Window> Tracer::Windows() const {
    std::vector<Window> windows(state_->window.load());
    for (const auto& row : Rows()) {
        if (row.firstWindow && row.firstWindow <= windows.size())
            ++windows[row.firstWindow - 1].newFunctions;
    }
    uint32_t total = 0;
    for (size_t i = 0; i < windows.size(); ++i) {
        windows[i].number = static_cast<uint32_t>(i + 1);
        total += windows[i].newFunctions;
        windows[i].total = total;
    }
    return windows;
}

bool Tracer::Write(const std::string& path) const {
    std::ofstream out(path, std::ios::binary);
    if (!out) return false;
    out << "# entries are capped lower bounds; concurrent entries during restore/step and suspended probe intervals are omitted\n";
    for (const auto& r : Rows()) if (r.skipped.compare(0, 10, "not-entry:") == 0)
        out << "# never_arm " << std::hex << std::setfill('0') << std::setw(8) << r.address
            << std::dec << ' ' << r.skipped << '\n';
    out << "address\tentries\twindows_hit\tfirst_window\tlast_window\tcapped\tskipped\n";
    for (const auto& r : Rows()) {
        out << std::hex << std::setfill('0') << std::setw(8) << r.address << std::dec << '\t'
            << r.entries << '\t' << r.windowsHit << '\t' << r.firstWindow << '\t' << r.lastWindow
            << '\t' << (r.capped ? 1 : 0) << '\t' << r.skipped << '\n';
    }
    out.flush();
    return out.good();
}

bool Tracer::Owns(uintptr_t address) const {
    auto* e = state_->Find(address);
    return e && e->everArmed && state_->active.load(std::memory_order_relaxed);
}
void* Tracer::IgnoreThread() {
    if (!state_->active.load(std::memory_order_relaxed)) return nullptr;
    auto* t = state_->Slot(true);
    if (t) ++t->ignored;
    return t;
}
void Tracer::UnignoreThread(void* token) {
    if (token) --static_cast<State::Thread*>(token)->ignored;
}

}  // namespace reach

#ifndef FC4_REACH_TEST
namespace {
// Process lifetime: retain the VEH and thread slots until process exit, even
// after Stop, to service exceptions already dispatched on another thread.
reach::Tracer* g_reach = nullptr;
std::string g_reachOutput;
int32_t g_reachMonth = INT32_MIN;
std::vector<int32_t> g_reachDates;
bool g_reachStarted = false, g_reachStopped = false;
bool g_reachIncludeProbes = false;
uint32_t g_reachProbePauses = 0;
}

void ReachPrepare() {
    if (g_reach || SettingInt("reach", nullptr, 0) != 1) return;
    std::string path = Setting("reach_contracts", nullptr);
    if (path.empty()) path = FC4_REACH_CONTRACTS_FILE;
    std::ifstream input(path);
    std::vector<reach::Contract> contracts;
    std::string error;
    if (!reach::ReadContracts(input, contracts, error)) {
        Log("reach unavailable contracts=%s reason=%s", path.c_str(), error.c_str());
        return;
    }
    auto* base = reinterpret_cast<uint8_t*>(GetModuleHandleW(nullptr));
    const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(base);
    if (!base || dos->e_magic != IMAGE_DOS_SIGNATURE) return;
    const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS*>(base + dos->e_lfanew);
    if (nt->Signature != IMAGE_NT_SIGNATURE || nt->FileHeader.Machine != IMAGE_FILE_MACHINE_I386) return;
    auto* tracer = new reach::Tracer;
    if (!tracer->Prepare(contracts, nt->OptionalHeader.ImageBase,
                         reinterpret_cast<uintptr_t>(base), nt->OptionalHeader.SizeOfImage)) {
        delete tracer;
        return;
    }
    g_reach = tracer;
    const std::string diagnostics = Setting("reach_diagnostics", nullptr);
    if (!diagnostics.empty() && !tracer->Diagnostics(diagnostics))
        Log("reach diagnostics unavailable path=%s", diagnostics.c_str());
    g_reachOutput = Setting("reach_output", nullptr);
    if (g_reachOutput.empty()) g_reachOutput = std::string(LogPath()) + ".reach.tsv";
    Log("reach prepared functions=%u contracts=%s (before entry hooks)",
        static_cast<unsigned>(contracts.size()), path.c_str());
}

void ReachStart(int32_t julian) {
    if (!g_reach || g_reachStarted || g_reachStopped) return;
    const std::string window = Setting("reach_window", nullptr);
    if (!window.empty() && window != "month") {
        Log("reach unavailable reason=unsupported-window value=%s (use month)", window.c_str());
        return;
    }
    const long cap = SettingInt("reach_cap", nullptr, 32);
    if (cap < 1 || cap > 1000000 || !g_reach->Start(static_cast<uint32_t>(cap))) {
        Log("reach unavailable reason=invalid-cap-or-install-failed cap=%ld", cap);
        return;
    }
    g_reachStarted = true;
    g_reachMonth = julian;
    g_reachDates.push_back(julian);
    g_reachIncludeProbes = SettingInt("reach_include_probes", nullptr, 0) == 1;
    // The VEH must remain executable for in-flight exceptions through exit.
    HMODULE pinned = nullptr;
    GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN,
                      reinterpret_cast<LPCSTR>(&ReachStart), &pinned);
    const auto s = g_reach->GetSummary();
    Log("reach installed functions=%u armed=%u skipped=%u cap=%ld window=month include_probes=%u output=%s",
        s.functions, s.armed, s.skipped, cap, g_reachIncludeProbes ? 1 : 0, g_reachOutput.c_str());
    for (const auto& r : g_reach->Rows()) if (!r.skipped.empty())
        Log("reach skip address=%08x reason=%s", r.address, r.skipped.c_str());
}

void ReachNewMonth(int32_t julian) {
    if (!g_reachStarted || g_reachStopped || julian == g_reachMonth) return;
    g_reachMonth = julian;
    g_reach->NextWindow();
    g_reachDates.push_back(julian);
}

bool ReachLifecycleEnabled() {
    return SettingInt("reach_lifecycle", nullptr, 0) == 1;
}

void ReachLifecycleStart() {
    if (!ReachLifecycleEnabled()) return;
    // No city/simulator exists yet. Give the load window an explicit sentinel;
    // PostCityInit opens the first window bearing the actual saved city date.
    ReachStart(harness::JulianFromCivil(2000, 1, 1));
    if (g_reachStarted)
        Log("reach lifecycle phase=load window=1 date=2000-01-01 sentinel=1");
}

void ReachCheckpoint() {
    if (!g_reachStarted || g_reachStopped) return;
    Log("reach checkpoint phase=post-city-shutdown saved=%u output=%s",
        g_reach->Write(g_reachOutput) ? 1u : 0u, g_reachOutput.c_str());
}

void ReachStop() {
    if (!g_reachStarted || g_reachStopped) return;
    g_reachStopped = true;
    g_reach->Stop();
    const bool saved = g_reach->Write(g_reachOutput);
    const auto s = g_reach->GetSummary();
    for (const auto& window : g_reach->Windows()) {
        const auto date = harness::CivilFromJulian(g_reachDates[window.number - 1]);
        Log("reach window %u date=%04d-%02d-%02d new=%u total=%u", window.number,
            date.year, date.month, date.day, window.newFunctions, window.total);
    }
    for (const auto& r : g_reach->Rows())
        if (r.skipped == "late-entry-conflict" || r.skipped == "thread-slot-exhaustion")
            Log("reach skip address=%08x reason=%s", r.address, r.skipped.c_str());
    Log("reach summary functions=%u armed=%u reached=%u skipped=%u windows=%u traps=%u ignored=%u aborted=%u slot_exhaustions=%u saved=%u probe_pauses=%u wx_breakpoints=%u wx_steps=%u",
        s.functions, s.armed, s.reached, s.skipped, s.windows, s.traps, s.ignored,
        s.abortedSteps, s.slotExhaustions, saved ? 1 : 0, g_reachProbePauses, s.wxBreakpoints, s.wxSteps);
}

ReachProbeScope::ReachProbeScope() : token_(nullptr) {
    if (g_reachStarted && !g_reachStopped && !g_reachIncludeProbes && g_reach->Suspend()) {
        token_ = g_reach;
        ++g_reachProbePauses;
    }
}
ReachProbeScope::~ReachProbeScope() {
    if (token_ && !g_reachStopped) static_cast<reach::Tracer*>(token_)->Resume();
}
#endif
}  // namespace fc4
