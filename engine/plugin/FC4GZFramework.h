// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
class cIGZUnknown;
class cIGZFrameWork;
namespace fc4 {
bool GZFrameworkScriptProbe();
cIGZUnknown* CreateGZBuffer();
void GZFrameworkStart(cIGZFrameWork*);
void GZFrameworkStop();
cIGZUnknown* GZCommandServerTestCreate(cIGZUnknown* helpers);
}
