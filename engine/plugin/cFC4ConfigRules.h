// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
#include "cIGZUnknown.h"
class cIGZString;
class cISC4RenderProperties;
class cFC4ISC4ConfigRules : public cIGZUnknown {
public:
    virtual void Init(cISC4RenderProperties*) = 0;
    virtual void SetOptionsFromConfiguration() = 0;
    virtual bool LogConfigurationToFile(const cIGZString&) = 0;
    virtual void ApplyOptionGroupSetting(uint32_t, uint32_t) = 0;
    virtual uint32_t GetOptionGroupAutoSetting(uint32_t) = 0;
};
cIGZUnknown* CreateFC4ConfigRules();
