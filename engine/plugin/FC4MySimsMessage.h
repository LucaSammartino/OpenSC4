#pragma once
#include <cstdint>

namespace fc4 {
// mysims.md section 3.2: only these message families carry an occupant in
// void1. Other messages can leave that untyped field stale or non-COM.
template<class ReadPayload, class Canonicalize>
std::uint64_t MySimsOccupantMessageToken(std::uint32_t type, ReadPayload read,
                                        Canonicalize canonicalize) {
    switch (type) {
    case 0x99EF1142: // occupant inserted
    case 0x99EF1145: // occupant changed
    case 0x99EF1143: // occupant removed
        if (auto* p = read()) return canonicalize(p);
        break;
    default:
        break;
    }
    return 0;
}
} // namespace fc4
