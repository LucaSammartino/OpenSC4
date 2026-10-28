// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4DrawCaptureLogic.h"
#include <cassert>
#include <cstring>
using namespace fc4::drawcapture;
int main() {
    assert(Hash(HashStart, "", 0) == HashStart);
    assert(Hash(HashStart, "hello", 5) == 0xa430d84680aabd0bull);
    assert(Hash(Hash(HashStart, "he", 2), "llo", 3) == Hash(HashStart, "hello", 5));
    assert(VertexBytes(2) == 20 && VertexBytes(38) == 44 && VertexBytes(0) == 0);
    assert(VertexBytes(0x80004001) == 20 && VertexBytes(0x80004101) == 24);
    assert(VertexBytes(0x80000040) == 0); // position-less packed-format quirk
    auto p = PixelBytes(3, 2, 2, 0, 4);
    assert(p.valid && p.row == 9 && p.stride == 12 && p.rows == 2);
    p = PixelBytes(5, 7, 7, 1, 0);
    assert(p.valid && p.row == 16 && p.rows == 2); // complete DXT1 blocks
    p = PixelBytes(5, 7, 10, 1, 0);
    assert(p.valid && p.row == 32 && p.rows == 2); // DXT5
    p = PixelBytes(7, 3, 3, 13, 0);
    assert(p.valid && p.row == 14 && p.rows == 3);
    assert(!PixelBytes(7, 3, 3, 13, 6).valid);
    assert(!PixelBytes(7, 3, 3, 14, 0).valid);
    assert(!PixelBytes(0xffffffff, 3, 3, 0, 0).valid);
    assert(Words[13] == 2 && Words[25] == 9 && Words[61] == 10);
    assert(Words[72] == 10 && Words[73] == 12 && Words[74] == 4 && Words[75] == 0);
    assert(std::strcmp(Methods[27], "TexEnv.vector") == 0);
}
