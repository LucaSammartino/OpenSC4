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

#include "cFC4CitySituation.h"

#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"

#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4AuraSimulator.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4CitySituationAdvice.h"
#include "cRZCOMDllDirector.h"

namespace {

// §3: the ten subscriptions of Init. The animation tick arrives as
// 0x86AD10EE with the elapsed milliseconds in data3.
constexpr uint32_t kMsgOccupantInserted = 0x99EF1142;
constexpr uint32_t kMsgOccupantRemoved = 0x99EF1143;
constexpr uint32_t kMsgAdviceAdded = 0x8A09FA64;
constexpr uint32_t kMsgAdviceRemoved = 0x8A09FA65;
constexpr uint32_t kMsgAdviceRemovedAll = 0x8A09FA67;
constexpr uint32_t kMsgPopupClosed = 0x6BB350E4;
constexpr uint32_t kMsgShowPopup = 0xEBB67EF7;
constexpr uint32_t kMsgRememberWindow = 0xCC0D905F;
constexpr uint32_t kMsgInitiateByGuid = 0xAC1045EF;
constexpr uint32_t kMsgPostCityInit = 0x26D31EC1;
constexpr uint32_t kMsgAnimationTick = 0x86AD10EE;
constexpr uint32_t kMsgAutomatonNotify = 0x09D92FC3;
constexpr uint32_t kMsgPlayerDriveLeft = 0x0BB324BD;
constexpr uint32_t kMsgCheat = 0x230E27AC;
constexpr uint32_t kMsgShowPopupNoAdvice = 0x2C16BDE7;

cFC4CitySituationManager* Self(void* u) { return static_cast<cFC4CitySituationManager*>(u); }

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

// ---- host callbacks (fc4_citysit_host) -----------------------------------------------------------------
//
// Wired: the cheat registration, the ten subscriptions, the indicator view's
// ShowAllIndicators, the animation tick, the preferences' reward list, the
// AddRef / Release pair, the advice and automaton queries the core needs to
// answer the interface, the aura's AddTransientEffect and the budget's
// DepositFunds (the §4.5 outcome), and the messages of §3.
//
// NOT wired (NULL; a missing callback reads as 0 / false / none): the mission
// gameplay — targets and their names, coverage cells, pursuit,
// CheckForSuccess's steps 2-4, the countdown and outcome effects, the
// player-drive mode, the joyride, the mission script, the Lua table and the
// popup. city-situation.md §0 puts all of that out of a replacement's scope
// and §8 sanctions answering "no situation" instead.

bool DebugEnabled(void*) {
    // §4.1 step 1 and §4.6: the cheat exists only with the debug flag
    // (cISC4App slot 21). Read it; without it no cheat is registered.
    if (cISC4App* app = App()) {
        const bool on = app->GetDebugFunctionalityEnabled();
        FC4_RELEASE(app);
        return on;
    }
    return false;
}

bool RegisterCheat(void*) {
    // §4.1 step 1: register +4 as a cheat target and `csitrigger` (0x8C4136DB)
    // with the cheat code manager. Left unwired: the cheat code manager is not
    // in this lane's interface set. Logged once at Init.
    return false;
}
void UnregisterCheat(void*) {}

void SubscribeCb(void* u, uint32_t t) {
    cFC4CitySituationManager* s = Self(u);
    if (cIGZMessageServer2* ms = s->Server()) ms->AddNotification(s->Target(), t);
}
void UnsubscribeCb(void* u, uint32_t t) {
    cFC4CitySituationManager* s = Self(u);
    if (cIGZMessageServer2* ms = s->Server()) ms->RemoveNotification(s->Target(), t);
}

bool AcquireIndicatorView(void*) {
    // The dispatch manager's indicator view (dispatch slot 22). The shadow and
    // the minimal replacement only need ShowAllIndicators, which is a separate
    // callback; without the view the core's indicator calls do nothing.
    return false;
}
void ReleaseIndicatorView(void*) {}

void RegisterAnimationTick(void*) {
    // §3: register the +4 sub-object with the animation tick manager
    // (cISC4AnimationTickManager, slot 5, priority 0xFF). Without it the
    // mission clock never advances, which is the minimal replacement's answer.
}
void UnregisterAnimationTick(void*) {}

void AcquireAutomataManagers(void*, fc4_token_callback, void*) {}
bool AcquireLotManager(void*) { return false; }
void ReleaseLotManager(void*) {}
bool AcquireMysim(void*) { return false; }
void ReleaseMysim(void*) {}
bool AcquireLua(void*) { return false; }
void ReleaseLua(void*) {}
void CreateLuaTable(void*) {}
void RemoveLuaTable(void*) {}
void ReleaseMissionResources(void*) {}

void ShowAllIndicators(void*, uint32_t, bool) {}

// §4.1 step 8 and §4.5 step 2: the user preferences' 64-entry reward list
// (cISC4App slot 38) and saving it. Read only; the minimal replacement keeps
// successCounts for Lua get_success_count and does not write the profile.
size_t PreferencesRewards(void*, uint32_t* out, size_t capacity) {
    if (out == nullptr || capacity == 0) return 0;
    out[0] = 0;
    return 0;
}
void SetPreferencesReward(void*, uint32_t, uint32_t) {}
void SavePreferences(void*) {}

void AddRefCb(void* u, uint64_t token) {
    if (token == 0) return;
    cIGZUnknown* o = reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(token));
    o->AddRef();
    FC4_ACQUIRED(o, "core add_ref");
}
void ReleaseCb(void* u, uint64_t token) {
    if (token == 0) return;
    cIGZUnknown* o = reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(token));
    FC4_RELEASE(o);  // the core's reference
}

