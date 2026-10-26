// SPDX-License-Identifier: GPL-3.0-or-later
// Included in the capture implementation's private namespace.
struct Count {
    uint64_t calls=0,sampled=0,compared=0,exact=0,floatOnly=0,different=0;
    uint64_t outside=0,refused=0,nested=0,mutation=0,detected=0,recorded=0,replaced=0;
    uint32_t maxUlp=0;
};
struct Function {
    const char *name,*owner,*ret,*windows,*rustEntry,*mode;
    uint32_t va,bytes,argCount,pop;bool receiver;uint32_t fingerprint;void* forwarder;
    void* original=nullptr;std::array<Count,4> counts{};
    bool selected=false,installed=false,mutate=false,valid=false;
    unsigned records=0,cases=0;
    std::vector<fc4::compare::WindowSpec> specs;
    const fc4::CompareInterposition* interposition=nullptr;
};
#include "FC4CompareRows.inc"
SRWLOCK sampleLock=SRWLOCK_INIT;
thread_local unsigned depth=0;
thread_local unsigned source=0;
const char* const sources[]={"natural","fixture","corpus","dense"};
bool active=false,fixtures=false,replayCorpus=false,persist=false;
unsigned denseCount=0,recordLimit=50,caseLimit=5;
uintptr_t imageBase=0;
constexpr uint32_t imageSize=0x7c8000;
std::string modeOverride;
bool Selected(std::string list,const Function& fn) {
    list.erase(std::remove_if(list.begin(),list.end(),[](char c){return c==' '||c=='\t';}),list.end());
    for (const auto& word:fc4::compare::Split(list,','))
        if (word=="all"||word==fn.name||word==std::string("class:")+fn.owner) return true;
    return false;
}
