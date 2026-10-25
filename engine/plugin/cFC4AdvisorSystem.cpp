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

// cFC4AdvisorSystem (cFC4AdvisorSystem.h): docs/specs/advisors.md §4, §5,
// §8.2, §12.1 and lua-host.md §1.3, §5.1, §8.2 over the Rust core.

#include "cFC4AdvisorSystem.h"
#include "FC4AdvisorMessage.h"

#include "FC4Log.h"
#include "FC4AdvisorApi.h"
#include "FC4RefAudit.h"
#include "cFC4AdvisorObjects.h"
#include "FC4Interfaces.h"
#include "FC4PropCityCommon.h"
#include "FC4Streams.h"
#include "cFC4AdviceSubject.h"
#include "cFC4MySim.h"
#include "FC4MySimGameHost.h"
#include "FC4Subsystems.h"
#include "cFC4Date.h"
#include "cFC4Message2Standard.h"
#include "compat/FC4Compat.h"

#include "GZServPtrs.h"
#include "cGZMessage.h"
#include "cIGZApp.h"
#include "cIGZCheatCodeManager.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZString.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4View3DWin.h"
#include "cISCLua.h"
#include "cRZBaseString.h"

#include <windows.h>

#include <array>
#include <cctype>
#include <cstdio>
#include <cstring>
#include <ctime>
#include <string>
#include <utility>
#include <vector>

namespace fc4 {
bool GameAdvisorWinProcOps(AdvisorWinProcOps* out);  // FC4AdvisorWindowGame.cpp
}

namespace {

constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_cIGZMessageTarget = 0xE98B2F57;
constexpr uint32_t kIID_cIGZCheatCodeManager = 0xA1085722;
constexpr uint32_t kIID_cISCLua = 0xEA10C4A2;  // lua-host.md §1.3

cFC4AdvisorSystem* Self(void* u) { return static_cast<cFC4AdvisorSystem*>(u); }

// The one live system the Lua trampolines reach (§14.2: the game keeps one in
// 0x00B207C0 too). Set in Init, cleared in Shutdown.
cFC4AdvisorSystem* g_live = nullptr;
std::map<int32_t, int32_t> g_mysimTables;

cISC4App* App() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app != nullptr && app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) && sc4 != nullptr) {
        FC4_ACQUIRED(sc4, "QueryInterface");
        return sc4;  // the caller releases it
    }
    return nullptr;
}

// ---- Lua table reads (§2.1, §2.2) ------------------------------------------------------------------------

// Field `name` of the table at the stack top as a number (missing / not a number = 0).
double FieldNumber(cISCLua* L, const char* name) {
    L->PushString(name);
    L->GetTable(-2);
    // [abi-ret] eax-or-void (interfaces.tsv cISCLua 14): the value is used as a bool
    const double v = L->IsNumber(-1) ? L->ToNumber(-1) : 0.0;
    L->Pop(1);
    return v;
}

bool FieldString(cISCLua* L, const char* name, std::string* out) {
    L->PushString(name);
    L->GetTable(-2);
    // [abi-ret] eax-or-void (interfaces.tsv cISCLua 15): the value is used as a bool
    const bool ok = L->IsString(-1);
    if (ok) out->assign(L->ToString(-1), L->Strlen(-1));
    else out->clear();
    L->Pop(1);
    return ok;
}

// Pushes global[table][i]; false (nothing pushed) when it is not a table.
bool PushEntry(cISCLua* L, const char* table, uint32_t i) {
    L->GetGlobal(table);
    // [abi-ret] eax-or-void (interfaces.tsv cISCLua 72): the value is used as a bool
    if (!L->IsTable(-1)) {
        L->Pop(1);
        return false;
    }
    L->RawGetI(-1, static_cast<int32_t>(i));
    L->Remove(-2);
    // [abi-ret] eax-or-void (interfaces.tsv cISCLua 72): the value is used as a bool
    if (!L->IsTable(-1)) {
        L->Pop(1);
        return false;
    }
    return true;
}

struct EntryText {
    std::string caption, name, trigger;
    std::vector<uint32_t> effects;
};
EntryText g_text;  // the strings handed out stay valid until the callback returns (fc4.h)

bool AdvisorCount(void* u, uint32_t* n) {
    cISCLua* L = Self(u)->Lua();
    if (L == nullptr) return false;
    L->GetGlobal("advisors");
    // [abi-ret] eax-or-void (interfaces.tsv cISCLua 72): the value is used as a bool
    const bool table = L->IsTable(-1);
    *n = table ? static_cast<uint32_t>(L->GetN(-1)) : 0;
    L->Pop(1);
    return table;
}

bool AdvisorEntry(void* u, uint32_t i, fc4_adv_advisor_entry* out) {
    cISCLua* L = Self(u)->Lua();
    if (L == nullptr || !PushEntry(L, "advisors", i)) return false;
    out->class_id = static_cast<uint32_t>(static_cast<int64_t>(FieldNumber(L, "class_id")));
    out->id = static_cast<int32_t>(FieldNumber(L, "id"));
    out->advice_type = static_cast<int32_t>(FieldNumber(L, "advice_type"));
    FieldString(L, "caption", &g_text.caption);
    FieldString(L, "name", &g_text.name);
    L->Pop(1);
    out->caption = reinterpret_cast<const uint8_t*>(g_text.caption.data());
    out->caption_len = g_text.caption.size();
    out->name = reinterpret_cast<const uint8_t*>(g_text.name.data());
    out->name_len = g_text.name.size();
    return true;
}

uint32_t AdviceCount(void* u) {
    cISCLua* L = Self(u)->Lua();
    if (L == nullptr) return 0;
    L->GetGlobal("advices");
    // [abi-ret] eax-or-void (interfaces.tsv cISCLua 72): the value is used as a bool
    const uint32_t n = L->IsTable(-1) ? static_cast<uint32_t>(L->GetN(-1)) : 0;
    L->Pop(1);
    return n;
}

