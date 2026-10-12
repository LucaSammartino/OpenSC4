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

// Drives the Rust core through fc4.h from C++, built with the same compiler,
// architecture and CRT as FreeCity4.dll. It checks that the hand-written
// header matches the Rust ABI (struct layout, bool, calling convention) and
// that re-entrant calls from a callback work.

#include "fc4.h"

#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace {

int g_failures = 0;

#define CHECK(cond)                                                          \
    do {                                                                     \
        if (!(cond)) {                                                       \
            std::fprintf(stderr, "%s:%d: CHECK failed: %s\n", __FILE__, __LINE__, #cond); \
            ++g_failures;                                                    \
        }                                                                    \
    } while (0)

struct Host {
    uint32_t now = 0;
    std::vector<fc4_message> posts;
    std::vector<std::pair<fc4_agent, fc4_message>> deliveries;
    int refs = 0;
    fc4_agent selfRemoving = 0;
};

uint32_t NowMs(void* user) {
    Host* h = static_cast<Host*>(user);
    return h->now++;  // free-running clock: 1 ms per read
}

bool Deliver(void* user, fc4_sim* sim, fc4_agent agent, const fc4_message* msg) {
    Host* h = static_cast<Host*>(user);
    h->deliveries.emplace_back(agent, *msg);
    if (agent == h->selfRemoving) {
        CHECK(fc4_sim_remove_agent_all_levels(sim, agent));  // re-entrant call
    }
    return true;
}

void AddRef(void* user, fc4_agent) { ++static_cast<Host*>(user)->refs; }
void Release(void* user, fc4_agent) { --static_cast<Host*>(user)->refs; }
void Post(void* user, const fc4_message* msg) { static_cast<Host*>(user)->posts.push_back(*msg); }

fc4_sim* Make(Host& h) {
    fc4_host host{};
    host.struct_size = sizeof(host);
    host.user = &h;
    host.now_ms = &NowMs;
    host.deliver = &Deliver;
    host.add_ref = &AddRef;
    host.release = &Release;
    host.post = &Post;
    return fc4_sim_create(&host);
}

void TestDays() {
    Host h;
    fc4_sim* sim = Make(h);
    CHECK(sim != nullptr);
    const char name[] = "day agent";
    CHECK(fc4_sim_add_agent(sim, 0x1234, FC4_LEVEL_DAY, name, sizeof(name) - 1, 3));
    const uint32_t table[3] = {2000, 1000, 50};
    fc4_tuning t{table, 3, nullptr, 0};
    CHECK(fc4_sim_init(sim, &t));
    CHECK(fc4_sim_set_sim_speed(sim, 2));
    CHECK(fc4_sim_is_hidden_paused(sim));  // Init leaves the simulator hidden-paused
    CHECK(fc4_sim_on_post_city_init(sim));
    CHECK(!fc4_sim_is_hidden_paused(sim));
    for (int i = 0; i < 100; ++i) CHECK(fc4_sim_on_tick(sim));

    fc4_state st{};
    st.struct_size = sizeof(st);
    CHECK(fc4_sim_get_state(sim, &st));
    CHECK(st.initialized == 1);
    CHECK(st.ms_per_day == 50);
    CHECK(st.target_fps == 15);
    CHECK(st.agent_counts[FC4_LEVEL_DAY] == 1);
    CHECK(st.julian_day > 2451545 + 20);

    int32_t expectJulian = 2451546;
    for (const fc4_message& m : h.posts) {
        if (m.type != 0x66956814u) continue;
        CHECK(m.reserved == 0);
        CHECK(m.data1 == expectJulian);
        fc4_date d{};
        fc4_date_from_julian(expectJulian, &d);
        CHECK(m.data3 == d.day_of_year);
        ++expectJulian;
    }
    CHECK(expectJulian - 1 == st.julian_day);

    fc4_date d{};
    CHECK(fc4_sim_get_date(sim, &d));
    CHECK(d.julian == st.julian_day);
    CHECK(fc4_sim_get_julian_day(sim) == st.julian_day);

    // Save / load round trip through typed fields.
    uint8_t buf[128];
    const size_t n = fc4_sim_save(sim, buf, sizeof(buf));
    CHECK(n == 79);
    const uint8_t* widths = nullptr;
    const size_t fields = fc4_record_layout(4, &widths);
    CHECK(fields == 30);
    size_t sum = 0;
    for (size_t i = 0; i < fields; ++i) sum += widths[i];
    CHECK(sum == n);
    CHECK(fc4_sim_load(sim, buf, n) == FC4_OK);
    uint8_t again[128];
    CHECK(fc4_sim_save(sim, again, sizeof(again)) == n);
    CHECK(std::memcmp(buf, again, n) == 0);
    buf[0] = 5;
    CHECK(fc4_sim_load(sim, buf, n) == FC4_ERR_UNSUPPORTED_VERSION);

    CHECK(h.refs == 1);
    fc4_sim_destroy(sim);
    CHECK(h.refs == 0);
}

void TestReentrancyAndNames() {
    Host h;
    h.selfRemoving = 7;
    fc4_sim* sim = Make(h);
    CHECK(fc4_sim_add_agent(sim, 7, FC4_LEVEL_ONE_SHOT, "a", 1, 1));
    CHECK(fc4_sim_add_agent(sim, 8, FC4_LEVEL_IDLE, "b", 1, 1));
    std::vector<std::string> names;
    const size_t count = fc4_sim_enumerate_agents(
        sim,
        [](void* ctx, const char* s, size_t len) { static_cast<std::vector<std::string>*>(ctx)->emplace_back(s, len); },
        &names);
    CHECK(count == 2 && names.size() == 2 && names[0] == "a" && names[1] == "b");
    CHECK(fc4_sim_init(sim, nullptr));
    fc4_sim_simulate_agents(sim);
    CHECK(h.deliveries.size() == 1);
    CHECK(h.refs == 1);
    CHECK(fc4_sim_get_agent_enabled(sim, "b", 1));
    CHECK(fc4_sim_set_agent_enabled(sim, "b", 1, false));
    CHECK(!fc4_sim_get_agent_enabled(sim, "b", 1));

    const char cheat[] = "SimDate 7 4 2013";
    CHECK(fc4_sim_handle_cheat(sim, 0x8A78BEFF, cheat, sizeof(cheat) - 1));
    CHECK(fc4_sim_get_julian_day(sim) == fc4_julian_from_civil(2013, 7, 4));
    CHECK(fc4_sim_emergency_pause(sim));
    CHECK(fc4_sim_is_emergency_paused(sim) && fc4_sim_is_paused(sim));
    CHECK(!fc4_sim_set_sim_speed(sim, 1));
    CHECK(fc4_sim_emergency_resume(sim));
    CHECK(!fc4_sim_is_any_paused(sim) || fc4_sim_is_hidden_paused(sim));
    fc4_sim_destroy(sim);
    CHECK(h.refs == 0);
}

}  // namespace

int main() {
    CHECK(fc4_abi_version() == FC4_ABI_VERSION);
    CHECK(fc4_sim_create(nullptr) == nullptr);
    CHECK(!fc4_sim_pause(nullptr));
    TestDays();
    TestReentrancyAndNames();
    if (g_failures == 0) std::printf("fc4 C ABI smoke test: all checks passed (%zu-bit)\n", sizeof(void*) * 8);
    return g_failures == 0 ? 0 : 1;
}
