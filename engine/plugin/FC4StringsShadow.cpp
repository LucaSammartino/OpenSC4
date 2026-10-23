// SPDX-License-Identifier: GPL-3.0-or-later
// Constructed same-process probes, not interception of arbitrary game calls.
// Native inputs are a bounded live city name and three deterministic witnesses.
// All mutations target private game-allocator strings, never the city's state.
#include "FC4StringsShadow.h"
#include "FC4Config.h"
#include "FC4GameList.h"
#include "FC4Log.h"
#include "cIGZString.h"
#include "cISC4City.h"
#include "fc4.h"
#include <algorithm>
#include <string>

namespace {
extern "C" int32_t fc4_strings_shadow_call(uint32_t, const uint8_t*, size_t,
    const uint8_t*, size_t, uint32_t, uint8_t*, size_t, size_t*, int32_t*);

struct String {
    fc4::GameString storage{};
    bool good;
    explicit String(const std::string& bytes) : good(fc4::GameStringInit(storage, bytes.data(), bytes.size())) {}
    ~String() { if (good) fc4::GameStringDestroy(storage); }
    String(const String&) = delete;
    String& operator=(const String&) = delete;
    cIGZString& ref() { return *reinterpret_cast<cIGZString*>(&storage); }
    std::string bytes() const { return std::string(storage.data(), storage.size()); }
};

std::string Hex(const std::string& s) {
    const char* digits = "0123456789abcdef";
    std::string out;
    for (unsigned char b : s) { out += digits[b >> 4]; out += digits[b & 15]; }
    return out.empty() ? "-" : out;
}

bool Compare(uint32_t op, const std::string& input, const std::string& other, uint32_t n,
             uint32_t sample) {
    String native(input);
    if (!native.good) return false;
    auto& s = native.ref();
    int32_t value = 0;
    const auto length = static_cast<uint32_t>(other.size());
    switch (op) {
        case 0: s.FromChar(input.data(), static_cast<uint32_t>(input.size())); break;
        case 1: {
            String source(input);
            if (!source.good) return false;
            s.Copy(source.ref());
            break;
        }
        case 2: s.Resize(n); break;
        case 3: s.Append(other.data(), length); break;
        case 4: s.Insert(n, other.data(), length); break;
        case 5: s.Replace(n, other.c_str(), 3); break;
        case 6: s.Erase(n, 3); break;
        case 7: case 8: value = s.IsEqual(other.data(), length, op == 7) ? 1 : 0; break;
        case 9: case 10: value = s.CompareTo(other.data(), length, op == 9); break;
        case 11: case 12: value = s.Find(other.c_str(), n, op == 11); break;
        case 13: case 14: value = s.RFind(other.c_str(), n, op == 13); break;
        case 15: s.Sprintf("%s|%+08d|%#010x|%%", input.c_str(), static_cast<int>(n), n); break;
        default: return false;
    }
    uint8_t out[512]{};
    size_t outLen = 0;
    int32_t coreValue = 0;
    const int32_t ok = fc4_strings_shadow_call(op,
        reinterpret_cast<const uint8_t*>(input.data()), input.size(),
        reinterpret_cast<const uint8_t*>(other.data()), other.size(), n,
        out, sizeof(out), &outLen, &coreValue);
    const std::string core(reinterpret_cast<const char*>(out), outLen);
    const std::string original = native.bytes();
    const bool exact = ok == 1 && coreValue == value && core == original;
    // These are records, not summary lines: every operation's inputs and both
    // outputs are retained even on success. Hex preserves counted interior NULs.
    fc4::Log("strings sample: sample=%u op=%u n=%u input=%s other=%s native=%s/%d core=%s/%d status=%d exact=%d",
        sample, op, n, Hex(input).c_str(), Hex(other).c_str(), Hex(original).c_str(), value,
        Hex(core).c_str(), coreValue, ok, exact);
    return exact;
}
}  // namespace

namespace fc4 {
void StringsShadowMonth(cISC4City* city, int32_t julian) {
    if (city == nullptr || SettingInt("strings_shadow", nullptr, 0) == 0) return;
    String name("");
    uint32_t mismatches = 0, compared = 0, excluded = 0;
    if (name.good) {
        city->GetCityName(name.ref());
        // [choice] deterministic bound, not a claim that a prefix is the whole
        // city name. Case/format paths with non-ASCII input remain RZS2/RZS5.
        const std::string inputs[] = {name.bytes().substr(0, 96), "abABabcdef", std::string("A\0X", 3), ""};
        const std::string others[] = {"Ab", "AB", std::string("A\0Y", 3), ""};
        for (uint32_t sample = 0; sample < 4; ++sample) {
            const auto& input = inputs[sample];
            const bool ascii = std::all_of(input.begin(), input.end(), [](unsigned char b) { return b < 0x7f; });
            for (uint32_t op = 0; op < 16; ++op) {
                if (!ascii && (op == 8 || op == 10 || op == 12 || op == 14 || op == 15)) { ++excluded; continue; }
                uint32_t n = 0;
                if (op == 2) n = static_cast<uint32_t>(input.size()) + 3;
                if (op >= 4 && op <= 6) n = static_cast<uint32_t>(input.size()) / 2;
                if (op == 13 || op == 14) n = static_cast<uint32_t>(input.size()) + 3;
                if (op == 15) n = static_cast<uint32_t>(julian) % 4096;
                ++compared;
                if (!Compare(op, input, others[sample], n, sample)) ++mismatches;
            }
        }
    } else {
        ++mismatches;  // failure to construct is never an exact empty sample
    }
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    Log("strings shadow %04d-%02d-%02d constructed: compared=%u excluded=%u; mismatches=%u",
        date.year, date.month, date.day_of_month, compared, excluded, mismatches);
}
}  // namespace fc4
