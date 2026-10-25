// SPDX-License-Identifier: GPL-3.0-or-later
// S3: opt-in observational entry detours. Contracts: view-utilities.md §§1,12,13.
#include "FC4Inplace.h"
#include "FC4InplaceDetour.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include <windows.h>
#include <tlhelp32.h>
#include <intrin.h>
#include <array>
#include <algorithm>
#include <string>

extern "C" void fc4_viewutil_shadow_cached(uint32_t, const uint32_t*, const uint32_t*, uint32_t*);
extern "C" void fc4_viewutil_shadow_local(uint32_t, const uint32_t*, uint32_t*);

namespace {
using Packet = std::array<uint32_t, 32>;
struct Count {
    uint64_t calls = 0, sampled = 0, compared = 0, exact = 0, floatOnly = 0;
    uint64_t different = 0, outside = 0, refused = 0, nested = 0, mutation = 0, detected = 0;
    uint32_t maxUlp = 0;
};
struct Function {
    const char* name; uint32_t va, bytes; void* original = nullptr;
    Count natural{}, fixture{}; bool installed = false, mutate = false;
};
// All sizes are published body_bytes, not an assertion about instruction coverage.
Function functions[] = {
    {"clip2",0x7f6620,104}, {"zoom-rotation",0x7f66a0,21},
    {"view-viewport",0x7f66c0,67}, {"fixed",0x7f6710,96},
    {"view-box",0x7f6770,413}, {"city-corner",0x7f6ba0,181},
    {"clip3",0x7f7710,51}, {"visible-edges",0x7f7750,44},
    {"world-view",0x7f77d0,126}, {"world-viewport",0x7f7850,75},
    {"world-viewport2",0x7f78e0,54}, {"view-world",0x7f7920,133},
    {"viewport-world",0x7f79b0,77}, {"box-rectangle",0x7f7a40,1018},
    {"view-point",0x7f7e50,270}, {"viewport-fixed",0x7f8390,83},
    {"rotation",0x7f6cc0,78}, {"ground",0x7f6ab0,49},
    {"cross",0x5d6f10,59}, {"primitive-indices",0x80aef0,235}
};
SRWLOCK sampleLock = SRWLOCK_INIT;
thread_local unsigned depth = 0;
thread_local bool fixture = false;
bool active = false, fixtures = false;
uintptr_t imageBase = 0;

bool Selected(std::string list, const char* name) {
    list.erase(std::remove_if(list.begin(), list.end(), [](char c) { return c == ' ' || c == '\t'; }), list.end());
    return list == "all" || ("," + list + ",").find("," + std::string(name) + ",") != std::string::npos;
}
bool Supported() {
    imageBase = reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr));
    if (imageBase != 0x400000) return false;
    const auto* dos = reinterpret_cast<const IMAGE_DOS_HEADER*>(imageBase);
    if (dos->e_magic != IMAGE_DOS_SIGNATURE || dos->e_lfanew <= 0 || dos->e_lfanew > 0x1000) return false;
    const auto* nt = reinterpret_cast<const IMAGE_NT_HEADERS32*>(imageBase + dos->e_lfanew);
    return nt->Signature == IMAGE_NT_SIGNATURE && nt->FileHeader.Machine == 0x14c &&
        nt->FileHeader.TimeDateStamp == 0x4c12bfb7 && nt->OptionalHeader.Magic == 0x10b &&
        nt->OptionalHeader.ImageBase == 0x400000 && nt->OptionalHeader.SizeOfImage == 0x7c8000;
}

