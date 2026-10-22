#include "../plugin/FC4MySimsMessage.h"
#include <cstdint>
#include <cstdlib>
#include <iostream>

int main() {
    // An unused void1 may be a dangling/non-COM pointer. Neither reading nor
    // canonicalizing that field is part of these message contracts.
    for (std::uint32_t type : {0x66956814u, 0xCA5501F2u, 0x26D31EC1u,
                              0x8A09FA6Cu, 0x4BC42C48u, 0xC2C09554u}) {
        auto poison_read = []() -> void* { std::abort(); };
        auto poison_query = [](void*) -> std::uint64_t { std::abort(); };
        if (fc4::MySimsOccupantMessageToken(type, poison_read, poison_query) != 0) return 1;
    }
    for (std::uint32_t type : {0x99EF1142u, 0x99EF1145u, 0x99EF1143u}) {
        int reads = 0, queries = 0;
        void* p = reinterpret_cast<void*>(std::uintptr_t{0x1234});
        auto read = [&]() -> void* { ++reads; return p; };
        auto query = [&](void* value) -> std::uint64_t {
            ++queries;
            return value == p ? 0x10203040u : 0u;
        };
        if (fc4::MySimsOccupantMessageToken(type, read, query) != 0x10203040u ||
            reads != 1 || queries != 1) return 2;
        auto no_payload = []() -> void* { return nullptr; };
        auto poison_query = [](void*) -> std::uint64_t { std::abort(); };
        if (fc4::MySimsOccupantMessageToken(type, no_payload, poison_query) != 0) return 3;
    }
    std::cout << "MySim message payload boundary passed\n";
    return 0;
}
