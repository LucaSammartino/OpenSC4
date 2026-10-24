// SPDX-License-Identifier: GPL-3.0-or-later
// sc-resources 10.1/10.2: natural startup rule-file -> shared command parser.
// Hardware execution breakpoints avoid copying/decoding executable bytes or
// guessing an instruction span. Only the installing (startup/game) thread is
// observed. Refuse if its debug registers are occupied; restore them on stop.
// No interface is acquired, created or released by this observer.
struct ParserBytes { const uint8_t* bytes; size_t length; };
extern "C" uint32_t fc4_scres_parser_compare(ParserBytes, const ParserBytes*, size_t);
constexpr uintptr_t kParserSites[] = {0x00679450, 0x00679120, 0x00681DA0, 0x00680EA0};
std::atomic<uint64_t> g_parserCalls{0}, g_parserBad{0}, g_parserRefused{0};
std::atomic<uint64_t> g_parserLines{0}, g_parserExpanded{0}, g_parserSkipped{0};
std::atomic<uint64_t> g_parserFileCalls{0}, g_parserOtherCalls{0};
std::atomic<uint64_t> g_parserPaths{0}, g_parserFiles{0}, g_parserThrows{0}, g_parserFileLines{0};
DWORD g_parserThread = 0;
PVOID g_parserHandler = nullptr;
CONTEXT g_parserSaved{};
bool g_parserArmed = false;
struct ParserFrame {
    ParserFrame* parent;
    void* state;
    std::string line;
    bool dispatched = false;
};
thread_local ParserFrame* g_parserFrame = nullptr;
thread_local bool g_parserForwardLine = false, g_parserForwardFile = false;
thread_local unsigned g_parserFileDepth = 0;

bool ParserCopyString(const char* value, std::string& copy) {
    if (!value) return false;
    const size_t length = strnlen(value, 1024 * 1024);
    if (length == 1024 * 1024) return false;
    copy.assign(value, length);
    return true;
}

#include "FC4SCResFile.inc"
bool ParserArm(bool install, uintptr_t probe = 0);

void ParserArguments(void* state, const void* vector) {
    auto* frame = g_parserFrame;
    if (!frame || frame->state != state || frame->dispatched) {
        ++g_parserExpanded; // Expanded/standalone input is not a physical line.
        return;
    }
    frame->dispatched = true;
    const uintptr_t begin = Word(vector, 0), end = Word(vector, 4), capacity = Word(vector, 8);
    if (end < begin || end > capacity || (end - begin) % 20 || (end - begin) / 20 > 4096) {
        ++g_parserRefused;
        return;
    }
    const size_t count = (end - begin) / 20;
    std::vector<std::string> copies(count);
    std::vector<ParserBytes> tokens(count);
    for (size_t i = 0; i < count; ++i) {
        // S is a 20-byte cIGZString, not this compiler's std::string.
        auto* s = reinterpret_cast<const cIGZString*>(begin + 20 * i);
        if (!ParserCopyString(s->ToChar(), copies[i])) { ++g_parserRefused; return; }
        tokens[i] = {reinterpret_cast<const uint8_t*>(copies[i].data()), copies[i].size()};
    }
    const ParserBytes line{reinterpret_cast<const uint8_t*>(frame->line.data()), frame->line.size()};
    const uint32_t difference = fc4_scres_parser_compare(line, tokens.data(), count);
    ++g_parserCalls;
    if (g_parserFileDepth) ++g_parserFileCalls;
    else ++g_parserOtherCalls;
    if (difference) {
        const uint64_t n = ++g_parserBad;
        if (n <= 8) fc4::Log("scres: native command token difference=%u tokens=%zu line=%.160s",
                             difference, count, frame->line.c_str());
    }
}

