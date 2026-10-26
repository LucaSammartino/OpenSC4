// SPDX-License-Identifier: GPL-3.0-or-later
// gz-framework-core section 4; command-system sections 3-5.
#include "FC4GZFramework.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Subsystems.h"
#include "cFC4Variant.h"
#include "cIGZCommandDispatcher.h"
#include "cIGZCommandGenerator.h"
#include "cIGZCommandParameterSet.h"
#include "cIGZCommandServer.h"
#include "cIGZFrameWork.h"
#include "cIGZSystemService.h"
#include "cRZBaseString.h"
#include <algorithm>
#include <atomic>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <mutex>
#include <new>
#include <string>
#include <vector>

using TextOut = void (__cdecl*)(void*, uint32_t, const uint8_t*, size_t);
extern "C" void* fc4_gzfw_registry_new();
extern "C" void fc4_gzfw_registry_free(void*);
extern "C" bool fc4_gzfw_registry_change(void*, uint32_t, uint32_t, const char*, const char*);
extern "C" uint32_t fc4_gzfw_registry_lookup(void*, const uint8_t*, size_t, bool);
extern "C" bool fc4_gzfw_registry_info(void*, uint32_t, void*, TextOut);
extern "C" size_t fc4_gzfw_registry_ids(void*, uint32_t*, size_t);
extern "C" void fc4_gzfw_words(const uint8_t*, size_t, void*, TextOut);
extern "C" void fc4_gzfw_quote(const uint8_t*, size_t, void*, TextOut);

