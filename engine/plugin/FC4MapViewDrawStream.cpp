// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4MapViewDrawStream.h"
#include "FC4DrawStream.h"
#include "FC4HarnessLogic.h"
#include "FC4InplaceDetour.h"
#include "FC4Log.h"
#include "compat/FC4Compat.h"
#include <windows.h>
#include <array>
#include <atomic>
#include <map>
#include <mutex>
#include <stdexcept>

// Private static-link transport, deliberately absent from the public DLL ABI.
extern "C" bool fc4_drawstream_raster(const uint8_t*, uint32_t, uint32_t, uint32_t, uint32_t,
    const uint32_t*, uint32_t, bool, uint32_t*);
extern "C" bool fc4_drawstream_notification(bool, bool, uint32_t, const uint32_t*, uint32_t,
    const uint32_t*, uint32_t*);

namespace fc4 {
namespace {
using namespace drawstream;
struct Counter {
    std::atomic<uint64_t> calls{0}, complete{0}, mismatches{0}, operations{0}, exact{0};
    void Add(const Stream& native, const Stream& core, bool full) {
        Inputs inputs;
        const auto result = Compare(native, core, full ? All : 0, inputs);
        const auto detail = Inspect(native, core);
        const auto call = ++calls;
        if (result.complete) { ++complete; if (!result.exact) ++mismatches; }
        operations += native.Events().size(); exact += detail.exactOperations;
        if (call <= 3 || (std::strcmp(detail.field, "none") != 0 && call <= 20))
            Log("drawstream sample call=%llu operations=%u exact=%u complete=%u first=%u field=%s[%u] native=0x%llX core=0x%llX valid=%u/%u",
                call, static_cast<unsigned>(native.Events().size()), static_cast<unsigned>(detail.exactOperations),
                result.complete, static_cast<unsigned>(detail.operation), detail.field, static_cast<unsigned>(detail.element),
                detail.native, detail.core, native.Valid(), core.Valid());
    }
} updateCount, rasterCount[2], notificationCount, uiCount;

template<class T> T Load(const void* object, size_t offset) {
    T value; std::memcpy(&value, static_cast<const uint8_t*>(object) + offset, sizeof(value)); return value;
}
bool Patch(void** table, unsigned slot, void* hook) {
    DWORD old;
    if (!VirtualProtect(table + slot, 4, PAGE_READWRITE, &old)) return false;
    InterlockedExchangePointer(table + slot, hook);
    DWORD ignored; VirtualProtect(table + slot, 4, old, &ignored); return true;
}
void* Detour(uintptr_t site, void* hook) {
    if (!site) return nullptr;
    auto* trampoline = static_cast<uint8_t*>(VirtualAlloc(nullptr, 64, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE));
    if (!trampoline) return nullptr;
    auto* entry = reinterpret_cast<uint8_t*>(site);
    const auto size = inplace::Trampoline(entry, 24, site, trampoline, reinterpret_cast<uintptr_t>(trampoline));
    if (!size) { VirtualFree(trampoline, 0, MEM_RELEASE); return nullptr; }
    DWORD old;
    if (!VirtualProtect(entry, size, PAGE_EXECUTE_READWRITE, &old)) { VirtualFree(trampoline, 0, MEM_RELEASE); return nullptr; }
    inplace::Jump(entry, site, reinterpret_cast<uintptr_t>(hook));
    for (unsigned i = 5; i < size; ++i) entry[i] = 0x90;
    DWORD ignored; VirtualProtect(entry, size, old, &ignored);
    FlushInstructionCache(GetCurrentProcess(), entry, size);
    return trampoline;
}
using OwnerBytes = std::array<uint8_t, 0x9e8>;
// DS29-1 dynamic state: no capacity/padding bytes enter the transcript.
// Nonempty highlight trees need the full effect/occupant adapter, so refuse
// complete credit for that profile rather than omitting their payloads.
void DynamicState(const void* owner, Stream& stream) {
    auto bytes = [&](size_t beginOffset, size_t endOffset, uint32_t operation) {
        const auto* begin = Load<const uint8_t*>(owner, beginOffset);
        const auto* end = Load<const uint8_t*>(owner, endOffset);
        const auto size = reinterpret_cast<uintptr_t>(end) - reinterpret_cast<uintptr_t>(begin);
        if (size > 1024 * 1024) { stream.Refuse("dynamic state capacity"); return; }
        stream.Data(Domain::State, operation, 0, begin, size);
    };
    bytes(0x988, 0x98c, 0x984);
    bytes(0x9bc, 0x9c0, 0x9bc);
    const auto* sentinel = Load<const void*>(owner, 0x9c8);
    if (!sentinel) { stream.Refuse("missing pending sentinel"); return; }
    const void* node = Load<const void*>(sentinel, 0);
    std::vector<uint32_t> pending;
    while (node != sentinel) {
        if (!node || pending.size() == 4096) { stream.Refuse("pending list capacity"); return; }
        pending.push_back(Load<uint32_t>(node, 8));
        node = Load<const void*>(node, 0);
    }
    stream.Data(Domain::State, 0x9c8, 0, pending.data(), pending.size() * 4);
    for (const auto offset : {0x968u, 0x974u}) {
        const auto* header = Load<const void*>(owner, offset);
        const auto count = Load<uint32_t>(owner, offset + 4);
        if (!header || count != 0 || Load<const void*>(header, 4) != nullptr) {
            stream.Refuse("nonempty highlight collection requires collaborator adapter");
            continue;
        }
        stream.Call(Domain::State, offset, 0, {0});
    }
}
void Append(Stream& target, const Stream& source) {
    for (const auto& event : source.Events()) target.Append(event);
    if (!source.Valid()) target.Refuse(source.Reason().c_str());
}
struct Surface {
    uint32_t width = 0, height = 0;
    uint8_t* bits = nullptr;
    int32_t pitch = 0;
    std::vector<uint32_t> native, core;
};
struct Frame {
    void* owner = nullptr;
    Objects objects;
    Stream native, core;
    std::map<void*, Surface> surfaces;
};
struct Raster {
    void* buffer = nullptr;
    Surface* surface = nullptr;
    Stream native, core;
    std::vector<uint8_t> cells;
    std::array<uint32_t, 256> palette{};
    uint32_t outer = 0, inner = 0, rotation = 0;
    bool locked = false, signedBytes = false, captured = false;
};
thread_local Frame* frame = nullptr;
thread_local Raster* raster = nullptr;
template<class T> struct Scope {
    T*& location; T* previous;
    Scope(T*& location, T* value) : location(location), previous(location) { location = value; }
    ~Scope() { location = previous; }
};
std::mutex tableMutex;
void** bufferTable = nullptr;
bool bufferHookOK = false;
uintptr_t bufferOriginal[36]{};
bool CopyRows(Surface& surface, std::vector<uint32_t>& output) {
    const int64_t pitch = surface.pitch;
    if (!surface.bits || pitch == 0 || pitch < static_cast<int64_t>(surface.width) * 4 ||
        pitch > 16 * 1024 * 1024 || !surface.width || !surface.height ||
        static_cast<uint64_t>(surface.width) * surface.height > 4 * 1024 * 1024) return false;
    output.resize(static_cast<size_t>(surface.width) * surface.height);
    for (uint32_t y = 0; y < surface.height; ++y)
        std::memcpy(output.data() + static_cast<size_t>(y) * surface.width,
            surface.bits + static_cast<size_t>(y) * surface.pitch, static_cast<size_t>(surface.width) * 4);
    return true;
}
template<unsigned Slot, class... Args>
uint32_t __fastcall BufferTap(void* self, void*, Args... args) {
    using Fn = uint32_t(__fastcall*)(void*, void*, Args...);
    auto* capture = raster && raster->buffer == self ? raster : nullptr;
    Surface* surface = nullptr;
    if (frame) {
        const auto found = frame->surfaces.find(self);
        if (found != frame->surfaces.end()) surface = &found->second;
    }
    // Snapshot while still locked; do not touch bits after Unlock.
    if constexpr (Slot == 7) {
        if (surface) try {
            if (!CopyRows(*surface, surface->native) && capture) capture->native.Refuse("surface rows unavailable");
        } catch (...) { if (capture) capture->native.Refuse("surface snapshot allocation"); }
    }
    const auto result = reinterpret_cast<Fn>(bufferOriginal[Slot])(self, nullptr, args...);
    if (surface) try {
        if constexpr (Slot == 34) surface->bits = reinterpret_cast<uint8_t*>(result);
        if constexpr (Slot == 35) surface->pitch = static_cast<int32_t>(result);
        if (capture) {
            if constexpr (Slot == 6) {
                capture->locked = (result & 0xff) != 0;
                capture->native.Call(Domain::Surface, 6, 1, {static_cast<uint32_t>(args)..., capture->locked ? 1u : 0u});
            }
            if constexpr (Slot == 34) capture->native.Call(Domain::Surface, 34, 1, {result ? 1u : 0u});
            if constexpr (Slot == 35) {
                capture->native.Call(Domain::Surface, 35, 1, {result});
                capture->captured = CopyRows(*surface, surface->core);
            }
            if constexpr (Slot == 7) {
                capture->native.Call(Domain::Surface, 7, 1, {static_cast<uint32_t>(args)...});
                capture->native.Data(Domain::Surface, 100, 1, surface->native.data(), surface->native.size() * 4,
                    {surface->width, surface->height, 9, 32});
            }
        }
    } catch (...) { if (capture) capture->native.Refuse("surface recording allocation"); }
    return result;
}
bool BufferHooks(void* buffer) {
    std::lock_guard<std::mutex> lock(tableMutex);
    auto** table = *static_cast<void***>(buffer);
    if (bufferTable) return bufferTable == table && bufferHookOK;
    for (unsigned i = 0; i < 36; ++i) bufferOriginal[i] = reinterpret_cast<uintptr_t>(table[i]);
    bufferTable = table;
    const bool ok = Patch(table, 6, reinterpret_cast<void*>(&BufferTap<6, uint32_t>)) &&
        Patch(table, 7, reinterpret_cast<void*>(&BufferTap<7, uint32_t>)) &&
        Patch(table, 34, reinterpret_cast<void*>(&BufferTap<34>)) && Patch(table, 35, reinterpret_cast<void*>(&BufferTap<35>));
    bufferHookOK = ok;
    return ok;
}
using FillFn = void(__cdecl*)(const void*, uint32_t, uint32_t, void*, uint32_t, uint32_t, const uint32_t*, uint32_t);
FillFn fillOriginal[2]{};
template<unsigned Signed>
void __cdecl Fill(const void* rows, uint32_t outer, uint32_t inner, void* buffer,
                  uint32_t width, uint32_t height, const uint32_t* palette, uint32_t rotation) {
    if (!frame) { fillOriginal[Signed](rows, outer, inner, buffer, width, height, palette, rotation); return; }
    Raster capture;
    capture.buffer = buffer; capture.outer = outer; capture.inner = inner; capture.rotation = rotation; capture.signedBytes = Signed != 0;
    try {
        if (!buffer || !rows || !palette || !outer || !inner || static_cast<uint64_t>(outer) * inner > 4 * 1024 * 1024)
            capture.native.Refuse("grid capacity");
        else {
            auto& surface = frame->surfaces[buffer];
            surface.width = width; surface.height = height; capture.surface = &surface;
            std::memcpy(capture.palette.data(), palette, 1024);
            capture.cells.reserve(static_cast<size_t>(outer) * inner);
            // SPEC-GAP: G uses the begin/end/capacity vector header; refuse
            // malformed lengths rather than interpreting unrelated storage.
            const auto* records = Load<const uint8_t*>(rows, 0);
            const auto* recordsEnd = Load<const uint8_t*>(rows, 4);
            if (!records || reinterpret_cast<uintptr_t>(recordsEnd) - reinterpret_cast<uintptr_t>(records) < static_cast<size_t>(outer) * 12)
                throw std::length_error("row vector length");
            for (uint32_t i = 0; i < outer; ++i) {
                const auto* row = Load<const uint8_t*>(records, static_cast<size_t>(i) * 12);
                const auto* rowEnd = Load<const uint8_t*>(records, static_cast<size_t>(i) * 12 + 4);
                if (!row || reinterpret_cast<uintptr_t>(rowEnd) - reinterpret_cast<uintptr_t>(row) < inner)
                    throw std::length_error("cell vector length");
                capture.cells.insert(capture.cells.end(), row, row + inner);
            }
            if (!BufferHooks(buffer)) capture.native.Refuse("unsupported buffer vtable");
        }
    } catch (...) { capture.native.Refuse("input snapshot allocation"); }
    {
        Scope<Raster> scope(raster, &capture);
        fillOriginal[Signed](rows, outer, inner, buffer, width, height, palette, rotation);
    }
    try {
        capture.core.Call(Domain::Surface, 6, 1, {0x8040, capture.locked ? 1u : 0u});
        if (capture.locked && capture.surface) {
            auto& surface = *capture.surface;
            capture.core.Call(Domain::Surface, 34, 1, {surface.bits ? 1u : 0u});
            capture.core.Call(Domain::Surface, 35, 1, {static_cast<uint32_t>(surface.pitch)});
            if (!capture.captured || !fc4_drawstream_raster(capture.cells.data(), outer, inner, width, height,
                    capture.palette.data(), rotation, Signed != 0, surface.core.data())) capture.core.Refuse("raster replay refused");
            capture.core.Call(Domain::Surface, 7, 1, {0x8040});
            capture.core.Data(Domain::Surface, 100, 1, surface.core.data(), surface.core.size() * 4, {width, height, 9, 32});
        }
        rasterCount[Signed].Add(capture.native, capture.core, true);
        for (const auto& event : capture.native.Events()) frame->native.Append(event);
        for (const auto& event : capture.core.Events()) frame->core.Append(event);
        if (!capture.native.Valid()) frame->native.Refuse("incomplete raster input");
        if (!capture.core.Valid()) frame->core.Refuse("incomplete raster replay");
    } catch (...) { frame->core.Refuse("raster replay allocation"); }
}
using OverlayFn = void(__fastcall*)(void*, void*, void*);
OverlayFn overlayOriginal = nullptr;
void __fastcall SetOverlay(void* self, void*, void* buffer) {
    if (frame) try {
        const auto receiver = frame->objects.Borrow(self);
        if (!buffer) {
            frame->native.Call(Domain::Surface, 4, receiver, {0});
            frame->core.Call(Domain::Surface, 4, receiver, {Load<uint32_t>(frame->owner, 0x144) == 0 ? 0u : 1u});
        } else {
            const auto found = frame->surfaces.find(buffer);
            if (found == frame->surfaces.end()) {
                frame->native.Call(Domain::Surface, 4, receiver, {1});
                frame->core.Refuse("presentation without captured fill");
            } else {
                const auto& surface = found->second;
                frame->native.Data(Domain::Surface, 4, receiver, surface.native.data(), surface.native.size() * 4,
                    {surface.width, surface.height, 9, 32});
                frame->core.Data(Domain::Surface, 4, receiver, surface.core.data(), surface.core.size() * 4,
                    {surface.width, surface.height, 9, 32});
            }
        }
    } catch (...) { frame->core.Refuse("presentation capture allocation"); }
    // The one real presentation; replay never calls the borrowed child.
    overlayOriginal(self, nullptr, buffer);
}
using UpdateFn = void(__fastcall*)(void*, void*);
UpdateFn updateOriginal = nullptr;
void __fastcall Update(void* self, void*) {
    Frame capture; capture.owner = self;
    OwnerBytes before; std::memcpy(before.data(), self, before.size());
    Stream dynamicBefore;
    if (before[0x142] == 0) try { DynamicState(self, dynamicBefore); }
    catch (...) { dynamicBefore.Refuse("entry snapshot allocation"); }
    {
        Scope<Frame> scope(frame, &capture);
        updateOriginal(self, nullptr);
    }
    try {
        // Active Update currently compares its raster/presentation substream.
        // Registration, source queries, clock and reference events are not credited.
        const bool inactive = before[0x142] == 0;
        if (inactive) {
            capture.native.Data(Domain::State, 0, 0, self, before.size());
            capture.core.Data(Domain::State, 0, 0, before.data(), before.size());
            DynamicState(self, capture.native);
            Append(capture.core, dynamicBefore);
        }
        const auto detail = Inspect(capture.native, capture.core);
        if (updateCount.calls.load() < 5 || std::strcmp(detail.field, "none") != 0)
            Log("drawstream pilot cSC4WinMapView::Update source=%u layer=%u operations=%u exact=%u first=%u field=%s[%u] native=0x%llX core=0x%llX complete=%u",
                Load<uint32_t>(before.data(), 0x144), Load<uint32_t>(before.data(), 0x148),
                static_cast<unsigned>(capture.native.Events().size()), static_cast<unsigned>(detail.exactOperations),
                static_cast<unsigned>(detail.operation), detail.field, static_cast<unsigned>(detail.element), detail.native, detail.core, inactive);
        updateCount.Add(capture.native, capture.core, inactive);
    } catch (...) { ++updateCount.calls; }
}

struct Notification {
    void* message = nullptr;
    Stream native, core;
    uint32_t type = 0, values[2]{};
    bool typeSeen = false, valueSeen[2]{};
};
thread_local Notification* notification = nullptr;
void** messageTable = nullptr;
bool messageHookOK = false;
uintptr_t messageOriginal[16]{};
template<unsigned Slot>
uint32_t __fastcall MessageTap(void* self, void*) {
    using Fn = uint32_t(__fastcall*)(void*, void*);
    const auto result = reinterpret_cast<Fn>(messageOriginal[Slot])(self, nullptr);
    auto* capture = notification && notification->message == self ? notification : nullptr;
    if (capture) try {
        if constexpr (Slot == 4) {
            capture->type = result; capture->typeSeen = true;
            capture->native.Call(Domain::Message, Slot, 1, {result});
        } else {
            constexpr unsigned index = Slot == 10 ? 0 : 1;
            capture->valueSeen[index] = result != 0;
            if (result) capture->values[index] = Load<uint32_t>(reinterpret_cast<void*>(result), 4);
            capture->native.Call(Domain::Message, Slot, 1, {result ? 1u : 0u, capture->values[index]});
        }
    } catch (...) { capture->native.Refuse("message result capture"); }
    return result;
}
bool MessageHooks(void* message) {
    std::lock_guard<std::mutex> lock(tableMutex);
    auto** table = *static_cast<void***>(message);
    if (messageTable) return table == messageTable && messageHookOK;
    for (unsigned i = 0; i < 16; ++i) messageOriginal[i] = reinterpret_cast<uintptr_t>(table[i]);
    messageTable = table;
    messageHookOK = Patch(table, 4, reinterpret_cast<void*>(&MessageTap<4>)) &&
        Patch(table, 10, reinterpret_cast<void*>(&MessageTap<10>)) && Patch(table, 15, reinterpret_cast<void*>(&MessageTap<15>));
    return messageHookOK;
}
using MessageFn = bool(__fastcall*)(void*, void*, void*);
MessageFn notificationOriginal = nullptr, uiOriginal = nullptr;
bool __fastcall Notify(void* self, void*, void* message) {
    auto* owner = static_cast<uint8_t*>(self) - 4; // map-view §1.3
    OwnerBytes before; std::memcpy(before.data(), owner, before.size());
    Notification capture; capture.message = message;
    Stream dynamicBefore;
    std::vector<uint32_t> subscriptions;
    try {
        DynamicState(owner, dynamicBefore);
        if (!message || !MessageHooks(message)) capture.native.Refuse("unsupported notification interface");
        const auto* begin = Load<const uint32_t*>(owner, 0x9bc);
        const auto* end = Load<const uint32_t*>(owner, 0x9c0);
        const auto bytes = reinterpret_cast<uintptr_t>(end) - reinterpret_cast<uintptr_t>(begin);
        if (bytes > 4096 * 4 || bytes % 4) capture.native.Refuse("subscription capacity");
        else if (bytes) subscriptions.assign(begin, end);
    } catch (...) { capture.native.Refuse("notification entry capture"); }
    bool result;
    {
        Scope<Notification> scope(notification, &capture);
        result = notificationOriginal(self, nullptr, message);
    }
    try {
        capture.core.Call(Domain::Message, 4, 1, {capture.type});
        if (capture.type == 0xa6b79602) {
            capture.core.Call(Domain::Message, 10, 1, {capture.valueSeen[0] ? 1u : 0u, capture.values[0]});
            capture.core.Call(Domain::Message, 15, 1, {capture.valueSeen[1] ? 1u : 0u, capture.values[1]});
        }
        uint32_t output[3]{before[0x142] ? 1u : 0u, before[0x9d9] ? 1u : 0u, 0};
        const uint32_t* values = capture.valueSeen[0] && capture.valueSeen[1] ? capture.values : nullptr;
        const bool replay = capture.typeSeen && fc4_drawstream_notification(before[0x142] != 0, before[0x9d9] != 0,
            capture.type, subscriptions.data(), static_cast<uint32_t>(subscriptions.size()), values, output);
        const bool pure = capture.type != 0x26d31ec1 && capture.type != 0x26d31ec2;
        if (!replay || output[2] != 0 || !pure) capture.core.Refuse("notification requires collaborator adapter");
        before[0x142] = static_cast<uint8_t>(output[0]); before[0x9d9] = static_cast<uint8_t>(output[1]);
        capture.native.Data(Domain::State, 0, 0, owner, before.size());
        capture.core.Data(Domain::State, 0, 0, before.data(), before.size());
        DynamicState(owner, capture.native);
        Append(capture.core, dynamicBefore);
        capture.native.Call(Domain::Return, 3, 0, {result ? 1u : 0u});
        capture.core.Call(Domain::Return, 3, 0, {1});
        notificationCount.Add(capture.native, capture.core, replay && output[2] == 0 && pure);
    } catch (...) { ++notificationCount.calls; }
    return result;
}
bool __fastcall UIMessage(void* self, void*, void* message) {
    // Inherited window delegation, selection, legend and commands remain unbound.
    ++uiCount.calls;
    return uiOriginal(self, nullptr, message);
}
void Method(const Counter& counter, const char* address, const char* name, int32_t julian) {
    const auto date = harness::CivilFromJulian(julian);
    Log("drawstream method %s %s (month) %04d-%02d-%02d: calls=%llu complete=%llu mismatches=%llu; resyncs=0",
        address, name, date.year, date.month, date.day, counter.calls.load(), counter.complete.load(), counter.mismatches.load());
}
} // namespace

void MapViewDrawStreamStartup() {
    auto** child = compat::MapViewDrawStreamTable(0);
    auto** target = compat::MapViewDrawStreamTable(1);
    auto** window = compat::MapViewDrawStreamTable(2);
    if (!child || !target || !window) { Log("drawstream MapView tables unavailable"); return; }
    overlayOriginal = reinterpret_cast<OverlayFn>(child[4]);
    notificationOriginal = reinterpret_cast<MessageFn>(target[3]);
    uiOriginal = reinterpret_cast<MessageFn>(window[3]);
    fillOriginal[0] = reinterpret_cast<FillFn>(Detour(compat::MapViewDrawStreamEntry(1), reinterpret_cast<void*>(&Fill<0>)));
    fillOriginal[1] = reinterpret_cast<FillFn>(Detour(compat::MapViewDrawStreamEntry(2), reinterpret_cast<void*>(&Fill<1>)));
    updateOriginal = reinterpret_cast<UpdateFn>(Detour(compat::MapViewDrawStreamEntry(0), reinterpret_cast<void*>(&Update)));
    const bool overlay = Patch(child, 4, reinterpret_cast<void*>(&SetOverlay));
    const bool notify = Patch(target, 3, reinterpret_cast<void*>(&Notify));
    const bool ui = Patch(window, 3, reinterpret_cast<void*>(&UIMessage));
    Log("drawstream MapView installed Update=%u u8=%u i8=%u overlay=%u notify=%u ui=%u; active Update credit=0 (raster boundary only)",
        updateOriginal != nullptr, fillOriginal[0] != nullptr, fillOriginal[1] != nullptr, overlay, notify, ui);
}
void MapViewDrawStreamMonth(int32_t julian) {
    const auto date = harness::CivilFromJulian(julian);
    Log("drawstream pilot cSC4WinMapView (month) %04d-%02d-%02d: draws=%llu operations=%llu exact=%llu; active owner complete=0",
        date.year, date.month, date.day, updateCount.calls.load(), updateCount.operations.load(), updateCount.exact.load());
    Method(updateCount, "0x007A2F60", "cSC4WinMapView::Update", julian);
    Method(notificationCount, "0x007A54D0", "cSC4WinMapView::DoMessage", julian);
    Method(uiCount, "0x007A56E0", "cSC4WinMapView::DoMessage", julian);
    Method(rasterCount[0], "0x0079ED90", "MakeMapColorBuffer<unsigned-byte>", julian);
    Method(rasterCount[1], "0x0079F230", "MakeMapColorBuffer<signed-byte>", julian);
}
} // namespace fc4
