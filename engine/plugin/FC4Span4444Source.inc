// SPDX-License-Identifier: GPL-3.0-or-later
// Included in the 4444 bridge only. Native source acquisition follows §9.1;
// it reads texture texels, never original renderer output.
bool SourceOf(const uint32_t* spans, uint32_t count, const uint32_t* rect,
              const uint8_t* descriptor, uint32_t mode, const Region& dst,
              std::vector<uint8_t>* out) {
    uint32_t d[7];
    std::memcpy(d, descriptor, sizeof(d));
    static const unsigned sizes[] = {2,2,2,2,4,4,1};
    if (d[4] >= 7 || mode >= 12 || rect[0] == 0 || static_cast<int32_t>(rect[1]) <= 0) return false;
    const unsigned bpp = sizes[d[4]], geometry = mode / 4;
    if (geometry == 2 && (!rect[2] || !rect[3])) return false;
    std::vector<size_t> offsets;
    size_t end = 0;
    for (uint32_t s = 0; s < count; ++s) {
        const uint32_t start = spans[2*s], x = start & 0xFFFF, y = start >> 16;
        const uint32_t width = spans[2*s+1] - start;
        uint32_t u = d[2]*x + d[0];  // wrapping unsigned 16.16
        uint32_t tx = geometry == 2 ? (x+d[0]) % rect[2] : 0;
        const uint32_t row = geometry == 0 ? y : geometry == 1 ? (d[3]*y+d[1]) >> 16
                                                                             : (y+d[1]) % rect[3];
        for (uint32_t i = 0; i < width; ++i) {
            const uint64_t col = geometry == 0 ? uint64_t(x)+i : geometry == 1 ? u >> 16 : tx;
            const uint64_t off = uint64_t(row)*rect[1] + col*bpp;
            if (off+bpp > (uint64_t(64) << 20)) return false;
            const uintptr_t address = uintptr_t(rect[0]) + size_t(off);
            const uintptr_t dstBegin = reinterpret_cast<uintptr_t>(dst.lo);
            // SP7: no published alias contract. Refuse overlapping images
            // rather than silently turn a sequential self-blit into a snapshot.
            if (address < dstBegin+dst.len && address+bpp > dstBegin) return false;
            offsets.push_back(size_t(off));
            if (off+bpp > end) end = size_t(off+bpp);
            u += d[2];
            if (geometry == 2 && ++tx == rect[2]) tx = 0;
        }
    }
    if (!end) return false;
    out->assign(end, 0);
    for (size_t off : offsets)
        std::memcpy(out->data()+off, reinterpret_cast<const void*>(uintptr_t(rect[0])+off), bpp);
    return true;
}