bool AdviceEntry(void* u, uint32_t i, fc4_adv_advice_entry* out) {
    cISCLua* L = Self(u)->Lua();
    if (L == nullptr || !PushEntry(L, "advices", i)) return false;
    auto u32 = [&](const char* f) { return static_cast<uint32_t>(static_cast<int64_t>(FieldNumber(L, f))); };
    auto i32 = [&](const char* f) { return static_cast<int32_t>(FieldNumber(L, f)); };
    out->guid = u32("guid");
    out->class_id = u32("class_id");
    out->advice_type = i32("advice_type");
    out->priority = i32("priority");
    out->mood = i32("mood");
    out->frequency = i32("frequency");
    out->timeout = i32("timeout");
    out->event = u32("event");
    out->command = u32("command");
    out->nd_type = i32("nd_type");  // §4.5: cSC4AdviceNeghborDeal::Init
    out->once = i32("once") != 0;
    out->news_only = i32("news_only") != 0;
    out->persist = i32("persist") != 0;
    out->no_timeout = i32("no_timeout") != 0;
    out->has_trigger = FieldString(L, "trigger", &g_text.trigger);
    g_text.effects.clear();
    L->PushString("effects");
    L->GetTable(-2);
    // [abi-ret] eax-or-void (interfaces.tsv cISCLua 72): the value is used as a bool
    if (L->IsTable(-1)) {
        const int32_t n = L->GetN(-1);
        for (int32_t k = 1; k <= n; ++k) {
            L->RawGetI(-1, k);
            // [abi-ret] eax-or-void (interfaces.tsv cISCLua 14): the value is used as a bool
            if (L->IsNumber(-1)) g_text.effects.push_back(static_cast<uint32_t>(static_cast<int64_t>(L->ToNumber(-1))));
            L->Pop(1);
        }
    }
    L->Pop(2);
    out->trigger = reinterpret_cast<const uint8_t*>(g_text.trigger.data());
    out->trigger_len = g_text.trigger.size();
    out->effects = g_text.effects.data();
    out->effect_count = g_text.effects.size();
    return true;
}

// §4.5 step 3: `_atrg_ = function () return (<trigger>); end`, keep a reference.
// [choice] `_atrg_` is set to nil first so that a chunk that fails to compile is
// not mistaken for the previous trigger's function (the SDK declares DoString
// void; lua-host.md §5.1: the status is in EAX, not read here).
bool CompileTrigger(void* u, const uint8_t* text, size_t len, int32_t* ref) {
    cISCLua* L = Self(u)->Lua();
    if (L == nullptr) return false;
    const int32_t top = L->GetTop();
    L->PushNil();
    L->SetGlobal("_atrg_");
    std::string chunk = "_atrg_ = function () return (";
    chunk.append(reinterpret_cast<const char*>(text), len);
    chunk += "); end";
    L->DoString(chunk.c_str());
    L->SetTop(top);
    L->GetGlobal("_atrg_");
    // [abi-ret] eax-or-void (interfaces.tsv cISCLua 71): the value is used as a bool
    if (!L->IsFunction(-1)) {
        L->SetTop(top);
        return false;
    }
    *ref = L->Ref(1);  // lua-host.md §5.1: the argument is ignored, the top is taken
    L->SetTop(top);
    return true;
}

void ReleaseTrigger(void* u, int32_t ref) {
    if (cISCLua* L = Self(u)->Lua()) L->Unref(ref);
}

void ReadResult(cISCLua* L, fc4_adv_lua_result* out) {
    *out = fc4_adv_lua_result{};
    // [abi-ret] eax-or-void (interfaces.tsv cISCLua 18): the value is used as a bool
    if (L->IsBoolean(-1)) {
        out->kind = FC4_ADV_LUA_BOOL;
        out->boolean = L->ToBoolean(-1) ? 1u : 0u;
    } else if (L->Type(-1) == cIGZLua5Thread::LuaTypeNumber) {
        out->kind = FC4_ADV_LUA_NUMBER;
        out->number = L->ToNumber(-1);
    } else if (L->Type(-1) == cIGZLua5Thread::LuaTypeString) {
        static std::string s;
        s.assign(L->ToString(-1), L->Strlen(-1));
        out->kind = FC4_ADV_LUA_STRING;
        out->str = reinterpret_cast<const uint8_t*>(s.data());
        out->len = s.size();
    }
}

void CallTrigger(void* u, int32_t ref, fc4_adv_lua_result* out) {
    *out = fc4_adv_lua_result{};
    cISCLua* L = Self(u)->Lua();
    if (L == nullptr) return;
    const int32_t top = L->GetTop();
    L->GetRef(ref);
    if (L->CallProtected(0, 1) != 0) out->kind = FC4_ADV_LUA_ERROR;
    else ReadResult(L, out);
    L->SetTop(top);
}

// §7.1: slots 12-14 write fields of the global `game` table and assume it
// exists. [choice] A missing `game` drops the write (the stack is left
// balanced) instead of raising a Lua error.
bool PushGame(cISCLua* L) {
    L->GetGlobal("game");
    // [abi-ret] eax-or-void (interfaces.tsv cISCLua 72): the value is used as a bool
    if (L->IsTable(-1)) return true;
    L->Pop(1);
    return false;
}

void SetGlobal(void* u, const char* name, double value) {
    cISCLua* L = Self(u)->Lua();
    if (L == nullptr || !PushGame(L)) return;
    L->PushString(name);
    L->PushNumber(value);
    L->SetTable(-3);
    L->Pop(1);
}

void SetGlobalString(void* u, const char* name, const uint8_t* value, size_t len) {
    cISCLua* L = Self(u)->Lua();
    if (L == nullptr || !PushGame(L)) return;
    L->PushString(name);
    L->PushLString(reinterpret_cast<const char*>(value), len);
    L->SetTable(-3);
    L->Pop(1);
}

// §8.1: the two tags (subject, string), made by create_tags at Init.
int32_t g_tags[2] = {-1, -1};

// §7.1 slot 14: game[name] = value; game[name .. "_subject"] = the subject as
// subject-tag userdata holding one reference (a null subject: null userdata).
void SetLocal(void* u, const char* name, double value, const fc4_adv_subject_state*, uint64_t token) {
    cISCLua* L = Self(u)->Lua();
    if (L == nullptr || !PushGame(L)) return;
    L->PushString(name);
    L->PushNumber(value);
    L->SetTable(-3);
    const std::string key = std::string(name) + "_subject";
    L->PushString(key.c_str());
    cIGZUnknown* subject = fc4::FromToken(token);
    if (subject != nullptr) FC4_ADDREF(subject);
    L->PushUserTag(subject, g_tags[0]);
    L->SetTable(-3);
    L->Pop(1);
}

void CollectGarbage(void* u) {
    if (cISCLua* L = Self(u)->Lua()) L->SetGCThreshold(0);  // §4.6
}