// Installation only: suspend other process threads before changing executable
// storage. No heap allocation/logging while suspended; reject an occupied entry.
struct PausedThreads {
    HANDLE handles[256]{}; unsigned count = 0; bool ok = true;
    PausedThreads(uintptr_t entry, unsigned bytes) {
        const auto snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if (snapshot == INVALID_HANDLE_VALUE) { ok = false; return; }
        THREADENTRY32 thread{}; thread.dwSize = sizeof(thread);
        BOOL next = Thread32First(snapshot, &thread);
        while (next) {
            if (thread.th32OwnerProcessID == GetCurrentProcessId() && thread.th32ThreadID != GetCurrentThreadId()) {
                if (count == 256) { ok = false; break; }
                HANDLE handle = OpenThread(THREAD_SUSPEND_RESUME | THREAD_GET_CONTEXT, FALSE, thread.th32ThreadID);
                if (!handle) { ok = false; break; }
                if (SuspendThread(handle) == DWORD(-1)) { CloseHandle(handle); ok = false; break; }
                handles[count++] = handle;
                CONTEXT context{}; context.ContextFlags = CONTEXT_CONTROL;
                if (!GetThreadContext(handle, &context) ||
                    (context.Eip >= entry && context.Eip < entry + bytes)) { ok = false; break; }
            }
            next = Thread32Next(snapshot, &thread);
        }
        CloseHandle(snapshot);
    }
    ~PausedThreads() { for (unsigned i = count; i; --i) { ResumeThread(handles[i-1]); CloseHandle(handles[i-1]); } }
};
bool Install(Function& fn, void* forwarder) {
    auto* entry = reinterpret_cast<uint8_t*>(fn.va);
    MEMORY_BASIC_INFORMATION memory{};
    if (!VirtualQuery(entry, &memory, sizeof(memory)) || memory.Type != MEM_IMAGE ||
        (memory.Protect & 0xff) != PAGE_EXECUTE_READ ||
        reinterpret_cast<uintptr_t>(memory.BaseAddress) + memory.RegionSize - fn.va < 24) return false;
    auto* trampoline = static_cast<uint8_t*>(VirtualAlloc(nullptr, 64, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE));
    if (!trampoline) return false;
    const auto bytes = fc4::inplace::Trampoline(entry, 24, fn.va, trampoline, reinterpret_cast<uintptr_t>(trampoline));
    DWORD previous = 0;
    if (!bytes || !VirtualProtect(trampoline, 64, PAGE_EXECUTE_READ, &previous)) {
        VirtualFree(trampoline, 0, MEM_RELEASE); return false;
    }
    FlushInstructionCache(GetCurrentProcess(), trampoline, 64);
    bool installed = false;
    {
        PausedThreads threads(fn.va, bytes);
        if (threads.ok && VirtualProtect(entry, 5, PAGE_EXECUTE_READWRITE, &previous)) {
            fn.original = trampoline;
            fc4::inplace::Jump(entry, fn.va, reinterpret_cast<uintptr_t>(forwarder));
            DWORD ignored = 0; VirtualProtect(entry, 5, previous, &ignored);
            FlushInstructionCache(GetCurrentProcess(), entry, 5);
            installed = true;
        }
    }
    if (!installed) VirtualFree(trampoline, 0, MEM_RELEASE);
    return installed;
}

