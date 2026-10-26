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
};
cIGZUnknown* CreateFC4EffectsResourceWithBindings(const FC4EffectsResourceBindings& bindings);