// The advices and the automata travel as the objects themselves.
bool IsSituationAdvice(void*, uint64_t object) {
    if (object == 0) return false;
    cIGZUnknown* o = reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(object));
    void* p = nullptr;
    const bool ok = o->QueryInterface(GZIID_cISC4CitySituationAdvice, &p) && p != nullptr;
    if (ok) {
        cIGZUnknown* q = static_cast<cIGZUnknown*>(p);
        FC4_ACQUIRED(q, "QueryInterface");
        FC4_RELEASE(q);
    }
    return ok;
}

void AdvisorAdvices(void*, fc4_token_callback, void*) {
    // §4.2: advisor 29's advices. Without the advisor interface in this
    // lane's set the list is empty: no situation is ever triggered.
}
uint64_t FindAdvice(void*, uint32_t) { return 0; }

// The advice fields the core reads (fc4_citysit_advice_info, §6 of the capi
// docs). Zeroed first by the core; nothing is readable here, so it stays zero.
void AdviceInfo(void*, uint64_t, fc4_citysit_advice_info*) {}
void RetireAdvice(void*, uint64_t) {}
void RunScript(void*, uint64_t, uint32_t) {}
float TimeLimit(void*, uint64_t) { return 0.0f; }
void BindMissionScript(void*, uint64_t, uint64_t) {}
bool BuildCoverageCells(void*, uint64_t) { return false; }
uint64_t FirstTarget(void*, uint64_t) { return 0; }
uint32_t TargetCount(void*, uint64_t) { return 0; }
void TargetName(void*, uint64_t, uint64_t, fc4_bytes_sink, void*) {}
bool AdviceAcceptsAutomaton(void*, uint64_t, uint64_t) { return false; }

bool IsAutomaton(void*, uint64_t object) {
    if (object == 0) return false;
    cIGZUnknown* o = reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(object));
    void* p = nullptr;
    const bool ok = o->QueryInterface(0xA9B40F05, &p) && p != nullptr;  // cISC4Automaton (§3)
    return ok;
}
bool OccupantHasProperty(void*, uint64_t, uint32_t) { return false; }
void CandidateVehicles(void*, uint64_t, fc4_token_callback, void*) {}
uint32_t AutomatonFlags(void*, uint64_t) { return 0; }
uint32_t AutomatonType(void*, uint64_t) { return 0; }
uint32_t TrackIndicator(void*, uint64_t) { return 0; }
void UntrackIndicator(void*, uint32_t) {}
void AddListener(void*, uint64_t) {}
void RemoveListener(void*, uint64_t) {}
void FreezeAutomata(void*, uint64_t, bool) {}
void PlaySound(void*, uint64_t, uint32_t) {}
void Position(void*, uint64_t, float* x, float* z) {
    if (x != nullptr) *x = 0.0f;
    if (z != nullptr) *z = 0.0f;
}

