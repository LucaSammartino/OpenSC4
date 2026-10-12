// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

#include "FC4CoreProps.h"

#include "FC4Properties.h"

#include <vector>

namespace fc4::props {

fc4_props* ToCore(const cISCPropertyHolder* holder, const Spec* specs, size_t count) {
    if (holder == nullptr) return nullptr;
    fc4_props* p = fc4_props_create();
    if (p == nullptr) return nullptr;
    for (size_t i = 0; i < count; ++i) {
        const uint32_t id = specs[i].id;
        switch (specs[i].kind) {
            case Kind::U32: {
                std::vector<uint32_t> v;
                if (GetU32s(holder, id, v)) fc4_props_set_u32s(p, id, v.data(), v.size());
                break;
            }
            case Kind::I64: {
                std::vector<int64_t> v;
                if (GetI64s(holder, id, v)) fc4_props_set_i64s(p, id, v.data(), v.size());
                break;
            }
            case Kind::F32: {
                std::vector<float> v;
                if (GetF32s(holder, id, v)) fc4_props_set_f32s(p, id, v.data(), v.size());
                break;
            }
        }
    }
    return p;
}

}  // namespace fc4::props
