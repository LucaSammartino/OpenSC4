// SPDX-License-Identifier: GPL-3.0-or-later
// Included within FC4Inplace's private namespace, reusing capture and the bridge.
// Different owners use explicit native byte layouts, never the round-2 Packet.
extern "C" uint32_t fc4_inplace3_language(uint32_t, uint32_t);
extern "C" uint32_t fc4_inplace3_lighting(const void*,uint32_t,size_t,const void*,size_t,void*,size_t,const void*);

void Round3Dispatch(fc4::inplace::Registers* registers, fc4::inplace::MachineState* machine) {
    const unsigned index=registers->index, op=index-20;
    auto& fn=functions[index];auto& count=fixture?fn.fixture:fn.natural;
    auto* args=reinterpret_cast<uint32_t*>(registers+1);
    const unsigned bytes=op==6?24:((op==5||op==9)?0:4);
    const auto native=[&] { fc4_inplace_invoke(registers,machine,fn.original,args,bytes,true); };
    const bool locking=depth==0;
    if (locking) AcquireSRWLockExclusive(&sampleLock);
    if (depth) ++count.nested;
    const auto call=++count.calls;
    if (!fixture && !fc4::inplace::Sample(call)) {
        native();if (locking) ReleaseSRWLockExclusive(&sampleLock);return;
    }
    ++count.sampled;++depth;
    Capture capture;
    auto* owner=reinterpret_cast<uint8_t*>(registers->ecx);
    capture.Add(owner,op<6?4:0x12c,-1,false);
    bool domain=true;
    const char* reason="invalid-window";
    if (op>=1 && op<=4 && args[0]==0) { domain=false;reason="unpublished-current-id-offset"; }
    if (op==6) {
        const uint64_t n=args[0],ns=args[2],cs=args[4];
        // SPEC-GAP: only bounded, non-overlapping valid streams are replayed.
        domain=n>0&&n<=4096&&ns>=12&&cs>=4&&ns<=4096&&cs<=4096;
        reason="stream-domain";
        if (domain) {
            capture.Add(reinterpret_cast<void*>(args[1]),size_t((n-1)*ns+12),-1,true);
            capture.Add(reinterpret_cast<void*>(args[3]),size_t((n-1)*cs+4),0,false);
            capture.Add(reinterpret_cast<void*>(args[5]),36,-1,true);
        }
    } else if (op==7||op==8) capture.Add(reinterpret_cast<void*>(args[0]),12,0,true);
    if (!domain || !capture.Prepare(reinterpret_cast<uintptr_t>(args)+bytes)) {
        ++count.refused;
        if (count.refused<=3) fc4::Log("inplace refused name=%s source=%s call=%llu reason=%s",fn.name,fixture?"fixture":"natural",call,domain?capture.reason:reason);
        native();--depth;if (locking) ReleaseSRWLockExclusive(&sampleLock);return;
    }
    // Allocate replay storage BEFORE the observed native interval.
    std::array<std::vector<uint8_t>,6> expected;
    for (unsigned i=0;i<capture.windowCount;++i) expected[i].resize(capture.windows[i].size);
    capture.Snapshot(false);
    for (unsigned i=0;i<capture.windowCount;++i)
        std::memcpy(expected[i].data(),capture.Saved(capture.windows[i],false),expected[i].size());
    native();
    capture.Snapshot(true);
    unsigned outsideRegion=0;size_t outsideOffset=0;
    if (capture.Outside(outsideRegion,outsideOffset)) {
        ++count.outside;
        if (count.outside<=3) fc4::Log("inplace outside name=%s source=%s call=%llu region=%u offset=%zu watched=%zu attribution=unknown",fn.name,fixture?"fixture":"natural",call,outsideRegion,outsideOffset,capture.total);
    } else {
        // Replay into isolated copies; no original input/receiver is restored or written.
        uint32_t returned=0;
        bool scalar=op<6||op==9;
        if (op<6) returned=fc4_inplace3_language(op,op==5?0:args[0]);
        else returned=fc4_inplace3_lighting(expected[0].data(),op-6,
            op==6?args[0]:0,op==6?expected[1].data():nullptr,op==6?args[2]:0,
            op==6?expected[2].data():(op==9?nullptr:expected[1].data()),
            op==6?args[4]:0,op==6?expected[3].data():nullptr);
        const uint32_t nativeReturn=op==9?(registers->eax&0xff):registers->eax;
        bool equal=true,floatOnly=true;unsigned firstWindow=0;size_t firstByte=0;
        const auto compare=[&](bool measure) {
            equal=true;floatOnly=true;
            for (unsigned i=0;i<capture.windowCount;++i) {
                const auto& w=capture.windows[i];const auto* actual=capture.Saved(w,true);
                for (size_t j=0;j<w.size;++j) if (actual[j]!=expected[i][j]) {
                    if (equal) { firstWindow=i;firstByte=j; }equal=false;
                    if (!w.floating||w.size%4) floatOnly=false;
                    else {
                        uint32_t a,b;const auto offset=j&~size_t(3);
                        std::memcpy(&a,actual+offset,4);std::memcpy(&b,expected[i].data()+offset,4);
                        const auto ulp=fc4::inplace::FloatDistance(a,b);
                        if (measure) count.maxUlp=std::max(count.maxUlp,ulp);
                        if (ulp>4) floatOnly=false;
                    }
                }
            }
            if (scalar&&nativeReturn!=returned) {
                if (equal) { firstWindow=6;firstByte=0; }equal=false;floatOnly=false;
            }
        };
        compare(true);++count.compared;
        if (equal) ++count.exact;
        else {
            ++count.different;if (floatOnly) ++count.floatOnly;
            if (count.different<=3) fc4::Log("inplace difference name=%s source=%s window=%u byte=%zu float_only=%u",fn.name,fixture?"fixture":"natural",firstWindow,firstByte,floatOnly?1:0);
        }
        if (fn.mutate&&count.mutation==0) {
            ++count.mutation;
            if (scalar) returned^=1;
            else for (unsigned i=0;i<capture.windowCount;++i) if (capture.windows[i].output>=0) { expected[i][0]^=1;break; }
            compare(false);count.detected+=!equal;
            fc4::Log("inplace mutation name=%s source=%s window=%u byte=%zu xor=1 detected=%llu",fn.name,fixture?"fixture":"natural",firstWindow,firstByte,count.detected);
        }
        // The original's live results and saved registers were never overwritten.
    }
    --depth;if (locking) ReleaseSRWLockExclusive(&sampleLock);
}

