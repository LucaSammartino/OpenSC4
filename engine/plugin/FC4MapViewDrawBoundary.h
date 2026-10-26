// SPDX-License-Identifier: GPL-3.0-or-later
// Included in the pilot's private namespace, after Frame and tableMutex.
// These are owner-call boundaries, not hooks on collaborator internals.
std::map<std::pair<void**, unsigned>, void*> boundaryOriginal;
bool BoundaryHook(void* object, unsigned slot, void* hook) {
    if (!object) return false;
    auto** table = *static_cast<void***>(object);
    std::lock_guard<std::mutex> lock(tableMutex);
    const auto key = std::make_pair(table, slot);
    if (boundaryOriginal.count(key)) return table[slot] == hook;
    boundaryOriginal.emplace(key, table[slot]);
    return Patch(table, slot, hook);
}
void* BoundaryOriginal(void* object, unsigned slot) {
    std::lock_guard<std::mutex> lock(tableMutex);
    return boundaryOriginal.at({*static_cast<void***>(object), slot});
}
void ObserveQuery(Frame& capture, unsigned slot, void* receiver,
                  std::initializer_list<uint32_t> args, std::initializer_list<uint32_t> result) {
    const auto id = capture.objects.Borrow(receiver);
    Event request{Domain::Query, slot, id, args, {}};
    Event response{Domain::Return, slot, id, result, {}};
    capture.inputs.Observe(request, response);
    capture.native.Append(std::move(request)); capture.native.Append(std::move(response));
}
template<unsigned Slot, class... Args>
uint32_t __fastcall BoundaryTap(void* self, void*, Args... args) {
    using Fn = uint32_t(__fastcall*)(void*, void*, Args...);
    auto* capture = frame && frame->clearProfile && !frame->boundaryDepth ? frame : nullptr;
    if (capture) {
        bool ownerCall = false;
        if constexpr (Slot == 11) ownerCall = self == static_cast<uint8_t*>(capture->owner) + 8;
        if constexpr (Slot == 37) ownerCall = self == capture->parent;
        if constexpr (Slot == 2 || Slot == 3) ownerCall = self == capture->child;
        if constexpr (Slot == 47) ownerCall = self == capture->childWindow;
        if (!ownerCall) capture = nullptr;
    }
    if (capture) ++capture->boundaryDepth;
    const auto result = reinterpret_cast<Fn>(BoundaryOriginal(self, Slot))(self, nullptr, args...);
    if (capture) {
        --capture->boundaryDepth;
        try {
            const std::array<uint32_t, sizeof...(Args)> words{static_cast<uint32_t>(args)...};
            if constexpr (Slot == 11) {
                capture->parent = reinterpret_cast<void*>(result);
                ObserveQuery(*capture, Slot, self, {}, {capture->objects.Borrow(capture->parent)});
                if (!BoundaryHook(capture->parent, 37, reinterpret_cast<void*>(&BoundaryTap<37, uint32_t, uint32_t, uint32_t>)))
                    capture->native.Refuse("parent child hook");
            }
            if constexpr (Slot == 37) {
                capture->child = (result & 0xff) && words[2] ? *reinterpret_cast<void**>(words[2]) : nullptr;
                ObserveQuery(*capture, Slot, self, {words[0], words[1]},
                    {(result & 0xff) ? 1u : 0u, capture->objects.Borrow(capture->child)});
                if ((result & 0xff) &&
                    (!BoundaryHook(capture->child, 2, reinterpret_cast<void*>(&BoundaryTap<2>)) ||
                     !BoundaryHook(capture->child, 3, reinterpret_cast<void*>(&BoundaryTap<3>))))
                    capture->native.Refuse("minimap query hooks");
            }
            if constexpr (Slot == 3) {
                capture->childWindow = reinterpret_cast<void*>(result);
                ObserveQuery(*capture, Slot, self, {}, {capture->objects.Borrow(capture->childWindow)});
                if (!BoundaryHook(capture->childWindow, 47, reinterpret_cast<void*>(&BoundaryTap<47, uint32_t>)))
                    capture->native.Refuse("child rectangle hook");
            }
            if constexpr (Slot == 47) {
                // GetArea's rectangle is the output; its scratch EAX is unused.
                const auto* r = reinterpret_cast<const uint32_t*>(words[0]);
                ObserveQuery(*capture, Slot, self, {}, {r[0], r[1], r[2], r[3]});
            }
            if constexpr (Slot == 2)
                capture->native.Call(Domain::Surface, 2, capture->objects.Borrow(self));
        } catch (...) { capture->native.Refuse("owner boundary capture"); }
    }
    return result;
}
using ClockFn = uint32_t(__cdecl*)();
ClockFn clockOriginal = nullptr;
uint32_t __cdecl Clock() {
    const auto result = clockOriginal();
    if (frame && frame->clearProfile && !frame->boundaryDepth) try {
        ObserveQuery(*frame, 0x009F361F, nullptr, {}, {result});
    } catch (...) { frame->native.Refuse("clock capture"); }
    return result;
}
const Event* ReplayQuery(Frame& capture, unsigned slot, uint32_t receiver,
                         std::initializer_list<uint32_t> args, size_t words) {
    Event request{Domain::Query, slot, receiver, args, {}};
    capture.core.Append(request);
    const auto* response = capture.inputs.Read(request);
    if (!response || response->words.size() != words) { capture.core.Refuse("query replay order/payload"); return nullptr; }
    capture.core.Append(*response);
    return response;
}
bool __cdecl ReplayClear(void* context, uint32_t operation, uint32_t* output) {
    auto& capture = *static_cast<Frame*>(context);
    try {
        if (operation == 0) {
            const auto* parent = ReplayQuery(capture, 11, capture.objects.Borrow(static_cast<uint8_t*>(capture.owner) + 8), {}, 1);
            if (!parent || parent->words[0] == 0) return false;
            const auto* child = ReplayQuery(capture, 37, parent->words[0], {0x4203, 0xCA318385}, 2);
            if (!child) return false;
            *output = child->words[0];
            if (*output) {
                const auto* window = ReplayQuery(capture, 3, child->words[1], {}, 1);
                if (!window || !ReplayQuery(capture, 47, window->words[0], {}, 4)) return false;
            }
        } else if (operation == 1) {
            capture.core.Call(Domain::Surface, 4, capture.objects.Borrow(capture.child), {0});
            *output = 0;
        } else if (operation == 2) {
            const auto* clock = ReplayQuery(capture, 0x009F361F, 0, {}, 1);
            if (!clock) return false;
            *output = clock->words[0];
        } else return false;
        return true;
    } catch (...) { capture.core.Refuse("clear replay allocation"); return false; }
}
