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

#include "FC4Seed.h"

#include "FC4Config.h"
#include "FC4Log.h"
#include "fc4.h"

#include <cstdlib>
#include <ctime>
#include <string>

namespace {

// 1901-01-01 to 1970-01-01, 25 202 days (cRZTime, crime.md §9).
constexpr int64_t kRzTimeUnixOffset = 2177452800;

// A whole-string number (decimal or 0x...) that fits in 32 bits.
bool ParseU32(const std::string& s, uint32_t* out) {
    size_t b = s.find_first_not_of(" \t");
    size_t e = s.find_last_not_of(" \t");
    if (b == std::string::npos) return false;
    const std::string t = s.substr(b, e - b + 1);
    if (t.empty() || t[0] == '-' || t[0] == '+') return false;
    char* end = nullptr;
    const unsigned long long v = std::strtoull(t.c_str(), &end, 0);
    if (end == t.c_str() || *end != '\0' || v > 0xFFFFFFFFull) return false;
    *out = static_cast<uint32_t>(v);
    return true;
}

struct SeedValue {
    uint32_t value = 0;
    bool fixed = false;
    std::string setting;  // the session_seed text, when set
    const char* source = "";
};

SeedValue ReadSeed() {
    SeedValue r;
    const char* source = "";
    const std::string s = fc4::Setting("session_seed", "FC4_SESSION_SEED", &source);
    r.setting = s;
    r.source = source;
    uint32_t v = 0;
    if (!s.empty() && ParseU32(s, &v)) {
        r.value = v;
        r.fixed = true;
        return r;
    }
    r.value = static_cast<uint32_t>(static_cast<int64_t>(std::time(nullptr)) + kRzTimeUnixOffset);
    return r;
}

const SeedValue& TheSeed() {
    static const SeedValue s = ReadSeed();  // thread-safe local static
    return s;
}

}  // namespace

namespace fc4 {

uint32_t SessionSeed(bool* fixed) {
    const SeedValue& s = TheSeed();
    if (fixed != nullptr) *fixed = s.fixed;
    return s.value;
}

void SessionSeedStartup() {
    const SeedValue& s = TheSeed();
    if (s.fixed) {
        Log("session seed: S = 0x%08X (session_seed = %s, %s; tooling: every generator of ours is seeded with "
            "derive(S, K), foundations section 6.3, so runs with the same S and inputs draw the same)",
            s.value, s.setting.c_str(), s.source);
    } else {
        if (!s.setting.empty()) {
            Log("session seed: WARNING session_seed = '%s' (%s) is not a 32-bit number; using the clock",
                s.setting.c_str(), s.source);
        }
        Log("session seed: S = 0x%08X (the cRZTime wall clock at game start, like the original; session_seed = N "
            "in FreeCity4.ini fixes it); every generator of ours is seeded with derive(S, K), foundations section "
            "6.3",
            s.value);
    }
}

uint32_t SubsystemSeed(const char* iniKey, const char* envName, bool* fixed, const char** source) {
    const std::string s = Setting(iniKey, envName);
    uint32_t v = 0;
    if (!s.empty() && ParseU32(s, &v)) {
        if (fixed != nullptr) *fixed = true;
        if (source != nullptr) *source = iniKey;
        return v;
    }
    bool f = false;
    const uint32_t seed = SessionSeed(&f);
    if (fixed != nullptr) *fixed = f;
    if (source != nullptr) *source = f ? "session_seed" : "session seed, cRZTime clock";
    return seed;
}

}  // namespace fc4
