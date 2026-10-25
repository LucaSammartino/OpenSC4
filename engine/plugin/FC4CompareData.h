// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <algorithm>
#include <cstdint>
#include <cstring>
#include <string>
#include <vector>
#include <limits>
#include <stdexcept>

namespace fc4::compare {
struct Buffer { uint8_t* data; uint32_t size, base, offset; };
struct Result { uint32_t eax=0, edx=0, floatBits=0; };
struct Field { uint32_t offset=0, width=4, count=0, stride=0; std::string type; bool dense=false; };
struct WindowSpec {
    uint32_t base=0, offset=0, fixed=0, countArg=0, strideArg=0, tail=0;
    bool span=false, write=false, read=false;
    std::vector<Field> fields;
    uint32_t Size(const uint32_t* args) const {
        if (!span) return fixed;
        const uint64_t count=args[countArg], stride=args[strideArg];
        if (!count || count>4096 || stride<tail || stride>4096) return 0;
        const uint64_t size=(count-1)*stride+tail;
        return size<=16*1024*1024 ? uint32_t(size) : 0;
    }
};
inline std::vector<std::string> Split(const std::string& text,char delimiter) {
    std::vector<std::string> result; size_t at=0;
    do { const auto end=text.find(delimiter,at);result.push_back(text.substr(at,end-at));
        if (end==std::string::npos) break;at=end+1;
    } while (true);
    return result;
}
inline uint32_t Number(const std::string& text) {
    size_t end=0;const auto value=std::stoul(text,&end,10);
    if (end!=text.size() || value>std::numeric_limits<uint32_t>::max()) throw std::invalid_argument("number");
    return uint32_t(value);
}
inline std::vector<WindowSpec> Parse(const char* text, unsigned argCount) {
    std::vector<WindowSpec> specs;
    if (std::string(text)=="-") return specs; // Argument/return-only functions.
    for (const auto& item:Split(text,';')) {
        const auto parts=Split(item,'/');if (parts.size()!=4) throw std::invalid_argument("window");
        WindowSpec w;
        const auto source=Split(parts[0],'+');if (source.size()!=2) throw std::invalid_argument("base");
        w.base=source[0]=="this"?0:1+Number(source[0].substr(3));
        if ((source[0]!="this"&&source[0].substr(0,3)!="arg") || w.base>argCount) throw std::invalid_argument("argument");
        w.offset=Number(source[1]);if (w.offset>65536) throw std::invalid_argument("offset");
        if (parts[1].substr(0,4)=="span") {
            const auto span=Split(parts[1].substr(4),',');if (span.size()!=3) throw std::invalid_argument("span");
            w.span=true;w.countArg=Number(span[0]);w.strideArg=Number(span[1]);w.tail=Number(span[2]);
            if (w.countArg>=argCount||w.strideArg>=argCount||!w.tail) throw std::invalid_argument("span arguments");
        } else { w.fixed=Number(parts[1]);if (!w.fixed||w.fixed>16*1024*1024) throw std::invalid_argument("size"); }
        w.write=parts[2]=="w"||parts[2]=="rw";w.read=parts[2]=="r"||parts[2]=="rw";
        if (!w.write&&!w.read) throw std::invalid_argument("access");
        if (parts[3]!="-") for (const auto& descriptor:Split(parts[3],',')) {
            const auto f=Split(descriptor,':');if (f.size()!=3) throw std::invalid_argument("field");
            Field field;field.offset=Number(f[0]);field.type=f[1];
            if (!field.type.empty()&&field.type.back()=='!') { field.dense=true;field.type.pop_back(); }
            if (field.type!="u8"&&field.type!="u32"&&field.type!="i32"&&field.type!="flags"&&field.type!="f32"&&field.type!="image") throw std::invalid_argument("field type");
            field.width=field.type=="u8"?1:4;
            const auto repeated=Split(f[2],'@');field.count=repeated[0]=="all"?0:Number(repeated[0]);
            if (repeated.size()>1) { if (!w.span) throw std::invalid_argument("stride");field.stride=1+Number(repeated[1]);if (field.stride>argCount) throw std::invalid_argument("stride argument"); }
            w.fields.push_back(field);
        }
        // One contiguous window per base keeps relocation unambiguous.
        for (const auto& other:specs) if (other.base==w.base) throw std::invalid_argument("duplicate base");
        specs.push_back(std::move(w));
    }
    if (specs.empty()||specs.size()>16) throw std::invalid_argument("window count");
    return specs;
}
template<class Visit> bool Fields(const WindowSpec& w,const uint32_t* args,size_t size,Visit visit) {
    for (const auto& field:w.fields) {
        const uint32_t repeats=field.stride?args[w.countArg]:1;
        const uint32_t stride=field.stride?args[field.stride-1]:0;
        const uint32_t count=field.count?field.count:uint32_t(size/field.width);
        for (uint32_t r=0;r<repeats;++r) for (uint32_t n=0;n<count;++n) {
            const uint64_t offset=uint64_t(field.offset)+uint64_t(r)*stride+uint64_t(n)*field.width;
            if (offset+field.width>size) return false;
            visit(field,size_t(offset));
        }
    }
    return true;
}
// Values only: unspecified bytes (including receiver pointers) are zeroed.
// Image pointers are persisted as RVAs and relocated only on input replay.
inline bool Sanitize(const WindowSpec& spec,const uint32_t* args,const uint8_t* input,
                     std::vector<uint8_t>& output,uint32_t imageBase,uint32_t imageSize) {
    std::fill(output.begin(),output.end(),0);bool safe=true;
    const bool fields=Fields(spec,args,output.size(),[&](const Field& field,size_t at) {
        uint32_t value=0;std::memcpy(&value,input+at,field.width);
        if (field.type=="image"&&value) {
            if (value<imageBase||value-imageBase>=imageSize) { safe=false;return; }
            value-=imageBase;
        }
        std::memcpy(output.data()+at,&value,field.width);
    });
    return fields&&safe;
}
}
