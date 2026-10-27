// SPDX-License-Identifier: GPL-3.0-or-later
// Private same-process recording host; facts: legacy-control §§6.1,6.2,7,8,9.
#pragma once
#include "FC4RefAudit.h"
#include "FC4InplaceDetour.h"
#include "FC4LegacyCtlHooks.h"
#include "cRZCOMDllDirector.h"
#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cIGZGraphicSystem.h"
#include "cIGZVariant.h"
#include "cIGZString.h"
#include "cRZBaseString.h"
#include <map>
#include <vector>
#include <string>
#include <array>
#include <mutex>
#include <algorithm>
#include <atomic>

#if defined(_M_IX86)
namespace fc4::legacyrecord {
using Words = std::vector<uint32_t>;
struct Property { uint32_t key, kind, number; const uint8_t* bytes; uint32_t length; };
using Exchange = uint32_t(__cdecl*)(void*, const uint32_t*, uint32_t, uint32_t*, uint32_t);
extern "C" int32_t fc4_legacyctl_shadow_run(const uint32_t*, uint32_t, const uint32_t*, const Property*, uint32_t,
    Exchange, void*, uint32_t*);
inline Words None() { return {0}; }
inline Words U(uint32_t n) { return {1,n}; }
inline Words I(uint32_t n) { return {2,n}; }
inline Words H(const void* p) { return {3,reinterpret_cast<uint32_t>(p)}; }
inline void Text(Words& w, const char* s, uint32_t n) {
    w.push_back(n);
    for (uint32_t i=0; i<n; ++i) w.push_back(static_cast<uint8_t>(s[i]));
}
inline Words S(cIGZString* s) { Words w{4}; if (s) Text(w,s->Data(),s->Strlen()); else w.push_back(0); return w; }
inline Words R(const void* r) {
    Words w{5,0,0,0,0}; if (r) std::memcpy(w.data()+1,r,16); return w;
}
inline Words Call(uint32_t op, const void* self, uint32_t slot, std::initializer_list<Words> args={}) {
    Words w{op,reinterpret_cast<uint32_t>(self),slot,static_cast<uint32_t>(args.size())};
    for (const auto& a:args) w.insert(w.end(),a.begin(),a.end());
    return w;
}
struct Input {
    struct Owned { uint32_t key=0,kind=0xffff,number=0; std::string bytes; };
    std::vector<Owned> owned;
    std::vector<Property> wire;
    bool valid=true;
    void Capture(void* props) {
        if (!props) { valid=false; return; }
        const auto* bounds=static_cast<const uint32_t*>(props);
        if (bounds[1]<bounds[0] || (bounds[1]-bounds[0])%8 || (bounds[1]-bounds[0])/8>65536) { valid=false; return; }
        const auto* pairs=reinterpret_cast<const uint32_t*>(bounds[0]);
        const uint32_t n=(bounds[1]-bounds[0])/8;
        owned.reserve(n);
        for (uint32_t k=0;k<n;++k) {
            Owned p; p.key=pairs[k*2];
            auto* v=reinterpret_cast<cIGZVariant*>(pairs[k*2+1]);
            if (v) {
                p.kind=v->GetType();
                if (p.kind==6) p.number=v->GetValUint32();
                else if (p.kind==7) p.number=static_cast<uint32_t>(v->GetValSint32());
                else if (p.kind==0x800e) {
                    cRZBaseString s;
                    if (v->GetValString(s)) p.bytes.assign(s.Data(),s.Strlen()); else valid=false;
                } else if (p.kind!=0) valid=false;
            }
            owned.push_back(std::move(p));
        }
        for (const auto& p:owned) wire.push_back({p.key,p.kind,p.number,reinterpret_cast<const uint8_t*>(p.bytes.data()),static_cast<uint32_t>(p.bytes.size())});
    }
    bool HasString(uint32_t key) const {
        return std::any_of(owned.begin(),owned.end(),[key](const Owned& p){return p.key==key && p.kind==0x800e;});
    }
};
struct Event { Words call,result; };
struct Trace {
    std::vector<Event> events;
    std::vector<void*> parameterTemporaries;
    size_t cursor=0; uint32_t bad=0,method=0; bool exhaustive=true,allOutputs=true;
};
inline thread_local Trace* active=nullptr;
inline thread_local unsigned suppressed=0;
struct Quiet { Quiet(){++suppressed;} ~Quiet(){--suppressed;} };
struct Scope {
    Trace* previous; unsigned prior;
    explicit Scope(Trace& t):previous(active),prior(suppressed){active=&t;suppressed=0;}
    ~Scope(){active=previous;suppressed=prior;}
};
inline bool Recording() { return active && !suppressed; }
inline void Record(Words call,Words result=None()) {
    if (Recording()) active->events.push_back({std::move(call),std::move(result)});
}
inline void ObserveUnknown(void*);
inline void ObserveWindow(void*);
inline void ObserveInterface(void*,uint32_t);
inline bool __fastcall Query(void* self,void*,uint32_t iid,void** out) {
    const bool record=Recording(); bool ok;
    { Quiet q; ok=reinterpret_cast<bool(__fastcall*)(void*,void*,uint32_t,void**)>(Original(self,0))(self,nullptr,iid,out); }
    if (record) {
        Record(Call(2,self,iid),ok && out ? H(*out):H(nullptr));
        if (ok && out && *out) ObserveInterface(*out,iid);
    }
    return ok;
}
template<bool Release> uint32_t __fastcall Ref(void* self,void*) {
    const bool record=Recording(); uint32_t n;
    { Quiet q; n=reinterpret_cast<uint32_t(__fastcall*)(void*,void*)>(Original(self,Release?8:4))(self,nullptr); }
    if(record) Record(Call(Release?4:3,self,0),Release?U(n):None());
    return n;
}
inline void ObserveUnknown(void* object) {
    if (!object) return;
    const bool ok=Patch(object,0,reinterpret_cast<void*>(&Query)) && Patch(object,4,reinterpret_cast<void*>(&Ref<false>)) && Patch(object,8,reinterpret_cast<void*>(&Ref<true>));
    if (!ok && active) active->exhaustive=false;
}
inline Words Parameter(cIGZVariant* v,uint32_t key) {
    if (!v) return None();
    Quiet q;
    const auto kind=v->GetType();
    if(kind==6) return U(v->GetValUint32());
    if(kind==0x800e) { cRZBaseString s; if(v->GetValString(s)) return S(&s); }
    if(kind==0x8000) {
        uint32_t data[3]{},n=sizeof(data);
        // [abi-ret] cIGZVariant::GetValVoid uses the SDK bool result and byte-count out parameter.
        if(v->GetValVoid(data,n)) {
            if(n==8) return {7,data[0],data[1]};
            if(n==12) return {9,data[0],data[1],data[2]};
        }
    }
    if(kind==0x10 && key==0xbaca6879) {
        // SPEC-GAP: §8 identifies the private comment-list object, not an SDK list interface.
        if(active) active->exhaustive=false;
        return None();
    }
    if(active) active->exhaustive=false;
    return None();
}
// Native argument forms: 0=U32, 1=I32, 2=identity, 3=string, 4=rectangle.
inline Words Arg(uint32_t n,unsigned form) {
    switch(form) {case 1:return I(n);case 2:return H(reinterpret_cast<void*>(n));case 3:return S(reinterpret_cast<cIGZString*>(n));case 4:return n?R(reinterpret_cast<void*>(n)):None();case 5:return U((n&255)!=0);case 6:return n?H(reinterpret_cast<void*>(n)):None();default:return U(n);}
}
template<uint32_t Slot,unsigned Op=7> uint32_t __fastcall Zero(void* self,void*) {
    Record(Call(Op,self,Slot)); Quiet q;
    return reinterpret_cast<uint32_t(__fastcall*)(void*,void*)>(Original(self,Slot))(self,nullptr);
}
template<uint32_t Slot,unsigned Op=7,unsigned Form=0> uint32_t __fastcall One(void* self,void*,uint32_t a) {
    if(Recording()) Record(Call(Op,self,Slot,{Arg(a,Form)})); Quiet q;
    return reinterpret_cast<uint32_t(__fastcall*)(void*,void*,uint32_t)>(Original(self,Slot))(self,nullptr,a);
}
template<uint32_t Slot,unsigned Op=7,unsigned A=0,unsigned B=0> uint32_t __fastcall Two(void* self,void*,uint32_t a,uint32_t b) {
    if(Recording()) Record(Call(Op,self,Slot,{Arg(a,A),Arg(b,B)})); Quiet q;
    return reinterpret_cast<uint32_t(__fastcall*)(void*,void*,uint32_t,uint32_t)>(Original(self,Slot))(self,nullptr,a,b);
}
template<uint32_t Slot,unsigned A=0,unsigned B=0,unsigned C=0> uint32_t __fastcall Three(void* self,void*,uint32_t a,uint32_t b,uint32_t c) {
    if(Recording()) Record(Call(8,self,Slot,{Arg(a,A),Arg(b,B),Arg(c,C)})); Quiet q;
    return reinterpret_cast<uint32_t(__fastcall*)(void*,void*,uint32_t,uint32_t,uint32_t)>(Original(self,Slot))(self,nullptr,a,b,c);
}
template<uint32_t Slot> uint32_t __fastcall ReadWord(void* self,void*) {
    uint32_t value; {Quiet q;value=reinterpret_cast<uint32_t(__fastcall*)(void*,void*)>(Original(self,Slot))(self,nullptr);}
    Record(Call(8,self,Slot),U(value)); return value;
}
template<uint32_t Slot,unsigned Form=0> uint32_t __fastcall Quad(void* self,void*,uint32_t a,uint32_t b,uint32_t c,uint32_t d) {
    if(Recording()) Record(Call(8,self,Slot,{Arg(a,Form),Arg(b,Form),Arg(c,Form),Arg(d,Form)})); Quiet q;
    return reinterpret_cast<uint32_t(__fastcall*)(void*,void*,uint32_t,uint32_t,uint32_t,uint32_t)>(Original(self,Slot))(self,nullptr,a,b,c,d);
}
inline uint32_t __fastcall Message(void* self,void*,cIGZString* text,uint32_t fore,uint32_t back,uint32_t a,uint32_t b) {
    if(Recording()) Record(Call(8,self,0xc,{S(text),U(fore),U(back),U(a),U(b)})); Quiet q;
    return reinterpret_cast<uint32_t(__fastcall*)(void*,void*,cIGZString*,uint32_t,uint32_t,uint32_t,uint32_t)>(Original(self,0xc))(self,nullptr,text,fore,back,a,b);
}
template<uint32_t Slot> uint32_t __fastcall Four(void* self,void*,uint32_t a,uint32_t b,uint32_t c,uint32_t d) {
    // §8's colour reader and §9.7/§9.8 pass byte channels; upper stack bits are undefined.
    Record(Call(8,self,Slot,{U(a),U(b&255),U(c&255),U(d&255)})); Quiet q;
    return reinterpret_cast<uint32_t(__fastcall*)(void*,void*,uint32_t,uint32_t,uint32_t,uint32_t)>(Original(self,Slot))(self,nullptr,a,b,c,d);
}
inline uint32_t __fastcall Points(void* self,void*,uint32_t n,const uint32_t* p,uint32_t a,uint32_t b) {
    if(Recording()) {
        if(n>32 || !p) active->exhaustive=false;
        else {Words points{11,n};points.insert(points.end(),p,p+n*2);Record(Call(8,self,0x34,{U(n),points,U(a),U(b)}));}
    }
    Quiet q;
    return reinterpret_cast<uint32_t(__fastcall*)(void*,void*,uint32_t,const uint32_t*,uint32_t,uint32_t)>(Original(self,0x34))(self,nullptr,n,p,a,b);
}
inline uint32_t __fastcall Fill(void* self,void*,uint32_t r,uint32_t g,uint32_t b) {
    Record(Call(7,self,0x1ac,{{6,r&255,g&255,b&255}})); Quiet q;
    return reinterpret_cast<uint32_t(__fastcall*)(void*,void*,uint32_t,uint32_t,uint32_t)>(Original(self,0x1ac))(self,nullptr,r,g,b);
}
inline uint32_t __fastcall ParameterRelease(void* self,void*) {
    bool record=false;
    if(Recording()) {
        auto& pending=active->parameterTemporaries;
        const auto found=std::find(pending.begin(),pending.end(),self);
        if(found!=pending.end()) {pending.erase(found);record=true;}
    }
    uint32_t n;
    {Quiet q;n=reinterpret_cast<uint32_t(__fastcall*)(void*,void*)>(Original(self,8))(self,nullptr);}
    if(record) Record(Call(4,self,0),U(n));
    return n;
}
inline uint32_t __fastcall SetParam(void* self,void*,uint32_t key,cIGZVariant* value) {
    if(Recording()) {
        // The transfer returns an opaque temporary identity to the recording
        // host, not the setter's (unspecified) EAX. Observe its subsequent
        // caller-side Release; nested retention inside SetParameter stays quiet.
        // Construction before this boundary is still unobserved: no full credit.
        active->allOutputs=false;
        const auto payload=Parameter(value,key);
        const bool temporary=value && !payload.empty() && payload[0]!=0;
        Record(Call(7,self,0x1c8,{U(key),payload}),temporary?H(value):None());
        // Variants share a table with unrelated inputs and allocations. Only
        // the reference handed to this setter belongs to this observation;
        // construction/reference traffic before it remains explicitly partial.
        if(temporary) {
            active->parameterTemporaries.push_back(value);
            if(!Patch(value,8,reinterpret_cast<void*>(&ParameterRelease),true)) active->exhaustive=false;
        }
    }
    Quiet q;
    return reinterpret_cast<uint32_t(__fastcall*)(void*,void*,uint32_t,cIGZVariant*)>(Original(self,0x1c8))(self,nullptr,key,value);
}
inline uint32_t __fastcall GetID(void* self,void*) {
    uint32_t n; {Quiet q;n=reinterpret_cast<uint32_t(__fastcall*)(void*,void*)>(Original(self,0xfc))(self,nullptr);}
    Record(Call(6,self,0xfc),U(n)); return n;
}
inline uint32_t __fastcall GetArea(void* self,void*,void* rect) {
    uint32_t n; {Quiet q;n=reinterpret_cast<uint32_t(__fastcall*)(void*,void*,void*)>(Original(self,0xbc))(self,nullptr,rect);}
    Record(Call(6,self,0xbc),R(rect)); return n;
}
inline void ObserveWindow(void* object) {
    if(!object) return;
    ObserveUnknown(object);
    const bool ok=Patch(object,0x10,reinterpret_cast<void*>(&Zero<0x10>)) &&
        Patch(object,0x14,reinterpret_cast<void*>(&Zero<0x14>)) &&
        Patch(object,0x38,reinterpret_cast<void*>(&One<0x38,7,2>)) &&
        Patch(object,0x3c,reinterpret_cast<void*>(&One<0x3c,7,2>)) &&
        Patch(object,0x5c,reinterpret_cast<void*>(&Zero<0x5c>)) &&
        Patch(object,0xfc,reinterpret_cast<void*>(&GetID)) && Patch(object,0xbc,reinterpret_cast<void*>(&GetArea)) &&
        Patch(object,0x100,reinterpret_cast<void*>(&One<0x100>)) &&
        Patch(object,0xd8,reinterpret_cast<void*>(&One<0xd8,7,4>)) &&
        Patch(object,0xd4,reinterpret_cast<void*>(&Two<0xd4,7,1,1>)) &&
        Patch(object,0xe0,reinterpret_cast<void*>(&Two<0xe0,7,1,1>)) &&
        Patch(object,0x110,reinterpret_cast<void*>(&Two<0x110,7,0,5>)) &&
        Patch(object,0x128,reinterpret_cast<void*>(&One<0x128,7,3>)) &&
        Patch(object,0x1ac,reinterpret_cast<void*>(&Fill)) && Patch(object,0x1c8,reinterpret_cast<void*>(&SetParam));
    if(!ok && active) active->exhaustive=false;
}
inline void ObserveInterface(void* object,uint32_t iid) {
    ObserveUnknown(object); bool ok=true;
    switch(iid) {
    case 0x22ba0121: ObserveWindow(object); break;
    case 0xc2afa76f:
        ok=Patch(object,0x30,reinterpret_cast<void*>(&One<0x30,8>)) && Patch(object,0x28,reinterpret_cast<void*>(&Four<0x28>));break;
    case 0xe303e5ae:
        ok=Patch(object,0xc,reinterpret_cast<void*>(&One<0xc,8>)) && Patch(object,0x14,reinterpret_cast<void*>(&Four<0x14>)) && Patch(object,0x34,reinterpret_cast<void*>(&Points));break;
    case 0x21325207:
        ok=Patch(object,0x20,reinterpret_cast<void*>(&One<0x20,8,1>)) && Patch(object,0x18,reinterpret_cast<void*>(&One<0x18,8,1>)) &&
            Patch(object,0x28,reinterpret_cast<void*>(&One<0x28,8>)) && Patch(object,0x14,reinterpret_cast<void*>(&One<0x14,8,1>));break;
    case 0xa13351b8:
        ok=Patch(object,0x1c,reinterpret_cast<void*>(&Zero<0x1c,8>)) && Patch(object,0x10,reinterpret_cast<void*>(&Two<0x10,8,3>)) &&
            Patch(object,0x28,reinterpret_cast<void*>(&One<0x28,8,2>)) && Patch(object,0x2c,reinterpret_cast<void*>(&One<0x2c,8,2>));break;
    case 0x5386d516:
        ok=Patch(object,0x1c,reinterpret_cast<void*>(&One<0x1c,8,2>));
        // Only DoExceptions' slot7 is captured; the Gen interpreter has more effects.
        if(active) active->exhaustive=false;
        break;
    case 0x212cdc1f:
        ok=Patch(object,0x1c,reinterpret_cast<void*>(&Two<0x1c,8,0,5>)) &&
            Patch(object,0x4c,reinterpret_cast<void*>(&One<0x4c,8>)) && Patch(object,0x24,reinterpret_cast<void*>(&Two<0x24,8>)) &&
            Patch(object,0x5c,reinterpret_cast<void*>(&Two<0x5c,8,1,1>)) && Patch(object,0x64,reinterpret_cast<void*>(&Two<0x64,8,1,1>)) &&
            Patch(object,0x54,reinterpret_cast<void*>(&One<0x54,8>)) && Patch(object,0x14,reinterpret_cast<void*>(&Two<0x14,8>));break;
    case 0x231a1c57:
        ok=Patch(object,0x140,reinterpret_cast<void*>(&ReadWord<0x140>)) && Patch(object,0x134,reinterpret_cast<void*>(&One<0x134,8>)) &&
            Patch(object,0x28,reinterpret_cast<void*>(&One<0x28,8>)) && Patch(object,0x30,reinterpret_cast<void*>(&Two<0x30,8>)) &&
            Patch(object,0x48,reinterpret_cast<void*>(&One<0x48,8>)) && Patch(object,0x5c,reinterpret_cast<void*>(&One<0x5c,8>)) &&
            Patch(object,0x94,reinterpret_cast<void*>(&Two<0x94,8,5>)) && Patch(object,0x84,reinterpret_cast<void*>(&One<0x84,8>)) &&
            Patch(object,0xa0,reinterpret_cast<void*>(&One<0xa0,8,1>)) && Patch(object,0xa8,reinterpret_cast<void*>(&One<0xa8,8,1>)) &&
            Patch(object,0x100,reinterpret_cast<void*>(&One<0x100,8,5>)) && Patch(object,0x108,reinterpret_cast<void*>(&One<0x108,8>)) &&
            Patch(object,0x110,reinterpret_cast<void*>(&Two<0x110,8,1,1>)) && Patch(object,0x120,reinterpret_cast<void*>(&One<0x120,8>)) &&
            Patch(object,0x144,reinterpret_cast<void*>(&One<0x144,8>)) && Patch(object,0xac,reinterpret_cast<void*>(&Zero<0xac,8>));break;
    case 0x4132242b:
        ok=Patch(object,0x8c,reinterpret_cast<void*>(&Two<0x8c,8,5,5>)) && Patch(object,0x98,reinterpret_cast<void*>(&One<0x98,8,2>)) &&
            Patch(object,0x9c,reinterpret_cast<void*>(&Two<0x9c,8>)) && Patch(object,0xa8,reinterpret_cast<void*>(&One<0xa8,8>)) &&
            Patch(object,0xcc,reinterpret_cast<void*>(&One<0xcc,8,5>)) && Patch(object,0xdc,reinterpret_cast<void*>(&Two<0xdc,8>)) &&
            Patch(object,0xd4,reinterpret_cast<void*>(&One<0xd4,8,5>)) && Patch(object,0xe4,reinterpret_cast<void*>(&Two<0xe4,8,1,1>)) &&
            Patch(object,0xec,reinterpret_cast<void*>(&Two<0xec,8,1,1>)) && Patch(object,0x44,reinterpret_cast<void*>(&One<0x44,8,5>)) &&
            Patch(object,0x1c,reinterpret_cast<void*>(&Three<0x1c,3,0,0>)) && Patch(object,0x38,reinterpret_cast<void*>(&Zero<0x38,8>)) &&
            Patch(object,0x40,reinterpret_cast<void*>(&Zero<0x40,8>)) && Patch(object,0x28,reinterpret_cast<void*>(&Two<0x28,8,0,5>)) &&
            Patch(object,0x6c,reinterpret_cast<void*>(&One<0x6c,8>)) && Patch(object,0x74,reinterpret_cast<void*>(&One<0x74,8>)) &&
            Patch(object,0x24,reinterpret_cast<void*>(&Zero<0x24,8>)) && Patch(object,0xbc,reinterpret_cast<void*>(&Zero<0xbc,8>)) &&
            Patch(object,0xc0,reinterpret_cast<void*>(&Zero<0xc0,8>));break;
    case 0x61325a2d:
        ok=Patch(object,0x1c,reinterpret_cast<void*>(&Two<0x1c,8,1,1>)) && Patch(object,0x24,reinterpret_cast<void*>(&One<0x24,8>)) &&
            Patch(object,0x2c,reinterpret_cast<void*>(&One<0x2c,8>)) && Patch(object,0x34,reinterpret_cast<void*>(&One<0x34,8>)) &&
            Patch(object,0x3c,reinterpret_cast<void*>(&Two<0x3c,8,1,1>)) && Patch(object,0x14,reinterpret_cast<void*>(&One<0x14,8,1>));break;
    case 0x412ce496:
        ok=Patch(object,0x2c,reinterpret_cast<void*>(&One<0x2c,8,2>)) && Patch(object,0x94,reinterpret_cast<void*>(&One<0x94,8>)) &&
            Patch(object,0x9c,reinterpret_cast<void*>(&Two<0x9c,8,1,1>)) && Patch(object,0x30,reinterpret_cast<void*>(&Two<0x30,8>)) &&
            Patch(object,0x40,reinterpret_cast<void*>(&One<0x40,8>)) && Patch(object,0x58,reinterpret_cast<void*>(&One<0x58,8>)) &&
            Patch(object,0x8c,reinterpret_cast<void*>(&One<0x8c,8>)) && Patch(object,0x54,reinterpret_cast<void*>(&One<0x54,8,5>)) &&
            Patch(object,0x6c,reinterpret_cast<void*>(&Two<0x6c,8,3>)) && Patch(object,0x70,reinterpret_cast<void*>(&Two<0x70,8,0,5>));break;
    case 0x8810:
        ok=Patch(object,0x14,reinterpret_cast<void*>(&Two<0x14,8,0,5>)) && Patch(object,0x50,reinterpret_cast<void*>(&One<0x50,8>)) &&
            Patch(object,0x4c,reinterpret_cast<void*>(&One<0x4c,8,2>)) && Patch(object,0x5c,reinterpret_cast<void*>(&Two<0x5c,8>)) &&
            Patch(object,0x78,reinterpret_cast<void*>(&Two<0x78,8,1,1>)) && Patch(object,0x74,reinterpret_cast<void*>(&Quad<0x74,1>)) &&
            Patch(object,0x30,reinterpret_cast<void*>(&Three<0x30,0,6,4>)) && Patch(object,0x6c,reinterpret_cast<void*>(&One<0x6c,8>)) &&
            Patch(object,0x8c,reinterpret_cast<void*>(&One<0x8c,8,3>)) && Patch(object,0xa0,reinterpret_cast<void*>(&Two<0xa0,8,1,1>)) &&
            Patch(object,0x98,reinterpret_cast<void*>(&One<0x98,8>)) && Patch(object,0xb4,reinterpret_cast<void*>(&Two<0xb4,8>));break;
    case 0xc12cea13:
        ok=Patch(object,0x18,reinterpret_cast<void*>(&Two<0x18,8>)) && Patch(object,0x2c,reinterpret_cast<void*>(&Two<0x2c,8,0,5>)) &&
            Patch(object,0x20,reinterpret_cast<void*>(&One<0x20,8,4>));break;
    case 0x21335c59:
        ok=Patch(object,0x10,reinterpret_cast<void*>(&One<0x10,8,2>)) && Patch(object,0x14,reinterpret_cast<void*>(&Two<0x14,8>)) &&
            Patch(object,0x38,reinterpret_cast<void*>(&Two<0x38,8>)) && Patch(object,0x44,reinterpret_cast<void*>(&One<0x44,8>)) &&
            Patch(object,0x4c,reinterpret_cast<void*>(&One<0x4c,8>)) && Patch(object,0x24,reinterpret_cast<void*>(&One<0x24,8,5>)) &&
            Patch(object,0x2c,reinterpret_cast<void*>(&One<0x2c,8>)) && Patch(object,0x84,reinterpret_cast<void*>(&Quad<0x84,1>)) &&
            Patch(object,0x64,reinterpret_cast<void*>(&One<0x64,8>)) && Patch(object,0x70,reinterpret_cast<void*>(&One<0x70,8,5>)) &&
            Patch(object,0x74,reinterpret_cast<void*>(&One<0x74,8>)) && Patch(object,0x7c,reinterpret_cast<void*>(&Two<0x7c,8,1,1>));break;
    case 0x612ce0c3:
        ok=Patch(object,0x10,reinterpret_cast<void*>(&One<0x10,8>)) && Patch(object,0x18,reinterpret_cast<void*>(&Two<0x18,8,0,5>)) &&
            Patch(object,0x4c,reinterpret_cast<void*>(&One<0x4c,8,2>)) && Patch(object,0x5c,reinterpret_cast<void*>(&Two<0x5c,8>)) &&
            Patch(object,0x6c,reinterpret_cast<void*>(&One<0x6c,8>)) && Patch(object,0x74,reinterpret_cast<void*>(&Two<0x74,8,1,1>)) &&
            Patch(object,0x7c,reinterpret_cast<void*>(&Quad<0x7c>)) && Patch(object,0x28,reinterpret_cast<void*>(&Two<0x28,8,1,1>)) &&
            Patch(object,0x30,reinterpret_cast<void*>(&One<0x30,8,1>)) && Patch(object,0x24,reinterpret_cast<void*>(&One<0x24,8,1>));break;
    case 0x032f0b3c:
        ok=Patch(object,0x20,reinterpret_cast<void*>(&One<0x20,8>)) && Patch(object,0x24,reinterpret_cast<void*>(&One<0x24,8>)) &&
            Patch(object,0x1c,reinterpret_cast<void*>(&One<0x1c,8,2>)) && Patch(object,0xc,reinterpret_cast<void*>(&Message));break;
    case 0x9a4527cb: break; // owner's GetControl wrapper records the borrowed result
    default: ok=false;break;
    }
    if(!ok && active) active->exhaustive=false;
}
inline bool __fastcall Create(void* self,void*,uint32_t cls,uint32_t iid,void** out) {
    const bool record=Recording() && iid==0x22ba0121; bool ok;
    {Quiet q;ok=reinterpret_cast<bool(__fastcall*)(void*,void*,uint32_t,uint32_t,void**)>(Original(self,0xc))(self,nullptr,cls,iid,out);}
    if(record) {
        // A failed factory may leave its out-pointer untouched.
        void* created=ok && out?*out:nullptr;
        Record(Call(1,nullptr,cls,{U(iid)}),H(created));
        if(created) ObserveWindow(created);
    }
    return ok;
}
inline void* __fastcall MainWindow(void* self,void*) {
    void* win; {Quiet q;win=reinterpret_cast<void*(__fastcall*)(void*,void*)>(Original(self,0xc))(self,nullptr);}
    Record(Call(19,nullptr,0xc),H(win)); if(Recording()) ObserveWindow(win); return win;
}
inline bool Setup() {
    Quiet q;
    auto* director=RZGetCOMDllDirector(); auto* com=director?director->GZCOM():nullptr;
    bool ok=com && Patch(com,0xc,reinterpret_cast<void*>(&Create),true);
    auto* fw=RZGetFrameWork(); cIGZUnknown* mgr=nullptr;
    if(fw && fw->GetSystemService(2752988254ul,1444ul,reinterpret_cast<void**>(&mgr)) && mgr) {
        FC4_ACQUIRED(mgr,"legacy WinMgr service");
        ok=Patch(mgr,0xc,reinterpret_cast<void*>(&MainWindow)) && ok;
        FC4_RELEASE(mgr);
    } else ok=false;
    return ok;
}
inline uint32_t __fastcall Colour(void* self,void*,uint32_t r,uint32_t g,uint32_t b) {
    uint32_t value; {Quiet q;value=reinterpret_cast<uint32_t(__fastcall*)(void*,void*,uint32_t,uint32_t,uint32_t)>(Original(self,0x88))(self,nullptr,r,g,b);}
    Record(Call(11,nullptr,0,{{6,r&255,g&255,b&255}}),U(value)); return value;
}
inline void* __fastcall Font(void* self,void*,uint32_t id) {
    void* value; {Quiet q;value=reinterpret_cast<void*(__fastcall*)(void*,void*,uint32_t)>(Original(self,0x14))(self,nullptr,id);}
    Record(Call(10,nullptr,id),H(value)); return value;
}
inline uint32_t __fastcall FontID(void* self,void*,cIGZString* name) {
    uint32_t value; {Quiet q;value=reinterpret_cast<uint32_t(__fastcall*)(void*,void*,cIGZString*)>(Original(self,0x98))(self,nullptr,name);}
    if(Recording()) Record(Call(9,nullptr,0,{S(name)}),U(value)); return value;
}
inline bool SetupDrawing() {
    Quiet q;
    auto* fw=RZGetFrameWork(); cIGZUnknown* graphics=nullptr; cIGZUnknown* fonts=nullptr;
    // Service identities from SDK GZServPtrs; calls from spec sections 9.2/9.9.
    // GS2 has a separate SDK vtable. Section 9.2 names the legacy GS interface.
    bool ok=fw && fw->GetSystemService(3289776732ul,GZIID_cIGZGraphicSystem,reinterpret_cast<void**>(&graphics)) && graphics;
    if(graphics) {
        FC4_ACQUIRED(graphics,"legacy graphics service");
        ok=Patch(graphics,0x88,reinterpret_cast<void*>(&Colour),true) && ok;
        FC4_RELEASE(graphics);
    }
    ok=fw && fw->GetSystemService(3267554063ul,583199519ul,reinterpret_cast<void**>(&fonts)) && fonts && ok;
    if(fonts) {
        FC4_ACQUIRED(fonts,"legacy font service");
        ok=Patch(fonts,0x14,reinterpret_cast<void*>(&Font),true) && Patch(fonts,0x98,reinterpret_cast<void*>(&FontID),true) && ok;
        FC4_RELEASE(fonts);
    }
    return ok;
}
inline uint32_t __cdecl ExchangeCall(void* context,const uint32_t* input,uint32_t count,uint32_t* output,uint32_t capacity) {
    auto& trace=*static_cast<Trace*>(context);
    if(trace.cursor>=trace.events.size()) {++trace.bad;return UINT32_MAX;}
    const auto& event=trace.events[trace.cursor++];
    if(event.call.size()!=count || !std::equal(event.call.begin(),event.call.end(),input)) {
        // Keep unsupported startup traces from consuming another method's diagnostics.
        static std::atomic<uint32_t> reports[30]{};
        if(trace.exhaustive && reports[trace.method].fetch_add(1)<8) {
            Log("legacyctl trace difference: method=%u index=%u native=(%u,%08X,%X) core=(%u,%08X,%X)",
                trace.method,static_cast<unsigned>(trace.cursor-1),event.call[0],event.call[1],event.call[2],input[0],input[1],input[2]);
            for(size_t i=0;i<std::min<size_t>(event.call.size(),count);++i) {
                if(event.call[i]!=input[i]) {
                    Log("legacyctl trace word difference: method=%u index=%u word=%u native=%08X core=%08X",
                        trace.method,static_cast<unsigned>(trace.cursor-1),static_cast<unsigned>(i),event.call[i],input[i]);
                    break;
                }
            }
        }
        ++trace.bad;
    }
    if(event.result.size()>capacity) return UINT32_MAX;
    std::copy(event.result.begin(),event.result.end(),output);return static_cast<uint32_t>(event.result.size());
}
struct Stats { std::atomic<uint32_t> calls{0},complete{0},bad{0}; };
inline Stats stats[30];
inline void Replay(Trace& trace,const uint32_t* before,uint32_t op,const std::array<uint32_t,4>& args,
    Input& input,const uint32_t* native,bool scalar=true,unsigned credit=UINT32_MAX,bool allOutputs=true) {
    trace.method=credit==UINT32_MAX?op:credit;
    auto& s=stats[trace.method]; ++s.calls;
    uint32_t expected[10]{};
    const bool supported=input.valid && fc4_legacyctl_shadow_run(before,op,args.data(),input.wire.data(),static_cast<uint32_t>(input.wire.size()),&ExchangeCall,&trace,expected)==1;
    if(!trace.exhaustive || !input.valid) return;
    if(!supported) return;
    uint32_t bad=trace.bad+(trace.cursor!=trace.events.size());
    for(unsigned i=0;i<5;++i) bad+=(expected[i]!=native[i]);
    if(scalar) bad+=(expected[5]!=native[5]);
    if(op==7) for(unsigned i=6;i<10;++i) bad+=(expected[i]!=native[i]);
    if(allOutputs && trace.allOutputs) ++s.complete;
    s.bad+=bad;
    if(bad && s.bad.load()<8) Log("legacyctl replay difference: op=%u calls=%u effects=%u consumed=%u mismatches=%u",op,s.calls.load(),static_cast<unsigned>(trace.events.size()),static_cast<unsigned>(trace.cursor),bad);
}
struct Interpreter { uintptr_t entry; unsigned prefix; uint32_t iid; const char* name; void* trampoline=nullptr; };
inline Interpreter interpreters[]={
    {0x0095660D,6,0,"InterpretPropertiesGZWinCommon"},
    {0x0094E33B,6,0xc2afa76f,"InterpretPropertiesGZWinFlatRect"},
    {0x00951084,9,0xe303e5ae,"InterpretPropertiesGZWinOutline"},
    {0x00950218,6,0x21325207,"InterpretPropertiesGZWinSlider"},
    {0x0094D17E,6,0xa13351b8,"InterpretPropertiesGZWinFolders"},
    {0x0094E516,6,0x212cdc1f,"InterpretPropertiesGZWinText"},
    {0x0094E7C8,6,0x231a1c57,"InterpretPropertiesGZWinTextEdit"},
    {0x0094EE39,6,0x4132242b,"InterpretPropertiesGZWinListBox"},
    {0x009503AB,6,0x61325a2d,"InterpretPropertiesGZWinScrollbar"},
    {0x0094F36D,6,0x412ce496,"InterpretPropertiesGZWinCombo"},
    {0x0094F757,5,0x8810,"InterpretPropertiesGZWinBtn"},
    {0x0095002C,6,0xc12cea13,"InterpretPropertiesGZWinBMP"},
    {0x009505B6,6,0x21335c59,"InterpretPropertiesGZWinLineInput"},
    {0x00950AF6,6,0x612ce0c3,"InterpretPropertiesGZWinSpinner"},
    {0x0094CE28,6,0x032f0b3c,"InterpretPropertiesGZWinTextTicker"},
    {0x009513BC,6,0x5386d516,"InterpretPropertiesSC3WinGen"},
    {0x00956C46,5,0xa1336cc0,"InterpretPropertiesGZWinOptGrp"},
    {0x00954212,5,0x3aa52e64,"InterpretPropertiesGZWinFileBrowser"},
    {0x009586B0,5,0x3ae8bad2,"InterpretPropertiesGZWinTreeView"},
    {0x0095A250,5,0xdaa6b9be,"InterpretPropertiesGZWinGrid"},
};
template<unsigned Index> void __cdecl Interpret(void* win,void* props,uint32_t first) {
    HookScope hooks;
    Input input; Trace trace;
    {
        Scope scope(trace); Quiet q; input.Capture(props); ObserveWindow(win);
        if(Index>=5 && Index<=14) trace.exhaustive=SetupDrawing() && trace.exhaustive;
    }
    // These foreign resource services are not captured yet; never treat absent
    // recording support as a native mismatch or a completed interpreter call.
    if((first&255) && ((Index==0 && input.HasString(0x106)) || (Index==3 && input.HasString(0xf016)) ||
        (Index==4 && (input.HasString(0xf016) || std::any_of(input.owned.begin(),input.owned.end(),[](const Input::Owned& p){return p.key==0xf000 && p.kind==6;}))))) trace.exhaustive=false;
    // SPEC-GAP: resource loads and complex interfaces have no complete recorder yet.
    if(Index>=15 || ((first&255) && ((Index==6 && (input.HasString(0x413) || input.HasString(0x411))) ||
        ((Index==8 || Index==10 || Index==11 || Index==13) && input.HasString(0xf016)) ||
        (Index==9 && input.HasString(0x800)) || (Index==10 && input.HasString(0xa12))))) trace.exhaustive=false;
    auto* parent=Recording()?active:nullptr;
    {
        Scope scope(trace);
        reinterpret_cast<void(__cdecl*)(void*,void*,uint32_t)>(interpreters[Index].trampoline)(win,props,first);
    }
    trace.exhaustive=hooks.Finish() && trace.exhaustive;
    if(parent) {
        parent->events.insert(parent->events.end(),trace.events.begin(),trace.events.end());
        parent->exhaustive=parent->exhaustive && trace.exhaustive;
        parent->allOutputs=parent->allOutputs && trace.allOutputs;
    }
    uint32_t before[5]{},native[10]{};
    // First-pass Common records payloads, but not the temporary variant/list
    // allocation and reference calls. Second pass contains only flag setters.
    const bool allOutputs=!(first&255) || (Index!=0 && !(Index==4 && input.HasString(0xf000)));
    Replay(trace,before,Index==0?10:11,{reinterpret_cast<uint32_t>(win),first&255,interpreters[Index].iid,0},input,native,false,10+Index,allOutputs);
}
inline bool InstallInterpreters() {
    void* hooks[]={reinterpret_cast<void*>(&Interpret<0>),reinterpret_cast<void*>(&Interpret<1>),reinterpret_cast<void*>(&Interpret<2>),reinterpret_cast<void*>(&Interpret<3>),reinterpret_cast<void*>(&Interpret<4>),
        reinterpret_cast<void*>(&Interpret<5>),reinterpret_cast<void*>(&Interpret<6>),reinterpret_cast<void*>(&Interpret<7>),reinterpret_cast<void*>(&Interpret<8>),reinterpret_cast<void*>(&Interpret<9>),
        reinterpret_cast<void*>(&Interpret<10>),reinterpret_cast<void*>(&Interpret<11>),reinterpret_cast<void*>(&Interpret<12>),reinterpret_cast<void*>(&Interpret<13>),reinterpret_cast<void*>(&Interpret<14>),
        reinterpret_cast<void*>(&Interpret<15>),reinterpret_cast<void*>(&Interpret<16>),reinterpret_cast<void*>(&Interpret<17>),reinterpret_cast<void*>(&Interpret<18>),reinterpret_cast<void*>(&Interpret<19>)};
    bool all=true;
    for(unsigned i=0;i<std::size(interpreters);++i) {
        auto& spec=interpreters[i];
        auto* source=reinterpret_cast<uint8_t*>(spec.entry);
        MEMORY_BASIC_INFORMATION memory{};
        if(VirtualQuery(source,&memory,sizeof(memory))!=sizeof(memory) || memory.State!=MEM_COMMIT ||
            (memory.Protect&(PAGE_NOACCESS|PAGE_GUARD)) ||
            spec.entry+spec.prefix>reinterpret_cast<uintptr_t>(memory.BaseAddress)+memory.RegionSize) {all=false;continue;}
        auto* code=static_cast<uint8_t*>(VirtualAlloc(nullptr,48,MEM_COMMIT|MEM_RESERVE,PAGE_READWRITE));
        if(!code) {all=false;continue;}
        const unsigned n=inplace::Trampoline(source,spec.prefix,spec.entry,code,reinterpret_cast<uintptr_t>(code));
        DWORD old=0;
        if(n!=spec.prefix || !VirtualProtect(code,48,PAGE_EXECUTE_READ,&old)) {VirtualFree(code,0,MEM_RELEASE);all=false;continue;}
        FlushInstructionCache(GetCurrentProcess(),code,48);
        if(!VirtualProtect(source,n,PAGE_EXECUTE_READWRITE,&old)) {VirtualFree(code,0,MEM_RELEASE);all=false;continue;}
        spec.trampoline=code;
        inplace::Jump(source,spec.entry,reinterpret_cast<uintptr_t>(hooks[i]));
        for(unsigned j=5;j<n;++j) source[j]=0x90;
        DWORD ignored=0;VirtualProtect(source,n,old,&ignored);FlushInstructionCache(GetCurrentProcess(),source,n);
    }
    return all;
}
} // namespace fc4::legacyrecord
#endif
