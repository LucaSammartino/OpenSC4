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

// Copies the properties a core subsystem reads from a game property holder
// (an exemplar) into an fc4_props for the Rust core.
#pragma once

#include "fc4.h"

#include <cstddef>
#include <cstdint>

class cISCPropertyHolder;

namespace fc4::props {

enum class Kind { U32, I64, F32 };

struct Spec {
    uint32_t id;
    Kind kind;
};

// A new fc4_props with every property of `specs` that `holder` has (read
// through the numeric helpers of FC4Properties.h). nullptr if `holder` is
// nullptr. The caller destroys it (fc4_props_destroy) or uses CoreProps.
fc4_props* ToCore(const cISCPropertyHolder* holder, const Spec* specs, size_t count);

// Owns an fc4_props.
class CoreProps {
public:
    explicit CoreProps(fc4_props* p = nullptr) : p_(p) {}
    ~CoreProps() { fc4_props_destroy(p_); }
    CoreProps(const CoreProps&) = delete;
    CoreProps& operator=(const CoreProps&) = delete;
    const fc4_props* Get() const { return p_; }
    fc4_props* Get() { return p_; }

private:
    fc4_props* p_;
};

}  // namespace fc4::props
