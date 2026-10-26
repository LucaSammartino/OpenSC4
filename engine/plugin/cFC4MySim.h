// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

// The MySim replacement (`enable = mysims`, mysims.md §1-§7): the MySims
// (cSC4MySim 0x4A1DBBBF, cSC4MyStreetSim 0x0BF606A5) and the MySim agent run
// inside the FC4 advisor system's core (cFC4AdvisorSystem, `enable =
// advisors`), which is created with the fc4_mysim_host this file fills. Each
// MySim's dispatch is a cFC4MySimDispatch (0xCBC14674), created through
// GZCOM by the "create" dispatch command and driven by the other commands.
//
// The MySims' GZCOM fronts are the advisor fronts (cFC4Advisor, §12.2 of
// advisors.md), with a cISC4MySim (§7.2) interface for MySim classes.
#pragma once

#include "fc4.h"

#include <cstdint>
#include <string>

class cIGZUnknown;
class cFC4Advisor;

constexpr uint32_t kFC4_CLSID_cSC4MySim = 0x4A1DBBBF;
constexpr uint32_t kFC4_CLSID_cSC4MyStreetSim = 0x0BF606A5;

namespace fc4 {

// The live FC4 advisor system's core (cFC4AdvisorSystem.cpp), null outside
// Init..Shutdown.
fc4_advisors* CurrentAdvisorCore();

// Fills `h` with the replacement's callbacks (city services, dispatch,
// messages, references); `user` is unused.
void FillMySimReplacementHost(fc4_mysim_host* h);

// The advisor system's Init / Shutdown: the agent's start, and the release of
// the dispatches left over after the core's shutdown.
void MySimReplacementStart(fc4_advisors* core);
void MySimReplacementStop(fc4_advisors* core);

// The number of live dispatches (tests, logs).
size_t MySimReplacementDispatchCount();

// Object-created callback: both advisor and MySim interfaces share one
// core handle, identity and reference count.
uint64_t CreateMySimFront(fc4_adv_obj* handle);
void FillMySimLuaHost(fc4_mysim_host* host);
bool ActivateMySimLua(int32_t id);
std::string MySimScriptAction(bool moveIn);

}  // namespace fc4

// GZCOM factories: a new MySim of the live advisor system's class store
// (its front, refcount 0), null without a live FC4 advisor system.
cIGZUnknown* CreateFC4MySim();
cIGZUnknown* CreateFC4MyStreetSim();
