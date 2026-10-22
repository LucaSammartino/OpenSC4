// SPDX-License-Identifier: GPL-3.0-or-later
// Included inside the span8 bridge only. Native factory/Init/setters/void slot22, SR9.
using CreateContext = uint8_t(__fastcall*)(void*, void*, void*, void**);
using InitContext = uint8_t(__fastcall*)(void*, void*, void*);
using VoidContext = void(__fastcall*)(void*, void*);
using ColourTable = void(__fastcall*)(void*, void*, uint32_t, uint32_t, const uint32_t*);
using DwordSetter = void(__fastcall*)(void*, void*, uint32_t);
using GradientSetter = void(__fastcall*)(void*, void*, const float*, float, float);
using SourceSetter = uint8_t(__fastcall*)(void*, void*, void*, float, float, float, float, int32_t*);
using TileSetter = void(__fastcall*)(void*, void*, const int32_t*);
using DrawSpans = void(__fastcall*)(void*, void*, const uint32_t*, uint32_t); // void, pop8
const uint32_t fixtureSpans[] = {0x10001,0x1001C,0x20003,0x2000A,0x3001B,0x3001C,0x80001,0x8001C};
constexpr uint32_t fixtureN = 4;
struct Result { uint64_t cases=0, exact=0, diff=0, changed=0, untouched=0, refused=0, noCall=0, faults=0; };
void Seed(uint8_t* p, size_t n) {
    uint32_t x = 0xB32D7981;
    for (size_t i=0;i<n;++i) { x ^= x<<13; x ^= x>>17; x ^= x<<5; p[i]=uint8_t(x>>24); }
}
void Compare(Result& r, const std::vector<uint8_t>& initial, const std::vector<uint8_t>& native,
             const uint8_t* actual, uint32_t stride) {
    ++r.cases;
    bool diff = std::memcmp(native.data(),actual,native.size()) != 0;
    diff ? ++r.diff : ++r.exact;
    if(fixtureKernel) { if(diff) ++fixtureKernel->diff; else ++fixtureKernel->exact; }
    bool changed = false;
    for (size_t i=0;i<initial.size();++i) {
        changed |= native[i] != initial[i];
        bool touched=false;
        for (unsigned s=0;s<fixtureN;++s) {
            size_t off=(fixtureSpans[s*2]>>16)*stride+(fixtureSpans[s*2]&65535);
            touched |= i>=off && i<off+fixtureSpans[s*2+1]-fixtureSpans[s*2];
        }
        if (!touched && (initial[i]!=actual[i] || initial[i]!=native[i])) ++r.untouched;
    }
    r.changed += changed;
}
void Report(const char* kind, const Result& r) {
    Log("span8 fixture %s: cases=%llu exact=%llu diff=%llu changed_cases=%llu guard_padding_corruptions=%llu refusals=%llu no_call=%llu faults=%llu resync=0 -> %s",
        kind,r.cases,r.exact,r.diff,r.changed,r.untouched,r.refused,r.noCall,r.faults,
        r.cases && r.cases==r.exact && r.changed==r.cases && !r.untouched && !r.refused && !r.noCall && !r.faults ? "EXACT" : "INCOMPLETE");
}
void OriginalGuard(void* p) { Original(*static_cast<Call*>(p)); }
struct DrawArg { void* ctx; };
void NativeGuard(void* p) {
    void* ctx=static_cast<DrawArg*>(p)->ctx;
    Slot<DrawSpans>(ctx,22)(ctx,nullptr,fixtureSpans,fixtureN);
}
struct Buffer {
    cIGZBuffer* p=nullptr; uint8_t* bits=nullptr; uint32_t stride=0, height=0;
    bool Make(uint32_t w,uint32_t h,uint32_t type,uint32_t bpp) {
        void* obj=nullptr;
        if (!GZCOM() || !GZCOM()->GetClassObject(0xC470D325,GZIID_cIGZBuffer,&obj) || !obj) return false;
        p=static_cast<cIGZBuffer*>(obj); FC4_ACQUIRED(p,"span8 buffer factory");
        if (!p->Init(w,h,static_cast<cGZBufferColorType::eColorType>(type),bpp) || !p->Lock(0x800)) return false;
        bits=static_cast<uint8_t*>(p->GetColorSurfaceBits()); stride=p->GetColorSurfaceStride(); height=h;
        bool ok=bits && stride==((w*(bpp/8)+3)&~3u);
        if (ok) Seed(bits,size_t(stride)*h);
        p->Unlock(0x800); return ok;
    }
    ~Buffer() { if(p) { auto left=FC4_RELEASE_CREATED(p); Log("span8 fixture buffer Release=%u",left); } }
};
void DirectFixture() {
    Result r;
    std::vector<uint8_t> arena(32*10), src(256*32);
    Seed(arena.data(),arena.size()); Seed(src.data(),src.size());
    const auto init=arena;
    uint32_t dst[6]={uint32_t(reinterpret_cast<uintptr_t>(arena.data())),32,29,10,2,1};
    uint32_t source[6]={uint32_t(reinterpret_cast<uintptr_t>(src.data())),256,64,32,0,0};
    uint32_t params[8]={0,0,0x10000,0x10000,0,0xC0123456,0,0};
    uint32_t grad[12]={0,0,0,0x00808000,0,0,0,0x00010000,0,0,0,0x00020000};
    void** self=table;
    auto run=[&](Call& c) {
        arena=init;
        if (!SehGuard("span8 direct original",OriginalGuard,&c)) ++r.faults;
        const auto native=arena;
        arena=init;
        auto calls=fixtureCalls, refused=fixtureRefusals;
        Dispatch(c,0);
        r.noCall += fixtureCalls==calls; r.refused += fixtureRefusals-refused;
        Compare(r,init,native,arena.data(),32);
    };
    fixtureClass=0;
    for (uint32_t slot=1;slot<=4;++slot) {
        Call c{&self,slot,fixtureSpans,fixtureN,dst,0x12345678,0x80123456}; c.grad=grad; run(c);
    }
    for (uint32_t row=0;row<13;++row) for (uint32_t k=0;k<12;++k) {
        params[4]=row/2; params[7]=row%2;
        const unsigned bpp=row/2<4?2:row/2<6?4:1;
        params[6]=0;
        std::memcpy(&params[6],src.data()+256+4*bpp,bpp);
        const uint32_t masks[7]={0xFFF,0x7FFF,0x7FFF,0xFFFF,0xFFFFFF,0xFFFFFF,0xFF};
        params[6]&=masks[row/2];
        Call c{&self,5,fixtureSpans,fixtureN,dst}; c.src=source; c.params=params; c.kind=k; run(c);
    }
    fixtureClass=3;
    Report("constructed_direct",r);
    fixtureKernel=nullptr;
    // Owned adapter's unsupported row is checked without ever calling its native body.
    params[4]=6; params[7]=1;
    Call row13{&self,5,fixtureSpans,fixtureN,dst}; row13.src=source;row13.params=params;
    FC4Span8Job j={};
    Log("span8 fixture safety: row13_refused=%u (SR8-3, not a native comparison)",unsigned(!Job(row13,j)));
}
void NativeFixture() {
    Result r;
    Buffer dst;
    if(!dst.Make(29,10,2,8)) { Log("span8 fixture native: buffer failed, no credit"); return; }
    void* ds=nullptr; auto* fw=RZGetFrameWork();
    if(!fw || !fw->GetSystemService(0xEB2EA3CB,0x4B2EA3B3,&ds) || !ds) return;
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(ds),"span8 draw service");
    void* ctx=nullptr;
    uint8_t made=Slot<CreateContext>(ds,4)(ds,nullptr,dst.p,&ctx);
    FC4_RELEASE(static_cast<cIGZUnknown*>(ds));
    if(!ctx) return;
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(ctx),"span8 native context factory");
    uint16_t lock0=Field<uint16_t>(dst.p,0x38), flags0=Field<uint16_t>(dst.p,0x44);
    bool right=made && *static_cast<uintptr_t*>(ctx)==va.contextVtable;
    bool initialized=right && Slot<InitContext>(ctx,3)(ctx,nullptr,dst.p);
    void* renderer=initialized ? Field<void*>(ctx,0xB0):nullptr;
    bool route=renderer && *static_cast<void***>(renderer)==table;
    Log("span8 fixture native route: factory=%u Init=%u type2_renderer=%u",unsigned(made),unsigned(initialized),unsigned(route));
    if(route) {
        std::vector<uint8_t> init(dst.bits,dst.bits+size_t(dst.stride)*dst.height);
        DrawArg draw{ctx};
        const uint32_t colour=0xC0123456;
        const float gradient[12]={0,0,0,0.5f, 0,0,0,0.00390625f, 0,0,0,0.0078125f};
        auto run=[&]() {
            std::memcpy(dst.bits,init.data(),init.size());
            if(!Entries(true)) { ++r.faults; return; }
            if(!SehGuard("span8 native original",NativeGuard,&draw)) ++r.faults;
            std::vector<uint8_t> native(dst.bits,dst.bits+init.size());
            std::memcpy(dst.bits,init.data(),init.size());
            if(!Entries(false)) { ++r.faults; return; }
            auto calls=fixtureCalls, refused=fixtureRefusals;
            fixtureKernel=nullptr;
            fixtureClass=1;
            if(!SehGuard("span8 native Rust",NativeGuard,&draw)) ++r.faults;
            fixtureClass=3;
            r.noCall += fixtureCalls==calls; r.refused += fixtureRefusals-refused;
            Compare(r,init,native,dst.bits,dst.stride);
        };
        Slot<ColourTable>(ctx,7)(ctx,nullptr,0,1,&colour);
        Slot<DwordSetter>(ctx,9)(ctx,nullptr,0);
        Slot<SourceSetter>(ctx,13)(ctx,nullptr,nullptr,0,0,0,0,nullptr);
        Slot<VoidContext>(ctx,14)(ctx,nullptr);
        for(unsigned slot=1;slot<=4;++slot) {
            Slot<DwordSetter>(ctx,8)(ctx,nullptr,(slot==3 || slot==4));
            Slot<GradientSetter>(ctx,12)(ctx,nullptr,(slot==2 || slot==4)?gradient:nullptr,0,0);
            run();
        }
        // Each native texture format; row13 is unreachable because the context clears type2 key.
        const uint32_t types[7]={7,6,4,5,9,8,2};
        for(unsigned f=0;f<7;++f) {
            Buffer src;
            if(!src.Make(64,32,types[f],f<4?16:f<6?32:8)) { ++r.refused; continue; }
            uint16_t sl=Field<uint16_t>(src.p,0x38), sf=Field<uint16_t>(src.p,0x44);
            // Construct buffer key state from the exact §10.7 fields consumed by GetColorKey.
            // Context setup stays entirely through public setters. No unreviewed key setter call.
            const uint32_t savedKey=Field<uint32_t>(src.p,0x24), savedFlags=Field<uint32_t>(src.p,0x28);
            uint32_t key=0; std::memcpy(&key,src.bits+src.stride+4*(f<4?2:f<6?4:1),f<4?2:f<6?4:1);
            const uint32_t masks[7]={0xFFF,0x7FFF,0x7FFF,0xFFFF,0xFFFFFF,0xFFFFFF,0xFF};
            key &= masks[f];
            for(unsigned keyed=0;keyed<(f==6?1u:2u);++keyed) {
            const uint32_t flags=(savedFlags&~2u)|(keyed?2u:0u);
            std::memcpy(reinterpret_cast<uint8_t*>(src.p)+0x24,&key,4);
            std::memcpy(reinterpret_cast<uint8_t*>(src.p)+0x28,&flags,4);
            for(unsigned k=0;k<12;++k) {
                int32_t out[4]={}; const int32_t tile[4]={0,0,8,8};
                Slot<GradientSetter>(ctx,12)(ctx,nullptr,nullptr,0,0);
                Slot<SourceSetter>(ctx,13)(ctx,nullptr,src.p,0,0,k/4==1?2.0f:1.0f,k/4==1?2.0f:1.0f,out);
                Slot<DwordSetter>(ctx,8)(ctx,nullptr,k&1);
                Slot<DwordSetter>(ctx,9)(ctx,nullptr,(k>>1)&1);
                if(k/4==2) Slot<TileSetter>(ctx,15)(ctx,nullptr,tile); else Slot<VoidContext>(ctx,14)(ctx,nullptr);
                run();
            }
            }
            std::memcpy(reinterpret_cast<uint8_t*>(src.p)+0x24,&savedKey,4);
            std::memcpy(reinterpret_cast<uint8_t*>(src.p)+0x28,&savedFlags,4);
            Slot<SourceSetter>(ctx,13)(ctx,nullptr,nullptr,0,0,0,0,nullptr);
            Log("span8 fixture texture type=%u locks_restored=%u",types[f],unsigned(sl==Field<uint16_t>(src.p,0x38) && sf==Field<uint16_t>(src.p,0x44)));
        }
    }
    if(right) Slot<VoidContext>(ctx,4)(ctx,nullptr); // Shutdown before Release restores target lock.
    bool locks=lock0==Field<uint16_t>(dst.p,0x38) && flags0==Field<uint16_t>(dst.p,0x44);
    bool cleared=right && !Field<void*>(ctx,0xB0);
    uint32_t left=FC4_RELEASE_CREATED(static_cast<cIGZUnknown*>(ctx));
    Log("span8 fixture lifetime: locks_restored=%u renderer_cleared=%u ctxRelease=%u slot0_original=%u",
        unsigned(locks),unsigned(cleared),left,unsigned(reinterpret_cast<uintptr_t>(table[0])==delta+0x0082FF00));
    Report("constructed_native",r);
}
void Fixture() { DirectFixture(); NativeFixture(); }