void __fastcall ParserLine(void* input, void*, const char* line) {
    FileCommand(line);
    ParserFrame frame{g_parserFrame, static_cast<uint8_t*>(input) - 0x98, {}};
    if (!ParserCopyString(line, frame.line)) ++g_parserRefused;
    g_parserFrame = &frame;
    struct Exit {
        ParserFrame& frame;
        ~Exit() {
            if (!frame.dispatched) ++g_parserSkipped; // blank/comment/macro-recording line
            g_parserFrame = frame.parent;
        }
    } exit{frame};
    ++g_parserLines;
    g_parserForwardLine = true;
    try {
        reinterpret_cast<void (__thiscall*)(void*, const char*)>(kParserSites[0])(input, line);
    } catch (...) {
        ++g_parserThrows;
        throw; // Native exceptions and cleanup remain the caller's responsibility.
    }
}

int32_t __fastcall ParserFile(void* self, void*, const char* path, const char* context) {
    ++g_parserFiles;
    ++g_parserFileDepth;
    FileFrame file{g_fileFrame, {}};
    if (g_fileReplay) {
        if (!ParserCopyString(path, file.path)) ++g_fileRefused;
        g_fileFrame = &file;
    }
    struct Exit {
        FileFrame& file;
        ~Exit() {
            --g_parserFileDepth;
            if (g_fileReplay) {
                g_fileFrame = file.parent; // Association ends on normal return and unwind.
                if (!ParserArm(true, file.parent ? kTextRead : kParserSites[2])) ++g_fileRefused;
            }
        }
    } exit{file};
    fc4::Log("scres: native resource-file depth=%u path=%.240s context=%.120s",
             g_parserFileDepth, path ? path : "<null>", context ? context : "<null>");
    g_parserForwardFile = true;
    try {
        const int32_t lines = reinterpret_cast<int32_t (__thiscall*)(void*, const char*, const char*)>(
            kParserSites[3])(self, path, context);
        // Includes contribute to their parent's total; retain that native total,
        // not a claim of unique physical lines.
        if (lines > 0) g_parserFileLines += static_cast<uint32_t>(lines);
        if (g_fileReplay) FileFinish(true, static_cast<uint32_t>(lines));
        return lines;
    } catch (...) {
        if (g_fileReplay) FileFinish(false, 0);
        ++g_parserThrows; throw;
    }
}

LONG CALLBACK ParserException(EXCEPTION_POINTERS* exception) {
    if (exception->ExceptionRecord->ExceptionCode != EXCEPTION_SINGLE_STEP ||
        GetCurrentThreadId() != g_parserThread) return EXCEPTION_CONTINUE_SEARCH;
    auto& c = *exception->ContextRecord;
    const uintptr_t site = c.Eip;
    unsigned index = 0;
    while (index != 4 && (index == 2 ? c.Dr2 : kParserSites[index]) != site) ++index;
    if (index == 4 || !(c.Dr6 & (1u << index))) return EXCEPTION_CONTINUE_SEARCH;
    c.Dr6 &= ~(1u << index);
    // RF permits exactly the original entry instruction to run once. It is
    // cleared by the CPU, so nested/direct calls remain observable.
    c.EFlags |= 0x10000;
    if (index == 0) {
        if (g_parserForwardLine) g_parserForwardLine = false;
        else c.Eip = reinterpret_cast<DWORD>(&ParserLine);
    } else if (index == 1) {
        ParserArguments(reinterpret_cast<void*>(c.Ecx), *reinterpret_cast<void**>(c.Esp + 4));
    } else if (index == 2) {
        if (FileProbe(c)) return EXCEPTION_CONTINUE_EXECUTION;
        const auto* stack = reinterpret_cast<const uintptr_t*>(c.Esp);
        ++g_parserPaths;
        fc4::Log("scres: native registered resource-file flags=%u path=%.240s",
                 static_cast<unsigned>(stack[2]), reinterpret_cast<const char*>(stack[1]));
    } else {
        if (g_parserForwardFile) g_parserForwardFile = false;
        else {
            if (g_fileReplay) c.Dr2 = kTextConstructor;
            c.Eip = reinterpret_cast<DWORD>(&ParserFile);
        }
    }
    return EXCEPTION_CONTINUE_EXECUTION;
}