struct Window { uint8_t* pointer; size_t size; int output; bool floating; };
struct Region { uint8_t* pointer = nullptr; size_t size = 0; uint8_t* before = nullptr; uint8_t* after = nullptr; };
struct Capture {
    std::array<Window, 6> windows{}; unsigned windowCount = 0;
    std::array<Region, 6> regions{}; unsigned regionCount = 0;
    size_t total = 0; bool ok = true; const char* reason = "invalid-window";
    bool Refuse(const char* why) { reason=why; return false; }
    ~Capture() { for (auto& r : regions) if (r.before) VirtualFree(r.before, 0, MEM_RELEASE); }
    void Add(void* pointer, size_t size, int output = -1, bool floating = true) {
        if (!pointer || windowCount == windows.size()) { ok = false; return; }
        windows[windowCount++] = {static_cast<uint8_t*>(pointer), size, output, floating};
    }
    bool Prepare(uintptr_t stackFloor) {
        if (!ok) return false;
        // Reject aliases involving outputs rather than silently changing native store order.
        for (unsigned i=0; i<windowCount; ++i) for (unsigned j=0; j<i; ++j) {
            const auto& a=windows[i]; const auto& b=windows[j];
            if ((a.output >= 0 || b.output >= 0) &&
                a.pointer < b.pointer+b.size && b.pointer < a.pointer+a.size) return Refuse("output-alias");
        }
        for (unsigned i=0; i<windowCount; ++i) {
            auto& w=windows[i]; MEMORY_BASIC_INFORMATION m{};
            if (!VirtualQuery(w.pointer,&m,sizeof(m)) || m.State != MEM_COMMIT || m.Type != MEM_PRIVATE ||
                (m.Protect != PAGE_READWRITE && m.Protect != PAGE_WRITECOPY) ||
                reinterpret_cast<uintptr_t>(w.pointer)+w.size > reinterpret_cast<uintptr_t>(m.BaseAddress)+m.RegionSize) return Refuse("not-private-writable-region");
            auto* begin=static_cast<uint8_t*>(m.BaseAddress);
            auto* end=begin+m.RegionSize;
            // All memory below the native caller's arguments is active observer/original
            // stack, not persistent call output. TLS identifies this thread's stack.
            const auto* tib = reinterpret_cast<const NT_TIB*>(NtCurrentTeb());
            if (begin >= tib->StackLimit && begin < tib->StackBase) begin = reinterpret_cast<uint8_t*>(stackFloor);
            if (w.pointer < begin || w.pointer+w.size > end) return Refuse("active-stack-or-cross-region");
            bool found=false;
            for (unsigned j=0; j<regionCount; ++j) if (regions[j].pointer==begin) found=true;
            if (found) continue;
            const auto size=static_cast<size_t>(end-begin);
            if (size > 64u*1024*1024 || total+size > 64u*1024*1024) return Refuse("region-cap");
            total += size;
            auto* storage=static_cast<uint8_t*>(VirtualAlloc(nullptr,size*2,MEM_RESERVE|MEM_COMMIT,PAGE_READWRITE));
            if (!storage) return Refuse("snapshot-allocation");
            regions[regionCount++]={begin,size,storage,storage+size};
        }
        return true;
    }
    void Snapshot(bool after) { for (unsigned i=0; i<regionCount; ++i) {
        auto& r=regions[i]; std::memcpy(after?r.after:r.before,r.pointer,r.size);
    } }
    const uint8_t* Saved(const Window& w, bool after) const {
        for (unsigned i=0; i<regionCount; ++i) { const auto& r=regions[i];
            if (w.pointer>=r.pointer && w.pointer+w.size<=r.pointer+r.size)
                return (after?r.after:r.before)+(w.pointer-r.pointer);
        }
        return nullptr;
    }
    void Restore(bool after) { for (unsigned i=0; i<windowCount; ++i) {
        const auto& w=windows[i]; std::memcpy(w.pointer,Saved(w,after),w.size);
    } }
    bool Outside(unsigned& region, size_t& offset) const {
        for (unsigned j=0; j<regionCount; ++j) { const auto& r=regions[j];
            // Fast equal-region path avoids per-byte work for read-only functions.
            if (std::memcmp(r.before,r.after,r.size)==0) continue;
            for (size_t n=0; n<r.size; ++n) if (r.before[n]!=r.after[n]) {
                bool captured=false;
                for (unsigned k=0; k<windowCount; ++k) {
                    const auto& w=windows[k];
                    if (r.pointer+n>=w.pointer && r.pointer+n<w.pointer+w.size) captured=true;
                }
                if (!captured) { region=j; offset=n; return true; }
            }
        }
        return false;
    }
};

