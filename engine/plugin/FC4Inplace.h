// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>

namespace fc4 {
// Inert unless compare/inplace selects rows, or watercraft_shadow selects its
// four complete value-method bindings.
void InplaceStartup();
void InplaceCityLoaded();
void InplaceReport();
// Complete watercraft value-method samples; called alongside the monthly shadow.
void InplaceWatercraftMethods(const char* date);
// Optional per-row outgoing-call interposition. Context and callbacks outlive the
// run; registration happens before Startup, callbacks must not reconfigure it.
// No serialized data includes these pointers. Internal C++ interface, not DLL ABI.
struct CompareCallView {
    const char* name;
    const char* source;
    const char* phase; // original / ours
    const uint32_t* arguments;
};
struct CompareInterposition {
    void* context;
    void (*begin)(void*,const CompareCallView&);
    void (*end)(void*,const CompareCallView&);
};
bool CompareAttach(const char* name,const CompareInterposition* callbacks);
// Selected native observers see every invocation, including nested calls. They
// bypass sampling, copying, persistence and replacement; native machine results
// are preserved by the existing entry shell. Register before InplaceStartup.
using CompareNativeObserver = void (*)(const char*, uint32_t, const uint32_t*, uint32_t, bool);
bool CompareObserveNative(const char* owner, CompareNativeObserver observer);
}