uint32_t CheckForSuccess(void*, uint64_t, float, uint32_t status) { return status; }
void RunPursuit(void*, uint64_t, float) {}
void StartCountdownEffect(void*) {}
void EndCountdownEffect(void*) {}
void OutcomeEffect(void*, uint64_t, bool, float, float) {}
void PlayerDriveMode(void*, uint64_t, bool) {}
void StartJoyride(void*, uint64_t) {}

// §4.5 steps 5 and 6: the aura simulator and the budget. Both are reachable
// through the city and are the outcome a success or a failure has on the city.
void AddTransientEffect(void*, float x, float z, float radius, float magnitude) {
    if (cISC4App* app = App()) {
        cISC4City* city = app->GetCity();
        cISC4AuraSimulator* aura = city != nullptr ? city->GetAuraSimulator() : nullptr;
        if (aura != nullptr) aura->AddTransientEffect(x, z, radius, magnitude);
        FC4_RELEASE(app);
    }
}
void DepositFunds(void*, int64_t amount) {
    if (cISC4App* app = App()) {
        cISC4City* city = app->GetCity();
        cISC4BudgetSimulator* budget = city != nullptr ? city->GetBudgetSimulator() : nullptr;
        if (budget != nullptr) budget->DepositFunds(amount);
        FC4_RELEASE(app);
    }
}

// §3: MessageSend of 0xABBBCA64 and MessagePost of the three self-addressed
// ones. A queued post carries IGZUnknown = the advice (0 = none).
void SendActivation(void* u, uint64_t advice, bool active) {
    Self(u)->Send(0xABBBCA64, active ? 1 : 0, 0, advice);
}
void PostShowPopup(void* u, uint64_t advice, uint64_t vehicle) {
    Self(u)->Post(kMsgShowPopup, advice, 0, static_cast<uint32_t>(vehicle));
}
void PostPopupClosed(void* u, uint64_t advice) { Self(u)->Post(kMsgPopupClosed, advice, 0, 0); }
void PostInitiate(void* u, uint32_t guid, bool showBriefing) {
    Self(u)->Post(kMsgInitiateByGuid, 0, guid, showBriefing ? 1u : 0u);
}

void HideMissionStatusPanel(void*) {}
void EndMysimDialog(void*) {}
uint32_t ShowPopup(void*, uint32_t, uint64_t, uint64_t) { return 0; }

uint64_t NextTarget(void*, uint64_t) { return 0; }
bool AdviceAcceptsGroup(void*, uint64_t, uint32_t) { return false; }
size_t AdviceGroups(void*, uint64_t, uint32_t*, size_t) { return 0; }
uint32_t AdvicePriority(void*, uint64_t) { return 0; }
void GroupAutomata(void*, uint32_t, fc4_token_callback, void*) {}
uint64_t CreateAutomatonForPlayerDrive(void*, uint64_t, uint32_t) { return 0; }
void RemoveAutomaton(void*, uint64_t) {}
void SetAutomatonFlag(void*, uint64_t, uint32_t) {}
void SetAutomatonGroup(void*, uint64_t, uint32_t) {}
void EnterPlayerDriveView(void*, uint64_t, uint64_t) {}
void ClearMissionState(void*, uint64_t) {}
void ReleaseSituationEffect(void*) {}
uint64_t AcquireMysimWindow(void*) { return 0; }

