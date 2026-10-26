// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4ConfigRulesAbi.h"
#include <cstdio>
#include <cstring>
#include <string>
#include <thread>
#include <vector>
#include <atomic>

#define CHECK(expr) do { if (!(expr)) { std::fprintf(stderr, "line %d: %s\n", __LINE__, #expr); return 1; } } while (0)

namespace {
FC4ConfigRulesBytes Bytes(const char* value) {
    return {reinterpret_cast<const uint8_t*>(value), std::strlen(value)};
}
std::string Text(FC4ConfigRulesBytes value) {
    return value.len ? std::string(reinterpret_cast<const char*>(value.data), value.len) : std::string{};
}
struct Write { uint32_t kind, index, value; };
struct Renderer {
    int adds = 0, drops = 0;
    uint32_t values[3][2]{};
    std::vector<Write> writes;
};
struct Service {
    bool available = true, fsaa = true, allowLog = true, open = false, missing = false;
    int measures = 0, closes = 0;
    std::vector<std::string> files, logPaths;
    std::string log;
};
void Retain(void* p) { ++static_cast<Renderer*>(p)->adds; }
void Release(void* p) { ++static_cast<Renderer*>(p)->drops; }
const char* names[3][2] = {{"ForceSoftwareDriver", "BoolFlag"}, {"Signed", "UnusedInt"}, {"Scale", "UnusedFloat"}};
int32_t Find(void*, uint32_t kind, FC4ConfigRulesBytes name) {
    for (int i = 0; i < 2; ++i) if (Text(name) == names[kind][i]) return i;
    return -1;
}
uint32_t Get(void* p, uint32_t kind, uint32_t index) {
    return index < 2 ? static_cast<Renderer*>(p)->values[kind][index] : 0;
}
void Set(void* p, uint32_t kind, uint32_t index, uint32_t value) {
    auto& r = *static_cast<Renderer*>(p);
    r.writes.push_back({kind, index, value});
    if (index < 2) r.values[kind][index] = value;
}
uint32_t Count(void*, uint32_t) { return 2; }
FC4ConfigRulesBytes Name(void*, uint32_t kind, uint32_t index) { return Bytes(names[kind][index]); }
int32_t Read(void* p, FC4ConfigRulesBytes path, FC4ConfigRulesBytes* output) {
    auto& s = *static_cast<Service*>(p);
    const auto file = Text(path);
    s.files.push_back(file);
    if (s.missing || file.find("Graphics Options.sgr") != std::string::npos) return 0;
    *output = Bytes(
        "optionGroup 2\noption 3\nproperty BoolFlag true\nproperty Signed -7\n"
        "property Scale 1.25\nproperty Signed 8\nend\n"
        "partialRule machine\nhas forcedFSAA\nstringMatch CPU fixture*\noption 2 3\nend\n"
        "partialRule memory\natLeast memory 256\noption 3 1\nend\n");
    return 1;
}
bool Measure(void* p, FC4ConfigRulesMeasurement* m) {
    auto& s = *static_cast<Service*>(p);
    ++s.measures;
    if (!s.available) return false;
    m->cpuFeatures = (1u << 23) | 1;
    m->cpuMHz = 2000;
    m->physicalBytes = 257u << 20;
    m->availableBytes = 128u << 20;
    m->screen[0] = 1024; m->screen[1] = 768; m->screen[2] = 32;
    m->videoBytes = 64u << 20; m->textureBytes = 32u << 20;
    const char* strings[] = {"user", "app", "version", "os", "ALPHA", "Fixture CPU"};
    for (int i = 0; i < 6; ++i) m->osStrings[i] = Bytes(strings[i]);
    std::memset(m->driverRecord, 0, sizeof(m->driverRecord));
    m->driverRecord[0x30] = 1; m->driverRecord[0x24] = 2;
    m->driverDescription = Bytes("discard\nSGL\n1\nDriver\n2\nCard\n3");
    m->hasSound = 0; m->backingStore = 1; m->forcedFSAA = s.fsaa;
    return true;
}
bool Open(void* p, FC4ConfigRulesBytes path) {
    auto& s = *static_cast<Service*>(p);
    s.logPaths.push_back(Text(path));
    return s.open = s.allowLog;
}
bool IsOpen(void* p) { return static_cast<Service*>(p)->open; }
bool WriteLog(void* p, FC4ConfigRulesBytes value) { static_cast<Service*>(p)->log += Text(value); return true; }
bool Close(void* p) { auto& s = *static_cast<Service*>(p); s.open = false; ++s.closes; return true; }
uint8_t Lowercase(void*, uint8_t b) { return b >= 'A' && b <= 'Z' ? b + ('a' - 'A') : b; }
}

