// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4OverlaysObservation.h"
#include "FC4Log.h"
#include "fc4.h"
#include "cISTEOverlayManager.h"
#include <Windows.h>
#include <array>
#include <algorithm>
#include <cstring>
#include <map>
#include <set>
#include <vector>

extern "C" uint32_t fc4_overlays_observe(const uint32_t*, uint32_t, uint32_t*);
extern "C" uint32_t fc4_overlays_replay_decal(const uint32_t*, uint32_t*);

namespace fc4 {
namespace {
constexpr uint32_t kLimit = 1048576;
constexpr uint32_t kNone = UINT32_MAX;
uint32_t g_lines = 0, g_clean = 0, g_incomplete = 0;
uint32_t g_replayLines = 0, g_replayClean = 0;
uint64_t g_replayDifferences = 0;
uint64_t g_checks = 0, g_differences = 0;
std::array<uint64_t, 9> g_counts{};

// Every read is bounded and copied, including pointers reached from list/map
// nodes. Never dereference foreign memory directly or retain it past this event.
bool Read(uint32_t address, void* out, size_t bytes) {
    if (!bytes) return true;
    if (!address || (address & 3) || uint64_t(address) + bytes > UINT32_MAX) return false;
    SIZE_T got = 0;
    return ReadProcessMemory(GetCurrentProcess(), reinterpret_cast<void*>(address), out, bytes, &got) && got == bytes;
}
template<size_t N> bool ReadWords(uint32_t address, std::array<uint32_t, N>& words) {
    return Read(address, words.data(), sizeof(words));
}
bool Span(uint32_t begin, uint32_t end, uint32_t capacity, uint32_t stride, uint32_t& count) {
    if (begin > end || end > capacity || (begin & 3) ||
        (end - begin) % stride || (capacity - begin) % stride) return false;
    count = (end - begin) / stride;
    return count <= kLimit && (!capacity || begin != 0);
}
struct Arena {
    std::array<uint32_t, 5> header{};
    uint32_t stride = 0;
    std::vector<uint32_t> data;
    std::vector<bool> live;
    const uint32_t* Record(uint32_t index) const { return data.data() + size_t(index) * stride; }
    bool Has(uint32_t id) const { return id < live.size() && live[id]; }
    bool Capture(uint32_t address, uint32_t words) {
        stride = words;
        if (!ReadWords(address, header)) return false;
        uint32_t count;
        if (!Span(header[0], header[1], header[2], words * 4, count)) return false;
        data.resize(size_t(count) * words);
        if (!Read(header[0], data.data(), data.size() * 4)) return false;
        live.resize(count);
        std::set<uint32_t> free;
        if (header[3] == kNone || header[4] == kNone) {
            if (header[3] != header[4]) return false;
        } else {
            uint32_t index = header[3];
            for (;;) {
                if (index >= count || !free.insert(index).second) return false;
                if (index == header[4]) {
                    if (Record(index)[0] != 0) return false;
                    break;
                }
                index = Record(index)[0];
            }
        }
        for (uint32_t i = 0; i < count; ++i) {
            live[i] = Record(i)[0] == kNone;
            if (live[i] == (free.count(i) != 0)) return false;
        }
        return true;
    }
    bool Unchanged(uint32_t address) const {
        std::array<uint32_t, 5> current{};
        if (!ReadWords(address, current) || current != header) return false;
        std::vector<uint32_t> copy(data.size());
        return Read(header[0], copy.data(), copy.size() * 4) && copy == data;
    }
};
struct Capture {
    uint32_t manager;
    const char* stage = "manager/arenas";
    std::array<uint32_t, 63> header{};
    Arena overlays, handles, groups;
    std::vector<uint32_t> wire, heads;
    std::map<uint32_t, uint32_t> tokens;
    std::map<uint32_t, std::array<uint32_t, 13>> bindingRecords;
    std::set<uint32_t> factories;
    std::map<uint32_t, std::array<uint32_t, 2>> nodes;
    uint32_t Token(uint32_t p) {
        if (!p) return 0;
        const auto found = tokens.find(p);
        if (found != tokens.end()) return found->second;
        const uint32_t id = static_cast<uint32_t>(tokens.size()) + 1;
        tokens.emplace(p, id);
        return id;
    }
    bool Factory(uint32_t address) {
        if (!address || factories.count(address)) return true;
        if (address > UINT32_MAX - 0x34) return false;
        factories.insert(address);
        std::array<uint32_t, 2> extent{};
        if (!ReadWords(address + 0x2c, extent)) return false;
        uint32_t count;
        if (!Span(extent[0], extent[1], extent[1], 4, count)) return false;
        std::vector<uint32_t> buckets(count);
        if (!Read(extent[0], buckets.data(), buckets.size() * 4)) return false;
        std::set<uint32_t> visited;
        for (uint32_t p : buckets) {
            while (p) {
                if (visited.size() >= kLimit || !visited.insert(p).second) return false;
                std::array<uint32_t, 3> node{};
                std::array<uint32_t, 13> record{};
                if (!ReadWords(p, node) || !ReadWords(node[2], record) || record[0] != node[1]) return false;
                if (!bindingRecords.emplace(node[1], record).second) return false;
                p = node[0];
            }
        }
        return true;
    }
    bool Cells() {
        const bool dynamic = (header[0x14 / 4] & 0xff00) != 0;
        if (dynamic) {
            // OV14: even retained storage after repeat Init is inactive. Do not
            // validate or dereference any inactive vector/scalar index words.
            wire.push_back(0);
            return true;
        }
        uint32_t count;
        if (!Span(header[0xf0 / 4], header[0xf4 / 4], header[0xf8 / 4], 4, count)) return false;
        const int32_t nx = header[0xe8 / 4], nz = header[0xec / 4];
        if (nx < 0 || nz < 0 || uint64_t(nx) * nz != count) return false;
        heads.resize(count);
        if (!Read(header[0xf0 / 4], heads.data(), heads.size() * 4)) return false;
        wire.push_back(count);
        uint32_t entries = 0;
        for (uint32_t p : heads) {
            const size_t length = wire.size();
            wire.push_back(0);
            std::set<uint32_t> visited;
            while (p) {
                if (++entries > kLimit || !visited.insert(p).second) return false;
                std::array<uint32_t, 2> node{};
                if (!ReadWords(p, node) || !overlays.Has(node[1])) return false;
                nodes.emplace(p, node);
                wire.push_back(node[1]);
                ++wire[length];
                p = node[0];
            }
        }
        return true;
    }
    bool Resources(std::vector<uint32_t>& normalized) {
        std::set<uint32_t> bindings, sources;
        for (size_t offset = 0; offset < normalized.size(); offset += 45) {
            // Byte +09 is padding. Stamp +0A is observed, never reset/collected.
            normalized[offset + 2] &= 0xffff00ff;
            for (uint32_t field = 43; field <= 44; ++field) {
                const uint32_t address = normalized[offset + field];
                if (address) (field == 43 ? bindings : sources).insert(address);
                normalized[offset + field] = Token(address);
            }
        }
        wire.push_back(static_cast<uint32_t>(bindings.size()));
        for (uint32_t address : bindings) {
            std::array<uint32_t, 4> binding{};
            if (!ReadWords(address, binding) || binding[1] > kLimit || !Factory(binding[2])) return false;
            // Driver handle words are opaque; only validate the array's range.
            std::vector<uint32_t> opaque(binding[1]);
            if (!Read(binding[0], opaque.data(), opaque.size() * 4)) return false;
            std::array<uint32_t, 15> info{};
            info[0] = Token(address);
            info[1] = binding[1];
            const auto found = bindingRecords.find(address);
            if (found != bindingRecords.end()) {
                const auto& r = found->second;
                info[2] = 1;
                info[3] = r[0] == address;
                info[4] = r[1] == binding[0];
                info[5] = r[5]; info[6] = r[6];
                info[7] = r[7]; info[8] = r[8];
                info[9] = r[3]; info[10] = r[4]; info[11] = r[9];
                info[12] = r[10]; info[13] = r[11] & 255; info[14] = r[12];
            }
            wire.insert(wire.end(), info.begin(), info.end());
        }
        wire.push_back(static_cast<uint32_t>(sources.size()));
        for (uint32_t address : sources) {
            uint32_t vtable = 0;
            if (!Read(address, &vtable, 4) || !vtable) return false;
            wire.push_back(Token(address)); wire.push_back(Token(vtable));
            // OV12: no guaranteed read-only dimension getter or CPU data API.
        }
        return true;
    }
    bool Graph() const {
        for (uint32_t i = 0; i < groups.live.size(); ++i) if (groups.Has(i)) {
            for (uint32_t p = 1; p <= 8; ++p) if (!overlays.Has(groups.Record(i)[p])) return false;
        }
        for (uint32_t i = 0; i < handles.live.size(); ++i) if (handles.Has(i)) {
            const uint32_t target = handles.Record(i)[11];
            if (target & 0x80000000) {
                if (!groups.Has(target & 0x7fffffff)) return false;
            } else if (!overlays.Has(target)) return false;
        }
        return true;
    }
    void Probe(cISTEOverlayManager* object, uint32_t id) {
        wire.push_back(id);
        wire.push_back(object->OverlayIsEnabled(id) ? 1 : 0);
        const float alpha = object->OverlayAlpha(id);
        uint32_t bits;
        std::memcpy(&bits, &alpha, 4); wire.push_back(bits);
        using Color = float* (__thiscall*)(void*, float*, uint32_t);
        using SelfLit = bool (__thiscall*)(void*, uint32_t);
        auto** table = *reinterpret_cast<void***>(object);
        float rgb[3]{};
        reinterpret_cast<Color>(table[24])(object, rgb, id);
        for (float value : rgb) { std::memcpy(&bits, &value, 4); wire.push_back(bits); }
        wire.push_back(object->OverlayDrawMode(id));
        wire.push_back(reinterpret_cast<SelfLit>(table[26])(object, id) ? 1 : 0);
        std::array<uint32_t, 9> placement{};
        for (uint32_t i = 0; i < 9; ++i) placement[i] = 0x3e800000 + i;
        cISTEOverlayManager::cDecalInfo out;
        std::memcpy(&out, placement.data(), sizeof(out));
        object->DecalInfo(id, &out);
        std::memcpy(placement.data(), &out, sizeof(out));
        wire.insert(wire.end(), placement.begin(), placement.end());
    }
    bool Run(cISTEOverlayManager* object) {
        manager = reinterpret_cast<uint32_t>(object);
        if (!ReadWords(manager, header) || !(header[5] & 255) ||
            !overlays.Capture(manager + 0x98, 45) || !handles.Capture(manager + 0xac, 12) ||
            !groups.Capture(manager + 0xc0, 9)) return false;
        stage = "graph";
        if (!Graph()) return false;
        wire = {1, (header[5] >> 8) & 255, header[0xe4 / 4], header[0xe8 / 4], header[0xec / 4]};
        std::vector<uint32_t> normalized = overlays.data;
        // Resource section is built separately, then appended after arena/cells.
        const auto prefix = wire;
        wire.clear();
        stage = "resources";
        if (!Resources(normalized)) return false;
        auto resources = std::move(wire);
        wire = prefix;
        for (const Arena* arena : {&overlays, &handles, &groups}) {
            wire.push_back(static_cast<uint32_t>(arena->live.size()));
            wire.push_back((arena->header[2] - arena->header[0]) / (arena->stride * 4));
            wire.push_back(arena->header[3]); wire.push_back(arena->header[4]);
            const auto& data = arena == &overlays ? normalized : arena->data;
            wire.insert(wire.end(), data.begin(), data.end());
        }
        stage = "cells";
        if (!Cells()) return false;
        wire.insert(wire.end(), resources.begin(), resources.end());
        const size_t probeCount = wire.size(); wire.push_back(0);
        for (uint32_t i = 0; i < overlays.live.size(); ++i) if (overlays.Has(i)) {
            Probe(object, i); ++wire[probeCount];
        }
        for (uint32_t i = 0; i < handles.live.size(); ++i) if (handles.Has(i)) {
            Probe(object, i | 0x80000000); ++wire[probeCount];
        }
        // Owning-thread, synchronous OnTick/month events: no native writes or
        // reentrant Draw. Recheck data as well as headers to detect mutation.
        stage = "stability";
        std::array<uint32_t, 63> after{};
        if (!ReadWords(manager, after) || after != header ||
            !overlays.Unchanged(manager + 0x98) || !handles.Unchanged(manager + 0xac) ||
            !groups.Unchanged(manager + 0xc0)) return false;
        if (!heads.empty()) {
            std::vector<uint32_t> afterHeads(heads.size());
            if (!Read(header[0xf0 / 4], afterHeads.data(), afterHeads.size() * 4) || afterHeads != heads) return false;
        }
        for (const auto& entry : nodes) {
            std::array<uint32_t, 2> node{};
            if (!ReadWords(entry.first, node) || node != entry.second) return false;
        }
        stage = "rust-invariants";
        return wire.size() <= 16777216;
    }
};

// Explicit test action, separate from the read-only observer. Copy resource and
// cached geometry inputs BEFORE the add (OV13); never seed native M/bounds into
// the production owner. This tests a controlled suffix, not missing Init/history.
struct Fixture {
    cISTEOverlayManager* object = nullptr;
    uint32_t id = kNone;
    std::array<uint32_t, 35> expected{};
    bool Start(cISTEOverlayManager* manager) {
        object = manager;
        Capture input{};
        const uint32_t address = reinterpret_cast<uint32_t>(manager);
        if (!ReadWords(address, input.header) || !(input.header[5] & 255)) return false;
        const uint32_t binding = input.header[0xd4 / 4];
        if (!binding || !input.Factory(input.header[0x90 / 4])) return false;
        const auto found = input.bindingRecords.find(binding);
        if (found == input.bindingRecords.end()) return false;
        float scale;
        std::memcpy(&scale, &input.header[0x68 / 4], 4);
        const float width = 1.0f / scale;
        uint32_t widthBits;
        std::memcpy(&widthBits, &width, 4);
        const uint32_t args[] = {input.header[0x5c / 4], input.header[0x60 / 4], widthBits,
            found->second[7], found->second[8], (input.header[5] >> 8) & 255,
            (input.header[5] >> 16) & 255};
        if (!fc4_overlays_replay_decal(args, expected.data())) return false;
        auto** table = *reinterpret_cast<void***>(manager);
        using Add = uint32_t (__thiscall*)(void*, const void*, const float*, float, float);
        const float position[] = {128.0f, 128.0f};
        id = reinterpret_cast<Add>(table[4])(manager, reinterpret_cast<void*>(binding), position, 32.0f, 0.0f);
        if (id == kNone) return false;
        using Position = void (__thiscall*)(void*, uint32_t, const float*);
        const float moved[] = {144.0f, 160.0f};
        reinterpret_cast<Position>(table[12])(manager, id, moved);
        manager->SetOverlayAlpha(id, 0.5f);
        using Color = void (__thiscall*)(void*, uint32_t, const float*);
        const float rgb[] = {0.25f, 0.5f, 0.75f};
        reinterpret_cast<Color>(table[18])(manager, id, rgb);
        manager->SetOverlayDrawMode(id, 3);
        using SelfLit = void (__thiscall*)(void*, uint32_t, uint32_t);
        reinterpret_cast<SelfLit>(table[20])(manager, id, 1);
        return true;
    }
    bool Compare(const Capture& capture, uint32_t& differences, bool& onlyOV15) const {
        if (id == kNone || !(id & 0x80000000) || !capture.handles.Has(id & 0x7fffffff)) return false;
        const auto* handle = capture.handles.Record(id & 0x7fffffff);
        if (!capture.overlays.Has(handle[11])) return false;
        const auto* record = capture.overlays.Record(handle[11]);
        std::array<uint32_t, 35> actual{};
        actual[0] = record[1]; actual[1] = record[2] & 255;
        std::copy(record + 3, record + 23, actual.begin() + 2);
        std::copy(record + 39, record + 43, actual.begin() + 22);
        std::copy(handle + 1, handle + 10, actual.begin() + 26);
        for (size_t i = 0; i < actual.size(); ++i) {
            if (actual[i] == expected[i]) continue;
            ++differences;
            // OV15: report the full flags word. No bit-7 mask or guessed fix.
            if (i != 0 || (actual[i] ^ expected[i]) != 0x80) onlyOV15 = false;
            Log("overlays fixture diff: field=%u original=%08x core=%08x", static_cast<unsigned>(i), actual[i], expected[i]);
        }
        return true;
    }
    ~Fixture() { if (id != kNone) object->RemoveOverlay(id); }
};
}
void OverlaysObservationReset() {
    g_lines = g_clean = g_incomplete = 0;
    g_replayLines = g_replayClean = 0;
    g_replayDifferences = 0;
    g_checks = g_differences = 0;
    g_counts.fill(0);
}
void OverlaysObservationSample(cISTEOverlayManager* manager, uint32_t type, const char* phase, int32_t julian, bool fixtureEnabled) {
    ++g_lines;
    Capture capture{};
    Fixture fixture;
    const bool fixtureOK = !fixtureEnabled || (manager && fixture.Start(manager));
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    std::array<uint32_t, 14> r{};
    if (!fixtureOK || !manager || !capture.Run(manager) || !fc4_overlays_observe(capture.wire.data(), static_cast<uint32_t>(capture.wire.size()), r.data())) {
        ++g_incomplete;
        ++g_checks; ++g_differences;
        Log("overlayrecords shadow %04d-%02d-%02d type=%u phase=%s: 0/1 mismatches=1 incomplete=1 julian=%d stage=%s dynamic=%u shift=%u nx=%d nz=%d overlay_slots=%u handle_slots=%u group_slots=%u scope=observation-invariants", date.year, date.month, date.day_of_month, type, phase, julian, capture.stage,
            (capture.header[5] >> 8) & 255, capture.header[0xe4 / 4], capture.header[0xe8 / 4], capture.header[0xec / 4],
            static_cast<uint32_t>(capture.overlays.live.size()), static_cast<uint32_t>(capture.handles.live.size()), static_cast<uint32_t>(capture.groups.live.size()));
        return;
    }
    if (fixtureEnabled) {
        uint32_t differences = 0;
        bool onlyOV15 = true;
        if (!fixture.Compare(capture, differences, onlyOV15)) { differences = 35; onlyOV15 = false; }
        ++g_replayLines;
        if (!differences) ++g_replayClean;
        g_replayDifferences += differences;
        Log("overlayreplay shadow %04d-%02d-%02d type=%u phase=%s: %u/35 mismatches=%u question=%s scope=controlled-suffix init=external-input history=unavailable(OV13) image_bytes=unavailable",
            date.year, date.month, date.day_of_month, type, phase, 35 - differences, differences,
            differences ? (onlyOV15 ? "OV15" : "unfiled") : "none");
    }
    g_checks += r[0]; g_differences += r[1];
    if (!r[1]) ++g_clean;
    for (size_t i = 0; i < g_counts.size(); ++i) g_counts[i] += r[2 + i];
    Log("overlayrecords shadow %04d-%02d-%02d type=%u phase=%s: %u/%u mismatches=%u julian=%d overlays=%u handles=%u groups=%u cells=%u entries=%u bindings=%u sources=%u unknown_bindings=%u observed_words=%u digest=%08x%08x data=unavailable history=unavailable(OV13) fixture_fields=%u scope=observation-invariants",
        date.year, date.month, date.day_of_month, type, phase, r[0] - r[1], r[0], r[1], julian, r[2], r[3], r[4], r[5], r[6], r[7], r[8], r[9], r[10], r[12], r[11], fixtureEnabled ? 35 : 0);
}
void OverlaysObservationSummary() {
    Log("overlays replay summary: %u/%u lines exact mismatches=%llu fields=%u scope=controlled-suffix",
        g_replayClean, g_replayLines, static_cast<unsigned long long>(g_replayDifferences), g_replayLines * 35);
    Log("overlays records summary: %u/%u lines exact mismatches=%llu incomplete=%u checks=%llu overlays=%llu handles=%llu groups=%llu cells=%llu entries=%llu bindings=%llu sources=%llu unknown_bindings=%llu observed_words=%llu image_bytes=0 data=unavailable history=unavailable(OV13) scope=observation-invariants-and-controlled-suffix",
        g_clean, g_lines, static_cast<unsigned long long>(g_differences), g_incomplete, static_cast<unsigned long long>(g_checks),
        g_counts[0], g_counts[1], g_counts[2], g_counts[3], g_counts[4], g_counts[5], g_counts[6], g_counts[7], g_counts[8]);
}
}