void CloseLua(void* u) { Self(u)->ReleaseLua(); }

bool DebugEnabled(void*) {
    cISC4App* app = App();
    const bool on = app != nullptr && app->GetDebugFunctionalityEnabled();
    if (app != nullptr) FC4_RELEASE(app);
    return on;
}

void RegisterCheat(void* u, uint32_t id, const char* name) { Self(u)->Cheat(id, name, true); }
void UnregisterCheat(void* u, uint32_t id) { Self(u)->Cheat(id, nullptr, false); }
void AdvisorWindow(void* u, bool start) { Self(u)->WindowCallback(start); }

bool HasCity(void*) {
    cISC4App* app = App();
    const bool has = app != nullptr && app->GetCity() != nullptr;
    if (app != nullptr) FC4_RELEASE(app);
    return has;
}

uint32_t NowMs(void*) { return static_cast<uint32_t>(GetTickCount()); }  // [choice] the wall clock in ms

// ---- city and simulator reads (§5.1, §13) ------------------------------------------------------------------

// The city the app holds (no reference: GetCity does not AddRef).
cISC4City* AppCity() {
    cISC4App* app = App();
    cISC4City* city = app != nullptr ? app->GetCity() : nullptr;
    if (app != nullptr) FC4_RELEASE(app);
    return city;
}

cFC4ISC4Simulator* CitySimulator() {
    cISC4City* city = AppCity();
    return city != nullptr ? fc4::AsGame(city->GetSimulator()) : nullptr;  // borrowed
}

bool CityEstablishedCb(void*) {
    cISC4City* city = AppCity();
    return city != nullptr && city->GetEstablished();  // cISC4City +0x58
}

bool DifficultyCb(void*, int32_t* out) {
    cISC4City* city = AppCity();
    if (city == nullptr || out == nullptr) return false;
    *out = static_cast<int32_t>(city->GetDifficultyLevel());  // +0x60
    return true;
}

void CellCountsCb(void*, uint32_t* x, uint32_t* z) {
    cISC4City* city = AppCity();
    if (x != nullptr) *x = city != nullptr ? city->CellCountX() : 0;  // +0x174
    if (z != nullptr) *z = city != nullptr ? city->CellCountZ() : 0;  // +0x178
}

// §5.1 step 2: the simulator date, cISC4Simulator +0x1C GetSimDate() (the
// simulator's own object: no reference is taken or dropped).
void SimDateCb(void*, fc4_adv_date* out) {
    if (out == nullptr) return;
    *out = fc4_adv_date{};
    cFC4ISC4Simulator* sim = CitySimulator();
    cFC4IGZDate* d = sim != nullptr ? sim->GetSimDate() : nullptr;
    if (d == nullptr) return;
    out->day_number = static_cast<int32_t>(d->DayNumber());
    out->year = static_cast<int32_t>(d->Year());
    out->month = static_cast<int32_t>(d->Month());
    out->day = static_cast<int32_t>(d->DayOfMonth());
}

// §5.1 step 2.3: the year of the simulator's start date (+0x14 GetSimStartDate
// into a local date).
int32_t StartYearCb(void*) {
    cFC4ISC4Simulator* sim = CitySimulator();
    if (sim == nullptr) return 0;
    cFC4Date start;
    // [abi-ret] via-call (interfaces.tsv cISC4Simulator 5): the status is not read; §5.1 only reads the date
    sim->GetSimStartDate(start);
    return static_cast<int32_t>(start.Year());
}

bool IsAnyPausedCb(void*) {
    cFC4ISC4Simulator* sim = CitySimulator();
    // [abi-ret] eax-or-void (interfaces.tsv cISC4Simulator 18): the value is used as a bool (AL, §13)
    return sim != nullptr && sim->IsAnyPaused();  // +0x48
}

void Deliver(const fc4_adv_out_message* m, bool sync);

// §5.1 tick_self / §10: the ticker 0xCA5501F2 is posted with no target.
void PostTickerCb(void*) {
    fc4_adv_out_message m{};
    m.msg_type = 0xCA5501F2;
    Deliver(&m, false);
}

// §6.9: View3D +0xBC AssertGlobalEffectTrigger(id) / +0xC0 Retract (the
// returned status is not read). No city view: nothing to notify.
void GlobalEffectCb(void*, uint32_t id, bool assert_) {
    cISC4View3DWin* v = fc4::propcity::View3D();
    if (v == nullptr) return;
    if (assert_) v->AssertGlobalEffectTrigger(id);
    else v->RetractGlobalEffectTrigger(id);
    fc4::propcity::Drop(reinterpret_cast<cIGZUnknown*>(v));
}

// An advisor subscribes its own front's message target (§5.4).
void AdvisorNotification(uint64_t token, uint32_t type, bool on) {
    cIGZUnknown* u = fc4::FromToken(token);
    cIGZMessageTarget2* t = nullptr;
    if (u == nullptr || !u->QueryInterface(kFC4_IID_cIGZMessageTarget2, reinterpret_cast<void**>(&t)) || t == nullptr) return;
    FC4_ACQUIRED(t, "QueryInterface");
    cIGZMessageServer2Ptr ms;
    if (ms) {
        if (on) ms->AddNotification(t, type);
        else ms->RemoveNotification(t, type);
    }
    FC4_RELEASE(t);
}
void SubscribeCb(void* u, uint32_t kind, int32_t, uint64_t token, uint32_t type) {
    if (kind == FC4_ADV_OBJ_SYSTEM) Self(u)->Subscribe(type, true);
    else if (kind == FC4_ADV_OBJ_ADVISOR) AdvisorNotification(token, type, true);
}
void UnsubscribeCb(void* u, uint32_t kind, int32_t, uint64_t token, uint32_t type) {
    if (kind == FC4_ADV_OBJ_SYSTEM) Self(u)->Subscribe(type, false);
    else if (kind == FC4_ADV_OBJ_ADVISOR) AdvisorNotification(token, type, false);
}

// §10: all object payloads use the message's IGZUnknown field, independently
// of data1/data2. SetVoid1 aliases data1 and leaves GetIGZUnknown null.
void Deliver(const fc4_adv_out_message* m, bool sync) {
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cIGZUnknown* object = m->obj_kind != 0 ? fc4::FromToken(m->obj_token) : nullptr;
    cFC4Message2Standard* msg = fc4::MakeAdvisorMessage(m->msg_type, m->data1, m->data2, object);
    if (sync) ms->MessageSend(msg);
    else ms->MessagePost(msg, false);
    msg->Release();
}
void PostCb(void*, const fc4_adv_out_message* m) { Deliver(m, false); }
void SendCb(void*, const fc4_adv_out_message* m) { Deliver(m, true); }

