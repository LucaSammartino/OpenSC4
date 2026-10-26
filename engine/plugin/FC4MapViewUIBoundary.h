// SPDX-License-Identifier: GPL-3.0-or-later
// Private pilot adapter. Inherited handling is a delegated boundary per
// map-view.md 1.3, with the same O+8 and borrowed message identities.
struct UIFrame {
    void* owner = nullptr;
    void* message = nullptr;
    OwnerBytes before{}, delegatedExit{};
    Objects objects;
    Stream native, core, dynamicBefore, delegatedDynamic;
    uint32_t kind = 0, command = 0;
    unsigned delegates = 0;
    bool result = false;
};
thread_local UIFrame* uiFrame = nullptr;
MessageFn baseUIOriginal = nullptr;
bool __fastcall BaseUI(void* self, void*, void* message) {
    auto* capture = uiFrame && self == static_cast<uint8_t*>(uiFrame->owner) + 8 &&
        message == uiFrame->message ? uiFrame : nullptr;
    if (capture) try {
        ++capture->delegates;
        capture->native.Call(Domain::Message, 0x0099A276, capture->objects.Borrow(self),
            {capture->objects.Borrow(message)});
        capture->native.Data(Domain::State, 0, 0, capture->owner, capture->before.size());
        DynamicState(capture->owner, capture->native);
    } catch (...) { capture->native.Refuse("base UI entry capture"); }
    const auto result = baseUIOriginal(self, nullptr, message);
    if (capture) try {
        capture->result = result;
        std::memcpy(capture->delegatedExit.data(), capture->owner, capture->delegatedExit.size());
        DynamicState(capture->owner, capture->delegatedDynamic);
        capture->native.Call(Domain::Return, 0x0099A276, 0, {result ? 1u : 0u});
        capture->native.Data(Domain::State, 1, 0, capture->delegatedExit.data(), capture->delegatedExit.size());
        Append(capture->native, capture->delegatedDynamic);
    } catch (...) { capture->native.Refuse("base UI result capture"); }
    return result;
}
bool __cdecl ReplayUI(void* context, uint32_t* output) {
    auto& capture = *static_cast<UIFrame*>(context);
    if (capture.delegates != 1 || !baseUIOriginal) return false;
    try {
        capture.core.Call(Domain::Message, 0x0099A276,
            capture.objects.Borrow(static_cast<uint8_t*>(capture.owner) + 8), {capture.objects.Borrow(capture.message)});
        capture.core.Data(Domain::State, 0, 0, capture.before.data(), capture.before.size());
        Append(capture.core, capture.dynamicBefore);
        // The inherited handler's result/state are foreign inputs, not a second
        // execution. MapView itself must leave that returned state unchanged.
        capture.core.Call(Domain::Return, 0x0099A276, 0, {capture.result ? 1u : 0u});
        capture.core.Data(Domain::State, 1, 0, capture.delegatedExit.data(), capture.delegatedExit.size());
        Append(capture.core, capture.delegatedDynamic);
        *output = capture.result ? 1u : 0u;
        return true;
    } catch (...) { capture.core.Refuse("UI replay allocation"); return false; }
}
bool __fastcall UIMessage(void* self, void*, void* message) {
    UIFrame capture;
    capture.owner = static_cast<uint8_t*>(self) - 8;
    capture.message = message;
    std::memcpy(capture.before.data(), capture.owner, capture.before.size());
    try {
        DynamicState(capture.owner, capture.dynamicBefore);
        if (!message) capture.native.Refuse("null window message");
        else {
            capture.kind = Load<uint32_t>(message, 0);
            if (capture.kind == 3) capture.command = Load<uint32_t>(message, 4);
        }
    } catch (...) { capture.native.Refuse("UI entry capture"); }
    bool result;
    {
        Scope<UIFrame> scope(uiFrame, &capture);
        result = uiOriginal(self, nullptr, message);
    }
    try {
        uint32_t expected = 0;
        const bool replay = fc4_drawstream_ui_delegation(capture.kind, capture.command, &capture, &ReplayUI, &expected);
        if (!replay) capture.core.Refuse("UI command requires selection/legend/panel host");
        capture.native.Data(Domain::State, 2, 0, capture.owner, capture.before.size());
        capture.core.Data(Domain::State, 2, 0, capture.delegatedExit.data(), capture.delegatedExit.size());
        DynamicState(capture.owner, capture.native);
        Append(capture.core, capture.delegatedDynamic);
        capture.native.Call(Domain::Return, 3, 0, {result ? 1u : 0u});
        capture.core.Call(Domain::Return, 3, 0, {expected});
        uiCount.Add(capture.native, capture.core, replay);
    } catch (...) { ++uiCount.calls; }
    return result;
}
