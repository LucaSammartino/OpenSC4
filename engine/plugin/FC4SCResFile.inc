// SPDX-License-Identifier: GPL-3.0-or-later
// SCR10: synchronous copies of borrowed physical lines, never owned COM refs.
constexpr uintptr_t kTextConstructor = 0x009307B5, kTextOpen = 0x0093066E;
constexpr uintptr_t kTextRead = 0x00930833;
struct FileEvent {
    uint32_t kind;
    ParserBytes path, line;
    uint32_t physical, value;
    int32_t position;
};
static_assert(sizeof(FileEvent) == 32 && offsetof(FileEvent, physical) == 20,
              "Rust repr(C) physical-file observation alignment");
extern "C" uint32_t fc4_scres_file_compare(const FileEvent*, size_t);
struct FileCopy {
    uint32_t kind;
    std::string path, line;
    uint32_t physical, value;
    int32_t position;
};
struct FileFrame {
    FileFrame* parent;
    std::string path;
    void* file = nullptr;
    bool constructed = false, opened = false, openSeen = false, terminal = false;
    uint32_t physical = 0;
    bool pending = false;
    std::string raw;
};
thread_local FileFrame* g_fileFrame = nullptr;
thread_local bool g_fileForwardOpen = false, g_fileForwardRead = false;
std::vector<FileCopy> g_fileCopies;
std::atomic<uint64_t> g_fileRoots{0}, g_fileReplays{0}, g_fileBad{0}, g_fileRefused{0};
std::atomic<uint64_t> g_fileReads{0}, g_fileFalse{0}, g_fileBlanks{0}, g_fileOpens{0}, g_fileOpenFailed{0};
std::atomic<uint64_t> g_fileCommands{0}, g_fileIncludes{0}, g_filePhysical{0}, g_fileTotals{0};
std::atomic<uint64_t> g_fileUndispatched{0};

void FileUndispatched() {
    auto* f = g_fileFrame;
    if (!f || !f->pending) return;
    f->pending = false;
    const uint64_t n = ++g_fileUndispatched;
    if (n <= 8) fc4::Log("scres: file input without command/include physical=%u bytes=%zu raw=%.240s",
                         f->physical, f->raw.size(), f->raw.c_str());
}

