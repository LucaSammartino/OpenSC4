// SPDX-License-Identifier: GPL-3.0-or-later
// Calls the real DLL's native tables, including descriptor return channels.
#include <windows.h>
#include "FC4TestSeam.h"
#include "cIGZUnknown.h"
#include <array>
#include <cstdint>
#include <cstdio>

namespace {
unsigned failures=0;
void Check(bool ok,const char* why) { if (!ok) { ++failures; std::fprintf(stderr,"FAIL %s\n",why); } }
template<class T> T Slot(void* p,unsigned i) { return reinterpret_cast<T>((*static_cast<void***>(p))[i]); }
using Get=uint32_t(__thiscall*)(void*);
using Flag=bool(__thiscall*)(void*,uint32_t);
using QI=bool(__thiscall*)(void*,uint32_t,void**);
struct Fake { void** table; uint32_t refs=1; unsigned init=0; };
uint32_t __fastcall Add(void* p,void*) { return ++static_cast<Fake*>(p)->refs; }
uint32_t __fastcall Release(void* p,void*) { return --static_cast<Fake*>(p)->refs; }
bool __fastcall Query(void* p,void*,uint32_t iid,void** out) {
    if (iid!=1 && iid!=0x287FB697) return false;
    *out=p; Add(p,nullptr); return true;
}
bool __fastcall Init(void* p,void*) { ++static_cast<Fake*>(p)->init; return true; }
void* __fastcall Dispatcher(void*,void*) { return nullptr; }
void BufferTest(FC4TestSeamFn seam) {
    auto* b=reinterpret_cast<void*>(seam(kFC4SeamGZFrameworkCreate,nullptr,nullptr));
    Check(b!=nullptr,"buffer factory"); if (!b) return;
    void* out=reinterpret_cast<void*>(0x1234);
    Check(!Slot<QI>(b,0)(b,0xfffffffe,&out) && out==reinterpret_cast<void*>(0x1234),"failed QI leaves sentinel");
    Check(Slot<QI>(b,0)(b,0x86D72B57,&out) && out==static_cast<char*>(b)+4,"buffer secondary face");
    Check(Slot<Get>(out,2)(out)==1,"shared buffer refs");
    Check(!Slot<Flag>(b,6)(b,0x8000),"fresh Lock fails");
    uint32_t key=0x12345678;
    using Key=bool(__thiscall*)(void*,uint32_t*);
    Check(!Slot<Key>(b,26)(b,&key) && key==0x12345678,"disabled key leaves output");
    using InitFn=bool(__thiscall*)(void*,uint32_t,uint32_t,uint32_t,uint32_t);
    Check(!Slot<InitFn>(b,3)(b,0,3,2,8),"zero width refused");
    Check(Slot<InitFn>(b,3)(b,10,3,2,8),"buffer init");
    Check(!Slot<InitFn>(b,3)(b,11,3,2,8),"second init refused");
    Check(Slot<Get>(b,9)(b)==10 && Slot<Get>(b,10)(b)==3 && Slot<Get>(b,35)(b)==12,"geometry and padding");
    using Pair=uint32_t*(__thiscall*)(void*,uint32_t*);
    uint32_t rect[4]{},pair[2]{};
    Check(Slot<Pair>(b,11)(b,rect)==rect && rect[2]==10 && rect[3]==3,"native rect pointer return");
    Check(Slot<Pair>(b,15)(b,pair)==pair && pair[0]==2 && pair[1]==8,"native pair pointer return");
    const auto pixels=Slot<uint8_t*(__thiscall*)(void*)>(b,34)(b);
    Check(pixels!=nullptr,"stable pixel storage"); if (pixels) pixels[35]=0x67;
    Check(Slot<Flag>(b,6)(b,0x8040) && Slot<Flag>(b,6)(b,0x0800),"nested Lock");
    Check(Slot<Get>(b,36)(b)==1,"dirty change count");
    Check(Slot<Flag>(b,7)(b,0x0800) && Slot<Flag>(b,7)(b,0x8040),"masked Unlock");
    Check(Slot<uint8_t*(__thiscall*)(void*)>(b,34)(b)==pixels && pixels[35]==0x67,"pixels survive lock cycle");
    using RGB=uint32_t(__thiscall*)(void*,uint32_t,uint32_t,uint32_t);
    Check(Slot<RGB>(b,30)(b,255,255,255)==0,"type2 RGB has no palette");
    Check(Slot<Get>(b,2)(b)==0,"final buffer release");
}
void ServerTest(FC4TestSeamFn seam) {
    std::array<void*,25> table{};
    table[0]=reinterpret_cast<void*>(&Query); table[1]=reinterpret_cast<void*>(&Add); table[2]=reinterpret_cast<void*>(&Release);
    table[3]=reinterpret_cast<void*>(&Init); table[4]=reinterpret_cast<void*>(&Init); table[17]=reinterpret_cast<void*>(&Dispatcher);
    Fake helper{table.data()};
    auto* s=reinterpret_cast<void*>(seam(kFC4SeamGZFrameworkCreate,&helper,reinterpret_cast<void*>(1)));
    Check(s!=nullptr,"server factory"); if (!s) return;
    Check(helper.refs==3,"server retains both helper faces");
    void* out=nullptr;
    Check(Slot<QI>(s,0)(s,0x287FB697,&out) && out==static_cast<char*>(s)+4,"command service face");
    void* life=nullptr;
    Check(Slot<QI>(out,0)(out,1,&life) && life==static_cast<char*>(s)+8,"service QI unknown returns lifetime");
    Slot<Get>(life,2)(life); Slot<Get>(out,2)(out);
    using Register=bool(__thiscall*)(void*,uint32_t,const char*,const char*);
    Check(!Slot<Register>(s,6)(s,0,"Name",nullptr),"command id zero rejected");
    Check(Slot<Register>(s,6)(s,9,"Name","original") && Slot<Register>(s,6)(s,3,"NAME",nullptr),"same name distinct ids");
    Check(!Slot<Register>(s,6)(s,9,"Changed",nullptr),"duplicate id does not replace");
    using Parse=uint32_t(__thiscall*)(void*,const char*,int32_t);
    Check(Slot<Parse>(s,13)(s,"\"nAmE\"",-1)==3,"lower id and case-insensitive lookup");
    Check(Slot<Parse>(s,13)(s,"Name argument",-1)==0,"one-word parser limit");
    Check(Slot<Flag>(s,9)(s,3) && Slot<Parse>(s,13)(s,"name",4)==9,"unregistration changes lookup");
    Check(Slot<bool(__thiscall*)(void*)>(s,3)(s) && helper.init==1,"native helper initialized");
    using ExecuteID=int32_t(__thiscall*)(void*,uint32_t,void*,void*);
    Check(Slot<ExecuteID>(s,20)(s,9,nullptr,nullptr)==2,"ID overload at native slot20");
    using ExecuteText=int32_t(__thiscall*)(void*,const char*,uint32_t,void*);
    Check(Slot<ExecuteText>(s,19)(s,"missing",7,nullptr)==2,"text overload at native slot19");
    Check(Slot<bool(__thiscall*)(void*)>(s,4)(s),"server shutdown");
    Check(Slot<Get>(s,2)(s)==0 && helper.refs==1,"balanced helper ownership");
}
}
int main(int argc,char** argv) {
    if (argc!=2) return 2;
    auto module=LoadLibraryA(argv[1]); if (!module) return 2;
    auto seam=reinterpret_cast<FC4TestSeamFn>(GetProcAddress(module,"FC4TestSeam")); if (!seam) return 2;
    BufferTest(seam); ServerTest(seam);
    std::printf("gzfw native owners: failures=%u\n",failures);
    FreeLibrary(module); return failures ? 1 : 0;
}
