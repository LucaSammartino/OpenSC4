// SPDX-License-Identifier: GPL-3.0-or-later
#include "FC4AdvisorListRead.h"
#include <cstdio>

int main() {
    fc4::AdvisorListRead read;
    int calls = 0, failures = 0;
    const auto check = [&](bool ok) { if (!ok) ++failures; };
    const auto proposals = [&](std::vector<fc4_adv_item>& items) {
        ++calls;  // stand-in for the proposal generator's random draws
        items.resize(2);
        items[0].kind = 0x4773FFE2;
        items[1].kind = 0x4773FFE0;
        return true;
    };
    fc4_adv_item out[2]{};
    check(read.Read(7, 0, out, 2, proposals) == -1 && calls == 0);
    check(read.Read(7, 0, nullptr, 0, proposals) == 2 && calls == 1);
    check(read.Read(6, 0, out, 2, proposals) == -1 && calls == 1);
    check(read.Read(7, 1, out, 2, proposals) == -1 && calls == 1);
    check(read.Read(7, 0, out, 1, proposals) == -1 && calls == 1);
    check(read.Read(7, 0, out, 2, proposals) == 2 && calls == 1);
    check(out[0].kind == 0x4773FFE2 && out[1].kind == 0x4773FFE0);
    check(read.Read(7, 0, out, 2, proposals) == -1 && calls == 1);
    check(read.Read(7, 0, nullptr, 0, proposals) == 2 && calls == 2);
    const auto empty = [&](std::vector<fc4_adv_item>&) { ++calls; return true; };
    check(read.Read(6, 5, nullptr, 0, empty) == 0 && calls == 3);
    check(read.Read(6, 5, out, 0, empty) == 0 && calls == 3);
    const auto missing = [](std::vector<fc4_adv_item>&) { return false; };
    check(read.Read(7, 0, nullptr, 0, missing) == -1);
    check(read.Read(7, 0, out, 2, proposals) == -1 && calls == 3);
    if (failures) std::printf("%d advisor list read failures\n", failures);
    return failures != 0;
}