fc4_citysit_host MakeHost(cFC4CitySituationManager* self) {
    fc4_citysit_host h{};
    h.struct_size = sizeof(h);
    h.user = self;
    h.debug_enabled = &DebugEnabled;
    h.register_cheat = &RegisterCheat;
    h.unregister_cheat = &UnregisterCheat;
    h.subscribe = &SubscribeCb;
    h.unsubscribe = &UnsubscribeCb;
    h.acquire_indicator_view = &AcquireIndicatorView;
    h.release_indicator_view = &ReleaseIndicatorView;
    h.register_animation_tick = &RegisterAnimationTick;
    h.unregister_animation_tick = &UnregisterAnimationTick;
    h.acquire_automata_managers = &AcquireAutomataManagers;
    h.acquire_lot_manager = &AcquireLotManager;
    h.release_lot_manager = &ReleaseLotManager;
    h.acquire_mysim = &AcquireMysim;
    h.release_mysim = &ReleaseMysim;
    h.acquire_lua = &AcquireLua;
    h.release_lua = &ReleaseLua;
    h.create_lua_table = &CreateLuaTable;
    h.remove_lua_table = &RemoveLuaTable;
    h.release_mission_resources = &ReleaseMissionResources;
    h.show_all_indicators = &ShowAllIndicators;
    h.preferences_rewards = &PreferencesRewards;
    h.set_preferences_reward = &SetPreferencesReward;
    h.save_preferences = &SavePreferences;
    h.add_ref = &AddRefCb;
    h.release = &ReleaseCb;
    h.is_situation_advice = &IsSituationAdvice;
    h.advisor_advices = &AdvisorAdvices;
    h.find_advice = &FindAdvice;
    h.advice_info = &AdviceInfo;
    h.retire_advice = &RetireAdvice;
    h.run_script = &RunScript;
    h.time_limit = &TimeLimit;
    h.bind_mission_script = &BindMissionScript;
    h.build_coverage_cells = &BuildCoverageCells;
    h.first_target = &FirstTarget;
    h.target_count = &TargetCount;
    h.target_name = &TargetName;
    h.advice_accepts_automaton = &AdviceAcceptsAutomaton;
    h.is_automaton = &IsAutomaton;
    h.occupant_has_property = &OccupantHasProperty;
    h.candidate_vehicles = &CandidateVehicles;
    h.automaton_flags = &AutomatonFlags;
    h.automaton_type = &AutomatonType;
    h.track_indicator = &TrackIndicator;
    h.untrack_indicator = &UntrackIndicator;
    h.add_listener = &AddListener;
    h.remove_listener = &RemoveListener;
    h.freeze_automata = &FreezeAutomata;
    h.play_sound = &PlaySound;
    h.position = &Position;
    h.check_for_success = &CheckForSuccess;
    h.run_pursuit = &RunPursuit;
    h.start_countdown_effect = &StartCountdownEffect;
    h.end_countdown_effect = &EndCountdownEffect;
    h.outcome_effect = &OutcomeEffect;
    h.player_drive_mode = &PlayerDriveMode;
    h.start_joyride = &StartJoyride;
    h.add_transient_effect = &AddTransientEffect;
    h.deposit_funds = &DepositFunds;
    h.send_activation = &SendActivation;
    h.post_show_popup = &PostShowPopup;
    h.post_popup_closed = &PostPopupClosed;
    h.post_initiate = &PostInitiate;
    h.hide_mission_status_panel = &HideMissionStatusPanel;
    h.end_mysim_dialog = &EndMysimDialog;
    h.show_popup = &ShowPopup;
    h.next_target = &NextTarget;
    h.advice_accepts_group = &AdviceAcceptsGroup;
    h.advice_groups = &AdviceGroups;
    h.advice_priority = &AdvicePriority;
    h.group_automata = &GroupAutomata;
    h.create_automaton_for_player_drive = &CreateAutomatonForPlayerDrive;
    h.remove_automaton = &RemoveAutomaton;
    h.set_automaton_flag = &SetAutomatonFlag;
    h.set_automaton_group = &SetAutomatonGroup;
    h.enter_player_drive_view = &EnterPlayerDriveView;
    h.clear_mission_state = &ClearMissionState;
    h.release_situation_effect = &ReleaseSituationEffect;
    h.acquire_mysim_window = &AcquireMysimWindow;
    return h;
}

}  // namespace

cIGZMessageServer2* cFC4CitySituationManager::Server() { return msgServerHeld ? msgServer_ : nullptr; }
void cFC4CitySituationManager::SetServer(cIGZMessageServer2* ms) { msgServer_ = ms; }
cISC4City* cFC4CitySituationManager::City() { return city_; }
void cFC4CitySituationManager::SetCity(cISC4City* c) {
    if (c == city_) return;
    if (c != nullptr) FC4_ADDREF(c);
    if (city_ != nullptr) FC4_RELEASE(city_);
    city_ = c;
}

// §3: a standard message sent synchronously.
void cFC4CitySituationManager::Send(uint32_t type, uint32_t data1, uint32_t data2, uint64_t object) {
    cIGZMessageServer2* ms = Server();
    if (ms == nullptr) return;
    ++sends_;
    cFC4Message2Standard* msg =
        FC4MakeMessage(type, static_cast<intptr_t>(data1), static_cast<intptr_t>(data2), 0);
    if (object != 0) msg->SetIGZUnknown(reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(object)));
    ms->MessageSend(msg);
    FC4_X87_CHECK("citysit MessageSend");
    msg->Release();
}

