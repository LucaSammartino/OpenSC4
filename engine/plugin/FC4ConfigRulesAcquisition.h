// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
#include <initializer_list>

namespace fc4::cfgrules {
struct Vertex { float x, y, z; uint32_t color; };
static_assert(sizeof(Vertex) == 16);

// graphics-rules 2.3: all target/state operations belong to the host. Keep the
// ordering here testable without hardware or a second implementation of rules.
template<class Host> bool Probe(Host& h, bool hardware, bool& result) {
    result = false;
    if (!hardware) return true;
    if (!h.Begin()) return false;
    Vertex vertices[256]{};
    h.ClearColor();
    for (int iteration = 0; iteration < 20; ++iteration) {
        for (int row = 0; row < 8; ++row) for (int col = 0; col < 8; ++col) {
            const float x = (col * 8.125f + 4.f) - .0625f;
            const float y = (row * 8.125f + 4.f) - .0625f;
            auto* v = vertices + (row * 8 + col) * 4;
            v[0] = {x, y, 0.f, UINT32_MAX};
            v[1] = {x, y + .125f, 0.f, UINT32_MAX};
            v[2] = {x + .125f, y + .125f, 0.f, UINT32_MAX};
            v[3] = {x + .125f, y, 0.f, UINT32_MAX};
        }
        h.Clear();
        for (uint32_t cap : {4u, 3u, 0u, 1u, 5u}) h.Disable(cap);
        h.Arrays(vertices);
        h.Draw();
        if (iteration == 19 && !h.Copy()) {
            h.Flush(); h.Clear(); h.Flush(); h.End();
            return false;
        }
        h.Flush();
    }
    h.Clear(); h.Flush(); h.End();
    unsigned lit = 0;
    if (h.HasBuffer()) for (int y = 4; y <= 60; y += 8)
        for (int x = 4; x <= 60; x += 8)
            if ((h.Pixel(x, y) & 0x00ffffff) != 0) ++lit;
    result = lit > 1;
    return true;
}

template<class Host> bool BackingStore(Host& h) {
    const bool saved = h.Get();
    h.Set(true);
    const bool present = h.Get();
    h.Set(saved);
    return present;
}
}