template<class Native>
bool Observe(unsigned index, void* self, Packet input, Capture& capture, uintptr_t stackFloor, Native native) {
    auto& fn=functions[index]; auto& count=fixture?fn.fixture:fn.natural;
    // Nested game calls share this thread's lock, and retain their own sampler.
    const bool locking=depth==0;
    if (locking) AcquireSRWLockExclusive(&sampleLock);
    if (depth) ++count.nested;
    const auto call=++count.calls;
    if (!fixture && !fc4::inplace::Sample(call)) { if (locking) ReleaseSRWLockExclusive(&sampleLock); return native(); }
    ++count.sampled; ++depth;
    if (!capture.Prepare(stackFloor)) {
        ++count.refused;
        if (count.refused<=3) fc4::Log("inplace refused name=%s source=%s call=%llu reason=%s",fn.name,fixture?"fixture":"natural",call,capture.reason);
        --depth; if (locking) ReleaseSRWLockExclusive(&sampleLock); return native();
    }
    Packet out{};
    for (unsigned i=0; i<capture.windowCount; ++i) { const auto& w=capture.windows[i];
        if (w.output>=0) std::memcpy(out.data()+w.output,w.pointer,w.size);
    }
    capture.Snapshot(false);
    const bool result=native();
    capture.Snapshot(true);
    unsigned region=0; size_t offset=0;
    if (capture.Outside(region,offset)) {
        ++count.outside;
        if (count.outside<=3) fc4::Log("inplace outside name=%s source=%s call=%llu region=%u offset=%zu watched=%zu attribution=unknown",
            fn.name,fixture?"fixture":"natural",call,region,offset,capture.total);
    } else {
        capture.Restore(false);
        std::array<uint32_t,0x470/4> owner{};
        if (self) { std::memcpy(owner.data(),self,sizeof(owner)); fc4_viewutil_shadow_cached(fn.va,owner.data(),input.data(),out.data()); }
        else fc4_viewutil_shadow_local(fn.va,input.data(),out.data());
        for (unsigned i=0; i<capture.windowCount; ++i) { const auto& w=capture.windows[i];
            if (w.output>=0) std::memcpy(w.pointer,out.data()+w.output,w.size);
        }
        bool equal=true, floatOnly=true; unsigned firstWindow=0; size_t firstWord=0;
        uint32_t firstNative=0,firstOurs=0;
        const auto compare=[&](bool measureUlp) {
        equal=true;floatOnly=true;
        for (unsigned i=0; i<capture.windowCount; ++i) { const auto& w=capture.windows[i];
            for (size_t n=0; n<w.size; n+=4) {
                uint32_t a,b; std::memcpy(&a,capture.Saved(w,true)+n,4); std::memcpy(&b,w.pointer+n,4);
                if (a==b) continue;
                // Unexpected owner/input words can contain pointers: report only
                // their window/offset, never their raw values.
                if (equal) { firstWindow=i;firstWord=n/4;firstNative=w.output<0?0:a;firstOurs=w.output<0?0:b; }
                equal=false;
                if (w.output<0 || !w.floating || !fc4::inplace::NearFloat(a,b)) floatOnly=false;
                if (measureUlp && w.floating && w.output>=0) count.maxUlp=std::max(count.maxUlp,fc4::inplace::FloatDistance(a,b));
            }
        }
        if (index==4 || index==14) {
            if (out[0]!=static_cast<uint32_t>(result)) {
                firstWindow=99;firstWord=0;firstNative=result;firstOurs=out[0];equal=false;floatOnly=false;
            }
        }
        };
        compare(true);
        ++count.compared;
        if (equal) ++count.exact;
        else {
            if (floatOnly) ++count.floatOnly; else ++count.different;
            if (count.floatOnly+count.different<=3) fc4::Log("inplace diff name=%s source=%s call=%llu window=%u word=%zu native=%08x ours=%08x float_only=%u",
                fn.name,fixture?"fixture":"natural",call,firstWindow,firstWord,firstNative,firstOurs,floatOnly?1:0);
        }
        // One-value debug error, compared against the unmutated Rust replay.
        // Only exact samples can demonstrate independent comparator teeth.
        if (fn.mutate && equal && count.mutation==0) {
            ++count.mutation;
            if (index==4 || index==14) {
                out[0]^=1;
            }
            for (unsigned i=0; i<capture.windowCount; ++i) { const auto& w=capture.windows[i];
                if (w.output<0) continue;
                uint32_t value; std::memcpy(&value,w.pointer,4); value^=1;
                std::memcpy(w.pointer,&value,4);
                break;
            }
            compare(false); // Same comparator as normal replay, including location.
            count.detected+=!equal;
            fc4::Log("inplace mutation name=%s source=%s window=%u word=%zu xor=1 detected=%llu",fn.name,fixture?"fixture":"natural",firstWindow,firstWord,count.detected);
        }
        capture.Restore(true); // Game always sees the original's output.
    }
    --depth; if (locking) ReleaseSRWLockExclusive(&sampleLock); return result;
}

