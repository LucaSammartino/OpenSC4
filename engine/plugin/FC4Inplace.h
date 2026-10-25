// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>

namespace fc4 {
// Every entry point is inert unless compare (or its inplace alias) selects rows.
void InplaceStartup();
void InplaceCityLoaded();
void InplaceReport();
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
}