// §3: a standard message queued to itself (IGZUnknown = the advice).
void cFC4CitySituationManager::Post(uint32_t type, uint64_t object, uint32_t data1, uint32_t data2) {
    cIGZMessageServer2* ms = Server();
    if (ms == nullptr) return;
    ++sends_;
    cFC4Message2Standard* msg =
        FC4MakeMessage(type, static_cast<intptr_t>(data1), static_cast<intptr_t>(data2), 0);
    if (object != 0) msg->SetIGZUnknown(reinterpret_cast<cIGZUnknown*>(static_cast<uintptr_t>(object)));
    ms->MessagePost(msg, false);  // priority 0
    FC4_X87_CHECK("citysit MessagePost");
    msg->Release();
}

cIGZUnknown* CreateFC4CitySituationManager() {
    return static_cast<cIGZUnknown*>(
        static_cast<cFC4ISC4CitySituationManager*>(new cFC4CitySituationManager()));
}

cFC4CitySituationManager::cFC4CitySituationManager() {
    {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            SetServer(ms);
            FC4_ADDREF(msgServer_);
        }
    }
    host_ = MakeHost(this);
    core_ = fc4_citysit_create(&host_, 0, 0);
    if (core_ == nullptr) fc4::Log("cFC4CitySituationManager: core creation failed");
    fc4::Log("cFC4CitySituationManager(%p) created, core %p", static_cast<void*>(Primary()),
             static_cast<void*>(core_));
}

cFC4CitySituationManager::~cFC4CitySituationManager() {
    fc4_citysit_destroy(core_);
    core_ = nullptr;
    SetCity(nullptr);
    FC4_RELEASE(msgServer_);
    msgServer_ = nullptr;
    fc4::Log("cFC4CitySituationManager destroyed (%u sends/posts)", sends_.load());
}

// §1: 0x6BB1437A at +0, 0x452294AA at +4, 0xE98B2F57 at +8, and 1 /
// 0xE4FDA3D4 at +0xC [quirk: cIGZUnknown gives the serializable sub-object].
bool cFC4CitySituationManager::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    uint32_t offset = 0;
    if (!fc4_citysit_query_interface(riid, &offset)) return false;
    switch (offset) {
        case 0: *ppvObj = Primary(); break;
        case 4: *ppvObj = static_cast<cIGZMessageTarget2*>(this); break;
        case 8: *ppvObj = static_cast<cIGZMessageTarget*>(this); break;
        case 0xC: *ppvObj = static_cast<cIGZSerializable*>(this); break;
        default: return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4CitySituationManager::AddRef() { return ++refCount_; }

// §1: at a count of 1 or less the object is destroyed (the count is not
// decremented first) and 0 returned.
uint32_t cFC4CitySituationManager::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;
    destroying_ = true;
    if (core_ != nullptr) {
        fc4_citysit_state st{};
        st.struct_size = sizeof(st);
        if (fc4_citysit_get_state(core_, &st) && st.initialized != 0) Shutdown();
    }
    delete this;
    return 0;
}

bool cFC4CitySituationManager::Init() {
    FC4_TRACE("CitySituationManager::Init");
    const bool ok = fc4_citysit_init(core_);
    fc4_citysit_state st{};
    st.struct_size = sizeof(st);
    fc4_citysit_get_state(core_, &st);
    fc4::Log("citysit Init: ok=%d city=%p server=%p; status=%u indicators=%u successes=%u", ok ? 1 : 0,
             static_cast<void*>(city_), static_cast<void*>(msgServer_), st.status, st.indicators_visible,
             st.success_counts);
    return true;  // §8 slot 3 returns AL (1)
}

bool cFC4CitySituationManager::Shutdown() {
    FC4_TRACE("CitySituationManager::Shutdown");
    if (!destroying_) AddRef();
    fc4_citysit_shutdown(core_);
    fc4_citysit_state st{};
    st.struct_size = sizeof(st);
    fc4_citysit_get_state(core_, &st);
    fc4::Log("citysit Shutdown: status=%u indicators=%u successes=%u; %u sends/posts", st.status,
             st.indicators_visible, st.success_counts, sends_.load());
    if (!destroying_) Release();
    return true;
}

// ---- the interface, forwarded (§8) -------------------------------------------------------------------

