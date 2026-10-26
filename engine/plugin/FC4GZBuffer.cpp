// SPDX-License-Identifier: GPL-3.0-or-later
// gz-framework-core section 6 and span-render-8 section 10.7.
#include "FC4GZFramework.h"
#include "FC4Log.h"
#include "cIGZUnknown.h"
#include <cstdint>
#include <new>

extern "C" void* fc4_gzfw_buffer_new();
extern "C" void fc4_gzfw_buffer_free(void*);
extern "C" uintptr_t fc4_gzfw_buffer_call(void*, uint32_t, uint32_t, uint32_t, uint32_t, uint32_t, uint32_t*);

namespace {
class Buffer;
class BufferFace : public cIGZUnknown {
public:
    bool QueryInterface(uint32_t, void**) override;
    uint32_t AddRef() override;
    uint32_t Release() override;
};
// Unique names preserve native slot order regardless of C++ overload ordering.
class BufferPrimary : public cIGZUnknown {
public:
    virtual bool Init(uint32_t, uint32_t, uint32_t, uint32_t) = 0;
    virtual bool Shutdown() = 0;
    virtual bool Uninitialize() = 0;
    virtual bool Lock(uint32_t) = 0;
    virtual bool Unlock(uint32_t) = 0;
    virtual bool IsLocked() = 0;
    virtual uint32_t Width() = 0;
    virtual uint32_t Height() = 0;
    virtual uint32_t* Rect(uint32_t*) = 0; // 11: explicit output, EAX pointer
    virtual const uint32_t* Area() = 0;
    virtual uint32_t Memory() = 0;
    virtual void ColorType(uint32_t*) = 0;
    virtual uint32_t* ColorPair(uint32_t*) = 0; // 15: explicit output, EAX pointer
    virtual uint32_t Bits() = 0;
    virtual uint32_t Bytes() = 0;
    virtual bool FillAll(uint32_t, void*) = 0;
    virtual bool FillXY(uint32_t, int32_t, int32_t, int32_t, int32_t, void*) = 0;
    virtual bool FillRect(uint32_t, const void*, void*) = 0;
    virtual uint32_t Pixel(uint32_t, uint32_t) = 0;
    virtual void SetPixel(uint32_t, uint32_t, uint32_t) = 0;
    virtual void SetKey(uint32_t) = 0;
    virtual uint32_t Key() = 0;
    virtual bool Transparent(uint32_t, uint32_t) = 0;
    virtual bool GetKey(uint32_t*) = 0;
    virtual void ClearKey() = 0;
    virtual bool Line(int32_t, int32_t, int32_t, int32_t, int32_t, uint32_t) = 0;
    virtual bool Blt(void*, const void*, const void*, const void*) = 0;
    virtual uint32_t RGB(uint32_t, uint32_t, uint32_t) = 0;
    virtual bool ToRGB(uint32_t, void*, void*, void*) = 0;
    virtual uint32_t RGBA(uint32_t, uint32_t, uint32_t, uint32_t) = 0;
    virtual bool ToRGBA(uint32_t, void*, void*, void*, void*) = 0;
    virtual void* Pixels() = 0;
    virtual uint32_t Stride() = 0;
    virtual uint32_t Changes() = 0;
    virtual void* Cache() = 0;
    virtual bool SetCache(void*) = 0;
    virtual bool Ready() = 0;
    virtual void* Destroy(uint32_t) = 0;
};
class Buffer final : public BufferPrimary, public BufferFace {
    void* core_ = fc4_gzfw_buffer_new();
    uint32_t area_[4]{};
    uintptr_t Call(uint32_t op, uint32_t a=0, uint32_t b=0, uint32_t c=0, uint32_t d=0, uint32_t* out=nullptr) {
        return fc4_gzfw_buffer_call(core_, op, a, b, c, d, out);
    }
    bool Gap(uint32_t slot) {
        // SPEC-GAP: GZF6 operations outside the covered bridge refuse work.
        fc4::Log("gzfw buffer: unsupported slot=%u gap=GZF6", slot);
        return false;
    }
public:
    ~Buffer() { fc4_gzfw_buffer_free(core_); }
    bool QueryInterface(uint32_t iid, void** out) override {
        if (iid == 1 || iid == 0x20732180) *out = static_cast<BufferPrimary*>(this);
        else if (iid == 0x86D72B57) *out = static_cast<BufferFace*>(this);
        else return false;
        AddRef(); return true;
    }
    uint32_t AddRef() override { return static_cast<uint32_t>(Call(0)); }
    uint32_t Release() override {
        const auto n = static_cast<uint32_t>(Call(1));
        if (!n) delete this;
        return n;
    }
    bool Init(uint32_t w, uint32_t h, uint32_t t, uint32_t bpp) override {
        const bool ok = Call(2,w,h,t,bpp) != 0;
        if (ok) Rect(area_);
        return ok;
    }
    bool Shutdown() override { return Gap(4); }
    bool Uninitialize() override { return Gap(5); }
    bool Lock(uint32_t f) override { return Call(3,f) != 0; }
    bool Unlock(uint32_t f) override { return Call(4,f) != 0; }
    bool IsLocked() override { return Call(15) != 0; }
    uint32_t Width() override { return static_cast<uint32_t>(Call(5)); }
    uint32_t Height() override { return static_cast<uint32_t>(Call(6)); }
    uint32_t* Rect(uint32_t* r) override { Call(7,0,0,0,0,r); return r; }
    const uint32_t* Area() override { return area_; }
    uint32_t Memory() override { return Stride()*Height(); }
    void ColorType(uint32_t* p) override { ColorPair(p); }
    uint32_t* ColorPair(uint32_t* p) override { Call(8,0,0,0,0,p); return p; }
    uint32_t Bits() override { uint32_t pair[2]{}; ColorPair(pair); return pair[1]; }
    uint32_t Bytes() override { return static_cast<uint32_t>(Call(9)); }
    bool FillAll(uint32_t, void*) override { return Gap(18); }
    bool FillXY(uint32_t, int32_t, int32_t, int32_t, int32_t, void*) override { return Gap(19); }
    bool FillRect(uint32_t, const void*, void*) override { return Gap(20); }
    uint32_t Pixel(uint32_t, uint32_t) override { Gap(21); return 0; }
    void SetPixel(uint32_t, uint32_t, uint32_t) override { Gap(22); }
    void SetKey(uint32_t) override { Gap(23); }
    uint32_t Key() override { Gap(24); return 0; }
    bool Transparent(uint32_t, uint32_t) override { return Gap(25); }
    bool GetKey(uint32_t* p) override { return Call(10,0,0,0,0,p) != 0; }
    void ClearKey() override { Gap(27); }
    bool Line(int32_t,int32_t,int32_t,int32_t,int32_t,uint32_t) override { return Gap(28); }
    bool Blt(void*,const void*,const void*,const void*) override { return Gap(29); }
    uint32_t RGB(uint32_t r,uint32_t g,uint32_t b) override { return static_cast<uint32_t>(Call(11,r,g,b)); }
    bool ToRGB(uint32_t,void*,void*,void*) override { return Gap(31); }
    uint32_t RGBA(uint32_t,uint32_t,uint32_t,uint32_t) override { Gap(32); return 0; }
    bool ToRGBA(uint32_t,void*,void*,void*,void*) override { return Gap(33); }
    void* Pixels() override { return reinterpret_cast<void*>(Call(12)); }
    uint32_t Stride() override { return static_cast<uint32_t>(Call(13)); }
    uint32_t Changes() override { return static_cast<uint32_t>(Call(14)); }
    void* Cache() override { Gap(37); return nullptr; }
    bool SetCache(void*) override { return Gap(38); }
    bool Ready() override { return Pixels() != nullptr; }
    void* Destroy(uint32_t flags) override { void* p=this; if (flags&1) delete this; else this->~Buffer(); return p; }
};
bool BufferFace::QueryInterface(uint32_t iid, void** out) { return static_cast<Buffer*>(this)->QueryInterface(iid,out); }
uint32_t BufferFace::AddRef() { return static_cast<Buffer*>(this)->AddRef(); }
uint32_t BufferFace::Release() { return static_cast<Buffer*>(this)->Release(); }
}
namespace fc4 {
cIGZUnknown* CreateGZBuffer() { return static_cast<BufferPrimary*>(new(std::nothrow) Buffer); }
}