template<class T> T Original(unsigned i) { return reinterpret_cast<T>(functions[i].original); }
uintptr_t Floor(void* returnSlot, unsigned arguments) { return reinterpret_cast<uintptr_t>(returnSlot)+4+arguments; }

// Adapter lines below are deliberately small: published native windows -> existing
// production Rust transport. The generic observer owns snapshots and comparisons.
#define OWNER Capture c; c.Add(s,0x470); Packet p{}
#define RUN(I,N,F) Observe(I,s,p,c,Floor(_AddressOfReturnAddress(),N),F)
#define VECTOR(I,NAME) void __fastcall NAME(void* s,void*,void* a) { OWNER; c.Add(a,12,0); std::memcpy(p.data(),a,12); \
    RUN(I,4,[&] { Original<void(__thiscall*)(void*,void*)>(I)(s,a);return false; }); }
void __fastcall Clip2(void* s,void*,void* a) { OWNER; c.Add(a,8,0); std::memcpy(p.data(),a,8);
    RUN(0,4,[&] { Original<void(__thiscall*)(void*,void*)>(0)(s,a);return false; }); }
#define TWO(I,NAME) void __fastcall NAME(void* s,void*,void* a,void* b) { OWNER;c.Add(a,4,0,false);c.Add(b,4,1,false); \
    RUN(I,8,[&] { Original<void(__thiscall*)(void*,void*,void*)>(I)(s,a,b);return false; }); }
TWO(1,ZoomRotation)
VECTOR(2,ViewViewport)
VECTOR(3,Fixed)
bool __fastcall ViewBox(void* s,void*,void* a,void* b) { OWNER;c.Add(a,12);c.Add(b,12);
    std::memcpy(p.data(),a,12);std::memcpy(p.data()+3,b,12);
    return RUN(4,8,[&] { return Original<bool(__thiscall*)(void*,void*,void*)>(4)(s,a,b); }); }
void __fastcall CityCorner(void* s,void*,int corner,void* a) { OWNER;c.Add(a,12,0);p[0]=corner;
    RUN(5,8,[&] { Original<void(__thiscall*)(void*,int,void*)>(5)(s,corner,a);return false; }); }
VECTOR(6,Clip3)
TWO(7,VisibleEdges)
VECTOR(8,WorldView)
VECTOR(9,WorldViewport)
VECTOR(10,WorldViewport2)
VECTOR(11,ViewWorld)
VECTOR(12,ViewportWorld)
void __fastcall BoxRectangle(void* s,void*,void* a,void* b,void* tl,void* br) { OWNER;
    c.Add(a,12);c.Add(b,12);c.Add(tl,8,0,false);c.Add(br,8,2,false);
    std::memcpy(p.data(),a,12);std::memcpy(p.data()+3,b,12);
    RUN(13,16,[&] { Original<void(__thiscall*)(void*,void*,void*,void*,void*)>(13)(s,a,b,tl,br);return false; }); }
