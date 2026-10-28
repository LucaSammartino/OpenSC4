// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
#include <cstddef>

namespace fc4 { namespace drawcapture {
constexpr uint64_t HashStart = 14695981039346656037ull;
inline uint64_t Hash(uint64_t hash, const void* data, size_t size) {
    const auto* bytes = static_cast<const uint8_t*>(data);
    for (size_t i = 0; i < size; ++i) hash = (hash ^ bytes[i]) * 1099511628211ull;
    return hash;
}
// graphics-formats.md 3.5, including the packed-stride quirk.
inline uint32_t VertexBytes(uint32_t format) {
    if (!(format & 0x80000000u)) {
        switch (format) {
        case 1: return 16; case 2: return 20; case 3: return 28;
        case 10: return 24; case 11: return 32; case 32: return 12;
        case 33: return 24; case 34: return 28; case 35: return 32;
        case 36: return 40; case 37: return 36; case 38: return 44;
        default: return 0;
        }
    }
    uint32_t size = (format & 3) * 12 + ((format >> 8) & 3) * 4 +
        ((format >> 10) & 15) * 4 + ((format >> 14) & 15) * 8;
    if (format & 15) size += ((format >> 2) & 7) * 4 + ((format >> 5) & 1) * 4 +
        ((format >> 6) & 1) * 12 + ((format >> 7) & 1) * 4 +
        ((format >> 18) & 15) * 12 + ((format >> 22) & 15) * 16;
    return size;
}
// renderer-driver.md 9.4. Row padding is excluded; only input pixel bytes
// are hashed. Compressed input consists of complete 4x4 blocks.
struct Pixels { uint64_t row = 0, stride = 0, rows = 0; bool valid = false; };
inline Pixels PixelBytes(uint32_t width, uint32_t height, uint32_t format,
                         uint32_t type, uint32_t rowLength) {
    if (width > 16384 || height > 16384 || rowLength > 16384) return {};
    if (!rowLength) rowLength = width;
    if (rowLength < width) return {};
    if (type <= 1 && (format == 7 || format == 8 || format == 9 || format == 10)) {
        const uint64_t block = format <= 8 ? 8 : 16;
        return {((width + 3ull) / 4) * block, ((rowLength + 3ull) / 4) * block,
                (height + 3ull) / 4, true};
    }
    unsigned bytes = 0;
    if (type <= 1 && format <= 3) bytes = (format == 1 || format == 3) ? 4 : 3;
    if ((type == 10 || type == 13) && (format == 1 || format == 3)) bytes = 2;
    if (type == 14 && (format == 0 || format == 2)) bytes = 2;
    if (!bytes) return {};
    return {uint64_t(width) * bytes, uint64_t(rowLength) * bytes, height, true};
}
// ABI argument dwords, NOT SDK overload declaration order. ClearDepth's
// double occupies two words; the two modulated blits share the 10/12-word ABI.
constexpr unsigned Words[83] = {
    2,0,0,3,4,3,1,2,1,3,2,1,4,2,1,1,1,1,3,1,3,2,2,1,2,9,2,3,3,3,
    2,2,3,1,2,2,2,1,3,3,1,1,0,0,1,1,1,2,2,2,0,1,1,4,2,2,2,2,2,1,
    5,10,2,0,1,2,4,1,9,11,10,12,10,12,4,0,1,0,0,0,0,0,2
};
constexpr const char* Methods[83] = {
    "QueryInterface","AddRef","Release","DrawArrays","DrawElements","InterleavedArrays",
    "MakeVertexFormat.standard","MakeVertexFormat.elements","VertexFormatStride",
    "VertexFormatElementOffset","VertexFormatNumElements","Clear","ClearColor","ClearDepth",
    "ClearStencil","ColorMask","DepthFunc","DepthMask","StencilFunc","StencilMask","StencilOp",
    "BlendFunc","AlphaFunc","ShadeModel","BindTexture","TexImage2D","PixelStore","TexEnv.vector",
    "TexEnv.scalar","TexParameter","Fog.vector","Fog.scalar","ColorMultiplier","AlphaMultiplier",
    "EnableVertexColors","GenTextures","DeleteTextures","IsTexture","PrioritizeTextures",
    "AreTexturesResident","MatrixMode","LoadMatrix","LoadIdentity","Flush","Enable","Disable",
    "IsEnabled","GetBoolean","GetInteger","GetFloat","GetError","TexStage","TexStageCoord",
    "TexStageMatrix","TexStageCombine.scale","TexStageCombine.operand","TexStageCombine.source",
    "TexStageCombine.function","SetTexture","GetTexture","CreateTexture","LoadTextureLevel",
    "SetCombiner","CountVideoModes","GetVideoModeInfo.current","GetVideoModeInfo.index",
    "SetVideoMode","PolygonOffset","BitBlt","StretchBlt","BitBltAlpha","StretchBltAlpha",
    "BitBltAlphaModulate","StretchBltAlphaModulate","SetViewport.rect","SetViewport.default",
    "GetViewport","GetDriverInfo","GetGZCLSID","Init","Shutdown","IsDeviceReady","Punt"
};
} }
