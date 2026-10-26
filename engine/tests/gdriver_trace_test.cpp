// SPDX-License-Identifier: GPL-3.0-or-later
#define CINTERFACE
#define DIRECT3D_VERSION 0x0700
#include <d3d.h>
#include "FC4GDriverTrace.h"
#include <cassert>
#include <cstddef>
#include <type_traits>
using namespace fc4::gdriver_trace;

template<typename R, typename... A> constexpr unsigned Arity(R(__stdcall*)(IDirect3DDevice7*, A...)) { return sizeof...(A); }
#define CHECK(slot, name, count) static_assert(offsetof(IDirect3DDevice7Vtbl, name) / sizeof(void*) == slot); \
    static_assert(Arity(IDirect3DDevice7Vtbl{}.name) == count)
CHECK(0,QueryInterface,2); CHECK(1,AddRef,0); CHECK(2,Release,0); CHECK(3,GetCaps,1);
CHECK(4,EnumTextureFormats,2); CHECK(5,BeginScene,0); CHECK(6,EndScene,0); CHECK(7,GetDirect3D,1);
CHECK(8,SetRenderTarget,2); CHECK(9,GetRenderTarget,1); CHECK(10,Clear,6); CHECK(11,SetTransform,2);
CHECK(12,GetTransform,2); CHECK(13,SetViewport,1); CHECK(14,MultiplyTransform,2); CHECK(15,GetViewport,1);
CHECK(16,SetMaterial,1); CHECK(17,GetMaterial,1); CHECK(18,SetLight,2); CHECK(19,GetLight,2);
CHECK(20,SetRenderState,2); CHECK(21,GetRenderState,2); CHECK(22,BeginStateBlock,0); CHECK(23,EndStateBlock,1);
CHECK(24,PreLoad,1); CHECK(25,DrawPrimitive,5); CHECK(26,DrawIndexedPrimitive,7); CHECK(27,SetClipStatus,1);
CHECK(28,GetClipStatus,1); CHECK(29,DrawPrimitiveStrided,5); CHECK(30,DrawIndexedPrimitiveStrided,7);
CHECK(31,DrawPrimitiveVB,5); CHECK(32,DrawIndexedPrimitiveVB,7); CHECK(33,ComputeSphereVisibility,5);
CHECK(34,GetTexture,2); CHECK(35,SetTexture,2); CHECK(36,GetTextureStageState,3); CHECK(37,SetTextureStageState,3);
CHECK(38,ValidateDevice,1); CHECK(39,ApplyStateBlock,1); CHECK(40,CaptureStateBlock,1); CHECK(41,DeleteStateBlock,1);
CHECK(42,CreateStateBlock,2); CHECK(43,Load,5); CHECK(44,LightEnable,2); CHECK(45,GetLightEnable,2);
CHECK(46,SetClipPlane,2); CHECK(47,GetClipPlane,2); CHECK(48,GetInfo,3);
#undef CHECK

unsigned forwarded = 0;
uint32_t __stdcall SetState(void*, uint32_t key, uint32_t value) {
    assert(key == 15 && value == 1); ++forwarded; return 0x80004005;
}
uint32_t __stdcall GetViewport(void*, uint32_t out) {
    const uint32_t viewport[6] = {1,2,800,600,0,0x3F800000};
    std::memcpy(reinterpret_cast<void*>(static_cast<uintptr_t>(out)), viewport, sizeof(viewport));
    return 0;
}
int main() {
    static_assert(sizeof(void*) == 4, "DX7 transport is x86 only");
    const auto hooks = Hooks();
    for (const auto hook : hooks) assert(hook);
    original[20] = reinterpret_cast<void*>(&SetState);
    assert((Hook<20,uint32_t,uint32_t>(nullptr,15,1)) == 0x80004005);
    assert(forwarded == 1 && !active);
    assert(Begin()); assert(!Begin());
    for (unsigned i = 0; i <= Limit; ++i) Hook<20,uint32_t,uint32_t>(nullptr,15,1);
    End();
    assert(trace.count == Limit && trace.refused && forwarded == Limit + 2);
    assert(trace.records[0].slot == 20 && trace.records[0].args[0] == 15);
    assert(trace.records[0].result == 0x80004005);
    original[15] = reinterpret_cast<void*>(&GetViewport);
    uint32_t out[6]{};
    assert(Begin());
    Hook<15,uint32_t>(nullptr, static_cast<uint32_t>(reinterpret_cast<uintptr_t>(out)));
    End();
    assert(trace.count == 1 && !trace.refused && trace.records[0].outLen == 6);
    assert(trace.records[0].out[2] == 800 && trace.records[0].out[5] == 0x3F800000);
    return 0;
}
