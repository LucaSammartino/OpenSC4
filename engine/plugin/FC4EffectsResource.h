// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include "cIGZUnknown.h"
#include <cstddef>

class cIGZIStream;
class cIGZOStream;

constexpr uint32_t kFC4_CLSID_cSC4EffectsResource = 0x2A5118AD;
constexpr uint32_t kFC4_IID_cISC4EffectsResource = 0xCA51189B;

// effects.md 18.9.1. This interface deliberately has no virtual destructor:
// slot 8 is the class ID, and persistence is a separate identity at +0x11C.
class FC4IEffectsResource : public cIGZUnknown {
public:
    virtual void* SetName(const char* name) = 0;
    virtual const char* GetName() = 0;
    virtual void Publish(cIGZUnknown* destination) = 0;
    virtual bool Write(cIGZOStream* stream) = 0;
    virtual bool Read(cIGZIStream* stream) = 0;
    virtual uint32_t ClassID() = 0;
};

// Raw GZCOM creator: count zero; the director acquires the requested identity.
cIGZUnknown* CreateFC4EffectsResource();

// effects.md 18.9.7: borrowed counted bytes, raw index and hidden iterator result.
struct FC4EffectsVisualName {
    const char* begin;
    const char* end;
    const char* capacity;
    uint32_t index;
};
struct FC4EffectsVisualIterator { void* node; void* dictionary; };
static_assert(sizeof(FC4EffectsVisualName) == 0x10);
static_assert(sizeof(FC4EffectsVisualIterator) == 8);

// The collection remains a separate native service. These calls construct and
// destroy C, and publish C into another collection; none calls native resource
// Read/Write or creates a native effects resource. Tests inject an isolated C.
struct FC4EffectsResourceBindings {
    void (*constructCollection)(void*);
    void (*destroyCollection)(void*);
    void (*publishCollection)(void* source, void* destination);
    void* (*allocateString)(uint32_t);
    void (*freeString)(void*);
    const void* stringVtable;
    // Family numbers are the directory component types (0 particle, 1 decal,
    // 3..8 built-ins, 16 dynamic, 2 visual); 17 is the collection camera.
    // These are borrowed typed operations, never native resource Read/Write.
    void (*constructDescription)(uint32_t family, void* value) = nullptr;
    void (*releaseDescription)(uint32_t family, void* value) = nullptr;
    void (*readDescription)(uint32_t family, uint16_t version, cIGZIStream*, void*) = nullptr;
    void (*writeDescription)(uint32_t family, cIGZOStream*, const void*) = nullptr;
    void* (*allocateObject)(uint32_t) = nullptr; // allocator C; containers use B above
    void (*growVisualNames)(void* dictionary, uint32_t count) = nullptr;
    FC4EffectsVisualIterator* (*insertVisualName)(void* dictionary,
        FC4EffectsVisualIterator* result, const FC4EffectsVisualName* pair) = nullptr;
};
cIGZUnknown* CreateFC4EffectsResourceWithBindings(const FC4EffectsResourceBindings& bindings);
