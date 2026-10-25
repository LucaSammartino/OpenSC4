// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4CompareData.h"
#include <cassert>
int main() {
    using namespace fc4::compare;
    const auto specs=Parse("this+0/16/r/4:u32:1,8:image:1;arg1+0/span0,2,12/r/0:f32!:3@2",3);
    uint32_t args[16]={2,0,16};
    assert(specs[1].Size(args)==28);
    uint32_t input[4]={0xfeedface,7,0x401234,0xdeadbeef};
    std::vector<uint8_t> clean(16);
    assert(Sanitize(specs[0],args,reinterpret_cast<uint8_t*>(input),clean,0x400000,0x7c8000));
    uint32_t output[4];std::memcpy(output,clean.data(),16);
    assert(output[0]==0&&output[1]==7&&output[2]==0x1234&&output[3]==0);
    input[2]=0x10001234;
    assert(!Sanitize(specs[0],args,reinterpret_cast<uint8_t*>(input),clean,0x400000,0x7c8000));
    unsigned fields=0;
    assert(Fields(specs[1],args,28,[&](const auto&,size_t at){assert(at<12||at>=16);++fields;}));
    assert(fields==6); // stride gap is never persisted or perturbed
    args[0]=0;assert(specs[1].Size(args)==0);
    bool refused=false;try { Parse("arg0+0/8/r/0:ptr:1",1); } catch (const std::exception&) {refused=true;}
    assert(refused);
}