namespace {
template<class T> T Slot(void* p, uint32_t n) { return reinterpret_cast<T>((*static_cast<void***>(p))[n]); }
size_t Length(const char* p,uint32_t n) { return p ? (n==0xffffffff ? std::strlen(p) : n) : 0; }
void __cdecl Append(void* p,uint32_t,const uint8_t* s,size_t n) { static_cast<std::string*>(p)->append(reinterpret_cast<const char*>(s),n); }
void __cdecl Words(void* p,uint32_t,const uint8_t* s,size_t n) { static_cast<std::vector<std::string>*>(p)->emplace_back(reinterpret_cast<const char*>(s),n); }
void __cdecl InfoText(void* p,uint32_t field,const uint8_t* s,size_t n) { static_cast<std::string*>(p)[field].assign(reinterpret_cast<const char*>(s),n); }
void Quote(std::string& out,const char* p,size_t n) { fc4_gzfw_quote(reinterpret_cast<const uint8_t*>(p),n,&out,Append); }
bool TraceEnabled() { return fc4::SettingInt("gzfw_trace",nullptr,0)!=0; }
void TraceID(uint32_t id,int32_t status) { if (TraceEnabled()) fc4::Log("gzfw stream: id=%08X status=%d",id,status); }
void TraceText(const char* p,uint32_t n,int32_t status) {
    if (TraceEnabled()) fc4::Log("gzfw stream: text='%.*s' bytes=%u status=%d",static_cast<int>(Length(p,n)),p?p:"",static_cast<uint32_t>(Length(p,n)),status);
}
class Server;
class Primary : public cIGZUnknown {
public:
    bool QueryInterface(uint32_t,void**) override;
    uint32_t AddRef() override;
    uint32_t Release() override;
    virtual bool Init()=0;
    virtual bool Shutdown()=0;
    virtual uint32_t CreateParameters(cIGZCommandParameterSet**)=0;
    virtual bool Register(uint32_t,const char*,const char*)=0;
    virtual bool RegisterMany(cIGZCommandServer::cGZCommandInfo*,uint32_t)=0;
    virtual bool Parameters(uint32_t,const char*)=0;
    virtual bool Unregister(uint32_t)=0;
    virtual bool UnregisterMany(cIGZCommandServer::cGZCommandInfo*,uint32_t)=0;
    virtual bool Info(uint32_t,cIGZString&,cIGZString*)=0;
    virtual uint32_t ID(const cIGZString&)=0;
    virtual uint32_t ParseID(const char*,int32_t)=0;
    virtual bool Enumerate(void*)=0;
    virtual bool RegisterGenerator(cIGZCommandGenerator*)=0;
    virtual bool UnregisterGenerator(cIGZCommandGenerator*)=0;
    virtual cIGZCommandDispatcher* Dispatcher()=0;
    virtual bool SetDispatcher(cIGZCommandDispatcher*)=0;
    virtual int32_t ExecuteText(const char*,uint32_t,cIGZCommandParameterSet*)=0;
    virtual int32_t ExecuteID(uint32_t,cIGZCommandParameterSet*,cIGZCommandParameterSet*)=0;
    virtual int32_t ExecuteFormatted(const char*,cIGZCommandParameterSet*,...)=0;
    virtual bool FromText(const char*,uint32_t,uint32_t&,cIGZCommandParameterSet*&)=0;
    virtual bool ToText(uint32_t,cIGZCommandParameterSet*,cIGZString&)=0;
    virtual bool OutputText(cIGZCommandParameterSet*,cIGZString&)=0;
};
class Service : public cIGZSystemService {
public:
    bool QueryInterface(uint32_t,void**) override;
    uint32_t AddRef() override;
    uint32_t Release() override;
    uint32_t GetServiceID() override;
    cIGZSystemService* SetServiceID(uint32_t) override;
    int32_t GetServicePriority() override;
    bool IsServiceRunning() override;
    cIGZSystemService* SetServiceRunning(bool) override;
    bool OnTick(uint32_t) override { return true; }
    bool OnIdle(uint32_t) override { return true; }
    int32_t GetServiceTickPriority() override;
};
class Lifetime : public cIGZUnknown {
public:
    bool QueryInterface(uint32_t,void**) override;
    uint32_t AddRef() override;
    uint32_t Release() override;
};
class Server final : public Primary,public Service,public Lifetime {
    std::atomic<uint32_t> refs_{0};
    std::recursive_mutex mutex_;
    void* core_=fc4_gzfw_registry_new();
    cIGZCommandServer* helpers_;
    cIGZSystemService* originalService_;
    cIGZCommandDispatcher* active_=nullptr;
    std::vector<cIGZCommandGenerator*> generators_;
    bool initialized_=false;
    bool Strings(uint32_t id,std::string (&s)[2]) { return fc4_gzfw_registry_info(core_,id,s,InfoText); }
    void Notify(cIGZCommandGenerator* g,uint32_t id,std::string (&s)[2],bool added) {
        if (!g) return; // SPEC-GAP: null-generator recursion misuse is refused.
        cRZBaseString name(s[0].c_str()),info(s[1].c_str());
        if (added) g->CommandRegistered(id,name,info); else g->CommandUnregistered(id,name,info);
    }
    void AppendParameters(cIGZCommandParameterSet* ps,std::string& s,bool leading) {
        if (!ps) return;
        for (uint32_t id=ps->GetFirstParameterID();id!=0xffffffff;id=ps->GetNextParameterID(id)) {
            auto* v=ps->GetParameter(id); cRZBaseString word; if (v) v->GetValString(word);
            if (leading) s+=' ';
            Quote(s,word.ToChar(),word.Strlen()); leading=true;
        }
    }
public:
    cIGZSystemService* Metadata() { return originalService_; }
    Server(cIGZCommandServer* h,cIGZSystemService* s):helpers_(h),originalService_(s) {}
    ~Server() {
        if (active_) FC4_RELEASE(active_);
        for (auto* g:generators_) if (g) FC4_RELEASE(g);
        FC4_RELEASE_AT_DESTRUCTION(originalService_);
        FC4_RELEASE_AT_DESTRUCTION(helpers_);
        fc4_gzfw_registry_free(core_);
    }
    uint32_t RefAdd() { return ++refs_; }
    uint32_t RefRelease() { if (refs_>1) return --refs_; delete this; return 0; }
    bool Query(uint32_t iid,void** out,bool inherited=false) {
        if (iid==1 && inherited) *out=static_cast<Lifetime*>(this);
        else if (iid==1 || iid==0xCB90253A || iid==0xCB90253B) *out=static_cast<Primary*>(this);
        else if (iid==0x287FB697) *out=static_cast<Service*>(this);
        else return false;
        RefAdd(); return true;
    }
    bool Init() override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        // Native helper classes are reached through interfaces only. Their
        // host server's registry stays empty; the live registry is Rust-owned.
        helpers_->Init();
        if (!active_) SetDispatcher(helpers_->GetCommandDispatcher());
        initialized_=true; return true;
    }
    bool Shutdown() override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        fc4_gzfw_registry_change(core_,3,0,nullptr,nullptr); SetDispatcher(nullptr);
        helpers_->Shutdown();
        for (auto* g:generators_) if (g) FC4_RELEASE(g);
        generators_.clear(); initialized_=false; return true;
    }
    uint32_t CreateParameters(cIGZCommandParameterSet** out) override { return Slot<uint32_t(__thiscall*)(void*,cIGZCommandParameterSet**)>(helpers_,5)(helpers_,out); }
    bool Register(uint32_t id,const char* name,const char* parameters) override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        if (!fc4_gzfw_registry_change(core_,0,id,name,parameters)) return false;
        std::string s[2]; Strings(id,s); for (auto* g:generators_) Notify(g,id,s,true); return true;
    }
    bool RegisterMany(cIGZCommandServer::cGZCommandInfo* p,uint32_t n) override {
        // SPEC-GAP: GZF3 bulk partial failure attempts all and ANDs results.
        bool ok=true; for (uint32_t i=0;i<n;++i) ok=Register(p[i].dwCommandID,p[i].pszCommandName,p[i].pszParameterInfo)&&ok; return ok;
    }
    bool Parameters(uint32_t id,const char* text) override { return fc4_gzfw_registry_change(core_,1,id,nullptr,text); }
    bool Unregister(uint32_t id) override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        std::string s[2]; if (!Strings(id,s)) return false;
        fc4_gzfw_registry_change(core_,2,id,nullptr,nullptr);
        for (auto* g:generators_) Notify(g,id,s,false); return true;
    }
    bool UnregisterMany(cIGZCommandServer::cGZCommandInfo* p,uint32_t n) override {
        // SPEC-GAP: GZF3 bulk partial failure attempts all and ANDs results.
        bool ok=true; for (uint32_t i=0;i<n;++i) ok=Unregister(p[i].dwCommandID)&&ok; return ok;
    }
    bool Info(uint32_t id,cIGZString& name,cIGZString* info) override {
        std::string s[2]; if (!Strings(id,s)) return false;
        name.FromChar(s[0].data(),static_cast<uint32_t>(s[0].size()));
        if (info) info->FromChar(s[1].data(),static_cast<uint32_t>(s[1].size())); return true;
    }
    uint32_t ID(const cIGZString& s) override { return fc4_gzfw_registry_lookup(core_,reinterpret_cast<const uint8_t*>(s.ToChar()),s.Strlen(),false); }
    uint32_t ParseID(const char* s,int32_t n) override { return fc4_gzfw_registry_lookup(core_,reinterpret_cast<const uint8_t*>(s),Length(s,n),true); }
    bool Enumerate(void*) override {
        // SPEC-GAP: GZF3 does not establish the enumeration argument contract.
        fc4::Log("gzfw command: unsupported EnumerateCommands gap=GZF3"); return false;
    }
    bool RegisterGenerator(cIGZCommandGenerator* g) override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        if (std::find(generators_.begin(),generators_.end(),g)!=generators_.end()) return true;
        if (g) FC4_ADDREF(g);
        generators_.push_back(g);
        std::vector<uint32_t> ids(fc4_gzfw_registry_ids(core_,nullptr,0)); fc4_gzfw_registry_ids(core_,ids.data(),ids.size());
        for (auto id:ids) { std::string s[2]; if (Strings(id,s)) Notify(g,id,s,true); } return true;
    }
    bool UnregisterGenerator(cIGZCommandGenerator* g) override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        auto it=std::find(generators_.begin(),generators_.end(),g);
        if (it!=generators_.end()) { generators_.erase(it); if (g) FC4_RELEASE(g); } return true;
    }
    cIGZCommandDispatcher* Dispatcher() override { std::lock_guard<std::recursive_mutex> lock(mutex_); return active_; }
    bool SetDispatcher(cIGZCommandDispatcher* d) override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        if (d) FC4_ADDREF(d);
        auto* old=active_; active_=d; if (old) FC4_RELEASE(old); return true;
    }
    int32_t ExecuteText(const char* text,uint32_t n,cIGZCommandParameterSet* out) override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        uint32_t id=0; cIGZCommandParameterSet* in=nullptr;
        if (!initialized_ || !FromText(text,n,id,in)) { TraceText(text,n,2); return 2; }
        FC4_ACQUIRED(in,"gzfw parsed parameter set");
        const auto status=ExecuteID(id,in,out); FC4_RELEASE_CREATED(in); TraceText(text,n,status); return status;
    }
    int32_t ExecuteID(uint32_t id,cIGZCommandParameterSet* in,cIGZCommandParameterSet* out) override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        // SPEC-GAP: GZF3 null-dispatcher execution returns failure status 2.
        const auto status=active_ ? active_->ExecuteCommand(id,in,out) : 2; TraceID(id,status); return status;
    }
    int32_t ExecuteFormatted(const char* format,cIGZCommandParameterSet* first,...) override {
        char buffer[512]{};
        const int n=_vsnprintf(buffer,511,format,reinterpret_cast<va_list>(&first)); buffer[511]=0;
        return n>0 ? ExecuteText(buffer,n,first) : 2;
    }
    bool FromText(const char* text,uint32_t n,uint32_t& id,cIGZCommandParameterSet*& out) override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        if (!initialized_) return false;
        // command-system section 5: slot 22 uses the default string generator;
        // registered generators receive notifications, not conversion requests.
        std::vector<std::string> words; fc4_gzfw_words(reinterpret_cast<const uint8_t*>(text),Length(text,n),&words,Words);
        if (words.empty()) return false;
        id=fc4_gzfw_registry_lookup(core_,reinterpret_cast<const uint8_t*>(words[0].data()),words[0].size(),false); if (!id) return false;
        cIGZCommandParameterSet* ps=nullptr; CreateParameters(&ps); if (!ps) return false;
        for (size_t i=1;i<words.size();++i) {
            // command-system section 3.4: copied RZChar array, without NUL.
            cFC4Variant value; value.SetRefRZChar(words[i].data(),static_cast<uint32_t>(words[i].size()));
            ps->SetParameter(static_cast<uint32_t>(i-1),*fc4::AsSdk(&value));
        }
        out=ps; return true;
    }
    bool ToText(uint32_t id,cIGZCommandParameterSet* ps,cIGZString& out) override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        if (!initialized_) return false;
        out.Erase(0,0xffffffff); std::string info[2]; if (!Strings(id,info)) return false;
        std::string text; Quote(text,info[0].data(),info[0].size()); AppendParameters(ps,text,true);
        out.FromChar(text.data(),static_cast<uint32_t>(text.size())); return true;
    }
    bool OutputText(cIGZCommandParameterSet* ps,cIGZString& out) override {
        std::lock_guard<std::recursive_mutex> lock(mutex_);
        if (!initialized_) return false;
        std::string text; AppendParameters(ps,text,false); out.FromChar(text.data(),static_cast<uint32_t>(text.size())); return true;
    }
};
bool Primary::QueryInterface(uint32_t i,void** p) { return static_cast<Server*>(this)->Query(i,p); }
uint32_t Primary::AddRef() { return static_cast<Server*>(this)->RefAdd(); }
uint32_t Primary::Release() { return static_cast<Server*>(this)->RefRelease(); }
bool Service::QueryInterface(uint32_t i,void** p) { return static_cast<Server*>(this)->Query(i,p,true); }
uint32_t Service::AddRef() { return static_cast<Server*>(this)->RefAdd(); }
uint32_t Service::Release() { return static_cast<Server*>(this)->RefRelease(); }
uint32_t Service::GetServiceID() { return static_cast<Server*>(this)->Metadata()->GetServiceID(); }
cIGZSystemService* Service::SetServiceID(uint32_t id) { static_cast<Server*>(this)->Metadata()->SetServiceID(id); return this; }
int32_t Service::GetServicePriority() { return static_cast<Server*>(this)->Metadata()->GetServicePriority(); }
bool Service::IsServiceRunning() { return static_cast<Server*>(this)->Metadata()->IsServiceRunning(); }
cIGZSystemService* Service::SetServiceRunning(bool b) { static_cast<Server*>(this)->Metadata()->SetServiceRunning(b); return this; }
int32_t Service::GetServiceTickPriority() { return static_cast<Server*>(this)->Metadata()->GetServiceTickPriority(); }
bool Lifetime::QueryInterface(uint32_t i,void** p) { return static_cast<Server*>(this)->Query(i,p,true); }
uint32_t Lifetime::AddRef() { return static_cast<Server*>(this)->RefAdd(); }
uint32_t Lifetime::Release() { return static_cast<Server*>(this)->RefRelease(); }