void AddRefCb(void*, uint64_t t) {
    if (cIGZUnknown* p = fc4::FromToken(t)) FC4_ADDREF(p);
}
void ReleaseCb(void*, uint64_t t) {
    if (cIGZUnknown* p = fc4::FromToken(t)) FC4_RELEASE(p);
}
uint64_t ResolveRefCb(void*, uint64_t t, uint32_t iid) {
    cIGZUnknown* p = fc4::FromToken(t);
    void* out = nullptr;
    if (p == nullptr || !p->QueryInterface(iid, &out) || out == nullptr) return 0;
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(out), "QueryInterface");
    return fc4::TokenOf(out);  // one reference for the core
}

fc4_adv_obj* NewSubjectHandle(void* u) {
    fc4_advisors* core = Self(u)->Core();
    return core != nullptr ? fc4_advisors_new_subject(core) : nullptr;
}

uint64_t ObjectCreated(void* u, uint32_t kind, uint32_t, fc4_adv_obj* handle) {
    if (kind == FC4_ADV_OBJ_SYSTEM) {
        fc4_adv_obj_destroy(handle);
        return fc4::TokenOf(Self(u)->Primary());
    }
    if (kind == FC4_ADV_OBJ_SUBJECT) {
        // §1.1, §12.4: the subject's GZCOM front owns the handle.
        auto* s = new cFC4AdviceSubject(handle, &NewSubjectHandle, u);
        return s->Token();
    }
    if (kind == FC4_ADV_OBJ_ADVISOR) {
        const uint32_t cls = fc4_adv_obj_class(handle);
        if (cls == kFC4_CLSID_cSC4MySim || cls == kFC4_CLSID_cSC4MyStreetSim)
            return fc4::CreateMySimFront(handle);
        return (new cFC4Advisor(handle))->Token();  // §12.2
    }
    if (kind == FC4_ADV_OBJ_ADVICE) return (new cFC4Advice(handle))->Token();    // §12.3
    fc4_adv_obj_destroy(handle);
    return 0;
}

// The core object behind a front is gone (FC4AdvLife.h).
void ObjectDroppedCb(void*, uint64_t token) { fc4::AdvFront::Dropped(token); }

// §6.9: the text rebuild reads the advice entry's title and message.
// [question AD12] the detokenizer pass (UI) is not run on them here.
void RebuildTextCb(void* u, uint32_t, uint64_t token) {
    cFC4ISC4Advice* a = FC4AdviceFront(token);
    cISCLua* L = Self(u)->Lua();
    if (a == nullptr || L == nullptr) return;
    auto* f = static_cast<cFC4Advice*>(a);
    std::string title, message;
    // [choice] the table index is the 1-based `advices` index (as advice_entry)
    if (PushEntry(L, "advices", static_cast<uint32_t>(f->TableIndex()))) {
        FieldString(L, "title", &title);
        FieldString(L, "message", &message);
        L->Pop(1);
    }
    f->SetText(title.data(), title.size(), message.data(), message.size());
}

// ---- register_api (§8.2, lua-host.md §8.2) ---------------------------------------------------------------

using fc4::advisors::ApiEntry;
using fc4::advisors::kApi;
using fc4::advisors::kApiCount;
using fc4::advisors::kCore;
using fc4::compat::Direct;

using fc4::advisors::VCall;

// §4.2 step 2 (AD10): cISC4App +0xD4 GetPluginDirectory(out, -1), then +0xDC
// GetSkuSpecificDirectory(out); `advisors\` is appended to each. AD11: the
// value is taken as it is (logged by the caller), never interpreted.
std::string AppDirectory(cISC4App* app, bool sku) {
    cRZBaseString out;
    // [abi-ret] void: the bool result is not read (advisors.md §4.2)
    if (sku) VCall<bool, cIGZString&>(app, 0xDC, out);
    else VCall<bool, cIGZString&, int32_t>(app, 0xD4, out, -1);
    return std::string(out.ToChar() ? out.ToChar() : "", out.Strlen());
}

// lua-host.md §1.3: the owning thread object sits in the 4 bytes before the state.
cISCLua* HostOf(lua_State* state) {
    cIGZUnknown* thread = *reinterpret_cast<cIGZUnknown**>(reinterpret_cast<char*>(state) - sizeof(void*));
    void* lua = nullptr;
    if (thread == nullptr || !thread->QueryInterface(kIID_cISCLua, &lua)) return nullptr;
    FC4_ACQUIRED(static_cast<cISCLua*>(lua), "QueryInterface");
    return static_cast<cISCLua*>(lua);  // one reference, released by the caller
}

// §8.1 tag-gc: on the system's Lua state, userdata of tag 0 or 1 releases the
// pointer it holds.
int32_t TagGc(lua_State* state) {
    cISCLua* L = HostOf(state);
    if (L == nullptr) return 0;
    // HostOf QIs even when this is another state or non-userdata: balance every path.
    // [abi-ret] eax-or-void (interfaces.tsv cISCLua 73): the value is used as a bool
    if (g_live != nullptr && L == g_live->Lua() && L->IsUserData(1)) {
        const int32_t tag = L->ToUserTag(1);
        if (tag == g_tags[0] || tag == g_tags[1]) {
            if (auto* p = static_cast<cIGZUnknown*>(L->ToUserData(1))) FC4_RELEASE(p);
        }
    }
    FC4_RELEASE(L);
    return 0;
}

// §4.1 step 3: NewTag twice, each with tag-gc as its "gc" method.
void CreateTags(void* u, int32_t* subjectTag, int32_t* stringTag) {
    cISCLua* L = Self(u)->Lua();
    for (int i = 0; i < 2; ++i) {
        g_tags[i] = -1;
        if (L == nullptr) continue;
        g_tags[i] = L->NewTag();
        L->PushCClosure(&TagGc, 0);
        L->SetTagMethod(g_tags[i], "gc");
    }
    if (subjectTag != nullptr) *subjectTag = g_tags[0];
    if (stringTag != nullptr) *stringTag = g_tags[1];
}