bool __fastcall ViewPoint(void* s,void*,void* a) { OWNER;c.Add(a,12);std::memcpy(p.data(),a,12);
    return RUN(14,4,[&] { return Original<bool(__thiscall*)(void*,void*)>(14)(s,a); }); }
VECTOR(15,ViewportFixed)
#undef VECTOR
#undef TWO
#undef RUN
#undef OWNER
void __cdecl Rotation(void* a,int rotation) { Capture c;c.Add(a,12,0);Packet p{};std::memcpy(p.data(),a,12);p[3]=rotation;
    Observe(16,nullptr,p,c,Floor(_AddressOfReturnAddress(),8),[&] { Original<void(__cdecl*)(void*,int)>(16)(a,rotation);return false; }); }
void __cdecl Ground(void* a,void* b,void* out) { Capture c;c.Add(a,12);c.Add(b,12);c.Add(out,12,0);Packet p{};
    std::memcpy(p.data(),a,12);std::memcpy(p.data()+3,b,12);
    Observe(17,nullptr,p,c,Floor(_AddressOfReturnAddress(),12),[&] { Original<void(__cdecl*)(void*,void*,void*)>(17)(a,b,out);return false; }); }
void __cdecl Cross(void* a,void* b,void* out) { Capture c;c.Add(a,12);c.Add(b,12);c.Add(out,12,0);Packet p{};
    std::memcpy(p.data(),a,12);std::memcpy(p.data()+3,b,12);p[6]=2;
    Observe(18,nullptr,p,c,Floor(_AddressOfReturnAddress(),12),[&] { Original<void(__cdecl*)(void*,void*,void*)>(18)(a,b,out);return false; }); }
void __cdecl PrimitiveIndices(int type,int triangle,void* out) { Capture c;c.Add(out,12,0,false);Packet p{};p[0]=type;p[1]=triangle;
    Observe(19,nullptr,p,c,Floor(_AddressOfReturnAddress(),12),[&] { Original<void(__cdecl*)(int,int,void*)>(19)(type,triangle,out);return false; }); }
#define FORWARDER(F) reinterpret_cast<void*>(F)
void* forwarders[] = {FORWARDER(Clip2),FORWARDER(ZoomRotation),FORWARDER(ViewViewport),FORWARDER(Fixed),
    FORWARDER(ViewBox),FORWARDER(CityCorner),FORWARDER(Clip3),FORWARDER(VisibleEdges),FORWARDER(WorldView),
    FORWARDER(WorldViewport),FORWARDER(WorldViewport2),FORWARDER(ViewWorld),FORWARDER(ViewportWorld),
    FORWARDER(BoxRectangle),FORWARDER(ViewPoint),FORWARDER(ViewportFixed),FORWARDER(Rotation),
    FORWARDER(Ground),FORWARDER(Cross),FORWARDER(PrimitiveIndices)};
#undef FORWARDER

void Report(const Function& fn, const Count& c, const char* source) {
    fc4::Log("inplace summary name=%s rva=%08x bytes=%u installed=%u source=%s calls=%llu sampled=%llu compared=%llu exact=%llu float_only=%llu different=%llu outside=%llu refused=%llu nested=%llu mutation=%llu detected=%llu max_ulp=%u",
        fn.name,fn.va-static_cast<uint32_t>(imageBase),fn.bytes,fn.installed?1:0,source,c.calls,c.sampled,c.compared,c.exact,c.floatOnly,c.different,c.outside,c.refused,c.nested,c.mutation,c.detected,c.maxUlp);
}
}