int main() {
    static_assert(sizeof(FC4ConfigRulesHost) == 15 * sizeof(void*));
    static_assert(sizeof(FC4ConfigRulesMeasurement) == (sizeof(void*) == 4 ? 164 : 248));
    static_assert(sizeof(FC4ConfigRulesLogInput) == (sizeof(void*) == 4 ? 32 : 64));
    Service s;
    Renderer a, b;
    const FC4ConfigRulesHost host = {&s, Retain, Release, Find, Get, Set, Count, Name,
        Read, Measure, Open, IsOpen, WriteLog, Close, Lowercase};
    void* owner = fc4_cfgrules_owner_create(&host);
    CHECK(owner);
    uint32_t setting = 99;
    CHECK(fc4_cfgrules_owner_auto(owner, 2, &setting) == 0 && setting == 0);
    CHECK(fc4_cfgrules_owner_init(owner, &a, Bytes("C:/R/"), Bytes("C:/U/"), false) == 0);
    CHECK(s.files.size() == 2 && s.files[0] == "C:/R/Graphics Rules.sgr" && s.files[1] == "C:/U/Graphics Options.sgr");
    CHECK(s.measures == 1 && a.adds == 1 && a.drops == 0 && a.writes.empty());
    // Init parses without evaluation. Recommendations become visible only on SetOptions.
    CHECK(fc4_cfgrules_owner_auto(owner, 2, &setting) == 0 && setting == 0);
    const FC4ConfigRulesLogInput input = {Bytes("app"), Bytes("off"), Bytes("fixture-date"), 12, 34, nullptr};
    CHECK(fc4_cfgrules_owner_evaluate(owner, &input) == 0);
    CHECK(s.measures == 1 && s.logPaths.empty());
    CHECK(fc4_cfgrules_owner_auto(owner, 2, &setting) == 0 && setting == 3);
    CHECK(fc4_cfgrules_owner_auto(owner, 3, &setting) == 0 && setting == 1);
    CHECK(fc4_cfgrules_owner_apply(owner, 2, 3) == 0 && a.writes.size() == 4);
    CHECK(a.writes[0].kind == 0 && a.writes[0].value == 1);
    CHECK(a.writes[1].kind == 1 && int32_t(a.writes[1].value) == -7);
    CHECK(a.writes[2].kind == 1 && a.writes[2].value == 8);
    CHECK(a.writes[3].kind == 2 && a.writes[3].value == 0x3fa00000);
    CHECK(fc4_cfgrules_owner_apply(owner, UINT32_MAX, UINT32_MAX) == 0 && a.writes.size() == 4);
    CHECK(fc4_cfgrules_owner_auto(owner, UINT32_MAX, &setting) == 0 && setting == 0);
    // Explicit logs regather, even with writeLog=off, without parsing or evaluation.
    s.fsaa = false;
    bool opened = false;
    CHECK(fc4_cfgrules_owner_log(owner, Bytes("explicit.txt"), &input, &opened) == 0 && opened);
    CHECK(s.measures == 2 && s.files.size() == 2 && a.writes.size() == 4 && s.closes == 1);
    CHECK(s.log.find("forcedFSAA = false\r\n") != std::string::npos);
    CHECK(s.log.find("memory = 260\r\n") != std::string::npos);
    CHECK(s.log.find("cardName = Card\r\n") != std::string::npos);
    CHECK(fc4_cfgrules_owner_auto(owner, 2, &setting) == 0 && setting == 3);
    s.allowLog = false;
    CHECK(fc4_cfgrules_owner_log(owner, Bytes("failed.txt"), &input, &opened) == 0 && !opened);
    CHECK(s.measures == 3 && s.closes == 1);
    // SetOptions logs renderer values and unsigned application preference bytes.
    s.allowLog = true;
    const uint8_t preferences[12] = {255, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12};
    const FC4ConfigRulesLogInput logged = {Bytes("app"), Bytes("on"), Bytes("fixture-date"), 12, 34, preferences};
    names[1][1] = "Unused\x80";
    CHECK(fc4_cfgrules_owner_evaluate(owner, &logged) == 0);
    CHECK(s.measures == 3 && s.files.size() == 2 && s.closes == 2);
    CHECK(s.log.find("current: 255 recommended:") != std::string::npos);
    CHECK(s.log.find("Signed = 8\r\n") != std::string::npos);
    CHECK(s.log.find("Scale = 1.25\r\n") != std::string::npos);
    CHECK(s.log.find("Unused\x80 = 0\r\n") != std::string::npos);
    // Failed native acquisition must never look like a successful zero-filled gather.
    s.available = false;
    CHECK(fc4_cfgrules_owner_init(owner, &b, Bytes("data"), Bytes("user"), true) == -2);
    CHECK(b.adds == 0 && a.drops == 0 && s.files.size() == 2);
    CHECK(fc4_cfgrules_owner_log(owner, Bytes("unavailable.txt"), &input, &opened) == -2 && !opened);
    CHECK(s.logPaths.size() == 3);
    CHECK(fc4_cfgrules_owner_auto(owner, 2, &setting) == 0 && setting == 3);
    s.available = true;
    CHECK(fc4_cfgrules_owner_init(owner, &a, Bytes("data"), Bytes("user"), false) == 0);
    CHECK(a.adds == 1 && a.drops == 0);
    CHECK(fc4_cfgrules_owner_init(owner, &b, Bytes("data"), Bytes("user"), false) == 0);
    CHECK(a.drops == 1 && b.adds == 1 && b.drops == 0);
    CHECK(fc4_cfgrules_owner_evaluate(owner, &input) == 0);
    std::atomic<bool> clean{true};
    std::thread getter([&] {
        for (int i = 0; i < 100; ++i) {
            uint32_t value = 99;
            if (fc4_cfgrules_owner_auto(owner, 3, &value) != 0 || value != 1) clean = false;
        }
    });
    for (int i = 0; i < 100; ++i) CHECK(fc4_cfgrules_owner_evaluate(owner, &input) == 0);
    getter.join();
    CHECK(clean);
    // A required-file parse error is distinct from unavailable measurement.
    s.missing = true;
    CHECK(fc4_cfgrules_owner_init(owner, &b, Bytes("data"), Bytes("user"), false) == 1);
    CHECK(b.adds == 1 && b.drops == 0);
    fc4_cfgrules_owner_destroy(owner);
    CHECK(a.adds == a.drops && b.adds == b.drops);
    CHECK(!fc4_cfgrules_owner_create(nullptr));
    CHECK(fc4_cfgrules_owner_auto(nullptr, 0, &setting) == -1);
    fc4_cfgrules_owner_destroy(nullptr);
    std::puts("cfgrules transport: ownership, five methods, I/O, threading PASS");
    return 0;
}
