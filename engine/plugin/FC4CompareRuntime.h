// SPDX-License-Identifier: GPL-3.0-or-later
// Generic registry dispatch. Included after Capture in FC4Inplace.cpp.
using Bytes=std::vector<uint8_t>;
struct Input {
    std::array<uint32_t,16> args{};
    std::vector<Bytes> windows;
    bool canonical=false;
};
std::string Path(const char* directory,const Function& fn,const char* extension) {
    return std::string(FC4_COMPARE_ROOT)+"/"+directory+"/"+fn.name+extension;
}
bool Canonical(const Function& fn,const Input& input,Input& output,bool initial=false) {
    output=input;if (input.canonical) return true;
    for (unsigned i=0;i<fn.specs.size();++i) {
        const auto& spec=fn.specs[i];
        if (!fc4::compare::Sanitize(spec,input.args.data(),input.windows[i].data(),output.windows[i],uint32_t(imageBase),imageSize)) return false;
        // Output-only storage can be uninitialized caller stack. It is not input.
        if (initial&&!spec.read) std::fill(output.windows[i].begin(),output.windows[i].end(),0);
        if (spec.base) output.args[spec.base-1]=0;
    }
    output.canonical=true;return true;
}
struct Copies {
    std::vector<uint8_t*> allocations;
    std::vector<fc4::compare::Buffer> buffers;
    std::array<uint32_t,16> args{};
    uint32_t self=0;bool ok=true;
    Copies(const Function& fn,const Input& input):args(input.args) {
        for (unsigned i=0;i<fn.specs.size();++i) {
            const auto& spec=fn.specs[i];const auto size=input.windows[i].size();
            auto* storage=static_cast<uint8_t*>(VirtualAlloc(nullptr,size+spec.offset,MEM_RESERVE|MEM_COMMIT,PAGE_READWRITE));
            if (!storage) { ok=false;return; }
            allocations.push_back(storage);auto* data=storage+spec.offset;
            std::memcpy(data,input.windows[i].data(),size);
            if (input.canonical&&!fc4::compare::Fields(spec,args.data(),size,[&](const auto& field,size_t at) {
                if (field.type=="image") { uint32_t v;std::memcpy(&v,data+at,4);if (v) {v+=uint32_t(imageBase);std::memcpy(data+at,&v,4);} }
            })) { ok=false;return; }
            buffers.push_back({data,uint32_t(size),spec.base,spec.offset});
            if (!spec.base) self=uint32_t(reinterpret_cast<uintptr_t>(storage));
            else args[spec.base-1]=uint32_t(reinterpret_cast<uintptr_t>(storage));
        }
    }
    ~Copies() { for (auto* p:allocations) VirtualFree(p,0,MEM_RELEASE); }
    Input Values(const Input& original) const {
        Input out;out.args=original.args;
        for (const auto& b:buffers) out.windows.emplace_back(b.data,b.data+b.size);
        return out;
    }
};
void Interpose(Function& fn,const char* phase,const uint32_t* args,bool begin) {
    const auto* hook=fn.interposition;if (!hook) return;
    const fc4::CompareCallView view{fn.name,sources[source],phase,args};
    const auto callback=begin?hook->begin:hook->end;
    if (callback) callback(hook->context,view);
}
void U32(Bytes& bytes,uint32_t value) {
    const auto* p=reinterpret_cast<const uint8_t*>(&value);bytes.insert(bytes.end(),p,p+4);
}
bool Take(const Bytes& bytes,size_t& at,uint32_t& value) {
    if (at+4>bytes.size()) return false;std::memcpy(&value,bytes.data()+at,4);at+=4;return true;
}
std::vector<Input> LoadCorpus(Function& fn) {
    std::vector<Input> result;std::ifstream file(Path("compare-corpus",fn,".bin"),std::ios::binary);
    uint64_t total=0;
    // The current write cap must not truncate a corpus recorded with a larger
    // cap on a previous run. Replay every stored input up to the format ceiling.
    while (file&&result.size()<1000) {
        uint32_t header[4]{};file.read(reinterpret_cast<char*>(header),sizeof(header));
        if (file.gcount()==0) break;
        if (file.gcount()!=sizeof(header)||header[0]!=0x43344346||header[1]!=1||header[2]!=fn.fingerprint||header[3]>16*1024*1024||header[3]<72) {
            fc4::Log("inplace corpus rejected name=%s reason=header-contract-or-cap",fn.name);break;
        }
        total+=header[3];if (total>64*1024*1024) break;
        Bytes bytes(header[3]);file.read(reinterpret_cast<char*>(bytes.data()),bytes.size());if (!file) break;
        Input input;input.canonical=true;size_t at=0;uint32_t count=0;
        bool valid=Take(bytes,at,count)&&count==fn.argCount;
        for (auto& arg:input.args) valid=Take(bytes,at,arg)&&valid;
        valid=Take(bytes,at,count)&&valid&&count==fn.specs.size();
        if (!valid) break;
        for (const auto& spec:fn.specs) {
            uint32_t size=0;if (!Take(bytes,at,size)||size!=spec.Size(input.args.data())||at+size>bytes.size()||(spec.base&&input.args[spec.base-1])) { valid=false;break; }
            input.windows.emplace_back(bytes.begin()+at,bytes.begin()+at+size);at+=size;
            // Reject crafted non-value bytes and invalid image offsets as well.
            Bytes check(size);bool imageValid=true;
            auto runtime=input.windows.back();
            valid=fc4::compare::Fields(spec,input.args.data(),size,[&](const auto& field,size_t offset) {
                if (field.type=="image") { uint32_t value;std::memcpy(&value,runtime.data()+offset,4);
                    if (value>=imageSize) imageValid=false;
                    else if (value) { value+=uint32_t(imageBase);std::memcpy(runtime.data()+offset,&value,4); }
                }
            })&&valid;
            valid=fc4::compare::Sanitize(spec,input.args.data(),runtime.data(),check,uint32_t(imageBase),imageSize)&&check==input.windows.back()&&imageValid&&valid;
        }
        if (!valid||at!=bytes.size()) { fc4::Log("inplace corpus rejected name=%s reason=payload-schema",fn.name);break; }
        result.push_back(std::move(input));
    }
    return result;
}
bool Record(Function& fn,const Input& input) {
    if (!persist||source>=2||fn.records>=recordLimit) return false;
    Input clean;if (!Canonical(fn,input,clean,true)) return false;
    Bytes payload;U32(payload,fn.argCount);for (auto arg:clean.args) U32(payload,arg);
    U32(payload,uint32_t(clean.windows.size()));
    for (const auto& w:clean.windows) { U32(payload,uint32_t(w.size()));payload.insert(payload.end(),w.begin(),w.end()); }
    if (payload.size()>16*1024*1024) return false;
    std::filesystem::create_directories(std::string(FC4_COMPARE_ROOT)+"/compare-corpus");
    const auto path=Path("compare-corpus",fn,".bin");
    if (std::filesystem::exists(path)&&std::filesystem::file_size(path)+payload.size()+16>64*1024*1024) return false;
    std::ofstream file(path,std::ios::binary|std::ios::app);
    const uint32_t header[]={0x43344346,1,fn.fingerprint,uint32_t(payload.size())};
    file.write(reinterpret_cast<const char*>(header),sizeof(header));file.write(reinterpret_cast<const char*>(payload.data()),payload.size());
    if (!file) { fc4::Log("inplace corpus write-failed name=%s",fn.name);return false; }
    ++fn.records;return true;
}
void JsonBytes(std::ostream& out,const Bytes& bytes) {
    out<<'[';bool first=true;for (auto b:bytes) {if (!first) out<<',';first=false;out<<unsigned(b);}out<<']';
}
void JsonWindows(std::ostream& out,const Function& fn,const Input& input) {
    out<<'[';for (unsigned i=0;i<fn.specs.size();++i) {
        if (i) out<<',';const auto& spec=fn.specs[i];
        out<<"{\"base\":"<<spec.base<<",\"offset\":"<<spec.offset<<",\"bytes\":";JsonBytes(out,input.windows[i]);
        out<<",\"image_offsets\":[";bool first=true;
        fc4::compare::Fields(spec,input.args.data(),input.windows[i].size(),[&](const auto& field,size_t at) {
            if (field.type=="image") {if (!first) out<<',';first=false;out<<at;}
        });out<<"]}";
    }out<<']';
}
void Case(Function& fn,const Input& input,const Input& native,const Input& ours,
          const fc4::compare::Result& nr,const fc4::compare::Result& rr,bool mutation,unsigned window,size_t byte) {
    if (fn.cases>=caseLimit) return;
    Input ci,cn,co;if (!Canonical(fn,input,ci,true)||!Canonical(fn,native,cn)||!Canonical(fn,ours,co)) return;
    std::filesystem::create_directories(std::string(FC4_COMPARE_ROOT)+"/compare-cases");
    std::ofstream out(Path("compare-cases",fn,".jsonl"),std::ios::app);
    out<<"{\"version\":1,\"name\":\""<<fn.name<<"\",\"rva\":"<<(fn.va-uint32_t(imageBase))<<",\"contract\":"<<fn.fingerprint
        <<",\"source\":\""<<sources[source]<<"\",\"mutation\":"<<(mutation?"true":"false")<<",\"window\":"<<window<<",\"byte\":"<<byte<<",\"args\":[";
    for (unsigned i=0;i<fn.argCount;++i) {if (i) out<<',';out<<ci.args[i];}
    out<<"],\"input\":";JsonWindows(out,fn,ci);out<<",\"native\":";JsonWindows(out,fn,cn);out<<",\"ours\":";JsonWindows(out,fn,co);
    const auto value=[&](const fc4::compare::Result& r) { return std::string(fn.ret)=="al"?(r.eax&255):(std::string(fn.ret)=="x87-f32"?r.floatBits:(std::string(fn.ret)=="void"?0:r.eax)); };
    out<<",\"native_return\":"<<value(nr)<<",\"ours_return\":"<<value(rr)<<"}\n";
    if (out) ++fn.cases;
}
struct Difference { bool equal=true,floatOnly=true;unsigned window=0;size_t byte=0;uint32_t maxUlp=0; };
Difference Compare(const Function& fn,const Input& native,const Input& ours,const fc4::compare::Result& nr,const fc4::compare::Result& rr) {
    Difference diff;
    for (unsigned i=0;i<fn.specs.size();++i) {
        const auto& a=native.windows[i];const auto& b=ours.windows[i];
        std::vector<bool> floats(a.size(),false);
        fc4::compare::Fields(fn.specs[i],native.args.data(),a.size(),[&](const auto& field,size_t at) {
            if (field.type=="f32") for (unsigned j=0;j<4;++j) floats[at+j]=true;
        });
        for (size_t j=0;j<a.size();++j) if (a[j]!=b[j]) {
            if (diff.equal) {diff.window=i;diff.byte=j;}diff.equal=false;
            if (!floats[j]||j/4*4+4>a.size()) diff.floatOnly=false;
            else { uint32_t x,y;std::memcpy(&x,a.data()+(j&~size_t(3)),4);std::memcpy(&y,b.data()+(j&~size_t(3)),4);
                const auto ulp=fc4::inplace::FloatDistance(x,y);diff.maxUlp=std::max(diff.maxUlp,ulp);if (!fc4::inplace::NearFloat(x,y)) diff.floatOnly=false;
            }
        }
    }
    const std::string ret=fn.ret;
    const auto a=ret=="al"?(nr.eax&255):(ret=="x87-f32"?nr.floatBits:nr.eax);
    const auto b=ret=="al"?(rr.eax&255):(ret=="x87-f32"?rr.floatBits:rr.eax);
    if (ret!="void"&&a!=b) {
        if (diff.equal) {diff.window=99;diff.byte=0;}diff.equal=false;
        if (ret=="x87-f32") {diff.maxUlp=std::max(diff.maxUlp,fc4::inplace::FloatDistance(a,b));if (!fc4::inplace::NearFloat(a,b)) diff.floatOnly=false;}
        else diff.floatOnly=false;
    }
    return diff;
}
fc4::compare::Result NativeResult(const Function& fn,const fc4::inplace::Registers& registers,fc4::inplace::MachineState& machine) {
    fc4::compare::Result result;result.eax=registers.eax;result.edx=registers.edx;
    if (std::string(fn.ret)=="x87-f32") {
        alignas(16) uint8_t saved[512];auto* native=machine.after;auto* value=&result.floatBits;
        __asm {
            fxsave saved
            mov eax,native
            fxrstor [eax]
            mov eax,value
            fst dword ptr [eax]
            fxrstor saved
        }
    }
    return result;
}
void InstallResult(const Function& fn,const fc4::compare::Result& value,fc4::inplace::Registers* registers,fc4::inplace::MachineState* machine) {
    std::memcpy(machine->after,machine->before,512);
    const std::string ret=fn.ret;
    if (ret=="eax") registers->eax=value.eax;
    else if (ret=="al") registers->eax=(registers->eax&~255u)|(value.eax&255);
    else if (ret=="x87-f32") {
        alignas(16) uint8_t saved[512];auto* state=machine->after;const auto* bits=&value.floatBits;
        __asm {
            fxsave saved
            mov eax,state
            fxrstor [eax]
            mov eax,bits
            fld dword ptr [eax]
            mov eax,state
            fxsave [eax]
            fxrstor saved
        }
    }
}
void Evaluate(Function& fn,Count& count,const Input& input,const Input& native,const Input& ours,
              const fc4::compare::Result& nr,fc4::compare::Result rr) {
    const auto diff=Compare(fn,native,ours,nr,rr);++count.compared;count.maxUlp=std::max(count.maxUlp,diff.maxUlp);
    if (diff.equal) ++count.exact;
    else {
        if (diff.floatOnly) ++count.floatOnly;else ++count.different;
        if (count.different+count.floatOnly<=3) fc4::Log("inplace difference name=%s source=%s window=%u byte=%zu float_only=%u",fn.name,sources[source],diff.window,diff.byte,diff.floatOnly?1:0);
        Case(fn,input,native,ours,nr,rr,false,diff.window,diff.byte);
    }
    if (fn.mutate&&diff.equal&&count.mutation==0) {
        Input mutated=ours;bool changed=false;
        for (unsigned i=0;i<fn.specs.size();++i) if (fn.specs[i].write&&!mutated.windows[i].empty()) {mutated.windows[i][0]^=1;changed=true;break;}
        if (!changed&&std::string(fn.ret)!="void") {rr.eax^=1;rr.floatBits^=1;changed=true;}
        if (changed) {
            ++count.mutation;const auto bad=Compare(fn,native,mutated,nr,rr);count.detected+=!bad.equal;
            fc4::Log("inplace mutation name=%s source=%s window=%u byte=%zu xor=1 detected=%llu",fn.name,sources[source],bad.window,bad.byte,count.detected);
            Case(fn,input,native,mutated,nr,rr,true,bad.window,bad.byte);
        }
    }
}
void Dispatch(fc4::inplace::Registers* registers,fc4::inplace::MachineState* machine) {
    if (registers->index>=std::size(functions)) {std::memcpy(machine->after,machine->before,512);return;}
    auto& fn=functions[registers->index];auto& count=fn.counts[source];
    const auto* args=reinterpret_cast<const uint32_t*>(registers+1);
    const auto original=[&] {fc4_inplace_invoke(registers,machine,fn.original,args,fn.argCount*4,fn.pop!=0||fn.argCount==0);};
    const bool locking=depth==0;if (locking) AcquireSRWLockExclusive(&sampleLock);
    if (depth) ++count.nested;
    const auto call=++count.calls;const std::string mode=modeOverride.empty()?fn.mode:modeOverride;
    if (mode!="replace"&&source==0&&!fc4::inplace::Sample(call)) {
        // An unsampled original can enter another selected function. Do not
        // retain an unmarked outer lock across that call.
        if (locking) ReleaseSRWLockExclusive(&sampleLock);original();return;
    }
    ++count.sampled;++depth;
    Capture capture;Input input;std::copy_n(args,fn.argCount,input.args.begin());
    bool valid=true;
    for (const auto& spec:fn.specs) {
        const auto size=spec.Size(input.args.data());const uintptr_t base=spec.base?args[spec.base-1]:registers->ecx;
        if (!size||!base||base+spec.offset+size>0xffffffffu) {valid=false;break;}
        capture.Add(reinterpret_cast<void*>(base+spec.offset),size,spec.write?0:-1,false);
    }
    valid=valid&&capture.Prepare(reinterpret_cast<uintptr_t>(args)+fn.argCount*4);
    if (!valid) {
        ++count.refused;if (count.refused<=3) fc4::Log("inplace refused name=%s source=%s reason=%s",fn.name,sources[source],capture.reason);
        original();--depth;if (locking) ReleaseSRWLockExclusive(&sampleLock);return;
    }
    for (unsigned i=0;i<capture.windowCount;++i) {const auto& w=capture.windows[i];input.windows.emplace_back(w.pointer,w.pointer+w.size);}
    if (Record(fn,input)) ++count.recorded;
    if (mode=="record") {original();--depth;if (locking) ReleaseSRWLockExclusive(&sampleLock);return;}
    Copies ours(fn,input);fc4::compare::Result rr;
    Interpose(fn,"ours",input.args.data(),true);
    const bool adapted=ours.ok&&fc4_compare_adapter(fn.rustEntry,ours.args.data(),ours.buffers.data(),ours.buffers.size(),&rr);
    Interpose(fn,"ours",input.args.data(),false);
    if (!adapted) {
        ++count.refused;if (count.refused<=3) fc4::Log("inplace refused name=%s source=%s reason=adapter-domain",fn.name,sources[source]);
        original();--depth;if (locking) ReleaseSRWLockExclusive(&sampleLock);return;
    }
    if (mode=="replace") {
        for (unsigned i=0;i<fn.specs.size();++i) if (fn.specs[i].write) std::memcpy(capture.windows[i].pointer,ours.buffers[i].data,ours.buffers[i].size);
        InstallResult(fn,rr,registers,machine);++count.replaced;
    } else {
        Interpose(fn,"original",input.args.data(),true);
        capture.Snapshot(false);original();capture.Snapshot(true);
        Interpose(fn,"original",input.args.data(),false);
        unsigned region=0;size_t offset=0;
        if (capture.Outside(region,offset)) {
            ++count.outside;if (count.outside<=3) fc4::Log("inplace outside name=%s source=%s region=%u offset=%zu watched=%zu attribution=unknown",fn.name,sources[source],region,offset,capture.total);
        } else {
            Input native;native.args=input.args;
            for (unsigned i=0;i<capture.windowCount;++i) {const auto& w=capture.windows[i];const auto* p=capture.Saved(w,true);native.windows.emplace_back(p,p+w.size);}
            Evaluate(fn,count,input,native,ours.Values(input),NativeResult(fn,*registers,*machine),rr);
        }
    }
    --depth;if (locking) ReleaseSRWLockExclusive(&sampleLock);
}
// Corpus/fixtures execute only in isolated native storage, never in live objects.
void Replay(Function& fn,const Input& input) {
    auto& count=fn.counts[source];++count.calls;++count.sampled;++depth;
    Copies native(fn,input),ours(fn,input);fc4::compare::Result rr;
    Interpose(fn,"ours",input.args.data(),true);
    const bool adapted=native.ok&&ours.ok&&fc4_compare_adapter(fn.rustEntry,ours.args.data(),ours.buffers.data(),ours.buffers.size(),&rr);
    Interpose(fn,"ours",input.args.data(),false);
    if (!adapted) {++count.refused;--depth;return;}
    Capture capture;for (unsigned i=0;i<fn.specs.size();++i) capture.Add(native.buffers[i].data,native.buffers[i].size,fn.specs[i].write?0:-1);
    if (!capture.Prepare(0)) {++count.refused;--depth;return;}
    fc4::inplace::Registers regs{};regs.ecx=native.self;
    fc4::inplace::MachineState machine;auto* before=machine.before;
    __asm {
        mov eax,before
        fxsave [eax]
    }
    Interpose(fn,"original",native.args.data(),true);
    capture.Snapshot(false);fc4_inplace_invoke(&regs,&machine,fn.original,native.args.data(),fn.argCount*4,fn.pop!=0||fn.argCount==0);capture.Snapshot(true);
    Interpose(fn,"original",native.args.data(),false);
    unsigned region=0;size_t offset=0;
    if (capture.Outside(region,offset)) ++count.outside;
    else Evaluate(fn,count,input,native.Values(input),ours.Values(input),NativeResult(fn,regs,machine),rr);
    --depth;
}
bool Perturb(const Function& fn,Input& input,unsigned iteration) {
    struct Candidate {int window;size_t offset;std::string type;};std::vector<Candidate> fields;
    std::array<bool,16> excluded{};
    for (unsigned i=0;i<fn.specs.size();++i) {
        const auto& spec=fn.specs[i];if (spec.base) excluded[spec.base-1]=true;
        if (spec.span) {excluded[spec.countArg]=true;excluded[spec.strideArg]=true;}
        if (spec.read) fc4::compare::Fields(spec,input.args.data(),input.windows[i].size(),[&](const auto& field,size_t at) {
            if (field.dense&&field.type!="image") fields.push_back({int(i),at,field.type});
        });
    }
    for (unsigned i=0;i<fn.argCount;++i) if (!excluded[i]) fields.push_back({-1,i,"i32"});
    if (fields.empty()) return false;
    for (unsigned n=0;n<1+(iteration%3==2&&fields.size()>1);++n) {
        const auto& f=fields[(iteration+n)%fields.size()];uint32_t value=0;
        auto* p=f.window<0?reinterpret_cast<uint8_t*>(&input.args[f.offset]):input.windows[f.window].data()+f.offset;
        const unsigned width=f.type=="u8"?1:4;std::memcpy(&value,p,width);
        if (f.type=="f32") {float v;std::memcpy(&v,&value,4);if (!std::isfinite(v)) return false;
            v=iteration%4==0?v+0.125f:(iteration%4==1?v-0.125f:(iteration%4==2?v*1.001f:v*0.999f));if (!std::isfinite(v)) return false;std::memcpy(&value,&v,4);
        } else if (f.type=="flags"||f.type=="u8") value^=1u<<(iteration%(width*8));
        else value+=iteration%2?uint32_t(-1):1;
        std::memcpy(p,&value,width);
    }
    return true;
}
void Report(const Function& fn,const Count& c,const char* inputSource) {
    fc4::Log("inplace summary name=%s rva=%08x bytes=%u installed=%u source=%s calls=%llu sampled=%llu compared=%llu exact=%llu float_only=%llu different=%llu outside=%llu refused=%llu nested=%llu mutation=%llu detected=%llu max_ulp=%u recorded=%llu replaced=%llu",
        fn.name,fn.va-uint32_t(imageBase),fn.bytes,fn.installed?1:0,inputSource,c.calls,c.sampled,c.compared,c.exact,c.floatOnly,c.different,c.outside,c.refused,c.nested,c.mutation,c.detected,c.maxUlp,c.recorded,c.replaced);
}
} // private namespace
extern "C" void fc4_inplace_dispatch(fc4::inplace::Registers* registers,fc4::inplace::MachineState* machine) { Dispatch(registers,machine); }
namespace fc4 {
bool CompareAttach(const char* name,const CompareInterposition* callbacks) {
    if (active||!name) return false;
    for (auto& fn:functions) if (std::string(fn.name)==name) {fn.interposition=callbacks;return true;}
    return false;
}
void InplaceStartup() {
    auto selection=Setting("compare",nullptr);if (selection.empty()) selection=Setting("inplace",nullptr);
    if (selection.empty()) return;
    if (!Supported()) {Log("inplace: unsupported executable identity; no patches");return;}
    modeOverride=Setting("compare_mode",nullptr);
    if (!modeOverride.empty()&&modeOverride!="compare"&&modeOverride!="record"&&modeOverride!="replace") {Log("inplace: invalid compare_mode; no patches");return;}
    fixtures=SettingInt("inplace_fixture",nullptr,0)!=0;
    replayCorpus=SettingInt("compare_corpus",nullptr,0)!=0;
    denseCount=unsigned(std::clamp<int>(SettingInt("compare_dense",nullptr,0),0,1000));
    recordLimit=unsigned(std::clamp<int>(SettingInt("compare_record_limit",nullptr,50),1,1000));
    caseLimit=unsigned(std::clamp<int>(SettingInt("compare_case_limit",nullptr,5),0,100));
    persist=SettingInt("compare_record",nullptr,0)!=0||modeOverride=="record";
    const auto mutations=Setting("inplace_mutate",nullptr);
    for (auto& fn:functions) {
        if (!Selected(selection,fn)) continue;fn.selected=true;fn.mutate=Selected(mutations,fn);
        if (std::string(fn.mode)=="record") persist=true;
        try {fn.specs=fc4::compare::Parse(fn.windows,fn.argCount);fn.valid=true;}
        catch (const std::exception&) {Log("inplace install name=%s installed=0 reason=invalid-window-schema",fn.name);continue;}
        fn.records=unsigned(LoadCorpus(fn).size());
        fn.installed=Install(fn,fn.forwarder);active|=fn.installed;
        Log("inplace install name=%s rva=%08x bytes=%u installed=%u reason=%s",fn.name,fn.va-uint32_t(imageBase),fn.bytes,fn.installed?1:0,fn.installed?"ok":"unsupported-entry-or-busy");
    }
    Log("inplace: active=%u sample=first50-then1in20 capture=private-regions cap_mib=64 writes=net-neighbourhood-only fixture=%u corpus=%u dense=%u mode=%s root=%s",active?1:0,fixtures?1:0,replayCorpus?1:0,denseCount,modeOverride.empty()?"registry":modeOverride.c_str(),FC4_COMPARE_ROOT);
}
void InplaceReport() {
    if (!active) return;AcquireSRWLockExclusive(&sampleLock);
    for (const auto& fn:functions) if (fn.selected) for (unsigned s=0;s<4;++s) Report(fn,fn.counts[s],sources[s]);
    ReleaseSRWLockExclusive(&sampleLock);
}
void InplaceCityLoaded() {
    if (!active) return;
    if (fixtures) {
        source=1;
        for (unsigned sample=0;sample<16;++sample) for (auto& fn:functions) {
            if (!fn.installed) continue;Input input;
            if (!fc4_compare_fixture(fn.rustEntry,sample,input.args.data(),nullptr,0)) continue;
            bool valid=true;for (const auto& spec:fn.specs) {const auto size=spec.Size(input.args.data());if (!size) {valid=false;break;}input.windows.emplace_back(size);}
            if (!valid) continue;
            Copies fixture(fn,input);if (!fixture.ok||!fc4_compare_fixture(fn.rustEntry,sample,fixture.args.data(),fixture.buffers.data(),fixture.buffers.size())) continue;
            input=fixture.Values(input);if (Record(fn,input)) ++fn.counts[1].recorded;
            if (sample==0) Log("inplace fixture begin name=%s",fn.name);
            Replay(fn,input);
        }
    }
    if (replayCorpus||denseCount) for (auto& fn:functions) if (fn.installed) {
        const auto inputs=LoadCorpus(fn);
        Log("inplace corpus name=%s inputs=%zu dense=%u",fn.name,inputs.size(),denseCount);
        for (const auto& input:inputs) {
            if (replayCorpus) {source=2;Replay(fn,input);}
            for (unsigned i=0;i<denseCount;++i) {Input perturbed=input;if (Perturb(fn,perturbed,i)) {source=3;Replay(fn,perturbed);} }
        }
    }
    source=0;InplaceReport();
}
} // namespace fc4
