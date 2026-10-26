// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstddef>
#include <cstdint>

// Private static-link transport; deliberately absent from fc4.h and the DLL exports.
// UTF-8 strings and file contents are the supported profile; renderer dump names
// remain arbitrary bytes. Returned bytes stay live through the next host call.
struct FC4ConfigRulesBytes { const uint8_t* data; size_t len; };
struct FC4ConfigRulesMeasurement {
    uint32_t cpuFeatures, cpuMHz, physicalBytes, availableBytes;
    uint32_t screen[3], videoBytes, textureBytes;
    FC4ConfigRulesBytes osStrings[6];
    uint8_t driverRecord[0x34];
    FC4ConfigRulesBytes driverDescription, soundNames[2];
    uint8_t hasSound, backingStore, forcedFSAA;
};
struct FC4ConfigRulesHost {
    void* context;
    // The property callbacks take the renderer pointer passed to Init.
    void (*retain)(void*);
    void (*release)(void*);
    int32_t (*find)(void*, uint32_t kind, FC4ConfigRulesBytes);
    uint32_t (*get)(void*, uint32_t kind, uint32_t index);
    void (*set)(void*, uint32_t kind, uint32_t index, uint32_t valueBits);
    uint32_t (*count)(void*, uint32_t kind);
    FC4ConfigRulesBytes (*name)(void*, uint32_t kind, uint32_t index);
    // All callbacks below take context. read: 1 opened, 0 missing, -1 error.
    int32_t (*read)(void*, FC4ConfigRulesBytes path, FC4ConfigRulesBytes* output);
    // Supply real measurements, including a completed GPU probe. False refuses
    // Init/explicit-log without installing fabricated fallback data.
    bool (*measure)(void*, FC4ConfigRulesMeasurement*);
    bool (*logOpen)(void*, FC4ConfigRulesBytes path);
    bool (*logIsOpen)(void*);
    bool (*logWrite)(void*, FC4ConfigRulesBytes);
    bool (*logClose)(void*);
    uint8_t (*lowercase)(void*, uint8_t);
};
struct FC4ConfigRulesLogInput {
    FC4ConfigRulesBytes appDirectory, writeLog, date;
    uint8_t hour, minute;
    const uint8_t* preferences; // null or 12 application bytes in spec SUMMARY order
};
// Every callback is required, must not unwind/reenter, and outlives the owner.
// State calls lock a mutex; the caller excludes destroy from concurrent calls.
// kind: 0 bool, 1 int, 2 float. Floats cross as raw binary32 bits.
// Status: 0 success, 1 parse fallback, -1 invalid, -2 missing acquisition,
// -3 unsupported core profile, -100 contained Rust panic. Never native returns.
extern "C" {
void* fc4_cfgrules_owner_create(const FC4ConfigRulesHost*);
void fc4_cfgrules_owner_destroy(void*);
int32_t fc4_cfgrules_owner_init(const void*, void* renderer, FC4ConfigRulesBytes data,
                              FC4ConfigRulesBytes user, bool prefsFirstTime);
int32_t fc4_cfgrules_owner_evaluate(const void*, const FC4ConfigRulesLogInput*);
int32_t fc4_cfgrules_owner_log(const void*, FC4ConfigRulesBytes path,
                             const FC4ConfigRulesLogInput*, bool* opened);
int32_t fc4_cfgrules_owner_apply(const void*, uint32_t group, uint32_t option);
int32_t fc4_cfgrules_owner_auto(const void*, uint32_t group, uint32_t* output);
int32_t fc4_cfgrules_owner_parse_error(const void*, void* context, void (*emit)(void*, FC4ConfigRulesBytes));
}
