// SPDX-License-Identifier: GPL-3.0-or-later
// gz-framework-core sections 4/6; span-render-8 section 10.7 native ABI.
#include "FC4GZFrameworkShadow.h"
#include "FC4Config.h"
#include "FC4GameList.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cIGZCOM.h"
#include "cIGZCommandServer.h"
#include "cIGZFrameWork.h"
#include "cIGZString.h"
#include "cRZCOMDllDirector.h"
#include "fc4.h"
#include <algorithm>
#include <string>

extern "C" uint32_t fc4_gzfw_buffer_compare(uint32_t, uint32_t, uint32_t, uint32_t, const uint32_t*);
extern "C" uint32_t fc4_gzfw_command_expected(const uint8_t*, uint32_t, uint32_t);

namespace {
template<class T> T Slot(void* object, uint32_t slot) {
    return reinterpret_cast<T>((*reinterpret_cast<void***>(object))[slot]);
}
uint32_t BufferSample(cIGZCOM* com, uint32_t width, uint32_t height, uint32_t kind, uint32_t bpp) {
    void* object = nullptr;
    if (!com || !com->GetClassObject(0xC470D325, 0x20732180, &object) || !object) return 1;
    auto* owner = static_cast<cIGZUnknown*>(object);
    FC4_ACQUIRED(owner, "GetClassObject gzfw private buffer");
    // The SDK's overloaded buffer methods do not match this table; use only
    // signatures explicitly specified in span-render-8 section 10.7.
    using Flags = bool (__thiscall*)(void*, uint32_t);
    using Init = bool (__thiscall*)(void*, uint32_t, uint32_t, uint32_t, uint32_t);
    using Get = uint32_t (__thiscall*)(void*);
    using Pair = void* (__thiscall*)(void*, uint32_t*);
    using Key = bool (__thiscall*)(void*, uint32_t*);
    using RGB = uint32_t (__thiscall*)(void*, uint32_t, uint32_t, uint32_t);
    const auto lock = Slot<Flags>(object, 6), unlock = Slot<Flags>(object, 7);
    const auto init = Slot<Init>(object, 3);
    uint32_t values[28]{};
    uint32_t key = 0x12345678;
    values[0] = lock(object, 0x8040);
    values[1] = unlock(object, 0x0800);
    values[2] = Slot<Key>(object, 26)(object, &key);
    values[3] = key;
    values[4] = init(object, 0, height, kind, bpp);
    values[5] = init(object, width, height, kind, bpp);
    values[6] = init(object, width + 1, height, kind, bpp);
    values[7] = Slot<Get>(object, 9)(object);
    values[8] = Slot<Get>(object, 10)(object);
    values[9] = Slot<Get>(object, 35)(object);
    values[10] = Slot<Get>(object, 17)(object);
    Slot<Pair>(object, 15)(object, &values[11]);
    Slot<Pair>(object, 11)(object, &values[13]);
    values[17] = lock(object, 0x8040);
    values[18] = lock(object, 0x0800);
    values[19] = unlock(object, 0x0800);
    values[20] = unlock(object, 0x8040);
    const auto rgb = Slot<RGB>(object, 30);
    values[21] = rgb(object, 0, 0, 0);
    values[22] = rgb(object, 255, 255, 255);
    values[23] = rgb(object, 17, 129, 241);
    void* queried = nullptr;
    values[24] = owner->QueryInterface(0x20732180, &queried);
    values[25] = queried == object;
    if (values[24] && queried) {
        auto* acquired = static_cast<cIGZUnknown*>(queried);
        FC4_ACQUIRED(acquired, "QueryInterface gzfw buffer");
        FC4_RELEASE(acquired);
    }
    queried = object;
    values[26] = owner->QueryInterface(0xfffffffe, &queried);
    values[27] = queried == object;
    if (values[26] && queried) {
        auto* acquired = static_cast<cIGZUnknown*>(queried);
        FC4_ACQUIRED(acquired, "QueryInterface unexpected gzfw buffer");
        FC4_RELEASE(acquired);
    }
    const uint32_t differences = fc4_gzfw_buffer_compare(width, height, kind, bpp, values);
    fc4::Log("gzfw buffer sample: width=%u height=%u type=%u bpp=%u init=%u stride=%u rgb=%08X/%08X/%08X differences=%u",
        width, height, kind, bpp, values[5], values[9], values[21], values[22], values[23], differences);
    FC4_RELEASE_CREATED(owner);
    return differences;
}

uint32_t Commands(cIGZFrameWork* framework, uint32_t& compared) {
    cIGZCommandServer* server = nullptr;
    if (!framework || !framework->GetSystemService(0xEB903A32, 0xCB90253A, reinterpret_cast<void**>(&server)) || !server) return 1;
    FC4_ACQUIRED(server, "GetSystemService gzfw command server");
    // command-system section 6 supplies these registrations independently of
    // native lookup results. Never register, execute or remove live commands.
    const char* names[] = {"ListCommands", "SetDebugLevel", "ExecuteScript", "ExecuteCheat", "fc4_nonexistent_command"};
    uint32_t differences = 0;
    for (const char* name : names) {
        std::string upper(name);
        for (char& c : upper) if (c >= 'a' && c <= 'z') c -= 'a' - 'A';
        const std::string inputs[] = {name, upper, std::string("\"") + name + "\"", std::string(name) + " argument", ""};
        for (const auto& text : inputs) {
            fc4::GameString storage{};
            if (!fc4::GameStringInit(storage, text.data(), text.size())) { ++differences; continue; }
            const auto expected = fc4_gzfw_command_expected(reinterpret_cast<const uint8_t*>(text.data()), static_cast<uint32_t>(text.size()), 0);
            const auto lookup = server->GetCommandID(*reinterpret_cast<cIGZString*>(&storage));
            const auto parsed = server->GetCommandIDFromCommandString(text.c_str(), static_cast<int32_t>(text.size()));
            const auto terminated = server->GetCommandIDFromCommandString(text.c_str(), -1);
            const auto expectedParse = fc4_gzfw_command_expected(reinterpret_cast<const uint8_t*>(text.data()), static_cast<uint32_t>(text.size()), 1);
            differences += lookup != expected;
            differences += parsed != expectedParse;
            differences += terminated != expectedParse;
            compared += 3;
            fc4::Log("gzfw command sample: text='%s' lookup=%08X/%08X parse=%08X/%08X terminated=%08X",
                text.c_str(), lookup, expected, parsed, expectedParse, terminated);
            fc4::GameStringDestroy(storage);
        }
    }
    FC4_RELEASE(server);
    return differences;
}
}

namespace fc4 {
void GZFrameworkShadowMonth(int32_t julian) {
    if (!SettingInt("gzfw_shadow", nullptr, 0)) return;
    auto* framework = RZGetFrameWork(); // borrowed
    auto* com = framework ? framework->GetCOMObject() : nullptr; // borrowed
    uint32_t bufferDifferences = 0, commandCompared = 0;
    const uint32_t kinds[] = {2, 4, 5, 7, 9};
    for (uint32_t kind : kinds) {
        bufferDifferences += BufferSample(com, 9 + static_cast<uint32_t>(julian) % 5, 3, kind, kind == 2 ? 8 : kind == 9 ? 32 : 16);
    }
    const uint32_t commandDifferences = Commands(framework, commandCompared);
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    Log("gzfw shadow %04d-%02d-%02d: scope=buffer-constructed,command-lookup-parse buffer_compared=140 command_compared=%u buffer_differences=%u command_differences=%u; mismatches=%u",
        date.year, date.month, date.day_of_month, commandCompared, bufferDifferences, commandDifferences, bufferDifferences + commandDifferences);
}
}
