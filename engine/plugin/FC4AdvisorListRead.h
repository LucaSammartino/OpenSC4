// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include "fc4.h"
#include <algorithm>
#include <vector>

namespace fc4 {
// The C host asks for a list's size, then its items. Some reads (notably
// advisors.md section 7.8 ProposeDeals) have side effects, so take one
// snapshot per pair. Only numeric fields are retained, never borrowed objects.
class AdvisorListRead {
public:
    template <class Collect>
    int64_t Read(uint32_t kind, uint32_t a, fc4_adv_item* out, size_t capacity, Collect collect) {
        if (out == nullptr) {
            pending_ = false;
            items_.clear();
            if (!collect(items_)) return -1;
            kind_ = kind;
            a_ = a;
            pending_ = true;
            return static_cast<int64_t>(items_.size());
        }
        if (!pending_ || kind != kind_ || a != a_ || capacity < items_.size()) return -1;
        std::copy(items_.begin(), items_.end(), out);
        pending_ = false;
        return static_cast<int64_t>(items_.size());
    }

private:
    std::vector<fc4_adv_item> items_;
    uint32_t kind_ = 0, a_ = 0;
    bool pending_ = false;
};
}  // namespace fc4