int32_t Dispatch(size_t index, lua_State* state) {
    cISCLua* L = HostOf(state);
    if (L == nullptr) return 0;
    int32_t pushed = 0;
    if (g_live != nullptr && g_live->Core() != nullptr) {
        const int32_t n = L->GetTop();
        std::vector<fc4_adv_lua_value> args(static_cast<size_t>(n > 0 ? n : 0));
        std::vector<std::string> strs(args.size());
        for (int32_t i = 1; i <= n; ++i) {
            fc4_adv_lua_value& a = args[static_cast<size_t>(i - 1)];
            a = fc4_adv_lua_value{};
            // [abi-ret] eax-or-void (interfaces.tsv cISCLua 18): the value is used as a bool
            if (L->IsBoolean(i)) {
                a.kind = FC4_ADV_VALUE_BOOL;
                a.boolean = L->ToBoolean(i) ? 1u : 0u;
            } else if (L->Type(i) == cIGZLua5Thread::LuaTypeNumber) {
                a.kind = FC4_ADV_VALUE_NUMBER;
                a.number = L->ToNumber(i);
            } else if (L->Type(i) == cIGZLua5Thread::LuaTypeString) {
                strs[static_cast<size_t>(i - 1)].assign(L->ToString(i), L->Strlen(i));
                a.kind = FC4_ADV_VALUE_STRING;
                a.str = reinterpret_cast<const uint8_t*>(strs[static_cast<size_t>(i - 1)].data());
                a.len = strs[static_cast<size_t>(i - 1)].size();
            }
            // Subject userdata (tag 0) needs the subject class (next round): passed as nil.
        }
        fc4_adv_lua_value res[4]{};
        int32_t r = fc4_advisors_call_lua(g_live->Core(), kApi[index].name, args.data(), args.size(), res, 4);
        if (r > 4) r = 4;
        for (int32_t k = 0; k < r; ++k) {
            const fc4_adv_lua_value& v = res[k];
            switch (v.kind) {
                case FC4_ADV_VALUE_BOOL: L->PushBoolean(v.boolean != 0); break;
                case FC4_ADV_VALUE_NUMBER: L->PushNumber(v.number); break;
                case FC4_ADV_VALUE_STRING: L->PushLString(reinterpret_cast<const char*>(v.str), v.len); break;
                default: L->PushNil(); break;
            }
            if (v.subject != nullptr) fc4_adv_obj_destroy(v.subject);
            ++pushed;
        }
    }
    FC4_RELEASE(L);
    return pushed;
}

template <size_t I>
int32_t Trampoline(lua_State* state) {
    return Dispatch(I, state);
}
template <size_t... I>
constexpr std::array<lua_CFunction, sizeof...(I)> MakeTrampolines(std::index_sequence<I...>) {
    return {&Trampoline<I>...};
}
const std::array<lua_CFunction, kApiCount> kTrampolines = MakeTrampolines(std::make_index_sequence<kApiCount>());

void RegisterOne(cISCLua* L, const char* name, lua_CFunction fn, bool global) {
    if (global) L->GetGlobals();
    else L->GetGlobal("game");
    L->Register(fn, "_tf_");
    L->PushString(name);
    L->GetGlobal("_tf_");
    L->SetTable(-3);  // unprotected, as the original (lua-host.md §8.2)
    L->Pop(1);
}

void RegisterApi(void* u, bool global) {
    cISCLua* L = Self(u)->Lua();
    if (L == nullptr) return;
    const int32_t top = L->GetTop();
    // §8.2: one walk in vector order; register_api(true) takes the global
    // entries, register_api(false) the `game` ones. An original whose address
    // the compat layer refused is left out (logged there once).
    for (size_t i = 0; i < kApiCount; ++i) {
        const ApiEntry& e = kApi[i];
        if (e.global != global) continue;
        lua_CFunction fn = kTrampolines[i];
        if (e.native != kCore) {
            const uintptr_t va = fc4::compat::DirectCall(e.native);
            if (va == 0) continue;
            fn = reinterpret_cast<lua_CFunction>(va);
        }
        RegisterOne(L, e.name, fn, global);
    }
    L->SetTop(top);
}

void ReloadScriptsCb(void* u) { Self(u)->ReloadScripts(); }

fc4_advisor_host MakeHost(cFC4AdvisorSystem* s) {
    fc4_advisor_host h{};
    h.struct_size = sizeof(h);
    h.user = s;
    h.advisor_count = &AdvisorCount;
    h.advisor_entry = &AdvisorEntry;
    h.advice_count = &AdviceCount;
    h.advice_entry = &AdviceEntry;
    h.compile_trigger = &CompileTrigger;
    h.release_trigger = &ReleaseTrigger;
    h.call_trigger = &CallTrigger;
    h.register_api = &RegisterApi;
    h.reload_scripts = &ReloadScriptsCb;
    h.collect_garbage = &CollectGarbage;
    h.close_lua = &CloseLua;
    h.set_global = &SetGlobal;
    h.set_global_string = &SetGlobalString;
    h.debug_enabled = &DebugEnabled;
    h.register_cheat = &RegisterCheat;
    h.unregister_cheat = &UnregisterCheat;
    h.advisor_window = &AdvisorWindow;
    h.has_city = &HasCity;
    h.now_ms = &NowMs;
    h.city_established = &CityEstablishedCb;
    h.difficulty = &DifficultyCb;
    h.cell_counts = &CellCountsCb;
    h.sim_date = &SimDateCb;
    h.start_year = &StartYearCb;
    h.is_any_paused = &IsAnyPausedCb;
    h.post_ticker = &PostTickerCb;
    h.set_local = &SetLocal;
    h.create_tags = &CreateTags;
    h.global_effect = &GlobalEffectCb;
    h.subscribe = &SubscribeCb;
    h.unsubscribe = &UnsubscribeCb;
    h.post = &PostCb;
    h.send = &SendCb;
    h.add_ref = &AddRefCb;
    h.release = &ReleaseCb;
    h.resolve_ref = &ResolveRefCb;
    h.object_created = &ObjectCreated;
    h.object_dropped = &ObjectDroppedCb;
    h.rebuild_text = &RebuildTextCb;
    return h;
}

}  // namespace

fc4_advisors* fc4::CurrentAdvisorCore() { return g_live != nullptr ? g_live->Core() : nullptr; }

bool fc4::ActivateMySimLua(int32_t id) {
    cISCLua* L = g_live ? g_live->Lua() : nullptr;
    const auto it = g_mysimTables.find(id);
    if (!L || it == g_mysimTables.end()) return false;
    L->GetRef(it->second);
    L->SetGlobal("mysim");
    return true;
}

