// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <windows.h>
#include <cstdint>
#include <map>
#include <vector>

namespace fc4 {
// prop-developer §4.4: observe the actual public creation calls and outcomes,
// including failures. An insertion notification is not a creation result.
class PropCreationTrace {
public:
    struct Event {
        uintptr_t lot;
        bool network;
        uint32_t index, id, depth;
        size_t nestedBegin, nestedEnd;
        uint64_t result;
    };
    using Create = void* (__fastcall*)(void*,void*,void*,uint32_t,uint32_t);
    using Retain = uint64_t (*)(void*,void*);
    using NestedCount = size_t (*)();
    ~PropCreationTrace() { Finish(); }
    bool Begin(void* lotDeveloper, void* networkManager, Retain retain, void* user, NestedCount count) {
        if (active_ || !installed_.empty()) return false;
        events_.clear(); valid_=true; retain_=retain; user_=user; count_=count;
        if (!Install(lotDeveloper,63,reinterpret_cast<void*>(&Lot)) ||
            !Install(networkManager,8,reinterpret_cast<void*>(&Network))) {
            valid_=false; Finish(); return false;
        }
        active_=this; return true;
    }
    void Finish() {
        if (active_==this) active_=nullptr;
        for (const auto& entry:installed_) {
            if (!Patch(entry.first.first,entry.first.second,reinterpret_cast<void*>(entry.second))) valid_=false;
        }
        installed_.clear();
    }
    bool Valid() const { return valid_; }
    const std::vector<Event>& Events() const { return events_; }
private:
    using Key=std::pair<void**,size_t>;
    static bool Patch(void** table,size_t slot,void* value) {
        DWORD old=0;
        if (!VirtualProtect(table+slot,sizeof(void*),PAGE_READWRITE,&old)) return false;
        InterlockedExchangePointer(table+slot,value);
        DWORD ignored=0; VirtualProtect(table+slot,sizeof(void*),old,&ignored);
        return true;
    }
    bool Install(void* service,size_t slot,void* callback) {
        if (!service) return false;
        auto** table=*static_cast<void***>(service);
        const Key key{table,slot};
        const auto original=reinterpret_cast<Create>(table[slot]);
        AcquireSRWLockExclusive(&lock_);
        originals_[key]=original;
        ReleaseSRWLockExclusive(&lock_);
        if (!Patch(table,slot,callback)) return false;
        installed_[key]=original;
        return true;
    }
    static void* __fastcall Lot(void* self,void* edx,void* lot,uint32_t index,uint32_t id) {
        return Invoke(self,edx,lot,index,id,false);
    }
    static void* __fastcall Network(void* self,void* edx,void* lot,uint32_t index,uint32_t id) {
        return Invoke(self,edx,lot,index,id,true);
    }
    static void* Invoke(void* self,void* edx,void* lot,uint32_t index,uint32_t id,bool network) {
        const Key key{*static_cast<void***>(self),network?8u:63u};
        AcquireSRWLockShared(&lock_);
        const auto found=originals_.find(key);
        const Create original=found==originals_.end()?nullptr:found->second;
        ReleaseSRWLockShared(&lock_);
        if (!original) return nullptr;
        auto* trace=active_;
        if (!trace) return original(self,edx,lot,index,id);
        const size_t slot=trace->events_.size();
        trace->events_.push_back({reinterpret_cast<uintptr_t>(lot),network,index,id,trace->depth_,
                                  trace->count_(),0,0});
        ++trace->depth_;
        void* result=original(self,edx,lot,index,id); // native call exactly once
        --trace->depth_;
        auto& event=trace->events_[slot];
        event.nestedEnd=trace->count_();
        event.result=result?trace->retain_(trace->user_,result):0;
        if (result && !event.result) trace->valid_=false;
        return result;
    }
    inline static SRWLOCK lock_=SRWLOCK_INIT;
    inline static std::map<Key,Create> originals_;
    inline static thread_local PropCreationTrace* active_=nullptr;
    std::map<Key,Create> installed_;
    std::vector<Event> events_;
    Retain retain_=nullptr;
    NestedCount count_=nullptr;
    void* user_=nullptr;
    uint32_t depth_=0;
    bool valid_=false;
};
}