uint16_t cFC4CitySituationManager::GetMissionSuccessCount(uint32_t guid) {
    return fc4_citysit_get_success_count(core_, guid);  // slot 5 returns AX
}

cISC4CitySituationAdvice* cFC4CitySituationManager::GetActiveSituation() {
    return reinterpret_cast<cISC4CitySituationAdvice*>(
        static_cast<uintptr_t>(fc4_citysit_active_situation(core_)));
}

uint32_t cFC4CitySituationManager::GetCurrentStatus() { return fc4_citysit_current_status(core_); }

cISC4Automaton* cFC4CitySituationManager::GetActiveAuto() {
    return reinterpret_cast<cISC4Automaton*>(static_cast<uintptr_t>(fc4_citysit_active_auto(core_)));
}

cISC4Occupant* cFC4CitySituationManager::GetCurrentTarget() {
    return reinterpret_cast<cISC4Occupant*>(static_cast<uintptr_t>(fc4_citysit_current_target(core_)));
}

uint32_t cFC4CitySituationManager::GetCurrentTargetIndex() { return fc4_citysit_current_target_index(core_); }
uint32_t cFC4CitySituationManager::GetNumTargets() { return fc4_citysit_num_targets(core_); }
uint32_t cFC4CitySituationManager::GetNumTargetsRemaining() { return fc4_citysit_num_targets_remaining(core_); }

// §8: slots 13-15 return float in ST0 (the C getters do too).
float cFC4CitySituationManager::GetCurrentTimeLimitSeconds() { return fc4_citysit_time_limit_seconds(core_); }
float cFC4CitySituationManager::GetCurrentSituationTime() { return fc4_citysit_situation_time(core_); }
float cFC4CitySituationManager::GetCurrentTimeRemainingSeconds() {
    return fc4_citysit_time_remaining_seconds(core_);
}

void cFC4CitySituationManager::GetCurrentTargetName(cIGZString& out) {
    // Slot 16 fills the caller's string (EAX = the argument). The name comes
    // from the advice's target through the host, which this replacement does
    // not wire: the string is left as it is.
    (void)out;
}

bool cFC4CitySituationManager::GetSituationAura(float& radius, float& magnitude) {
    float r = 0, m = 0;
    const bool ok = fc4_citysit_situation_aura(core_, &r, &m);
    if (ok) {
        radius = r;
        magnitude = m;
    }
    return ok;
}

bool cFC4CitySituationManager::GetSituationMoney(int64_t& money) {
    int64_t v = 0;
    const bool ok = fc4_citysit_situation_money(core_, &v);
    if (ok) money = v;
    return ok;
}

bool cFC4CitySituationManager::IsSituationAvailable(cISC4Automaton* automaton) {
    return fc4_citysit_is_situation_available(core_, fc4::TokenOf(automaton));
}
bool cFC4CitySituationManager::IsSituationAvailable(cISC4Occupant* occupant) {
    return fc4_citysit_is_situation_available(core_, fc4::TokenOf(occupant));
}

bool cFC4CitySituationManager::InitiateSituationByGuid(uint32_t guid, bool showBriefing, uint32_t group) {
    return fc4_citysit_initiate_situation_by_guid(core_, guid, showBriefing, group);
}
bool cFC4CitySituationManager::InitiateSituationForOccupant(cISC4Occupant* occ, cISC4CitySituationAdvice* a) {
    return fc4_citysit_initiate_situation_for_occupant(core_, fc4::TokenOf(occ), fc4::TokenOf(a));
}
bool cFC4CitySituationManager::InitiateSituationByAutomataGroup(uint32_t group, bool showBriefing, bool search) {
    return fc4_citysit_initiate_situation_by_automata_group(core_, group, showBriefing, search);
}
void cFC4CitySituationManager::StartJoyride(cISC4Automaton* automaton, bool b, cIGZMessageTarget2* target) {
    fc4_citysit_start_joyride(core_, fc4::TokenOf(automaton), b, fc4::TokenOf(target));
}
void cFC4CitySituationManager::TreatAutomatonAsGroup(cISC4Automaton* automaton, uint32_t group) {
    fc4_citysit_treat_automaton_as_group(core_, fc4::TokenOf(automaton), group);
}

