// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include <cstdint>
#include <cstring>

// network-occupants.md 4.6.3: caller-owned STLport vector, allocator B.
// The injected allocator also permits checking growth/ownership without a game.
namespace fc4::netpole {
struct Cell { int32_t x, z; };
struct CellVector {
    Cell* begin = nullptr;
    Cell* end = nullptr;
    Cell* capacity = nullptr;

    template<class Allocate, class Free>
    bool Assign(const Cell* cells, uint32_t count, Allocate allocate, Free release) {
        end = begin;
        for (uint32_t i = 0; i < count; ++i) {
            if (end == capacity) {
                const auto used = begin ? static_cast<uint32_t>(end - begin) : 0u;
                const auto next = used + (used ? used : 1u);
                auto* replacement = static_cast<Cell*>(allocate(next * sizeof(Cell)));
                if (!replacement) return false;
                if (used) std::memcpy(replacement, begin, used * sizeof(Cell));
                if (begin) release(begin);
                begin = replacement;
                end = begin + used;
                capacity = begin + next;
            }
            *end++ = cells[i];
        }
        return true;
    }
};
static_assert(sizeof(Cell) == 8, "native cell stride");
static_assert(sizeof(CellVector) == 3 * sizeof(void*), "native vector header");
}
