// SPDX-License-Identifier: GPL-3.0-or-later
// Included after CHECK in overlay_factory_test's private namespace. Receivers
// consume individual native argument words, independently of the adapter types.
#undef DrawState // windows.h's text-drawing macro is not the overlay command.
struct NativeDeviceSink {
    void** table = nullptr;
    void* entries[46]{};
    uint32_t calls = 0, tag = UINT32_MAX, a = 0, b = 0, c = 0;
    const void* pointer = nullptr;
    float scalar = 0;
    static NativeDeviceSink* receiverless;
    static NativeDeviceSink& Of(void* p) { return *static_cast<NativeDeviceSink*>(p); }
    void Record(uint32_t t, uint32_t x = 0, uint32_t y = 0, uint32_t z = 0, const void* ptr = nullptr) {
        ++calls; tag = t; a = x; b = y; c = z; pointer = ptr;
    }
    template<uint32_t Tag> static void __fastcall Word(void* p, void*, uint32_t a) { Of(p).Record(Tag, a); }
    template<uint32_t Tag> static void __fastcall Pointer(void* p, void*, const void* v) { Of(p).Record(Tag, 0, 0, 0, v); }
    static void __stdcall Transparency(float f) { receiverless->Record(2); receiverless->scalar = f; }
    static void __stdcall ResetTransparency() { receiverless->Record(5); }
    static void __fastcall ResetShade(void* p, void*) { Of(p).Record(6); }
    template<uint32_t Tag> static void __fastcall StagePointer(void* p, void*, const void* v, uint32_t s) {
        Of(p).Record(Tag, s, 0, 0, v);
    }
    template<uint32_t Tag> static void __fastcall Three(void* p, void*, uint32_t a, uint32_t b, uint32_t c) {
        Of(p).Record(Tag, a, b, c);
    }
    static void __fastcall Texture(void* p, void*, uint32_t enabled, uint32_t stage) { Of(p).Record(12, enabled, stage); }
    static void __stdcall Environment(const float* v) { receiverless->Record(13, 0, 0, 0, v); }
    static void __fastcall Alpha(void* p, void*, uint32_t f, float reference) { Of(p).Record(15, f); Of(p).scalar = reference; }
    static void __fastcall Colors(void* p, void*, uint32_t ambient, uint32_t diffuse) { Of(p).Record(16, ambient, diffuse); }
    NativeDeviceSink() {
        // SDK cIGZGDriver/renderer-driver: 34 vertex colours; 44/45 enable/disable.
        table = entries;
        entries[34] = reinterpret_cast<void*>(Colors);
        entries[44] = reinterpret_cast<void*>(Word<17>);
        entries[45] = reinterpret_cast<void*>(Word<18>);
    }
    static fc4::overlay_device::Calls Calls() {
        namespace d = fc4::overlay_device;
        return {reinterpret_cast<d::Word>(Word<0>), reinterpret_cast<d::Record>(Pointer<1>),
            Transparency, reinterpret_cast<d::Record>(Pointer<3>), reinterpret_cast<d::Record>(Pointer<4>),
            ResetTransparency, reinterpret_cast<d::Reset>(ResetShade),
            reinterpret_cast<d::StageRecord>(StagePointer<7>), reinterpret_cast<d::Word>(Word<8>),
            reinterpret_cast<d::StageWords>(Three<9>), reinterpret_cast<d::StageWords>(Three<10>),
            reinterpret_cast<d::StageRecord>(StagePointer<11>), reinterpret_cast<d::Pair>(Texture),
            Environment, reinterpret_cast<d::Word>(Word<14>), reinterpret_cast<d::Alpha>(Alpha)};
    }
};
NativeDeviceSink* NativeDeviceSink::receiverless = nullptr;

bool SubmitNativeDevice(const fc4::overlay_device::Calls& calls,
                        const fc4::overlay_device::Inputs& inputs, const FC4OverlayCommand& command) {
    uintptr_t before = 0, after = 0;
    __asm mov before, esp
    const bool result = fc4::overlay_device::Submit(calls, inputs, command);
    __asm mov after, esp
    CHECK(before == after);
    return result;
}

