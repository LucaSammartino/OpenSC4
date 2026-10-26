// SPDX-License-Identifier: GPL-3.0-or-later
// Calls the real DLL's native tables, including descriptor return channels.
#include <windows.h>
#include "FC4TestSeam.h"
#include "cIGZUnknown.h"
#include "cIGZCommandParameterSet.h"
#include "cIGZVariant.h"
#include "cRZBaseString.h"
#include <array>
#include <cstdint>
#include <cstdio>
#include <map>
#include <string>
#include <vector>

namespace {
unsigned failures=0;
void Check(bool ok,const char* why) { if (!ok) { ++failures; std::fprintf(stderr,"FAIL %s\n",why); } }
template<class T> T Slot(void* p,unsigned i) { return reinterpret_cast<T>((*static_cast<void***>(p))[i]); }
using Get=uint32_t(__thiscall*)(void*);
using Flag=bool(__thiscall*)(void*,uint32_t);
using QI=bool(__thiscall*)(void*,uint32_t,void**);
struct Parameters final : cIGZCommandParameterSet {
    uint32_t refs=0;
    bool strings=true;
    std::map<uint32_t,std::string> values;
    bool QueryInterface(uint32_t,void**) override { return false; }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }
    uint32_t GetParameterCount() const override { return static_cast<uint32_t>(values.size()); }
    void SetParameterCount(uint32_t) override { values.clear(); }
    cIGZVariant* operator[](uint32_t) override { return nullptr; }
    cIGZVariant* GetParameter(uint32_t) override { return nullptr; }
    void SetParameter(uint32_t id,const cIGZVariant& value) override {
        cRZBaseString text; value.GetValString(text); values[id]=text.ToChar();
        strings=strings && value.GetType()==0x800e && value.GetCount()==text.Strlen();
    }
    void AppendParameter(const cIGZVariant&) override {}
    void RemoveParameter(uint32_t) override {}
    void SetStatusParameterValue(int32_t) override {}
    bool CompareParameterString(uint32_t,const char*,bool) override { return false; }
    bool ValidateParameter(uint32_t,uint32_t) override { return false; }
    uint32_t GetFirstParameterID() const override { return 0xffffffff; }
    uint32_t GetNextParameterID(uint32_t) const override { return 0xffffffff; }
};
struct Fake {
    void** table;
    uint32_t refs=1;
    unsigned init=0, conversions=0, executions=0;
    Parameters* parameters=nullptr;
    std::vector<uint32_t> registered;
    uint32_t id=0;
    void* input=nullptr;
    void* output=nullptr;
};
uint32_t __fastcall Add(void* p,void*) { return ++static_cast<Fake*>(p)->refs; }
uint32_t __fastcall Release(void* p,void*) { return --static_cast<Fake*>(p)->refs; }
bool __fastcall Query(void* p,void*,uint32_t iid,void** out) {
    if (iid!=1 && iid!=0x287FB697) return false;
    *out=p; Add(p,nullptr); return true;
}
bool __fastcall Init(void* p,void*) { ++static_cast<Fake*>(p)->init; return true; }
void* __fastcall Dispatcher(void*,void*) { return nullptr; }
uint32_t __fastcall CreateParameters(void* p,void*,cIGZCommandParameterSet** out) {
    auto* ps=static_cast<Fake*>(p)->parameters;
    ps->values.clear(); ps->strings=true; *out=ps; return ps->AddRef();
}
int32_t __fastcall Execute(void* p,void*,uint32_t id,void* in,void* out) {
    auto& f=*static_cast<Fake*>(p); ++f.executions; f.id=id; f.input=in; f.output=out;
    return -1234567;
}
bool __fastcall Registered(void* p,void*,uint32_t id,const void*,const void*) {
    static_cast<Fake*>(p)->registered.push_back(id); return true;
}
bool __fastcall FromText(void* p,void*,const char*,uint32_t,uint32_t&,void*&) {
    ++static_cast<Fake*>(p)->conversions; return false;
}
bool __fastcall ToText(void* p,void*,uint32_t,void*,void*) {
    ++static_cast<Fake*>(p)->conversions; return false;
}
bool __fastcall OutputText(void* p,void*,void*,void*) {
    ++static_cast<Fake*>(p)->conversions; return false;
}
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
    Check(Slot<Get>(b,36)(b)==2,"allocation and dirty Lock change count");
    Check(Slot<Flag>(b,7)(b,0x0800) && Slot<Flag>(b,7)(b,0x8040),"masked Unlock");
    Check(Slot<uint8_t*(__thiscall*)(void*)>(b,34)(b)==pixels && pixels[35]==0x67,"pixels survive lock cycle");
    using RGB=uint32_t(__thiscall*)(void*,uint32_t,uint32_t,uint32_t);
    Check(Slot<RGB>(b,30)(b,255,255,255)==0,"type2 RGB has no palette");
    Check(Slot<Get>(b,2)(b)==0,"final buffer release");
}
struct CacheFixture {
    void** table;
    uint32_t refs=1,initializations=0,invalidations=0;
    bool accepts=true,detached=false,freedBeforeRelease=false;
    void* buffer=nullptr;
};
uint32_t __fastcall CacheAdd(void* p,void*) { return ++static_cast<CacheFixture*>(p)->refs; }
bool __fastcall CacheQuery(void* p,void*,uint32_t iid,void** out) {
    if (iid!=1 && iid!=0xFA59F7B9) return false;
    *out=p; CacheAdd(p,nullptr); return true;
}
uint32_t __fastcall CacheRelease(void* p,void*) {
    auto& c=*static_cast<CacheFixture*>(p);
    if (c.buffer) c.freedBeforeRelease=Slot<void*(__thiscall*)(void*)>(c.buffer,34)(c.buffer)==nullptr;
    return --c.refs;
}
bool __fastcall CacheInit(void* p,void*,void* b) {
    auto& c=*static_cast<CacheFixture*>(p); ++c.initializations; c.buffer=b;
    c.detached=Slot<void*(__thiscall*)(void*)>(b,37)(b)==nullptr;
    return c.accepts;
}
void __fastcall CacheInvalidate(void* p,void*) { ++static_cast<CacheFixture*>(p)->invalidations; }
void BufferWorkedCases(FC4TestSeamFn seam) {
    using InitFn=bool(__thiscall*)(void*,uint32_t,uint32_t,uint32_t,uint32_t);
    using NoArg=bool(__thiscall*)(void*);
    using VoidCall=void(__thiscall*)(void*);
    using Fill=bool(__thiscall*)(void*,uint32_t,const void*);
    using FillRect=bool(__thiscall*)(void*,uint32_t,const void*,const void*);
    using Pixel=uint32_t(__thiscall*)(void*,uint32_t,uint32_t);
    using SetPixel=void(__thiscall*)(void*,uint32_t,uint32_t,uint32_t);
    using SetKey=void(__thiscall*)(void*,uint32_t);
    using SetCache=bool(__thiscall*)(void*,void*);
    using Blit=bool(__thiscall*)(void*,void*,const void*,const void*,const void*);
    using Line=bool(__thiscall*)(void*,int32_t,int32_t,int32_t,int32_t,uint32_t,uint32_t);
    const auto create=[&](uint32_t w,uint32_t h,uint32_t type,uint32_t bpp) {
        void* p=reinterpret_cast<void*>(seam(kFC4SeamGZFrameworkCreate,nullptr,nullptr));
        Check(p && Slot<InitFn>(p,3)(p,w,h,type,bpp),"worked buffer creation"); return p;
    };
    std::array<void*,9> table{}; table[0]=reinterpret_cast<void*>(&CacheQuery); table[1]=reinterpret_cast<void*>(&CacheAdd); table[2]=reinterpret_cast<void*>(&CacheRelease);
    table[3]=reinterpret_cast<void*>(&CacheInit); table[8]=reinterpret_cast<void*>(&CacheInvalidate);
    CacheFixture cache{table.data()},failed{table.data()}; failed.accepts=false;
    void* b=create(3,2,5,16);
    auto* bytes=Slot<uint8_t*(__thiscall*)(void*)>(b,34)(b); bytes[6]=0x6a; bytes[14]=0x7b;
    Check(Slot<Fill>(b,18)(b,0x1234,nullptr) && Slot<Pixel>(b,21)(b,2,1)==0x1234 && bytes[6]==0x6a && bytes[14]==0x7b,"GZB1 packed fill preserves padding");
    Check(Slot<SetCache>(b,38)(b,&cache) && cache.refs==2 && cache.detached && cache.initializations==1,"cache initializes while detached, then retains");
    Check(Slot<Get>(b,1)(b)==2 && Slot<Get>(b,2)(b)==1,"cache borrows buffer");
    Check(Slot<void*(__thiscall*)(void*)>(b,37)(b)==&cache && cache.refs==2,"cache getter borrowed");
    Check(Slot<NoArg>(b,4)(b) && !Slot<NoArg>(b,39)(b) && Slot<Get>(b,13)(b)==0 && cache.refs==2,"GZB1 shutdown keeps cache");
    Check(Slot<InitFn>(b,3)(b,5,2,5,16) && Slot<Get>(b,35)(b)==12 && Slot<Get>(b,13)(b)==32 && Slot<Get>(b,36)(b)==3,"GZB1 reinit and legacy size estimate");
    Check(Slot<SetCache>(b,38)(b,&cache) && cache.initializations==2 && cache.refs==2 && cache.detached,"same-pointer attachment releases and reinitializes");
    Check(!Slot<SetCache>(b,38)(b,&failed) && failed.refs==1 && failed.detached && cache.refs==1,"failed replacement detaches old and retains neither");
    Check(Slot<void*(__thiscall*)(void*)>(b,37)(b)==nullptr,"failed attachment leaves null");
    Check(Slot<SetCache>(b,38)(b,&cache),"reattach after failure");
    Slot<SetKey>(b,23)(b,0x1234); Slot<VoidCall>(b,27)(b);
    Check(cache.invalidations==2 && Slot<Get>(b,36)(b)==3,"key changes explicitly invalidate without dirty counter");
    CacheRelease(&cache,nullptr); Slot<Get>(b,2)(b);
    Check(cache.refs==0 && cache.freedBeforeRelease,"buffer frees pixels before final cache release");

    b=create(2,1,3,8); uint32_t pair[2]{};
    using Pair=uint32_t*(__thiscall*)(void*,uint32_t*);
    Check(Slot<Pair>(b,14)(b,pair)==pair && pair[0]==5 && pair[1]==16,"GZB3 automatic format without graphics"); Slot<Get>(b,2)(b);
    b=create(5,4,6,16);
    using RGBA=uint32_t(__thiscall*)(void*,uint32_t,uint32_t,uint32_t,uint32_t);
    using Unmap=bool(__thiscall*)(void*,uint32_t,uint8_t*,uint8_t*,uint8_t*,uint8_t*);
    uint8_t rgba[4]{};
    Check(Slot<RGBA>(b,32)(b,18,52,86,128)==0x88ca && Slot<Unmap>(b,33)(b,0x88ca,&rgba[0],&rgba[1],&rgba[2],&rgba[3]) && rgba[0]==16 && rgba[1]==49 && rgba[2]==82 && rgba[3]==132,"GZB4 packed alpha quirk");
    Slot<SetPixel>(b,22)(b,0,0,0x08ca); Slot<SetKey>(b,23)(b,0xf00088ca);
    Check(Slot<bool(__thiscall*)(void*,uint32_t,uint32_t)>(b,25)(b,0,0) && Slot<Get>(b,24)(b)==1,"GZB4 key masks alpha and high word");
    uint32_t key=0; Check(Slot<bool(__thiscall*)(void*,uint32_t*)>(b,26)(b,&key) && key==0xf00088ca,"full stored key retained");
    Slot<VoidCall>(b,27)(b); key=0xdeadbeef;
    Check(!Slot<bool(__thiscall*)(void*,uint32_t*)>(b,26)(b,&key) && key==0xdeadbeef,"cleared key leaves output");
    Slot<Fill>(b,18)(b,0,nullptr); int32_t rect[4]={-1,-1,4,3},clip[4]={1,1,3,2};
    Check(Slot<FillRect>(b,20)(b,0xabcd,rect,clip) && Slot<Pixel>(b,21)(b,1,1)==0xabcd && Slot<Pixel>(b,21)(b,2,1)==0xabcd && Slot<Pixel>(b,21)(b,0,1)==0,"GZB5 clipped fill");
    const auto count=Slot<Get>(b,36)(b); int32_t disjoint[4]={7,0,8,1};
    Check(!Slot<FillRect>(b,20)(b,1,disjoint,nullptr) && Slot<Get>(b,36)(b)==count,"disjoint fill not modified");
    Slot<Fill>(b,18)(b,0,nullptr); const auto before=Slot<Get>(b,36)(b);
    Check(Slot<Line>(b,28)(b,0,0,4,2,0x1234,1) && Slot<Get>(b,36)(b)==before,"GZB8 diagonal line not modified");
    const int expected[5]={0,1,1,2,2};
    for (uint32_t y=0;y<4;++y) for (uint32_t x=0;x<5;++x) Check(Slot<Pixel>(b,21)(b,x,y)==(y==expected[x] ? 0x1234 : 0),"GZB8 inclusive Bresenham points");
    Slot<Get>(b,2)(b);

    b=create(2,1,9,24); bytes=Slot<uint8_t*(__thiscall*)(void*)>(b,34)(b); bytes[0]=0xa7;
    Check(Slot<Fill>(b,18)(b,0,nullptr) && bytes[0]==0xa7 && Slot<Get>(b,36)(b)==2,"GZB5 24-bit fill succeeds without writing");
    Slot<SetPixel>(b,22)(b,1,0,0xff123456);
    Check(Slot<Pixel>(b,21)(b,1,0)==0x123456 && Slot<Get>(b,36)(b)==2,"24-bit access uses three bytes without dirtiness"); Slot<Get>(b,2)(b);
    void* s=create(2,1,9,32); b=create(3,1,6,16);
    Slot<SetPixel>(s,22)(s,0,0,0x00112233); Slot<SetPixel>(s,22)(s,1,0,0xff445566); Slot<SetKey>(s,23)(s,0x66112233);
    Slot<Fill>(b,18)(b,0xffff,nullptr); int32_t sr[4]={0,0,2,1},dr[4]={1,0,99,99};
    Check(Slot<Blit>(b,29)(b,s,sr,dr,nullptr) && Slot<Pixel>(b,21)(b,0,0)==0xffff && Slot<Pixel>(b,21)(b,1,0)==0xffff && Slot<Pixel>(b,21)(b,2,0)==0xa14c && Slot<Get>(b,36)(b)==3,"GZB6 keyed conversion ignores destination size");
    Slot<Get>(s,2)(s); Slot<Get>(b,2)(b);
    b=create(4,1,6,16); for (uint32_t x=0;x<4;++x) Slot<SetPixel>(b,22)(b,x,0,0x8001+x);
    sr[2]=3; dr[0]=1; dr[1]=0;
    Check(Slot<Blit>(b,29)(b,b,sr,dr,nullptr),"GZB7 self-copy");
    for (uint32_t x=0;x<4;++x) Check(Slot<Pixel>(b,21)(b,x,0)==0x8001,"GZB7 forward overlap propagates stores");
    Slot<Get>(b,2)(b);
}
void ServerTest(FC4TestSeamFn seam) {
    std::array<void*,25> table{};
    table[0]=reinterpret_cast<void*>(&Query); table[1]=reinterpret_cast<void*>(&Add); table[2]=reinterpret_cast<void*>(&Release);
    table[3]=reinterpret_cast<void*>(&Init); table[4]=reinterpret_cast<void*>(&Init); table[17]=reinterpret_cast<void*>(&Dispatcher);
    table[5]=reinterpret_cast<void*>(&CreateParameters);
    Fake helper{table.data()};
    Parameters parameters; helper.parameters=&parameters;
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
    std::array<void*,12> generatorTable{};
    generatorTable[0]=reinterpret_cast<void*>(&Query);
    generatorTable[1]=reinterpret_cast<void*>(&Add); generatorTable[2]=reinterpret_cast<void*>(&Release);
    generatorTable[5]=reinterpret_cast<void*>(&Registered); generatorTable[6]=reinterpret_cast<void*>(&Registered);
    generatorTable[9]=reinterpret_cast<void*>(&FromText); generatorTable[10]=reinterpret_cast<void*>(&ToText);
    generatorTable[11]=reinterpret_cast<void*>(&OutputText);
    Fake generator{generatorTable.data()};
    using Pointer=bool(__thiscall*)(void*,void*);
    Check(Slot<Pointer>(s,15)(s,&generator) && generator.refs==2,"generator retained");
    Check(generator.registered==std::vector<uint32_t>({3,9}),"unsigned-id generator replay");
    Check(Slot<Pointer>(s,15)(s,&generator) && generator.refs==2 && generator.registered.size()==2,"duplicate generator has no replay or retain");
    using Parse=uint32_t(__thiscall*)(void*,const char*,int32_t);
    Check(Slot<Parse>(s,13)(s,"\"nAmE\"",-1)==3,"lower id and case-insensitive lookup");
    Check(Slot<Parse>(s,13)(s,"Name argument",-1)==0,"one-word parser limit");
    Check(Slot<Flag>(s,9)(s,3) && Slot<Parse>(s,13)(s,"name",4)==9,"unregistration changes lookup");
    Check(Slot<bool(__thiscall*)(void*)>(s,3)(s) && helper.init==1,"native helper initialized");
    using ExecuteID=int32_t(__thiscall*)(void*,uint32_t,void*,void*);
    Check(Slot<ExecuteID>(s,20)(s,9,nullptr,nullptr)==2,"ID overload at native slot20");
    using ExecuteText=int32_t(__thiscall*)(void*,const char*,uint32_t,void*);
    Check(Slot<ExecuteText>(s,19)(s,"missing",7,nullptr)==2,"text overload at native slot19");
    using Convert=bool(__thiscall*)(void*,const char*,uint32_t,uint32_t&,cIGZCommandParameterSet*&);
    uint32_t id=0x1234;
    auto* parsed=reinterpret_cast<cIGZCommandParameterSet*>(0x1234);
    Check(!Slot<Convert>(s,22)(s,"",0,id,parsed) && id==0x1234 && parsed==reinterpret_cast<cIGZCommandParameterSet*>(0x1234),"empty conversion preserves outputs");
    Check(!Slot<Convert>(s,22)(s,"missing",7,id,parsed) && id==0 && parsed==reinterpret_cast<cIGZCommandParameterSet*>(0x1234),"unknown conversion writes only id");
    Check(Slot<Convert>(s,22)(s,"nAmE 5 \"a b\"",0xffffffff,id,parsed) && id==9 && parsed==&parameters,"default generator parses registered name");
    Check(parameters.values==std::map<uint32_t,std::string>({{0,"5"},{1,"a b"}}) && parameters.strings && parameters.refs==1,"string parameters and caller-owned reference");
    parsed->Release();
    std::string many="Name"; for (unsigned i=0;i<300;++i) many+=" x";
    Check(Slot<Convert>(s,22)(s,many.c_str(),0xffffffff,id,parsed) && parameters.values.size()==255 && parameters.values.rbegin()->first==254,"default generator caps input at 256 words");
    parsed->Release();
    cRZBaseString text("sentinel");
    using ConvertBack=bool(__thiscall*)(void*,uint32_t,void*,cIGZString&);
    Check(Slot<ConvertBack>(s,23)(s,9,nullptr,text) && std::string(text.ToChar())=="Name","default generator formats registered name");
    Check(!Slot<ConvertBack>(s,23)(s,99,nullptr,text) && text.Strlen()==0,"unknown command clears text");
    using ConvertOutput=bool(__thiscall*)(void*,void*,cIGZString&);
    text.FromChar("sentinel");
    Check(Slot<ConvertOutput>(s,24)(s,nullptr,text) && text.Strlen()==0,"null output set formats as empty");
    Check(generator.conversions==0,"notification generators are not used by default conversion slots");
    std::array<void*,6> dispatcherTable{};
    dispatcherTable[0]=reinterpret_cast<void*>(&Query);
    dispatcherTable[1]=reinterpret_cast<void*>(&Add); dispatcherTable[2]=reinterpret_cast<void*>(&Release);
    dispatcherTable[5]=reinterpret_cast<void*>(&Execute);
    Fake dispatcher{dispatcherTable.data()};
    Check(Slot<Pointer>(s,18)(s,&dispatcher) && dispatcher.refs==2,"dispatcher retained");
    Check(Slot<ExecuteID>(s,20)(s,9,&parameters,&generator)==-1234567 && dispatcher.id==9 && dispatcher.input==&parameters && dispatcher.output==&generator,"ID dispatch forwards arguments and full EAX status");
    Check(Slot<ExecuteText>(s,19)(s,"Name 7",6,&generator)==-1234567 && dispatcher.input==&parameters && dispatcher.output==&generator && parameters.refs==0,"text dispatch releases its created parameter set");
    using Formatted=int32_t(__cdecl*)(void*,const char*,...);
    Check(Slot<Formatted>(s,21)(s,"Name %d",37)==-1234567 && dispatcher.output==reinterpret_cast<void*>(37) && parameters.values.at(0)=="37" && parameters.refs==0,"cdecl formatted execution reuses first variadic word as output");
    const unsigned executions=dispatcher.executions;
    const std::string overflow(512,'x');
    Check(Slot<Formatted>(s,21)(s,"",nullptr)==2 && Slot<Formatted>(s,21)(s,overflow.c_str(),nullptr)==2 && dispatcher.executions==executions,"empty and overflowing formats do not dispatch");
    Check(Slot<Pointer>(s,16)(s,&generator) && generator.refs==1,"generator unregistration releases reference");
    Check(Slot<bool(__thiscall*)(void*)>(s,4)(s),"server shutdown");
    Check(dispatcher.refs==1,"shutdown releases active dispatcher");
    Check(Slot<Get>(s,2)(s)==0 && helper.refs==1,"balanced helper ownership");
}
}
int main(int argc,char** argv) {
    // Keep a failed native assertion/access violation from waiting on a UI.
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
    if (argc!=2) return 2;
    auto module=LoadLibraryA(argv[1]); if (!module) return 2;
    auto seam=reinterpret_cast<FC4TestSeamFn>(GetProcAddress(module,"FC4TestSeam")); if (!seam) return 2;
    BufferTest(seam); BufferWorkedCases(seam); ServerTest(seam);
    std::printf("gzfw native owners: failures=%u\n",failures);
    FreeLibrary(module); return failures ? 1 : 0;
}