void fc4::FillMySimLuaHost(fc4_mysim_host* h) {
    h->create_table = [](void*, int32_t id) -> bool {
        cISCLua* L = g_live ? g_live->Lua() : nullptr;
        if (!L) return false;
        if (g_mysimTables.find(id) == g_mysimTables.end()) {
            L->PushNewTable();
            g_mysimTables[id] = L->Ref();
        }
        return true;
    };
    h->activate_lua = [](void*, int32_t id) -> bool { return ActivateMySimLua(id); };
    h->tuning = [](void*, const char* key, double* out) -> bool {
        cISCLua* L = g_live ? g_live->Lua() : nullptr;
        if (!L || !key || !out) return false;
        const int32_t top = L->GetTop();
        L->GetGlobal("tuning_constants");
        bool ok = false;
        if (L->IsTable(-1)) {
            L->PushString(key);
            L->GetTable(-2);
            ok = L->IsNumber(-1);
            if (ok) *out = L->ToNumber(-1);
        }
        L->SetTop(top);
        return ok;
    };
    h->set_field = [](void*, int32_t id, const char* key, const fc4_mysim_field* f) {
        cISCLua* L = g_live ? g_live->Lua() : nullptr;
        const auto it = g_mysimTables.find(id);
        if (!L || !f || !key || it == g_mysimTables.end()) return;
        const int32_t top = L->GetTop();
        L->GetRef(it->second);
        L->PushString(key);
        if (f->kind == FC4_MYSIM_FIELD_NUMBER) L->PushNumber(f->number);
        else if (f->kind == FC4_MYSIM_FIELD_STRING)
            L->PushLString(reinterpret_cast<const char*>(f->str), static_cast<int32_t>(f->len));
        else if (f->object == 0 && f->subject == nullptr) L->PushNil();
        else {
            fc4_adv_obj* s = fc4_advisors_new_subject(CurrentAdvisorCore());
            fc4_adv_subject_state state{};
            if (f->subject) state = *f->subject;
            else {
                state.object = f->object;
                fc4_mysim_host services{};
                FillMySimGameServices(&services);
                float p[3]{};
                if (services.occupant_position(nullptr, f->object, p)) {
                    state.x = p[0]; state.z = p[2]; state.has_location = 1;
                }
            }
            fc4_adv_subject_set(s, &state);
            cIGZUnknown* subject = FromToken(fc4_adv_obj_token(s));
            if (subject) FC4_ADDREF(subject);
            L->PushUserTag(subject, g_tags[0]);
            fc4_adv_obj_destroy(s);
        }
        L->SetTable(-3);
        L->SetTop(top);
    };
}

cIGZUnknown* CreateFC4AdvisorSystem() { return (new cFC4AdvisorSystem())->Primary(); }

// ---- the cheat handler (+8) -------------------------------------------------------------------------------

bool cFC4AdvisorCheatHandler::QueryInterface(uint32_t riid, void** ppvObj) { return owner_->QueryInterface(riid, ppvObj); }
uint32_t cFC4AdvisorCheatHandler::AddRef() { return owner_->AddRef(); }
uint32_t cFC4AdvisorCheatHandler::Release() { return owner_->Release(); }
// §1.1 keeps the +8 interface for QueryInterface; §5.2 (AD9): the system is not
// on the manager's cIGZMessageTarget list, cheats arrive in the message-2
// DoMessage. Nothing is delivered here.
bool cFC4AdvisorCheatHandler::DoMessage(cGZMessage&) { return true; }

// ---- object -----------------------------------------------------------------------------------------------

cFC4AdvisorSystem::cFC4AdvisorSystem() : cheat_(this) {
    // §9: the generator is seeded with 0xFFFFFFFF at construction, which
    // foundations §6.1 Seed() turns into time(NULL) XOR ((c << 16) | c), c = clock().
    const uint32_t c = static_cast<uint32_t>(std::clock());
    const uint32_t seed = fc4::advisors::WallClockSeed(static_cast<uint32_t>(std::time(nullptr)), c);
    random_.mul = seed;
    random_.lcg = seed;
    fc4::AdvisorWinProcOps ops{};
    if (fc4::GameAdvisorWinProcOps(&ops)) window_.SetOps(ops);
    host_ = MakeHost(this);
    // `enable = mysims` (cFC4MySim.h): the core runs the MySim classes and agent.
    const bool mysims = fc4::MySimsReplaced();
    if (mysims) fc4::FillMySimReplacementHost(&mysimHost_);
    core_ = fc4_advisors_create(&host_, mysims ? &mysimHost_ : nullptr, GetTickCount());  // §9: wall-clock seed
    g_live = this;  // §1.2: class-store Read and factory requests precede Init.
    self_ = core_ ? fc4_advisors_system_object(core_) : nullptr;
    if (self_ != nullptr) fc4_adv_obj_set_token(self_, fc4::TokenOf(Primary()));
    global_ = fc4::AdvisorSystemGlobalSlot(fc4::compat::AdvisorSystemGlobal());
    global_.Publish(Primary());  // §14.2
    fc4::Log("cFC4AdvisorSystem(%p) created, core %p, window ops %d", static_cast<void*>(Primary()),
             static_cast<void*>(core_), window_.HasOps() ? 1 : 0);
}

cFC4AdvisorSystem::~cFC4AdvisorSystem() {
    if (g_live == this) g_live = nullptr;
    global_.Withdraw();
    map_.Clear();
    if (self_ != nullptr) fc4_adv_obj_destroy(self_);
    fc4_advisors_destroy(core_);
    core_ = nullptr;
    fc4::AdvFront::DestroyAll();  // fronts still referenced by the game; their core objects may call the host (this)
    ReleaseLua();
}

// §1.1: 0x4A0DD617 and 1 (+0), 0x452294AA (+4), 0xE98B2F57 (+8), 0xE4FDA3D4 (+0x10).
bool cFC4AdvisorSystem::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr || !fc4_advisors_query_interface(riid)) return false;
    switch (riid) {
        case kIID_cIGZMessageTarget2: *ppvObj = Target(); break;
        case kIID_cIGZMessageTarget: *ppvObj = CheatTarget(); break;
        case GZIID_cIGZSerializable: *ppvObj = static_cast<cIGZSerializable*>(this); break;
        default: *ppvObj = Primary(); break;
    }
    AddRef();
    return true;
}