void FileRecord(uint32_t kind, const std::string& line = {}, uint32_t value = 0, int32_t position = 0) {
    auto* f = g_fileFrame;
    if (!g_fileReplay || !f) return;
    if (g_fileCopies.size() >= 100000) { ++g_fileRefused; return; }
    g_fileCopies.push_back({kind, f->path, line, f->physical, value, position});
}
bool FileCopyString(cIGZString* s, std::string& copy) {
    if (!s) return false;
    const auto n = s->Strlen();
    if (n > 1024 * 1024) return false;
    copy.assign(s->Data(), n);
    return true;
}
int32_t FilePosition(void* file) {
    return static_cast<cIGZFile*>(file)->Position(); // SCR10 slot 6, read-only.
}
bool __fastcall FileOpen(void* self, void*, uint32_t access, uint32_t creation, uint32_t sharing) {
    g_fileForwardOpen = true;
    const bool ok = reinterpret_cast<bool (__thiscall*)(void*, uint32_t, uint32_t, uint32_t)>(kTextOpen)(self, access, creation, sharing);
    auto* f = g_fileFrame;
    if (!f || f->file != self || !f->constructed) { ++g_fileRefused; return ok; }
    if (f->openSeen || access != 1 || creation != 2 || sharing != 7) ++g_fileRefused;
    f->openSeen = true;
    f->opened = ok;
    ++g_fileOpens;
    if (!ok) ++g_fileOpenFailed;
    const int32_t position = ok ? FilePosition(self) : 0;
    // SCR10 mode-2 output may draw on uninitialized native temporary storage.
    // Never claim deterministic replay of those samples.
    if (ok && position != 0) ++g_fileRefused;
    FileRecord(0, {}, ok ? 1u : 0u, position);
    return ok;
}
bool __fastcall FileRead(void* self, void*, cIGZString* output) {
    if (g_fileFrame && g_fileFrame->file == self) FileUndispatched();
    g_fileForwardRead = true;
    const bool ok = reinterpret_cast<bool (__thiscall*)(void*, cIGZString*)>(kTextRead)(self, output);
    auto* f = g_fileFrame;
    if (!f || f->file != self || !f->opened) { ++g_fileRefused; return ok; }
    std::string line;
    if (!FileCopyString(output, line)) ++g_fileRefused;
    const int32_t position = FilePosition(self);
    if (f->terminal) ++g_fileRefused;
    if (ok) {
        ++f->physical; ++g_fileReads; ++g_filePhysical;
        if (line.empty()) ++g_fileBlanks;
    } else { f->terminal = true; ++g_fileFalse; }
    FileRecord(ok ? 1u : 2u, line, 0, position);
    f->pending = ok;
    f->raw = line;
    return ok; // AL is the only native bool result.
}
bool FileProbe(CONTEXT& c) {
    if (!g_fileReplay || !g_fileFrame) return false;
    auto* f = g_fileFrame;
    if (c.Eip == kTextConstructor) {
        if (f->constructed) ++g_fileRefused;
        const auto* path = *reinterpret_cast<const char**>(c.Esp + 4);
        std::string copy;
        if (!ParserCopyString(path, copy) || copy != f->path) ++g_fileRefused;
        f->file = reinterpret_cast<void*>(c.Ecx);
        f->constructed = true;
        c.Dr2 = kTextOpen;
    } else if (c.Eip == kTextOpen) {
        if (g_fileForwardOpen) { g_fileForwardOpen = false; c.Dr2 = kTextRead; }
        else c.Eip = reinterpret_cast<DWORD>(&FileOpen);
    } else if (c.Eip == kTextRead) {
        if (g_fileForwardRead) g_fileForwardRead = false;
        else c.Eip = reinterpret_cast<DWORD>(&FileRead);
    } else return false;
    return true;
}
void FileCommand(const char* line) {
    if (!g_fileReplay || !g_fileFrame) return;
    std::string copy;
    if (!ParserCopyString(line, copy)) ++g_fileRefused;
    g_fileFrame->pending = false;
    FileRecord(3, copy);
    ++g_fileCommands;
}
void FileFinish(bool success, uint32_t count) {
    auto* f = g_fileFrame;
    FileUndispatched();
    if (!f->openSeen || !f->constructed || (success && !f->terminal)) ++g_fileRefused;
    FileRecord(success ? 4u : 5u, {}, count);
    if (success) g_fileTotals += count;
    if (f->parent) ++g_fileIncludes;
    if (f->parent) return;
    ++g_fileRoots;
    std::vector<FileEvent> events;
    events.reserve(g_fileCopies.size());
    for (auto& e : g_fileCopies) events.push_back({e.kind,
        {reinterpret_cast<const uint8_t*>(e.path.data()), e.path.size()},
        {reinterpret_cast<const uint8_t*>(e.line.data()), e.line.size()},
        e.physical, e.value, e.position});
    const uint32_t bad = fc4_scres_file_compare(events.data(), events.size());
    g_fileBad += bad;
    ++g_fileReplays;
    fc4::Log("scres: physical file replay path=%.240s events=%zu physical=%u native_total=%u returned=%u differences=%u",
             f->path.c_str(), events.size(), f->physical, count, success ? 1u : 0u, bad);
    g_fileCopies.clear();
}
void FileReport(const fc4_date& date, const char* event) {
    if (!g_fileReplay) return;
    const uint64_t mismatch = g_fileBad.load() + g_fileRefused.load() + (g_fileReads.load() ? 0u : 1u);
    fc4::Log("scres shadow %04d-%02d-%02d %s: file_parser_replay roots=%" PRIu64
             " replays=%" PRIu64 " reads=%" PRIu64 " blank=%" PRIu64 " false_eof_or_failure=%" PRIu64
             " opens=%" PRIu64 " open_failed=%" PRIu64 " commands=%" PRIu64 " includes=%" PRIu64
             " physical=%" PRIu64 " inclusive_totals=%" PRIu64 " undispatched=%" PRIu64
             " refused=%" PRIu64 " resyncs=0; mismatches=%" PRIu64,
             date.year, date.month, date.day_of_month, event, g_fileRoots.load(), g_fileReplays.load(),
             g_fileReads.load(), g_fileBlanks.load(), g_fileFalse.load(), g_fileOpens.load(), g_fileOpenFailed.load(),
             g_fileCommands.load(), g_fileIncludes.load(), g_filePhysical.load(), g_fileTotals.load(),
             g_fileUndispatched.load(), g_fileRefused.load(), mismatch);
    ++g_lines;
    if (!mismatch) ++g_clean;
}