struct ParserArmRequest { HANDLE thread; bool install; bool ok; uintptr_t probe; };
DWORD WINAPI ParserArmThread(void* value) {
    auto& request = *static_cast<ParserArmRequest*>(value);
    if (SuspendThread(request.thread) == static_cast<DWORD>(-1)) return 0;
    CONTEXT c{};
    c.ContextFlags = CONTEXT_DEBUG_REGISTERS;
    if (GetThreadContext(request.thread, &c)) {
        if (request.probe) {
            c.Dr2 = request.probe;
            request.ok = SetThreadContext(request.thread, &c) != 0;
        } else if (request.install) {
            if (!(c.Dr7 & 0xFF)) {
                g_parserSaved = c;
                c.Dr0 = kParserSites[0]; c.Dr1 = kParserSites[1];
                c.Dr2 = kParserSites[2]; c.Dr3 = kParserSites[3];
                c.Dr6 = 0; c.Dr7 = 0x55; // four local execution breakpoints
                request.ok = SetThreadContext(request.thread, &c) != 0;
            }
        } else {
            request.ok = SetThreadContext(request.thread, &g_parserSaved) != 0;
        }
    }
    ResumeThread(request.thread);
    return 0;
}
bool ParserArm(bool install, uintptr_t probe) {
    ParserArmRequest request{OpenThread(THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT | THREAD_SET_CONTEXT,
                                       FALSE, g_parserThread), install, false, probe};
    if (!request.thread) return false;
    HANDLE helper = CreateThread(nullptr, 0, ParserArmThread, &request, 0, nullptr);
    if (helper) { WaitForSingleObject(helper, INFINITE); CloseHandle(helper); }
    CloseHandle(request.thread);
    return request.ok;
}
void ParserStart() {
    g_parserThread = GetCurrentThreadId();
    g_parserHandler = AddVectoredExceptionHandler(1, ParserException);
    if (g_parserHandler) g_parserArmed = ParserArm(true);
    if (!g_parserArmed) {
        if (g_parserHandler) RemoveVectoredExceptionHandler(g_parserHandler);
        g_parserHandler = nullptr;
        ++g_parserRefused;
    }
    fc4::Log("scres: natural file/command observation %s thread=%u scope=startup_thread partial=command_tokens",
             g_parserArmed ? "ON" : "REFUSED", g_parserThread);
}
void ParserStop() {
    if (g_parserArmed && !ParserArm(false)) {
        fc4::Log("scres: refused restoring parser debug registers; retaining handler");
        return;
    }
    g_parserArmed = false;
    if (g_parserHandler) RemoveVectoredExceptionHandler(g_parserHandler);
    g_parserHandler = nullptr;
}
void ParserReport(const fc4_date& date, const char* event) {
    FileReport(date, event);
    const uint64_t mismatch = g_parserBad.load() + g_parserRefused.load() +
        (g_parserCalls.load() == 0 ? 1u : 0u);
    fc4::Log("scres shadow %04d-%02d-%02d %s: partial=native_command_tokens calls=%" PRIu64
             " file_calls=%" PRIu64 " other_calls=%" PRIu64
             " lines=%" PRIu64 " expanded=%" PRIu64 " nondispatch=%" PRIu64
             " paths=%" PRIu64 " files=%" PRIu64 " file_line_totals=%" PRIu64
             " exceptions=%" PRIu64 " refused=%" PRIu64 "; mismatches=%" PRIu64,
             date.year, date.month, date.day_of_month, event, g_parserCalls.load(),
             g_parserFileCalls.load(), g_parserOtherCalls.load(), g_parserLines.load(),
             g_parserExpanded.load(), g_parserSkipped.load(), g_parserPaths.load(), g_parserFiles.load(),
             g_parserFileLines.load(), g_parserThrows.load(), g_parserRefused.load(), mismatch);
    ++g_lines;
    if (!mismatch) ++g_clean;
}