uint32_t cFC4AdvisorSystem::AddRef() { return ++refCount_; }

uint32_t cFC4AdvisorSystem::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;
    destroying_ = true;
    if (initialised_) Shutdown();
    delete this;
    return 0;
}

fc4_advisors_state cFC4AdvisorSystem::State() const {
    fc4_advisors_state st{};
    st.struct_size = sizeof(st);
    if (core_ != nullptr) fc4_advisors_get_state(core_, &st);
    return st;
}

// §4.1 step 1: the app's Lua context; this object holds one reference of its own.
void cFC4AdvisorSystem::AcquireLua() {
    if (lua_ != nullptr) return;
    cISC4App* app = App();
    cISCLua* lua = app ? app->GetLuaInterpreter() : nullptr;
    if (lua != nullptr) {
        FC4_ADDREF(lua);
        lua_ = lua;
    }
    if (app != nullptr) FC4_RELEASE(app);
}

void cFC4AdvisorSystem::ReleaseLua() {
    cISCLua* lua = lua_;
    lua_ = nullptr;
    if (lua) for (const auto& table : g_mysimTables) lua->Unref(table.second);
    g_mysimTables.clear();
    if (lua != nullptr) FC4_RELEASE(lua);
}

void cFC4AdvisorSystem::Subscribe(uint32_t type, bool on) {
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    if (on) ms->AddNotification(Target(), type);
    else ms->RemoveNotification(Target(), type);
}

void cFC4AdvisorSystem::Cheat(uint32_t id, const char* name, bool on) {
    cISC4App* app = App();
    cIGZCheatCodeManager* raw = app ? app->GetCheatCodeManager() : nullptr;
    cIGZCheatCodeManager* cm = nullptr;
    if (raw != nullptr && raw->QueryInterface(kIID_cIGZCheatCodeManager, reinterpret_cast<void**>(&cm)) && cm) {
        FC4_ACQUIRED(cm, "QueryInterface");
        // §5.2 (AD9): the system is a message-2 notification target of the
        // manager (SendCheatNotifications), not on its cIGZMessageTarget list.
        if (on) {
            cm->RegisterCheatCode(id, cRZBaseString(name ? name : ""));
            cm->AddNotification2(Target(), id);
        } else {
            cm->RemoveNotification2(Target(), id);
            cm->UnregisterCheatCode(id);
        }
        FC4_RELEASE(cm);
    }
    if (app != nullptr) FC4_RELEASE(app);
}

// §4.2 steps 1-3 (AD10, AD11). The core clears reloadPending, then
// register_api(false) and the tutorial hand-over follow in the core.
void cFC4AdvisorSystem::ReloadScripts() {
    if (lua_ == nullptr) return;
    // 1. `_on_reload`, protected, no arguments.
    const int32_t top = lua_->GetTop();
    lua_->GetGlobal("_on_reload");
    if (lua_->Type(-1) == cIGZLua5Thread::LuaTypeFunction) lua_->CallProtected(0, 0);
    lua_->SetTop(top);
    // 2. the plugin directory, then the SKU-specific one (the later wins).
    using UpdateFn = int32_t(__cdecl*)(uint32_t group, const char* dir);
    using LoadFn = int32_t(__cdecl*)(cISCLua* lua, uint32_t group, uint32_t firstPackage);
    const uintptr_t update = fc4::compat::DirectCall(Direct::UpdateResourcesFromDirectory);
    const uintptr_t load = fc4::compat::DirectCall(Direct::LoadPackageScripts);
    constexpr uint32_t kScriptGroup = 0x4A5E8EF6;
    cISC4App* app = App();
    if (app != nullptr && update != 0) {
        const fc4::advisors::ReloadDirectories dirs =
            fc4::advisors::AdvisorReloadDirectories(AppDirectory(app, false), AppDirectory(app, true));
        for (const std::string* dir : {&dirs.first, &dirs.second}) {
            fc4::Log("advisors reload: directory \"%s\"", dir->c_str());  // AD11: recorded, not interpreted
            reinterpret_cast<UpdateFn>(update)(kScriptGroup, dir->c_str());
        }
    }
    if (app != nullptr) FC4_RELEASE(app);
    // 3. the packages of the script group, from package 0.
    if (load != 0) reinterpret_cast<LoadFn>(load)(lua_, kScriptGroup, 0);
}

// §4.1 step 10 / §4.6: slots 24 and 28 through the own vtable.
void cFC4AdvisorSystem::WindowCallback(bool start) {
    if (start) StartAdvisorWindow();
    else RemoveWinProcs();
}

// ---- slots (§12.1) ----------------------------------------------------------------------------------------

bool cFC4AdvisorSystem::Init() {
    AcquireLua();
    g_live = this;
    const bool ok = core_ != nullptr && fc4_advisors_init(core_);
    initialised_ = true;
    if (fc4::MySimsReplaced()) fc4::MySimReplacementStart(core_);
    const fc4_advisors_state st = State();
    fc4::Log("advisors Init: ok=%d lua=%p advisors=%u", ok ? 1 : 0, static_cast<void*>(lua_), st.advisor_count);
    return true;
}

bool cFC4AdvisorSystem::Shutdown() {
    if (core_ != nullptr) fc4_advisors_shutdown(core_);
    if (fc4::MySimsReplaced()) fc4::MySimReplacementStop(core_);
    ReleaseLua();
    if (g_live == this) g_live = nullptr;
    initialised_ = false;
    return true;
}

// §4.3: AddAdvisor/RemoveAdvisor/GetAdvisor/the map need the advisor class
// (next round): a null advisor returns true and does nothing (§4.3).
bool cFC4AdvisorSystem::AddAdvisor(cISC4Advisor* a) { return a == nullptr; }
bool cFC4AdvisorSystem::RemoveAdvisor(int32_t) { return false; }
cISC4Advisor* cFC4AdvisorSystem::GetAdvisor(int32_t id) {
    fc4_adv_obj* o = core_ ? fc4_advisors_advisor_object(core_, id) : nullptr;
    const uint64_t t = o ? fc4_adv_obj_token(o) : 0;
    if (o != nullptr) fc4_adv_obj_destroy(o);
    return reinterpret_cast<cISC4Advisor*>(static_cast<uintptr_t>(t));  // no AddRef (§12.1)
}
fc4::GameMapObject* cFC4AdvisorSystem::GetAdvisors() {
    if (map_.Object() == nullptr) map_.Rebuild({});
    return map_.Object();
}
void* cFC4AdvisorSystem::GetRandom() { return &random_; }  // §12.1 slot 9 (AD8): the member's address
cISCLua* cFC4AdvisorSystem::GetScriptingContext() { return lua_; }

