// SPDX-License-Identifier: GPL-3.0-or-later
// gz-framework-core section 6 and span-render-8 section 10.7.
#include "FC4GZFramework.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cIGZUnknown.h"
#include "cIGZFrameWork.h"
#include "cIGZGraphicSystem2.h"
#include "cGZGPixelFormatDesc.h"
#include "cRZCOMDllDirector.h"
#include <algorithm>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <mutex>
#include <new>

extern "C" void* fc4_gzfw_buffer_new();
extern "C" void fc4_gzfw_buffer_free(void*);
extern "C" uintptr_t fc4_gzfw_buffer_call(void*, uint32_t, uint32_t, uint32_t, uint32_t, uint32_t, uint32_t*);
extern "C" uint32_t fc4_gzfw_buffer_convert(uint32_t, uint32_t, uint32_t);

namespace {
template<class T> T Slot(void* p, uint32_t n) { return reinterpret_cast<T>((*static_cast<void***>(p))[n]); }
struct R { int32_t l, t, r, b; };
int32_t Add(int32_t a,int32_t b) { return static_cast<int32_t>(static_cast<uint32_t>(a)+static_cast<uint32_t>(b)); }
int32_t Sub(int32_t a,int32_t b) { return static_cast<int32_t>(static_cast<uint32_t>(a)-static_cast<uint32_t>(b)); }
uint32_t Load(const uint8_t* p,uint32_t bytes) {
    uint32_t value=0;
    for (uint32_t i=0;i<bytes;++i) value|=uint32_t(p[i])<<(8*i);
    return value;
}
void Store(uint8_t* p,uint32_t bytes,uint32_t value) {
    for (uint32_t i=0;i<bytes;++i) p[i]=static_cast<uint8_t>(value>>(8*i));
}
uint32_t PixelBytes(uint32_t b) { b&=255; return b>=2 && b<=4 ? b : 1; }
uint32_t TypeBytes(uint32_t t) { return t>=4 && t<=7 ? 2 : t==8 || t==9 ? 4 : 0; }
uint32_t KeyMask(uint32_t t) { return t==4 || t==6 ? 0x7fff : t==5 ? 0xffff : t==7 ? 0xfff : t==8 || t==9 ? 0xffffff : 0xffffffff; }
class Buffer;
class BufferFace : public cIGZUnknown {
public:
    bool QueryInterface(uint32_t, void**) override;
    uint32_t AddRef() override;
    uint32_t Release() override;
    virtual bool SetDrawingColor(uint32_t, uint32_t, uint32_t, uint32_t) = 0;
    virtual bool GetDrawingColor(uint8_t*, uint8_t*, uint8_t*, uint8_t*) = 0;
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
    virtual uint32_t* ColorType(uint32_t*) = 0;
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
    virtual bool Clip(BufferPrimary*, R*, R*, const R*) = 0;
    virtual bool AllocateStorage() = 0;
    virtual bool AllocatePixels() = 0;
    virtual bool FreePixels() = 0;
    virtual void ResolveColorType() = 0;
    virtual bool BlitClipped(BufferPrimary*, const R*, const R*) = 0;
};
class Buffer final : public BufferPrimary, public BufferFace {
    void* core_ = fc4_gzfw_buffer_new();
    uint32_t area_[4]{};
    std::recursive_mutex cacheMutex_;
    cIGZUnknown* cache_=nullptr;
    uintptr_t Call(uint32_t op, uint32_t a=0, uint32_t b=0, uint32_t c=0, uint32_t d=0, uint32_t* out=nullptr) {
        return fc4_gzfw_buffer_call(core_, op, a, b, c, d, out);
    }
    void Resolve(uint32_t& type,uint32_t& bpp) {
        if (type!=1 && type!=3) return;
        type=5; bpp=16;
        auto* fw=RZGetFrameWork(); // borrowed
        cIGZUnknown* graphics=nullptr;
        if (!fw || !fw->GetSystemService(0xC416025C,0x0073283C,reinterpret_cast<void**>(&graphics)) || !graphics) return;
        FC4_ACQUIRED(graphics,"gzfw automatic-format graphics");
        cIGZGraphicSystem2* extended=nullptr;
        if (graphics->QueryInterface(0xC47B747C,reinterpret_cast<void**>(&extended))) {
            FC4_ACQUIRED(extended,"gzfw automatic-format graphics2");
            cGZGPixelFormatDesc settings{};
            settings.field0_0x0=0x40;
            extended->GetScreenPixelFormat(settings);
            bpp=(settings.field0_0x0&63)>16 ? 32 : 16; type=bpp==32 ? 9 : 7;
            FC4_RELEASE(extended);
        } else {
            uint32_t pair[2]{};
            Slot<void(__thiscall*)(void*,uint32_t*)>(graphics,9)(graphics,pair);
            type=pair[0]; bpp=pair[1];
        }
        FC4_RELEASE(graphics);
    }
public:
    ~Buffer() { Shutdown(); FC4_RELEASE_AT_DESTRUCTION(cache_); fc4_gzfw_buffer_free(core_); }
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
    bool SetDrawingColor(uint32_t r, uint32_t g, uint32_t b, uint32_t a) override {
        return Call(27,r,g,b,a)!=0;
    }
    bool GetDrawingColor(uint8_t* r, uint8_t* g, uint8_t* b, uint8_t* a) override {
        uint32_t rgba[4];
        if (!Call(28,0,0,0,0,rgba)) return false;
        *r=static_cast<uint8_t>(rgba[0]);
        *g=static_cast<uint8_t>(rgba[1]);
        *b=static_cast<uint8_t>(rgba[2]);
        *a=static_cast<uint8_t>(rgba[3]);
        return true;
    }
    bool Init(uint32_t w, uint32_t h, uint32_t t, uint32_t bpp) override {
        if (Ready() || !w || !h) return false;
        Resolve(t,bpp);
        const bool ok = Call(2,w,h,t,bpp) != 0;
        if (ok) Rect(area_);
        return ok;
    }
    bool Shutdown() override { return Call(16)!=0; }
    bool Uninitialize() override { return Shutdown(); }
    bool Lock(uint32_t f) override { return Call(3,f) != 0; }
    bool Unlock(uint32_t f) override { return Call(4,f) != 0; }
    bool IsLocked() override { return Call(15) != 0; }
    uint32_t Width() override { return static_cast<uint32_t>(Call(5)); }
    uint32_t Height() override { return static_cast<uint32_t>(Call(6)); }
    uint32_t* Rect(uint32_t* r) override { Call(7,0,0,0,0,r); return r; }
    const uint32_t* Area() override { return area_; }
    uint32_t Memory() override { return static_cast<uint32_t>(Call(18)); }
    uint32_t* ColorType(uint32_t* p) override { return ColorPair(p); }
    uint32_t* ColorPair(uint32_t* p) override { Call(8,0,0,0,0,p); return p; }
    uint32_t Bits() override { uint32_t pair[2]{}; ColorPair(pair); return pair[1]; }
    uint32_t Bytes() override { return static_cast<uint32_t>(Call(9)); }
    bool FillAll(uint32_t pixel, void* clip) override { return FillRect(pixel,area_,clip); }
    bool FillXY(uint32_t pixel, int32_t l, int32_t t, int32_t r, int32_t b, void* clip) override {
        R rect{l,t,r,b}; return FillRect(pixel,&rect,clip);
    }
    bool FillRect(uint32_t pixel, const void* rect, void* clip) override {
        R d=*static_cast<const R*>(rect),s{0,0,Sub(d.r,d.l),Sub(d.b,d.t)};
        if (!Clip(this,&s,&d,static_cast<const R*>(clip))) return false;
        const auto bits=Bits(), stride=Stride(); auto* pixels=static_cast<uint8_t*>(Pixels());
        if (bits==8 || bits==16 || bits==32) {
            const auto bytes=bits>>3;
            for (int32_t y=d.t;y<d.b;++y) for (int32_t x=d.l;x<d.r;++x)
                Store(pixels+uint32_t(y)*stride+uint32_t(x)*bytes,bytes,pixel);
        }
        Call(25); return true;
    }
    uint32_t Pixel(uint32_t x, uint32_t y) override {
        const auto bytes=PixelBytes(Bytes());
        return Load(static_cast<uint8_t*>(Pixels())+y*Stride()+x*bytes,bytes);
    }
    void SetPixel(uint32_t x, uint32_t y, uint32_t pixel) override {
        const auto bytes=PixelBytes(Bytes());
        Store(static_cast<uint8_t*>(Pixels())+y*Stride()+x*bytes,bytes,pixel);
    }
    void SetKey(uint32_t pixel) override {
        Call(19,pixel,1); std::lock_guard<std::recursive_mutex> lock(cacheMutex_);
        if (cache_) Slot<void(__thiscall*)(void*)>(cache_,8)(cache_);
    }
    uint32_t Key() override { return static_cast<uint32_t>(Call(20)); }
    bool Transparent(uint32_t x, uint32_t y) override {
        uint32_t key; if (!GetKey(&key) || x>=Width() || y>=Height()) return false;
        uint32_t pair[2]; ColorPair(pair); const auto mask=KeyMask(pair[0]);
        return (Pixel(x,y)&mask)==(key&mask);
    }
    bool GetKey(uint32_t* p) override { return Call(10,0,0,0,0,p) != 0; }
    void ClearKey() override {
        Call(19); std::lock_guard<std::recursive_mutex> lock(cacheMutex_);
        if (cache_) Slot<void(__thiscall*)(void*)>(cache_,8)(cache_);
    }
    bool Line(int32_t,int32_t,int32_t,int32_t,int32_t,uint32_t) override;
    bool Blt(void* source,const void* sourceRect,const void* destinationRect,const void* clip) override {
        R s=*static_cast<const R*>(sourceRect),d=*static_cast<const R*>(destinationRect);
        d.r=Add(d.l,Sub(s.r,s.l)); d.b=Add(d.t,Sub(s.b,s.t));
        return Clip(this,&s,&d,static_cast<const R*>(clip)) && BlitClipped(static_cast<BufferPrimary*>(source),&s,&d);
    }
    uint32_t RGB(uint32_t r,uint32_t g,uint32_t b) override { return static_cast<uint32_t>(Call(11,r,g,b)); }
    bool ToRGB(uint32_t pixel,void* r,void* g,void* b) override { uint8_t a; return ToRGBA(pixel,r,g,b,&a); }
    uint32_t RGBA(uint32_t r,uint32_t g,uint32_t b,uint32_t a) override { return static_cast<uint32_t>(Call(21,r,g,b,a)); }
    bool ToRGBA(uint32_t pixel,void* r,void* g,void* b,void* a) override {
        uint32_t rgba[4]; if (!Call(22,pixel,0,0,0,rgba)) return false;
        *static_cast<uint8_t*>(r)=static_cast<uint8_t>(rgba[0]); *static_cast<uint8_t*>(g)=static_cast<uint8_t>(rgba[1]);
        *static_cast<uint8_t*>(b)=static_cast<uint8_t>(rgba[2]); *static_cast<uint8_t*>(a)=static_cast<uint8_t>(rgba[3]); return true;
    }
    void* Pixels() override { return reinterpret_cast<void*>(Call(12)); }
    uint32_t Stride() override { return static_cast<uint32_t>(Call(13)); }
    uint32_t Changes() override { return static_cast<uint32_t>(Call(14)); }
    void* Cache() override { std::lock_guard<std::recursive_mutex> lock(cacheMutex_); return cache_; }
    bool SetCache(void* p) override {
        std::lock_guard<std::recursive_mutex> lock(cacheMutex_);
        auto* old=cache_; cache_=nullptr; if (old) FC4_RELEASE(old);
        if (!Slot<bool(__thiscall*)(void*,void*)>(p,3)(p,static_cast<BufferPrimary*>(this))) return false;
        auto* incoming=static_cast<cIGZUnknown*>(p);
        if (cache_!=incoming) { FC4_ADDREF(incoming); old=cache_; cache_=incoming; if (old) FC4_RELEASE(old); }
        return true;
    }
    bool Ready() override { return Call(17)!=0; }
    void* Destroy(uint32_t flags) override { void* p=this; if (flags&1) delete this; else this->~Buffer(); return p; }
    bool Clip(BufferPrimary*,R*,R*,const R*) override;
    bool AllocateStorage() override { return AllocatePixels(); }
    bool AllocatePixels() override { ResolveColorType(); return Call(24)!=0; }
    bool FreePixels() override { return Call(26)!=0; }
    void ResolveColorType() override { uint32_t p[2]; ColorPair(p); Resolve(p[0],p[1]); Call(23,p[0],p[1]); }
    bool BlitClipped(BufferPrimary*,const R*,const R*) override;
};
bool Buffer::Clip(BufferPrimary* bounds,R* s,R* d,const R* clip) {
    const auto empty=[&]() { return d->l>=d->r || d->t>=d->b; };
    const auto right=[&](int32_t edge) { if (d->r>edge) { s->r=Sub(s->r,Sub(d->r,edge)); d->r=edge; } };
    const auto left=[&](int32_t edge) { if (d->l<edge) { s->l=Add(s->l,Sub(edge,d->l)); d->l=edge; } };
    const auto bottom=[&](int32_t edge) { if (d->b>edge) { s->b=Sub(s->b,Sub(d->b,edge)); d->b=edge; } };
    const auto top=[&](int32_t edge) { if (d->t<edge) { s->t=Add(s->t,Sub(edge,d->t)); d->t=edge; } };
    right(static_cast<int32_t>(bounds->Width())); if (empty()) return false;
    left(0); if (empty()) return false;
    bottom(static_cast<int32_t>(bounds->Height())); if (empty()) return false;
    top(0); if (empty()) return false;
    if (clip) { right(clip->r); left(clip->l); bottom(clip->b); top(clip->t); }
    return !empty();
}
bool Buffer::BlitClipped(BufferPrimary* source,const R* s,const R* d) {
    Call(25);
    uint32_t sp[2],dp[2]; source->ColorPair(sp); ColorPair(dp);
    auto* src=static_cast<uint8_t*>(source->Pixels()); auto* dst=static_cast<uint8_t*>(Pixels());
    if (!src || !dst) return true;
    const auto sb=TypeBytes(sp[0]),db=TypeBytes(dp[0]);
    int32_t ss=static_cast<int32_t>(source->Stride()),ds=static_cast<int32_t>(Stride());
    int32_t sy=s->t,dy=d->t;
    if (source==static_cast<BufferPrimary*>(this) && s->t<d->b && s->b>d->t) {
        sy=Sub(s->b,1); dy=Sub(d->b,1); ss=Sub(0,ss); ds=Sub(0,ds);
    }
    src+=uint32_t(sy)*source->Stride()+uint32_t(s->l)*sb;
    dst+=uint32_t(dy)*Stride()+uint32_t(d->l)*db;
    uint32_t key=0; const bool keyed=source->GetKey(&key); const auto mask=KeyMask(sp[0]);
    const auto readBytes=sb ? sb : 2,writeBytes=db ? db : 2;
    for (int32_t y=0;y<Sub(d->b,d->t);++y) {
        for (int32_t x=0;x<Sub(d->r,d->l);++x) {
            const auto p=Load(src+uint32_t(x)*readBytes,readBytes);
            if (!keyed || (p&mask)!=(key&mask)) Store(dst+uint32_t(x)*writeBytes,writeBytes,fc4_gzfw_buffer_convert(sp[0],dp[0],p));
        }
        src+=ss; dst+=ds;
    }
    return true;
}
bool ClipLine(int32_t& x1,int32_t& y1,int32_t& x2,int32_t& y2,int32_t right,int32_t bottom) {
    const auto code=[&](int32_t x,int32_t y) { return (y<0 ? 1 : 0)|(y>bottom ? 2 : 0)|(x>right ? 4 : 0)|(x<0 ? 8 : 0); };
    const auto ratio=[](int32_t a,int32_t b,int32_t divisor) {
        const auto product=static_cast<int32_t>(uint32_t(a)*uint32_t(b));
        return static_cast<int32_t>(int64_t(product)/divisor);
    };
    for (;;) {
        const auto c1=code(x1,y1),c2=code(x2,y2);
        if (!(c1|c2)) return true;
        if (c1&c2) return false;
        const auto c=c1 ? c1 : c2; int32_t x,y;
        if (c&1) { y=0; x=Add(x1,ratio(Sub(x2,x1),Sub(0,y1),Sub(y2,y1))); }
        else if (c&2) { y=bottom; x=Add(x1,ratio(Sub(x2,x1),Sub(bottom,y1),Sub(y2,y1))); }
        else if (c&4) { x=right; y=Add(y1,ratio(Sub(y2,y1),Sub(right,x1),Sub(x2,x1))); }
        else { x=0; y=Add(y1,ratio(Sub(y2,y1),Sub(0,x1),Sub(x2,x1))); }
        if (c1) { x1=x; y1=y; } else { x2=x; y2=y; }
    }
}
bool Buffer::Line(int32_t x1,int32_t y1,int32_t x2,int32_t y2,int32_t pixel,uint32_t thickness) {
    const auto w=static_cast<int32_t>(Width()),h=static_cast<int32_t>(Height()),n=static_cast<int32_t>(thickness);
    if (!ClipLine(x1,y1,x2,y2,Sub(w,n),Sub(h,n))) return true;
    if (y1==y2) { FillXY(pixel,std::min(x1,x2),y1,Add(std::max(x1,x2),1),Add(y1,n),nullptr); return true; }
    if (x1==x2) { FillXY(pixel,x1,std::min(y1,y2),Add(x1,n),Add(std::max(y1,y2),1),nullptr); return true; }
    auto* pixels=static_cast<uint8_t*>(Pixels()); if (!pixels) return true;
    const int32_t sx=x2>x1 ? 1 : -1,sy=y2>y1 ? 1 : -1;
    const auto dx=static_cast<int32_t>(std::abs(int64_t(x2)-x1)),dy=static_cast<int32_t>(std::abs(int64_t(y2)-y1));
    const bool xMajor=dx>=dy; const auto major=xMajor ? dx : dy,minor=xMajor ? dy : dx;
    int64_t error=int64_t(2)*minor-major; int32_t x=x1,y=y1;
    const auto stride=Stride(),bits=Bits(),pitch=2*((stride>>1)&0xffff); bool boundary=false;
    for (int64_t i=0;i<=major;++i) {
        if (thickness==1) {
            if (bits==8 || bits==16 || bits==24 || bits==32) Store(pixels+uint32_t(y)*stride+uint32_t(x)*(bits>>3),bits>>3,pixel);
        } else {
            if (x<Sub(n,1) || x>Sub(w,n) || y<Sub(n,1) || y>Sub(h,n)) boundary=true;
            int32_t left=sx<0 ? Add(Sub(x,n),1) : x,top=sy<0 ? Add(Sub(y,n),1) : y;
            if (boundary) left=sx<0 ? std::max(left,0) : std::min(left,Sub(w,1));
            const auto words=boundary ? std::min(thickness,static_cast<uint32_t>(Sub(w,left))) : thickness;
            for (uint32_t row=0;row<(thickness&0xffff);++row) {
                const auto ry=Add(top,static_cast<int32_t>(row));
                if (boundary && (ry<0 || static_cast<uint32_t>(ry)>=Height())) continue;
                for (uint32_t col=0;col<words;++col) Store(pixels+uint32_t(ry)*pitch+uint32_t(left)*2+col*2,2,pixel);
            }
        }
        if (error<0) error+=int64_t(2)*minor;
        else { if (xMajor) y=Add(y,sy); else x=Add(x,sx); error+=int64_t(2)*(minor-major); }
        if (xMajor) x=Add(x,sx); else y=Add(y,sy);
    }
    return true;
}
bool BufferFace::QueryInterface(uint32_t iid, void** out) { return static_cast<Buffer*>(this)->QueryInterface(iid,out); }
uint32_t BufferFace::AddRef() { return static_cast<Buffer*>(this)->AddRef(); }
uint32_t BufferFace::Release() { return static_cast<Buffer*>(this)->Release(); }
}
namespace fc4 {
cIGZUnknown* CreateGZBuffer() { return static_cast<BufferPrimary*>(new(std::nothrow) Buffer); }
}
