// SPDX-License-Identifier: GPL-3.0-or-later
// html-document.md sections 1.4, 1.6, 2 and 8.1. Native entry shells preserve
// registers/FPU, including optimized private callers. No gameplay writes.
#include "FC4HTMLDocumentShadow.h"
#include "FC4Inplace.h"
#include "FC4Log.h"
#include "cIGZString.h"
#include "fc4.h"
#include <algorithm>
#include <array>
#include <atomic>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

namespace fc4 {
namespace {
struct Method {
    uint32_t address, bytes;
    const char* name;
    std::atomic<uint32_t> calls{0}, complete{0}, mismatches{0}, unobserved{0};
};
#define HTML_METHOD(address, bytes, name) {address, bytes, name},
Method methods[] = {
#include "FC4HTMLDocumentMethods.inc"
};
#undef HTML_METHOD
std::atomic<bool> seen{false};

#if defined(_M_IX86)
struct Bytes { const uint8_t* data; uint32_t size; };
struct TextState { Bytes source, pending, tag, attributes; uint32_t position; };
using Character = uint32_t(__cdecl*)(uint32_t, const uint8_t*, uint32_t, uint32_t);
extern "C" uint32_t fc4_html_document_text_check(uint32_t, const TextState*, const TextState*, uint32_t, Character);
extern "C" uint32_t fc4_html_document_scalar_check(uint32_t, const int32_t*, const int32_t*, const int32_t*, uint32_t, const int32_t*, uint32_t);
struct SetState { const uint32_t* members; uint32_t count; };
extern "C" uint32_t fc4_html_document_set_check(uint32_t, uint32_t, const SetState*, const SetState*, const SetState*, const SetState*, uint32_t);

// STLport set<long>: docs/abi/allocators.md, "map and set". Copy all
// semantic members, never pass a native container or node into Rust. Canonical
// unsigned sorting compares bit patterns independently of native tree shape.
bool SnapshotSet(uint32_t address, std::vector<uint32_t>& members) {
    const auto* set = reinterpret_cast<const uint32_t*>(address);
    const auto* header = reinterpret_cast<const uint32_t*>(set[0]);
    if (!header) return false;
    std::vector<uint32_t> pending;
    if (header[1]) pending.push_back(header[1]);
    while (!pending.empty()) {
        const auto nodeAddress = pending.back();
        pending.pop_back();
        if (nodeAddress == set[0] || members.size() >= set[1]) return false;
        const auto* node = reinterpret_cast<const uint32_t*>(nodeAddress);
        members.push_back(node[4]);
        if (node[2]) pending.push_back(node[2]);
        if (node[3]) pending.push_back(node[3]);
    }
    std::sort(members.begin(), members.end());
    return members.size() == set[1] && std::adjacent_find(members.begin(), members.end()) == members.end();
}
SetState SetView(const std::vector<uint32_t>& members) {
    return {members.data(), static_cast<uint32_t>(members.size())};
}

struct Text {
    std::array<std::string, 4> values;
    uint32_t position = 0;
    TextState View() const {
        auto bytes = [&](unsigned i) { return Bytes{reinterpret_cast<const uint8_t*>(values[i].data()), static_cast<uint32_t>(values[i].size())}; };
        return {bytes(0), bytes(1), bytes(2), bytes(3), position};
    }
};
std::string String(const void* p) {
    auto* s = static_cast<const cIGZString*>(p);
    return std::string(s->Data(), s->Strlen());
}
Text Snapshot(uint32_t address) {
    auto* p = reinterpret_cast<const uint8_t*>(address);
    Text text;
    text.values = {String(p + 0x180), String(p + 0x100), String(p + 0xD4), String(p + 0xEC)};
    std::memcpy(&text.position, p + 0x194, 4);
    return text;
}
std::array<int32_t, 10> Scalars(uint32_t address) {
    auto* p = reinterpret_cast<const uint8_t*>(address);
    std::array<int32_t, 10> words{};
    std::memcpy(words.data(), p + 0x1A8, 28);
    std::memcpy(&words[7], p + 0x54, 4);
    std::memcpy(&words[8], p + 0x6C, 4);
    words[9] = p[0x29];
    return words;
}
int Operation(uint32_t address) {
    switch (address) {
        // SPEC-GAP: character-helper argument ABI is unpublished; HD9 blocks CDATA/whitespace/split completion.
        case 0x008FE8D6: return 4;
        case 0x008FE92B: return 5;
        case 0x009006B4: return 6; // source string reference
        case 0x008FEEB8: return 7; // table setter
        case 0x008FEE66: return 8; // table getter
        case 0x009006C0: return 9; // slot-15 byte
        case 0x008FEE4A: return 10; // grammar-set membership
        case 0x008FF187: return 11; // set union
        default: return -1;
    }
}
struct Frame {
    Method* method;
    int operation;
    Text before;
    std::array<int32_t, 10> scalars{};
    std::vector<int32_t> input;
    std::vector<uint32_t> set, sourceSet;
    bool setsObserved = false;
};
thread_local std::vector<Frame> frames;

void Observe(const char* name, uint32_t self, const uint32_t* args, uint32_t value, bool begin) {
    const auto address = static_cast<uint32_t>(std::strtoul(name + 5, nullptr, 16));
    auto* method = std::find_if(std::begin(methods), std::end(methods), [&](const Method& m) { return m.address == address; });
    if (method == std::end(methods)) return;
    const int op = Operation(address);
    if (begin) {
        seen = true;
        ++method->calls;
        // SPEC-GAP: complete parser observation needs element, attribute-map and foreign-call traces.
        // Entry counts alone never earn complete credit for those methods.
        if (op < 0) return;
        Frame frame{method, op};
        if (op >= 10) {
            frame.setsObserved = SnapshotSet(op == 10 ? args[1] : args[0], frame.set);
            if (op == 11) frame.setsObserved = SnapshotSet(args[1], frame.sourceSet) && frame.setsObserved;
        } else if (op == 0 || op == 1 || op == 6) frame.before = Snapshot(self);
        else if (op <= 5) {
            frame.before.values[2] = String(reinterpret_cast<const void*>(args[0]));
            if (op == 3) frame.before.values[3] = String(reinterpret_cast<const void*>(args[1]));
        } else {
            frame.scalars = Scalars(self);
            if (op == 7 && args[1]) {
                auto* sizes = reinterpret_cast<const int32_t*>(args[0]);
                frame.input.assign(sizes, sizes + std::min(args[1], 7u));
            }
        }
        frames.push_back(std::move(frame));
        return;
    }
    if (op < 0) return;
    if (frames.empty() || frames.back().method != method) {
        ++method->mismatches;
        Log("html-document missing entry snapshot address=%08X", address);
        return;
    }
    Frame frame = std::move(frames.back());
    frames.pop_back();
    uint32_t bad = 0;
    if (op >= 10) {
        std::vector<uint32_t> after, sourceAfter;
        bool observed = SnapshotSet(op == 10 ? args[1] : args[0], after);
        if (op == 11) observed = SnapshotSet(args[1], sourceAfter) && observed;
        if (!frame.setsObserved || !observed) {
            // An incomplete snapshot never becomes a comparison or a credit.
            if (++method->unobserved <= 8) Log("html-document missing complete set observation address=%08X", address);
            return;
        }
        const auto b = SetView(frame.set), a = SetView(after);
        const auto sb = SetView(frame.sourceSet), sa = SetView(sourceAfter);
        // functions.tsv specifies AL for IsInSet; the entry shell preserves the
        // full EAX, whose high bytes are not part of this method's result.
        bad = fc4_html_document_set_check(op - 10, args[0], &b, &a, &sb, &sa, value & 255u);
    } else if (op <= 6) {
        Text after;
        if (op == 0 || op == 1 || op == 6) after = Snapshot(self);
        else {
            after.values[2] = String(reinterpret_cast<const void*>(args[0]));
            if (op == 3) after.values[3] = String(reinterpret_cast<const void*>(args[1]));
        }
        const auto b = frame.before.View(), a = after.View();
        bad = fc4_html_document_text_check(op, &b, &a, value, nullptr);
        if (op == 6 && value != self + 0x180) bad |= 32;
    } else {
        const auto after = Scalars(self);
        const auto* out = op == 8 ? reinterpret_cast<const int32_t*>(args[0]) : nullptr;
        const auto count = op == 9 ? args[0] & 255u : args[1];
        const auto scalarOp = op == 7 ? 0u : op == 8 ? 1u : 2u;
        bad = fc4_html_document_scalar_check(scalarOp, frame.scalars.data(), after.data(), frame.input.data(), count, out, value);
    }
    if (bad) {
        const auto n = ++method->mismatches;
        if (n <= 8) Log("html-document difference address=%08X method=%s mask=%08X", address, method->name, bad);
    }
    ++method->complete;
}
#endif
}

void HTMLDocumentShadowStartup() {
#if defined(_M_IX86)
    // Selection remains the stable compare setting: class:cHTMLDocument.
    // Registration alone installs nothing and never creates a native document.
    CompareObserveNative("cHTMLDocument", &Observe);
#endif
}
void HTMLDocumentShadowMonth(int32_t julian) {
    if (!seen.load()) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    struct Count { uint32_t calls, complete, mismatches; };
    std::array<Count, std::size(methods)> counts{};
    uint32_t calls = 0, complete = 0, bad = 0;
    for (unsigned i = 0; i < std::size(methods); ++i) {
        const auto& m = methods[i];
        // Errors are published before completion. Reading completion first and
        // entries last cannot report an exact completed error or complete>calls.
        const auto done = m.complete.load();
        const auto errors = m.mismatches.load();
        const auto entered = m.calls.load();
        counts[i] = {entered, done, errors};
        calls += entered; complete += done; bad += errors;
    }
    Log("html-document shadow (month) %04d-%02d-%02d: calls=%u complete=%u partial=%u mismatches=%u; resyncs=0",
        date.year, date.month, date.day_of_month, calls, complete, calls - complete, bad);
    for (unsigned i = 0; i < std::size(methods); ++i) {
        const auto& m = methods[i];
        const auto& count = counts[i];
        Log("html-document method 0x%08X cHTMLDocument::%s (month) %04d-%02d-%02d: calls=%u complete=%u mismatches=%u; resyncs=0",
            m.address, m.name, date.year, date.month, date.day_of_month,
            count.calls, count.complete, count.mismatches);
    }
}
}