// §12.1 slot 11 (AD8).
bool cFC4AdvisorSystem::DoLink(const cIGZString& link) {
    // 1-2. copy (Data +0x18, Strlen +0x1C), trim, sscanf "%08x%08x".
    const char* data = VCall<const char*>(&link, 0x18);
    const uint32_t len = VCall<uint32_t>(&link, 0x1C);
    const fc4::advisors::AdvisorLink l = fc4::advisors::ParseAdvisorLink(data, len);
    if (!l.parsed) return false;
    // 3. advisor slot 13 GetAdviceBySessionID (+0x34), advice slot 22 DoLink (+0x58).
    if (l.TriesAdvice()) {
        cISC4Advisor* a = GetAdvisor(static_cast<int32_t>(l.advisorID));
        void* advice = a ? VCall<void*, int32_t>(a, 0x34, static_cast<int32_t>(l.sessionID)) : nullptr;
        if (advice != nullptr) {
            cRZBaseString rest(l.AdviceText().c_str());
            // [abi-ret] void: the advice's result is not read (§12.1 slot 11 returns true)
            VCall<bool, const cIGZString*>(advice, 0x58, static_cast<const cIGZString*>(&rest));
            return true;
        }
    }
    // 4. Lua through the own slots 18/19; DoBuffer = cISCLua slot 55 (+0xDC), status ignored.
    const std::string chunk = l.LuaChunk();
    PushContextAdvice(nullptr);
    if (lua_ != nullptr) {
        const char* p = chunk.c_str();
        VCall<int32_t, const char*, int32_t, const char*>(lua_, 0xDC, p, static_cast<int32_t>(std::strlen(p)), p);
    }
    PopContextAdvice();
    return true;
}
bool cFC4AdvisorSystem::SetGlobalValueString(const char* name, cIGZString* value) {
    if (lua_ == nullptr || value == nullptr) return false;
    SetGlobalString(this, name, reinterpret_cast<const uint8_t*>(value->ToChar()), value->Strlen());
    return true;
}
bool cFC4AdvisorSystem::SetGlobalValueNumber(const char* name, double value) {
    if (lua_ == nullptr) return false;
    SetGlobal(this, name, value);
    return true;
}
bool cFC4AdvisorSystem::SetLocalValue(const char*, double, cISC4AdviceSubject*) { return false; }
bool cFC4AdvisorSystem::PushEvent(int32_t eventId, cISC4AdviceSubject*) {
    if (core_ != nullptr) fc4_advisors_push_event(core_, static_cast<uint32_t>(eventId), nullptr);
    return true;
}
bool cFC4AdvisorSystem::SubscribeToEvent(int32_t, cISC4Advisor*) { return false; }
bool cFC4AdvisorSystem::UnsubscribeFromEvent(int32_t, cISC4Advisor*) { return false; }
void cFC4AdvisorSystem::PushContextAdvice(cISC4Advice*) {}
cISC4Advice* cFC4AdvisorSystem::PopContextAdvice() {
    if (core_ != nullptr) fc4_advisors_pop_context_advice(core_);
    return nullptr;
}
cISC4Advice* cFC4AdvisorSystem::GetContextAdvice() { return nullptr; }
int32_t cFC4AdvisorSystem::GetAdvicePopupPriorityThreshold() { return State().thresholds[0]; }
int32_t cFC4AdvisorSystem::GetDataTag(int32_t i) {
    const fc4_advisors_state st = State();
    return i == 0 ? st.tag_subject : st.tag_string;  // §12.1: no bounds check in the original
}
void* cFC4AdvisorSystem::GetAdvisorWinProc() { return window_.Get(); }
void cFC4AdvisorSystem::StartAdvisorWindow() {
    if (window_.HasOps()) window_.Start();
}
void cFC4AdvisorSystem::ShowAdvisorWindow() { window_.Show(); }
void cFC4AdvisorSystem::HideAdvisorWindow() { window_.Hide(); }
    // [abi-ret] AdvisorWindowSlot::IsVisible reads byte +0x56 (FC4AdvisorWindow.h), no game call
bool cFC4AdvisorSystem::IsAdvisorWindowVisible() { return window_.IsVisible(); }
void cFC4AdvisorSystem::RemoveWinProcs() { window_.Remove(); }

// §5: the ticker, PostCityInit, CityEstablished.
bool cFC4AdvisorSystem::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    // §5.2 (AD9): cheat 0x230E27AC, ID = data1, text = GetVoid2 (+0x3C) as a
    // borrowed cIGZString*, used with no null test as the original.
    if (pMessage->GetType() == fc4::advisors::kCheatMessage) {
        const fc4::advisors::CheatPayload c = fc4::advisors::ReadCheatPayload(pMessage);
        fc4_advisors_cheat(core_, c.id, static_cast<const cIGZString*>(c.text)->ToChar());
        return true;
    }
    fc4_adv_message m{};
    m.msg_type = pMessage->GetType();
    cIGZMessage2Standard* std = nullptr;
    if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&std)) && std != nullptr) {
        FC4_ACQUIRED(std, "QueryInterface");
        m.data1 = static_cast<uint32_t>(std->GetData1());
        m.data2 = static_cast<uint32_t>(std->GetData2());
        m.data3 = static_cast<uint32_t>(std->GetData3());
        m.data4 = static_cast<uint32_t>(std->GetData4());
        m.void1 = fc4::TokenOf(std->GetVoid1());
        FC4_RELEASE(std);
    }
    return fc4_advisors_do_message(core_, &m);
}

// §11.1 holds advisor references: written by the next round with the advisor class.
bool cFC4AdvisorSystem::Write(cIGZOStream& out) {
    fc4::OStreamAdapter stream(out);
    return self_ != nullptr && fc4_adv_obj_write(self_, stream.Get());
}
bool cFC4AdvisorSystem::Read(cIGZIStream& in) {
    fc4::IStreamAdapter stream(in);
    return self_ != nullptr && fc4_adv_obj_read(self_, stream.Get());
}
uint32_t cFC4AdvisorSystem::GetGZCLSID() { return kFC4_CLSID_cSC4AdvisorSystem; }
