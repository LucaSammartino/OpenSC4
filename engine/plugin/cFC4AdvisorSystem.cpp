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

#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
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
#include "cISCLua.h"
#include "cRZBaseString.h"

#include <windows.h>

#include <array>
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
    const double v = L->IsNumber(-1) ? L->ToNumber(-1) : 0.0;
    L->Pop(1);
    return v;
}

bool FieldString(cISCLua* L, const char* name, std::string* out) {
    L->PushString(name);
    L->GetTable(-2);
    const bool ok = L->IsString(-1);
    if (ok) out->assign(L->ToString(-1), L->Strlen(-1));
    else out->clear();
    L->Pop(1);
    return ok;
}

// Pushes global[table][i]; false (nothing pushed) when it is not a table.
bool PushEntry(cISCLua* L, const char* table, uint32_t i) {
    L->GetGlobal(table);
    if (!L->IsTable(-1)) {
        L->Pop(1);
        return false;
    }
    L->RawGetI(-1, static_cast<int32_t>(i));
    L->Remove(-2);
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
    if (L->IsTable(-1)) {
        const int32_t n = L->GetN(-1);
        for (int32_t k = 1; k <= n; ++k) {
            L->RawGetI(-1, k);
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

void SetGlobal(void* u, const char* name, double value) {
    if (cISCLua* L = Self(u)->Lua()) {
        L->PushNumber(value);
        L->SetGlobal(name);
    }
}

void SetGlobalString(void* u, const char* name, const uint8_t* value, size_t len) {
    if (cISCLua* L = Self(u)->Lua()) {
        L->PushLString(reinterpret_cast<const char*>(value), len);
        L->SetGlobal(name);
    }
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

void SubscribeCb(void* u, uint32_t kind, int32_t, uint64_t, uint32_t type) {
    if (kind == FC4_ADV_OBJ_SYSTEM) Self(u)->Subscribe(type, true);
}
void UnsubscribeCb(void* u, uint32_t kind, int32_t, uint64_t, uint32_t type) {
    if (kind == FC4_ADV_OBJ_SYSTEM) Self(u)->Subscribe(type, false);
}

// fc4.h: obj_kind FC4_ADV_OBJ_HOST names a host object by token; advisor and
// advice objects are the next round's classes (their tokens come from
// object_created), so only the host kind carries an object here.
void Deliver(const fc4_adv_out_message* m, bool sync) {
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cFC4Message2Standard* msg = cFC4Message2Standard::New();
    msg->AddRef();
    msg->SetType(m->msg_type);
    msg->SetData1(static_cast<intptr_t>(m->data1));
    msg->SetData2(static_cast<intptr_t>(m->data2));
    if (m->obj_kind != 0 && m->obj_token != 0) msg->SetVoid1(fc4::FromToken(m->obj_token));
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

uint64_t ObjectCreated(void* u, uint32_t kind, uint32_t, fc4_adv_obj* handle) {
    if (kind == FC4_ADV_OBJ_SYSTEM) {
        fc4_adv_obj_destroy(handle);
        return fc4::TokenOf(Self(u)->Primary());
    }
    // Advisor, advice and subject objects need their GZCOM classes (next round).
    fc4_adv_obj_destroy(handle);
    return 0;
}

// ---- register_api (§8.2, lua-host.md §8.2) ---------------------------------------------------------------

struct LuaEntry {
    const char* name;
    bool global;
};
// §8.3 table order (simulation collector). [choice] The spec gives the
// collectors' contents, not their order inside the vector (question AD7);
// the order only decides which function `_tf_` names afterwards.
constexpr LuaEntry kCoreEntries[] = {
    {"trend_value", false}, {"trend_slope", false}, {"trend_count", false}, {"ordinance_is_on", false},
    {"ordinance_is_available", false}, {"ordinance_set_on", false}, {"ordinance_set_off", false},
    {"ordinance_enact_fee", false}, {"ordinance_repeal_fee", false}, {"ordinance_monthly_cost", false},
    {"ordinance_name", false}, {"ordinance_description", false}, {"event_feedback", false},
    {"expire_advice", false}, {"retire_advice", false}, {"advice_subject", false}, {"trigger_event", false},
    {"difficulty_level", false}, {"random_chance", false}, {"is_tutorial", false}, {"building_is_group", false},
    {"event_building_distance", false}, {"event_disaster_distance", false}, {"nd_cancel", false},
    {"mysim_distance_to_closest_building", false}, {"mysim_distance_to_closest_landmark_or_reward", false},
    {"mysim_local_school_grade", false}, {"mysim_local_hospital_grade", false}, {"mysim_dispatch_tool", false},
    {"reward_instance_count", false}, {"get_reward_building_state", false}, {"unlock_reward_building", false},
    {"_combine_guids", true}, {"print", true},
};
constexpr size_t kCoreCount = sizeof(kCoreEntries) / sizeof(kCoreEntries[0]);

struct CompatEntry {
    const char* name;
    fc4::compat::Direct which;
};
constexpr CompatEntry kCompatEntries[] = {
    {"camera_jump", fc4::compat::Direct::LuaCameraJump},          {"camera_zoom", fc4::compat::Direct::LuaCameraZoom},
    {"tool_plop_network", fc4::compat::Direct::LuaToolPlopNetwork}, {"tool_plop_zone", fc4::compat::Direct::LuaToolPlopZone},
    {"tool_plop_building", fc4::compat::Direct::LuaToolPlopBuilding}, {"tool_plop_flora", fc4::compat::Direct::LuaToolPlopFlora},
    {"tool_button", fc4::compat::Direct::LuaToolButton},            {"window_budget", fc4::compat::Direct::LuaWindowBudget},
    {"window_map", fc4::compat::Direct::LuaWindowMap},              {"window_graph", fc4::compat::Direct::LuaWindowGraph},
    {"window_query", fc4::compat::Direct::LuaWindowQuery},
};

// lua-host.md §1.3: the owning thread object sits in the 4 bytes before the state.
cISCLua* HostOf(lua_State* state) {
    cIGZUnknown* thread = *reinterpret_cast<cIGZUnknown**>(reinterpret_cast<char*>(state) - sizeof(void*));
    void* lua = nullptr;
    if (thread == nullptr || !thread->QueryInterface(kIID_cISCLua, &lua)) return nullptr;
    return static_cast<cISCLua*>(lua);  // one reference, released by the caller
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
        int32_t r = fc4_advisors_call_lua(g_live->Core(), kCoreEntries[index].name, args.data(), args.size(), res, 4);
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
const std::array<lua_CFunction, kCoreCount> kTrampolines = MakeTrampolines(std::make_index_sequence<kCoreCount>());

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
    for (size_t i = 0; i < kCoreCount; ++i) {
        if (kCoreEntries[i].global == global) RegisterOne(L, kCoreEntries[i].name, kTrampolines[i], global);
    }
    if (!global) {
        for (const CompatEntry& e : kCompatEntries) {
            const uintptr_t va = fc4::compat::DirectCall(e.which);
            if (va != 0) RegisterOne(L, e.name, reinterpret_cast<lua_CFunction>(va), false);
        }
    }
    L->SetTop(top);
}

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
    h.subscribe = &SubscribeCb;
    h.unsubscribe = &UnsubscribeCb;
    h.post = &PostCb;
    h.send = &SendCb;
    h.add_ref = &AddRefCb;
    h.release = &ReleaseCb;
    h.resolve_ref = &ResolveRefCb;
    h.object_created = &ObjectCreated;
    return h;
}

}  // namespace

cIGZUnknown* CreateFC4AdvisorSystem() { return (new cFC4AdvisorSystem())->Primary(); }

// ---- the cheat handler (+8) -------------------------------------------------------------------------------

bool cFC4AdvisorCheatHandler::QueryInterface(uint32_t riid, void** ppvObj) { return owner_->QueryInterface(riid, ppvObj); }
uint32_t cFC4AdvisorCheatHandler::AddRef() { return owner_->AddRef(); }
uint32_t cFC4AdvisorCheatHandler::Release() { return owner_->Release(); }
// §5.2. [choice] The cheat's text is not read from cGZMessage (its layout for
// cheat notifications is not in the specs): the core gets the ID only.
bool cFC4AdvisorCheatHandler::DoMessage(cGZMessage& msg) {
    if (owner_->Core() != nullptr) fc4_advisors_cheat(owner_->Core(), msg.dwMessageType, nullptr);
    return true;
}

// ---- object -----------------------------------------------------------------------------------------------

cFC4AdvisorSystem::cFC4AdvisorSystem() : cheat_(this) {
    fc4::AdvisorWinProcOps ops{};
    if (fc4::GameAdvisorWinProcOps(&ops)) window_.SetOps(ops);
    host_ = MakeHost(this);
    core_ = fc4_advisors_create(&host_, nullptr, GetTickCount());  // §9: wall-clock seed
    self_ = core_ ? fc4_advisors_system_object(core_) : nullptr;
    if (self_ != nullptr) fc4_adv_obj_set_token(self_, fc4::TokenOf(Primary()));
    global_ = fc4::AdvisorSystemGlobalSlot(fc4::compat::AdvisorSystemGlobal());
    global_.Publish(Primary());  // §14.2
    fc4::Log("cFC4AdvisorSystem(%p) created, core %p, window ops %d", static_cast<void*>(Primary()),
             static_cast<void*>(core_), window_.HasOps() ? 1 : 0);
}

cFC4AdvisorSystem::~cFC4AdvisorSystem() {
    global_.Withdraw();
    map_.Clear();
    if (self_ != nullptr) fc4_adv_obj_destroy(self_);
    fc4_advisors_destroy(core_);
    core_ = nullptr;
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
        if (on) {
            cm->RegisterCheatCode(id, cRZBaseString(name ? name : ""));
            cm->AddNotification(CheatTarget(), id);
        } else {
            cm->RemoveNotification(CheatTarget(), id);
            cm->UnregisterCheatCode(id);
        }
        FC4_RELEASE(cm);
    }
    if (app != nullptr) FC4_RELEASE(app);
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
    const fc4_advisors_state st = State();
    fc4::Log("advisors Init: ok=%d lua=%p advisors=%u", ok ? 1 : 0, static_cast<void*>(lua_), st.advisor_count);
    return true;
}

bool cFC4AdvisorSystem::Shutdown() {
    if (core_ != nullptr) fc4_advisors_shutdown(core_);
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
void* cFC4AdvisorSystem::GetRandom() { return nullptr; }  // the generator lives in the core (question AD7)
cISCLua* cFC4AdvisorSystem::GetScriptingContext() { return lua_; }
bool cFC4AdvisorSystem::DoLink(const cIGZString&) { return false; }  // body not specified (question AD7)
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
bool cFC4AdvisorSystem::IsAdvisorWindowVisible() { return window_.IsVisible(); }
void cFC4AdvisorSystem::RemoveWinProcs() { window_.Remove(); }

// §5: the ticker, PostCityInit, CityEstablished.
bool cFC4AdvisorSystem::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
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
bool cFC4AdvisorSystem::Write(cIGZOStream&) { return false; }
bool cFC4AdvisorSystem::Read(cIGZIStream&) { return false; }
uint32_t cFC4AdvisorSystem::GetGZCLSID() { return kFC4_CLSID_cSC4AdvisorSystem; }