namespace fc4 {
void InplaceStartup() {
    const auto selection=Setting("inplace",nullptr);
    if (selection.empty()) return;
    if (!Supported()) { Log("inplace: unsupported executable identity; no patches");return; }
    const auto mutations=Setting("inplace_mutate",nullptr);
    fixtures=SettingInt("inplace_fixture",nullptr,0)!=0;
    for (unsigned i=0;i<std::size(functions);++i) {
        auto& fn=functions[i];
        if (!Selected(selection,fn.name)) continue;
        fn.mutate=Selected(mutations,fn.name);
        fn.installed=Install(fn,forwarders[i]); active|=fn.installed;
        Log("inplace install name=%s rva=%08x bytes=%u installed=%u reason=%s",fn.name,
            fn.va-static_cast<uint32_t>(imageBase),fn.bytes,fn.installed?1:0,fn.installed?"ok":"unsupported-entry-or-busy");
    }
    Log("inplace: active=%u sample=first50-then1in20 capture=private-regions cap_mib=64 writes=net-neighbourhood-only fixture=%u",active?1:0,fixtures?1:0);
}
void InplaceReport() {
    if (!active) return;
    AcquireSRWLockExclusive(&sampleLock);
    for (const auto& fn:functions) { Report(fn,fn.natural,"natural");Report(fn,fn.fixture,"fixture"); }
    ReleaseSRWLockExclusive(&sampleLock);
}
void InplaceCityLoaded() {
    if (!active || !fixtures) return;
    // Explicitly constructed data, not credited as a natural test-city call.
    // VirtualAlloc isolates fixture storage from observer stack/CRT heap changes.
    auto* words=static_cast<uint32_t*>(VirtualAlloc(nullptr,0x2000,MEM_COMMIT|MEM_RESERVE,PAGE_READWRITE));
    if (!words) return;
    const auto bits=[](float f) { uint32_t u;std::memcpy(&u,&f,4);return u; };
    words[0]=1;words[0x0c/4]=4;words[0x20/4]=words[0x24/4]=bits(4096);
    words[0x108/4]=words[0x110/4]=bits(-256); words[0x10c/4]=words[0x114/4]=bits(256);
    words[0x118/4]=bits(1);words[0x11c/4]=bits(8192);
    words[0x44c/4]=words[0x454/4]=bits(1);
    for (unsigned offset=0x458;offset<=0x46c;offset+=4) words[offset/4]=bits(1);
    auto* a=words+0x500/4;auto* b=a+8;auto* out=b+8;auto* br=out+8;
    fixture=true;
    for (unsigned sample=0;sample<16;++sample) for (unsigned i=0;i<std::size(functions);++i) {
        if (!functions[i].installed) continue;
        words[0x10/4]=sample%4;
        for (unsigned j=0;j<3;++j) { a[j]=bits(float(int(sample)-8)+float(j));b[j]=bits(float(sample)+float(j)+1);out[j]=bits(10); }
        const auto va=functions[i].va;
        switch (i) {
        case 1:case 7: reinterpret_cast<void(__thiscall*)(void*,void*,void*)>(va)(words,out,br);break;
        case 4: reinterpret_cast<bool(__thiscall*)(void*,void*,void*)>(va)(words,a,b);break;
        case 5: reinterpret_cast<void(__thiscall*)(void*,int,void*)>(va)(words,sample%5,out);break;
        case 13: reinterpret_cast<void(__thiscall*)(void*,void*,void*,void*,void*)>(va)(words,a,b,out,br);break;
        case 14: reinterpret_cast<bool(__thiscall*)(void*,void*)>(va)(words,a);break;
        case 16: reinterpret_cast<void(__cdecl*)(void*,int)>(va)(a,sample%4);break;
        case 17:case 18: reinterpret_cast<void(__cdecl*)(void*,void*,void*)>(va)(a,b,out);break;
        case 19: reinterpret_cast<void(__cdecl*)(int,int,void*)>(va)(sample%6,sample,out);break;
        default: reinterpret_cast<void(__thiscall*)(void*,void*)>(va)(words,a);break;
        }
    }
    fixture=false;
    VirtualFree(words,0,MEM_RELEASE);
    InplaceReport();
}
}
