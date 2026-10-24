// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4PropLotCompare.h"
#include <cstdio>

int main() {
    unsigned failures = 0;
    auto check = [&](bool ok) { if (!ok) ++failures; };
    unsigned char native[0x1C]{}, ours[0x1C]{};
    // PL23-P and PL23-Z: wealth never makes padding deterministic.
    for (unsigned char wealth : {0, 2}) {
        native[0x14] = ours[0x14] = wealth;
        native[0x15] = 0x11; native[0x16] = 0x22; native[0x17] = 0x33;
        auto c = fc4::proplot::Profile(native, ours, true);
        check(!c.rawEqual && c.definedEqual && c.fixups == 1);
        check(!fc4::proplot::Profile(native, ours, false).definedEqual);
        native[0x18] = 1;
        check(!fc4::proplot::Profile(native, ours, true).definedEqual);
        native[0x18] = 0;
    }
    fc4_lotdev_cfg_object a{}, b{};
    std::vector<uint32_t> empty;
    a.object_type = b.object_type = 3;
    a.foundation = b.foundation = 0x890B7314;
    a.tag = 0x11223344;
    auto c = fc4::proplot::Object(a, empty, b, empty, true);
    check(!c.rawEqual && c.definedEqual && c.fixups == 1); // PL23-F
    check(!fc4::proplot::Object(a, empty, b, empty, false).definedEqual);
    b.foundation++;
    check(!fc4::proplot::Object(a, empty, b, empty, true).definedEqual); // PL23-D
    b.foundation = a.foundation;
    b.offset[0] = 1;
    check(!fc4::proplot::Object(a, empty, b, empty, true).definedEqual);
    b.offset[0] = 0;
    check(!fc4::proplot::Object(a, {7}, b, {7}, true).definedEqual);
    a.object_type = b.object_type = 1;
    check(!fc4::proplot::Object(a, empty, b, empty, true).definedEqual);
    std::printf("PL23 comparison failures=%u\n", failures);
    return failures ? 1 : 0;
}