void CheckNativeDevice() {
    using Kind = FC4OverlayCommandKind;
    namespace device = fc4::overlay_device;
    NativeDeviceSink sink;
    NativeDeviceSink::receiverless = &sink;
    auto calls = NativeDeviceSink::Calls();
    CHECK(calls.Count() == 16);
    uint32_t stateRecords[5][2]{};
    uint8_t combinerRecords[3][12]{};
    device::Inputs inputs{&sink, reinterpret_cast<cIGZGDriver*>(&sink)};
    for (uint32_t i = 0; i != 5; ++i) inputs.states[i] = stateRecords[i];
    for (uint32_t i = 0; i != 3; ++i) inputs.combiners[i] = combinerRecords[i];
    FC4OverlayCommand command{};
    auto submit = [&]() { return SubmitNativeDevice(calls, inputs, command); };
    command.kind = Kind::DepthOffset; command.a = 17;
    CHECK(submit() && sink.tag == 0 && sink.a == 17);
    command.kind = Kind::DrawState; command.a = 4;
    CHECK(submit() && sink.tag == 1 && sink.pointer == stateRecords[4]);
    command.kind = Kind::Transparency; command.data[0] = 0.25f;
    CHECK(submit() && sink.tag == 2 && sink.scalar == 0.25f);
    command.kind = Kind::Shade; command.a = 0;
    CHECK(submit() && sink.tag == 3 && sink.pointer == command.data);
    command.a = 1;
    CHECK(submit() && sink.tag == 4 && sink.pointer == command.data);
    command.kind = Kind::VertexColors; command.a = 1; command.b = 0;
    CHECK(submit() && sink.tag == 16 && sink.a == 1 && sink.b == 0);
    command.a = 0; command.b = 1;
    CHECK(submit() && sink.a == 0 && sink.b == 1);
    command.kind = Kind::ResetTransparency;
    CHECK(submit() && sink.tag == 5);
    command.kind = Kind::ResetShade;
    CHECK(submit() && sink.tag == 6);
    command.kind = Kind::Transform; command.a = 1;
    CHECK(submit() && sink.tag == 7 && sink.a == 1 && sink.pointer == command.data);
    command.kind = Kind::ClearTransform; command.a = 0;
    CHECK(submit() && sink.tag == 8 && sink.a == 0);
    // Distinct values catch stage-first versus the native stage-last order.
    command.kind = Kind::Filtering; command.a = 1; command.b = 2; command.c = 7;
    CHECK(submit() && sink.tag == 9 && sink.a == 2 && sink.b == 7 && sink.c == 1);
    command.kind = Kind::Wrap; command.b = 3; command.c = 2;
    CHECK(submit() && sink.tag == 10 && sink.a == 3 && sink.b == 2 && sink.c == 1);
    command.kind = Kind::ShadowCombiner;
    for (uint32_t i = 0; i != 3; ++i) {
        command.b = i;
        CHECK(submit() && sink.tag == 11 && sink.a == 1 && sink.pointer == combinerRecords[i]);
    }
    command.kind = Kind::TextureEnabled; command.a = 1; command.b = 0;
    CHECK(submit() && sink.tag == 12 && sink.a == 0 && sink.b == 1);
    command.kind = Kind::Environment; command.data[3] = 0.5f;
    CHECK(submit() && sink.tag == 13 && sink.pointer == command.data && command.data[3] == 0.5f);
    command.kind = Kind::AlphaTest; command.a = 1; command.data[0] = 0.125f;
    uint32_t before = sink.calls;
    CHECK(submit() && sink.calls == before + 2 && sink.tag == 15 && sink.a == 6 && sink.scalar == 0.125f);
    command.a = 0; before = sink.calls;
    CHECK(submit() && sink.calls == before + 1 && sink.tag == 14 && sink.a == 0);
    command.kind = Kind::DeviceFlag7; command.a = 1;
    CHECK(submit() && sink.tag == 17 && sink.a == 7);
    command.a = 0;
    CHECK(submit() && sink.tag == 18 && sink.a == 7);

    before = sink.calls;
    command.kind = Kind::DrawState; command.a = 5;
    CHECK(!submit() && sink.calls == before);
    command.a = 0; inputs.states[0] = nullptr;
    CHECK(!submit() && sink.calls == before);
    command.kind = Kind::ShadowCombiner; command.b = UINT32_MAX;
    CHECK(!submit() && sink.calls == before);
    command.b = 0; inputs.combiners[0] = nullptr;
    CHECK(!submit() && sink.calls == before);
    command.kind = Kind::AlphaTest; command.a = 1; calls.alphaFunction = nullptr;
    CHECK(!submit() && sink.calls == before); // must not first enable alpha testing
    command.kind = Kind::ResetShade; calls.resetShade = nullptr;
    CHECK(!submit() && sink.calls == before);
    command.kind = Kind::VertexColors; inputs.driver = nullptr;
    CHECK(!submit() && sink.calls == before);
    command.kind = Kind::DeviceFlag7;
    CHECK(!submit() && sink.calls == before);
    command.kind = static_cast<Kind>(UINT32_MAX);
    CHECK(!submit() && sink.calls == before);
    command.kind = Kind::DepthOffset; inputs.context = nullptr;
    CHECK(!submit() && sink.calls == before);
    NativeDeviceSink::receiverless = nullptr;
}