void cFC4CitySituationManager::SetMaxCSI(uint8_t n) { fc4_citysit_set_max_csi(core_, n); }
uint8_t cFC4CitySituationManager::GetMaxCSI() { return fc4_citysit_max_csi(core_); }
int32_t cFC4CitySituationManager::GetTrackedAutomataCount() {
    return static_cast<int32_t>(fc4_citysit_tracked_automata_count(core_));
}
void cFC4CitySituationManager::SetCSIVisible(bool visible) { fc4_citysit_set_csi_visible(core_, visible); }
bool cFC4CitySituationManager::GetCSIVisible() { return fc4_citysit_csi_visible(core_); }
bool cFC4CitySituationManager::HandleTuningParametersChanged() {
    return fc4_citysit_handle_tuning_parameters_changed(core_);
}
uint32_t cFC4CitySituationManager::CountdownIsInProgress() {
    return fc4_citysit_countdown_in_progress(core_) ? 1u : 0u;  // §8: EAX 0 or 1
}
void cFC4CitySituationManager::ResetTutorials() { fc4_citysit_reset_tutorials(core_); }
void cFC4CitySituationManager::InitiatePlayerDriveMode(bool on) { fc4_citysit_initiate_player_drive_mode(core_, on); }

bool cFC4CitySituationManager::InitiateSituation(cISC4CitySituationAdvice* advice, cISC4Automaton* automaton,
                                                 bool showBriefing) {
    return fc4_citysit_initiate_situation(core_, fc4::TokenOf(advice), fc4::TokenOf(automaton), showBriefing);
}
void cFC4CitySituationManager::ActivateSituation() { fc4_citysit_activate_situation(core_); }
void cFC4CitySituationManager::DeactivateSituation() { fc4_citysit_deactivate_situation(core_); }
void cFC4CitySituationManager::EndSituation() { fc4_citysit_end_situation(core_); }

// ---- messages (§3) ----------------------------------------------------------------------------------------

bool cFC4CitySituationManager::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("citysit DoMessage", pMessage->GetType());
    const uint32_t type = pMessage->GetType();
    uint32_t data1 = 0, data2 = 0, data3 = 0;
    uint64_t object = 0;
    cIGZMessage2Standard* m = nullptr;
    if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) && m != nullptr) {
        FC4_ACQUIRED(m, "QueryInterface");
        data1 = static_cast<uint32_t>(m->GetData1());
        data2 = static_cast<uint32_t>(m->GetData2());
        data3 = static_cast<uint32_t>(m->GetData3());
        object = fc4::TokenOf(m->GetIGZUnknown());
        if (type == kMsgOccupantInserted || type == kMsgOccupantRemoved) {
            object = fc4::TokenOf(reinterpret_cast<void*>(static_cast<uintptr_t>(m->GetData1())));
        }
        FC4_RELEASE(m);
    }
    // The animation tick's elapsed milliseconds travel in data3 as a float
    // bit pattern (§3); the core's tick takes a float.
    // §3: the message may also carry void1 and a string (the target's
    // occupant and the advice's text). Neither is read here.
    return fc4_citysit_do_message(core_, type, data1, data2, data3, object, 0, nullptr, 0);
}

// §1: the +8 sub-object is the shared "do nothing" message target.
bool cFC4CitySituationManager::DoMessage(cGZMessage& sMessage) {
    (void)sMessage;
    return true;
}

bool cFC4CitySituationManager::Write(cIGZOStream& s) {
    FC4_TRACE("CitySituationManager::Write");
    fc4::OStreamAdapter a(s);
    const bool ok = fc4_citysit_write(core_, a.Get());
    fc4_citysit_state st{};
    st.struct_size = sizeof(st);
    fc4_citysit_get_state(core_, &st);
    fc4::Log("citysit Write -> %d (%u successes, indicators=%u)", ok ? 1 : 0, st.success_counts,
             st.indicators_visible);
    return ok;
}

// §7: Read runs before Init; nothing here is sized from the city.
bool cFC4CitySituationManager::Read(cIGZIStream& s) {
    FC4_TRACE("CitySituationManager::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_citysit_read(core_, a.Get());
    fc4_citysit_state st{};
    st.struct_size = sizeof(st);
    fc4_citysit_get_state(core_, &st);
    fc4::Log("citysit Read -> %d (%u successes, indicators=%u)", ok ? 1 : 0, st.success_counts,
             st.indicators_visible);
    return ok;
}

uint32_t cFC4CitySituationManager::GetGZCLSID() { return kFC4_CLSID_cSC4CitySituationManager; }