void Round3Fixture(unsigned i,unsigned sample,uint32_t* words) {
    const auto va=functions[i].va;
    if (sample==0) fc4::Log("inplace fixture begin name=%s",functions[i].name);
    if (i<26) {
        const uint32_t value=i==20?((sample<<10)|9):(sample+1);
        if (i==25) reinterpret_cast<uint32_t(__thiscall*)(void*)>(va)(words);
        else reinterpret_cast<uint32_t(__thiscall*)(void*,uint32_t)>(va)(words,value);
        return;
    }
    const auto bits=[](float value) { uint32_t result;std::memcpy(&result,&value,4);return result; };
    words[0x24/4]=sample&1;
    for (unsigned j=0;j<3;++j) { words[(0x64/4)+j]=bits(float(j)-1);words[(0x8c/4)+j]=bits(float(j+1)/4); }
    for (unsigned j=0;j<18;++j) words[(0xe4/4)+j]=bits(float((j+sample)%5)/8);
    auto* normals=words+0x600/4;auto* output=words+0x800/4;auto* matrix=words+0xa00/4;
    for (unsigned j=0;j<9;++j) matrix[j]=bits(j%4==0?1.f:0.f);
    for (unsigned j=0;j<12;++j) normals[j]=bits(float(int((j+sample)%3)-1)/2);
    if (i==26) reinterpret_cast<void(__thiscall*)(void*,unsigned,void*,unsigned,void*,unsigned,void*)>(va)(words,4,normals,12,output,8,matrix);
    else if (i==29) reinterpret_cast<uint32_t(__thiscall*)(void*)>(va)(words);
    else reinterpret_cast<void(__thiscall*)(void*,void*)>(va)(words,output);
}
