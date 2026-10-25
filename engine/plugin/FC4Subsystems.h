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

// The replaced subsystems and their GZCOM classes. To replace another class,
// write its factory and add one line to the table in FC4Subsystems.cpp.
//
// Each subsystem can be switched off with the environment variable
// FC4_DISABLE (a comma- or space-separated list of keys, case-insensitive, or
// "all"), e.g. FC4_DISABLE=budget,simulator. The original game classes are
// then used. FC4_DISABLE_SIMULATOR=1 still works for the simulator.
//
// Subsystems that are off by default (landvalue, weather, ordinance, demand) are switched on with
// FreeCity4.ini `enable = landvalue` (or FC4_ENABLE); `disable` still wins.
// A default-off subsystem that is not enabled is not listed as disabled.
#pragma once

#include <cstddef>
#include <cstdint>

class cIGZUnknown;

namespace fc4 {

// Creates a new object with reference count 0 (the director's
// QueryInterface takes the first reference).
using ObjectFactory = cIGZUnknown* (*)();

struct ClassEntry {
    uint32_t clsid;
    const char* name;  // the original class name, for logs
    ObjectFactory create;
};

struct Subsystem {
    const char* key;               // for FC4_DISABLE
    const char* legacyDisableEnv;  // e.g. FC4_DISABLE_SIMULATOR (=1), or nullptr
    const ClassEntry* classes;
    size_t classCount;
    bool defaultOn;  // false: only with `enable = <key>`
};

const Subsystem* Subsystems(size_t* count);

// Whether the subsystem is enabled in this process (read once, logged).
bool SubsystemEnabled(const Subsystem& s);

// The enabled class entry for `clsid`, or nullptr.
const ClassEntry* FindEnabledClass(uint32_t clsid, const Subsystem** owner = nullptr);

// The disabled subsystems, comma-separated, or "-" (static buffer, any thread).
const char* DisabledSubsystemsList();
// Our simulator class is registered (replacement mode).
bool SimulatorReplaced();
// `enable = mysims` (with `advisors`): the MySim classes are ours (cFC4MySim.h).
bool MySimsReplaced();

}  // namespace fc4
