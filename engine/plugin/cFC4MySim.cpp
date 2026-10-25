// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

#include "cFC4MySim.h"

#include "FC4AdvisorMessage.h"
#include "FC4Log.h"
#include "FC4MySimGameHost.h"
#include "FC4MySimsMessage.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4AdvisorObjects.h"
#include "cFC4Message2Standard.h"
#include "cFC4MySimDispatch.h"

#include "GZServPtrs.h"
#include "cIGZCOM.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cRZCOMDllDirector.h"

#include <cstring>
#include <map>

namespace {

constexpr uint32_t kIIDUnknown = 0x00000001;
constexpr uint32_t kIIDMessageTarget2 = 0x452294AA;
constexpr uint32_t kMsgAdviceCommand = 0x8A09FA6C;  // §3.2: `unknown` = the advisor, by ID for the core

// One dispatch per MySim (advisor ID), one reference each (§7.2.1).
std::map<int32_t, cFC4MySimDispatch*>& Dispatches() {
    static std::map<int32_t, cFC4MySimDispatch*> m;
    return m;
}

cFC4MySimDispatch* DispatchOf(int32_t sim) {
    auto& m = Dispatches();
    auto it = m.find(sim);
    return it != m.end() ? it->second : nullptr;
}

bool MySimState(int32_t sim, fc4_mysim_state* st) {
    fc4_advisors* core = fc4::CurrentAdvisorCore();
    std::memset(st, 0, sizeof(*st));
    st->struct_size = sizeof(*st);
    return core != nullptr && fc4_mysim_get_state(core, sim, st);
}

// "create" (§7.2.1, mysims.md §4.1): a new dispatch through GZCOM, owned by
// the MySim's advisor, initialised, with the residence as home.
void CreateDispatch(int32_t sim) {
    if (DispatchOf(sim) != nullptr) return;
    fc4_advisors* core = fc4::CurrentAdvisorCore();
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* obj = nullptr;
    if (core == nullptr || com == nullptr ||
        !com->GetClassObject(kFC4_CLSID_cSC4MySimDispatch, kFC4_IID_cSC4MySimDispatch, &obj) || obj == nullptr) {
        fc4::Log("mysims: dispatch for MySim %d not created", sim);
        return;
    }
    cIGZUnknown* u = static_cast<cIGZUnknown*>(obj);
    cFC4MySimDispatch* d = cFC4MySimDispatch::FromUnknown(u);
    if (d == nullptr) {  // another plugin's dispatch class: not ours to drive
        u->Release();
        fc4::Log("mysims: dispatch class 0x%08X is not FC4's; MySim %d has none", kFC4_CLSID_cSC4MySimDispatch, sim);
        return;
    }
    FC4_ACQUIRED(u, "GetClassObject");
    d->SetOwner(core, sim);
    d->Init();
    fc4_mysim_state st{};
    if (MySimState(sim, &st) && st.residence != 0) d->SetHome(st.residence);
    Dispatches()[sim] = d;
}

void ReleaseDispatch(int32_t sim) {
    auto& m = Dispatches();
    auto it = m.find(sim);
    if (it == m.end()) return;
    cFC4MySimDispatch* d = it->second;
    m.erase(it);
    d->Shutdown();
    d->SetOwner(nullptr, 0);
    FC4_RELEASE(static_cast<cFC4IMySimDispatch*>(d));
}

// ---- fc4_mysim_host: the dispatch -----------------------------------------------------------------------

bool HDispatch(void*, int32_t sim, fc4_mysim_dispatch* out) {
    cFC4MySimDispatch* d = DispatchOf(sim);
    return d != nullptr && d->Snapshot(out);
}
bool HFindNewJob(void*, int32_t sim) {
    cFC4MySimDispatch* d = DispatchOf(sim);
    return d != nullptr && d->FindNewJob();
}
bool HChangeJob(void*, int32_t sim, uint64_t building) {
    cFC4MySimDispatch* d = DispatchOf(sim);
    return d != nullptr && d->ChangeJob(building);
}
bool HJobDestination(void*, int32_t sim, fc4_mysim_job_destination* out) {
    cFC4MySimDispatch* d = DispatchOf(sim);
    return d != nullptr && d->JobDestination(out);
}

// §7.2.1: LeaveForWork / ReturnHome issue DispatchCommute with the MySim's
// state and the hour of the clock message (8 / 17). The MySim state the
// original sets on success (5 / 4) stays the core's: the ABI has no setter
// [choice].
void Commute(int32_t sim, int32_t hour) {
    cFC4MySimDispatch* d = DispatchOf(sim);
    fc4_mysim_state st{};
    if (d == nullptr || !MySimState(sim, &st) || st.state == 0) return;
    d->Commute(st.residence != 0, st.state, hour);
}

void HDispatchCommand(void*, int32_t sim, const char* what) {
    if (what == nullptr) return;
    if (std::strcmp(what, "create") == 0) {
        CreateDispatch(sim);
    } else if (std::strcmp(what, "shutdown") == 0) {
        ReleaseDispatch(sim);
    } else if (std::strcmp(what, "start_commute") == 0 || std::strcmp(what, "update_residence") == 0) {
        cFC4MySimDispatch* d = DispatchOf(sim);
        fc4_mysim_state st{};
        if (d != nullptr && MySimState(sim, &st) && st.residence != 0) d->SetHome(st.residence);
    } else if (std::strcmp(what, "leave_for_work") == 0) {
        Commute(sim, 8);
    } else if (std::strcmp(what, "return_home") == 0) {
        Commute(sim, 17);
    }
    // night_sound, move_in_control, ghost, reset_commute: audio and UI (§0), not run.
}

// ---- messages ---------------------------------------------------------------------------------------------

void HPost(void*, const fc4_mysim_post* m) {
    cIGZMessageServer2Ptr ms;
    if (!ms || m == nullptr) return;
    cFC4Message2Standard* msg = fc4::MakeAdvisorMessage(m->msg_type, m->data1, m->data2, fc4::FromToken(m->void1));
    ms->MessagePost(msg, false);
    msg->Release();
}

// The agent's subscriptions (§3.2): one static target forwarding to the core agent.
class AgentForwarder final : public cIGZMessageTarget2 {
public:
    static AgentForwarder& Get() {
        static AgentForwarder t;
        return t;
    }
    bool QueryInterface(uint32_t riid, void** ppvObj) override {
        if (ppvObj == nullptr) return false;
        if (riid == kIIDUnknown || riid == kIIDMessageTarget2) {
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            return true;
        }
        return false;
    }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }

    bool DoMessage(cIGZMessage2* msg) override {
        fc4_advisors* core = fc4::CurrentAdvisorCore();
        cIGZMessage2Standard* m = nullptr;
        if (core == nullptr || msg == nullptr ||
            !msg->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) || m == nullptr) {
            return true;
        }
        fc4_adv_message a{};
        a.msg_type = m->GetType();
        a.data1 = static_cast<uint32_t>(m->GetData1());
        a.data2 = static_cast<uint32_t>(m->GetData2());
        a.data3 = static_cast<uint32_t>(m->GetData3());
        a.data4 = static_cast<uint32_t>(m->GetData4());
        a.void1 = fc4::MySimsOccupantMessageToken(
            a.msg_type, [&] { return static_cast<cIGZUnknown*>(m->GetVoid1()); },
            [](cIGZUnknown* p) { return fc4::TokenOf(p); });
        if (a.msg_type == kMsgAdviceCommand) {
            // The advisor front names its advisor by ID (fc4.h).
            cFC4ISC4Advisor* adv = nullptr;
            cIGZUnknown* u = m->GetIGZUnknown();
            if (u != nullptr && u->QueryInterface(kFC4_IID_cISC4Advisor, reinterpret_cast<void**>(&adv)) && adv) {
                a.unknown = static_cast<uint64_t>(static_cast<uint32_t>(adv->GetID()));
                adv->Release();
            }
        }
        m->Release();
        fc4_mysims_agent_do_message(core, &a);
        return true;
    }
};

