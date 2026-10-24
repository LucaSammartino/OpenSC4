// SPDX-License-Identifier: GPL-3.0-or-later
// Exercise the real adapter's memory/queue/list safeguards without a game.
#include "../plugin/FC4OverlaysObservation.cpp"
#include <cstdio>

extern "C" uint32_t fc4_overlays_observe(const uint32_t*, uint32_t, uint32_t*) { return 0; }
extern "C" uint32_t fc4_overlays_replay_decal(const uint32_t*, uint32_t*) { return 0; }
extern "C" void fc4_date_from_julian(int32_t, fc4_date*) {}
namespace fc4 { void Log(const char*, ...) {} }

int main() {
    using namespace fc4;
    uint32_t count = 123;
    if (!Span(0, 0, 0, 0xb4, count) || count != 0) return 1;
    if (Span(0, 4, 8, 4, count) || Span(8, 4, 8, 4, count) || Span(4, 9, 12, 4, count)) return 2;
    std::array<uint32_t, 27> records{};
    records[0] = 0; records[9] = 0; records[18] = UINT32_MAX;
    const auto begin = reinterpret_cast<uint32_t>(records.data());
    std::array<uint32_t, 5> header = {begin, begin + sizeof(records), begin + sizeof(records), 1, 0};
    Arena arena;
    if (!arena.Capture(reinterpret_cast<uint32_t>(header.data()), 9)) return 3;
    if (arena.Has(0) || arena.Has(1) || !arena.Has(2) || arena.Has(3)) return 4;
    if (!arena.Unchanged(reinterpret_cast<uint32_t>(header.data()))) return 5;
    records[18] = 0;
    if (arena.Unchanged(reinterpret_cast<uint32_t>(header.data()))) return 6;
    records[18] = UINT32_MAX;
    header[4] = 2; // cycle 1 -> 0 -> 0 never reaches tail 2
    if (arena.Capture(reinterpret_cast<uint32_t>(header.data()), 9)) return 7;
    records[9] = 27; // out-of-range next index
    if (arena.Capture(reinterpret_cast<uint32_t>(header.data()), 9)) return 8;
    if (Read(0, &count, 4) || Read(UINT32_MAX - 3, &count, 8) || Read(1, &count, 4)) return 9;

    Capture capture{};
    capture.overlays.live = {true, true};
    std::array<uint32_t, 2> node0 = {0, 0};
    std::array<uint32_t, 2> node1 = {reinterpret_cast<uint32_t>(node0.data()), 1};
    std::array<uint32_t, 4> heads = {reinterpret_cast<uint32_t>(node1.data()), 0, 0, 0};
    capture.header[0xe8 / 4] = capture.header[0xec / 4] = 2;
    capture.header[0xf0 / 4] = reinterpret_cast<uint32_t>(heads.data());
    capture.header[0xf4 / 4] = capture.header[0xf8 / 4] = capture.header[0xf0 / 4] + sizeof(heads);
    if (!capture.Cells() || capture.wire != std::vector<uint32_t>({4, 2, 1, 0, 0, 0, 0})) return 10;
    node0[0] = reinterpret_cast<uint32_t>(node1.data());
    capture.wire.clear();
    if (capture.Cells()) return 11;
    node0[0] = 0; node0[1] = 2;
    if (capture.Cells()) return 12;
    // OV14-B: retained static storage, even poisoned extents, is inactive.
    capture.header[5] = 0x100;
    capture.header[0xe4 / 4] = UINT32_MAX;
    capture.header[0xe8 / 4] = 0x80000000;
    capture.header[0xec / 4] = 0x7fffffff;
    capture.header[0xf0 / 4] = 1;
    capture.header[0xf4 / 4] = 0;
    capture.wire.clear();
    if (!capture.Cells() || capture.wire != std::vector<uint32_t>({0})) return 13;
    capture.header[5] = 0;
    if (capture.Cells()) return 14;
    // Missing managers/capture must fail both requested scopes, never report
    // an empty replay summary as though the requested fixture succeeded.
    OverlaysObservationReset();
    OverlaysObservationSample(nullptr, 0, "test", 0, true);
    if (g_lines != 1 || g_clean != 0 || g_incomplete != 1 ||
        g_replayLines != 1 || g_replayClean != 0 || g_replayDifferences != 35 ||
        g_checks != 1 || g_differences != 1) return 15;
    OverlaysObservationSample(nullptr, 1, "test", 0, false);
    if (g_lines != 2 || g_incomplete != 2 || g_replayLines != 1 ||
        g_replayDifferences != 35) return 16;
    OverlaysObservationReset();
    if (g_lines || g_clean || g_incomplete || g_replayLines ||
        g_replayClean || g_replayDifferences || g_checks || g_differences) return 17;
    std::puts("overlays observer safeguards: PASS (extents, FIFO zero links, mutation, lists, cycles, invalid pointers)");
    return 0;
}