cIGZCommandServer* traced=nullptr;
void** originalTable=nullptr;
void* traceTable[25]{};
using TextFn=int32_t(__thiscall*)(void*,const char*,uint32_t,cIGZCommandParameterSet*);
using IDFn=int32_t(__thiscall*)(void*,uint32_t,cIGZCommandParameterSet*,cIGZCommandParameterSet*);
int32_t __fastcall NativeText(void* p,void*,const char* s,uint32_t n,cIGZCommandParameterSet* out) {
    const auto status=reinterpret_cast<TextFn>(originalTable[19])(p,s,n,out); TraceText(s,n,status); return status;
}
int32_t __fastcall NativeID(void* p,void*,uint32_t id,cIGZCommandParameterSet* in,cIGZCommandParameterSet* out) {
    const auto status=reinterpret_cast<IDFn>(originalTable[20])(p,id,in,out); TraceID(id,status); return status;
}
}
namespace fc4 {
cIGZUnknown* GZCommandServerTestCreate(cIGZUnknown* helpers) {
    if (!helpers) return nullptr;
    cIGZSystemService* service=nullptr;
    if (!helpers->QueryInterface(0x287FB697,reinterpret_cast<void**>(&service)) || !service) return nullptr;
    FC4_ACQUIRED(service,"gzfw test helper service"); FC4_ADDREF(helpers);
    auto* server=new(std::nothrow) Server(reinterpret_cast<cIGZCommandServer*>(helpers),service);
    if (!server) { FC4_RELEASE(service); FC4_RELEASE(helpers); return nullptr; }
    server->RefAdd(); return static_cast<Primary*>(server);
}
void GZFrameworkStart(cIGZFrameWork* fw) {
    static bool installed=false;
    if (installed) return;
    bool enabled=false; size_t count=0; const auto* subs=Subsystems(&count);
    for (size_t i=0;i<count;++i) if (std::strcmp(subs[i].key,"gzfw")==0) enabled=SubsystemEnabled(subs[i]);
    if (!fw || (!enabled && !TraceEnabled())) return;
    cIGZCommandServer* native=nullptr;
    if (!fw->GetSystemService(0xEB903A32,0xCB90253A,reinterpret_cast<void**>(&native)) || !native) { Log("gzfw: command service unavailable at install"); return; }
    FC4_ACQUIRED(native,"gzfw original helper service");
    if (!enabled) {
        traced=native; originalTable=*reinterpret_cast<void***>(native); std::copy(originalTable,originalTable+25,traceTable);
        traceTable[19]=reinterpret_cast<void*>(&NativeText); traceTable[20]=reinterpret_cast<void*>(&NativeID);
        *reinterpret_cast<void***>(native)=traceTable; installed=true; Log("gzfw: native command stream trace installed; replacement=0"); return;
    }
    cIGZSystemService* service=nullptr;
    if (!native->QueryInterface(0x287FB697,reinterpret_cast<void**>(&service)) || !service) { FC4_RELEASE(native); return; }
    FC4_ACQUIRED(service,"gzfw original system-service face");
    auto* ours=new(std::nothrow) Server(native,service);
    if (!ours) { FC4_RELEASE(service); FC4_RELEASE(native); return; }
    ours->RefAdd();
    if (!fw->RemoveSystemService(service)) { ours->RefRelease(); Log("gzfw: RemoveSystemService refused"); return; }
    if (!fw->AddSystemService(static_cast<Service*>(ours))) {
        fw->AddSystemService(service); ours->RefRelease(); Log("gzfw: AddSystemService refused; original restored"); return;
    }
    ours->RefRelease(); installed=true; Log("gzfw: FC4 command service installed; native helpers=parameter-set,dispatcher; registry=Rust");
}
void GZFrameworkStop() {
    if (traced) { *reinterpret_cast<void***>(traced)=originalTable; FC4_RELEASE(traced); traced=nullptr; }
}
}