void HSubscribe(void*, uint32_t type, bool on) {
    cIGZMessageServer2Ptr ms;
    if (!ms) return;
    cIGZMessageTarget2* t = &AgentForwarder::Get();
    if (on) ms->AddNotification(t, type);
    else ms->RemoveNotification(t, type);
}

// ---- references -------------------------------------------------------------------------------------------

void HAddRef(void*, uint64_t t) {
    if (cIGZUnknown* p = fc4::FromToken(t)) FC4_ADDREF(p);
}
void HRelease(void*, uint64_t t) {
    if (cIGZUnknown* p = fc4::FromToken(t)) FC4_RELEASE(p);
}
uint64_t HResolveRef(void*, uint64_t t, uint32_t iid) {
    cIGZUnknown* p = fc4::FromToken(t);
    void* out = nullptr;
    if (p == nullptr || !p->QueryInterface(iid, &out) || out == nullptr) return 0;
    FC4_ACQUIRED(static_cast<cIGZUnknown*>(out), "QueryInterface");
    return fc4::TokenOf(out);  // one reference for the core
}

// A MySim of the live class store, through its advisor front.
cIGZUnknown* CreateMySimOf(uint32_t clsid) {
    fc4_advisors* core = fc4::CurrentAdvisorCore();
    fc4_adv_obj* h = core != nullptr ? fc4_advisors_create_object(core, clsid) : nullptr;
    if (h == nullptr) return nullptr;
    cIGZUnknown* front = fc4::FromToken(fc4_adv_obj_token(h));
    // [choice] the factory's reference keeps the new core object alive past
    // the handle (a front holds only a weak handle at refcount 0); the
    // caller's QueryInterface adds its own.
    if (front != nullptr) front->AddRef();
    fc4_adv_obj_destroy(h);
    return front;
}

}  // namespace

namespace fc4 {

void FillMySimReplacementHost(fc4_mysim_host* h) {
    std::memset(h, 0, sizeof(*h));
    h->struct_size = sizeof(*h);
    h->user = nullptr;
    // Lua (§4.4): no MySim table; the tuning reads the code defaults (§4.11).
    h->create_table = nullptr;
    h->set_field = nullptr;
    h->activate_lua = nullptr;
    h->tuning = nullptr;
    h->ui_blocked = nullptr;
    h->register_agent_lua = nullptr;
    FillMySimGameServices(h);
    h->dispatch = &HDispatch;
    h->find_new_job = &HFindNewJob;
    h->change_job = &HChangeJob;
    h->job_destination = &HJobDestination;
    h->commute_time = nullptr;        // no trip simulation over the ABI: none
    h->neighbor_city_name = nullptr;  // neighbour jobs: none
    h->dispatch_command = &HDispatchCommand;
    h->post = &HPost;
    h->automata_trigger = nullptr;  // effects (§0)
    h->subscribe = &HSubscribe;
    h->add_ref = &HAddRef;
    h->release = &HRelease;
    h->resolve_ref = &HResolveRef;
}

void MySimReplacementStart(fc4_advisors* core) {
    const bool ok = core != nullptr && fc4_mysims_agent_init(core);
    fc4::Log("mysims: agent Init ok=%d", ok ? 1 : 0);
}

void MySimReplacementStop(fc4_advisors* core) {
    if (core != nullptr) fc4_mysims_agent_shutdown(core);
    while (!Dispatches().empty()) ReleaseDispatch(Dispatches().begin()->first);
}

size_t MySimReplacementDispatchCount() { return Dispatches().size(); }

}  // namespace fc4

cIGZUnknown* CreateFC4MySim() { return CreateMySimOf(kFC4_CLSID_cSC4MySim); }
cIGZUnknown* CreateFC4MyStreetSim() { return CreateMySimOf(kFC4_CLSID_cSC4MyStreetSim); }
