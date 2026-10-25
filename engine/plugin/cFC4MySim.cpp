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
#include "FC4MySimInterface.h"
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
#include "SC4String.h"
#include "cGZPersistResourceKey.h"
#include <cstdio>

#include <cstring>
#include <map>
#include <vector>

namespace {

class MySimFront final : public cFC4Advisor, public fc4::MySimPublic {
public:
    explicit MySimFront(fc4_adv_obj* h) : cFC4Advisor(h) {}
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        if (iid == 0x0A1DA42F) {
            *out = static_cast<fc4::MySimPublic*>(this);
            AddRef();
            return true;
        }
        if (iid == 0x8BC01FFE) iid = 1; // §7.3 class interface
        return cFC4Advisor::QueryInterface(iid, out);
    }
    uint32_t AddRef() override { return AddRefFront(); }
    uint32_t Release() override { return ReleaseFront(); }
    void* AsAdvisor() override { return static_cast<cFC4ISC4Advisor*>(this); }
    void MoveIn(uint8_t slot, cIGZString& name, bool male, uint8_t sign,
                SC4String& bio, uint32_t bitmap, SC4String& path) override {
        Pin h(*this);
        if (h) fc4_adv_mysim_move_in(h.get(), slot, Bytes(name), name.Strlen(), male, sign,
            Bytes(bio), bio.Strlen(), bitmap, Bytes(path), path.Strlen());
    }
    bool FindNewResidence() override { return fc4_mysim_find_new_residence(Core(), GetID()); }
    bool ChangeResidence(cISC4BuildingOccupant* b) override { return fc4_mysim_change_residence(Core(), GetID(), Tok(b)); }
    bool TestForResidence(cISC4BuildingOccupant* b, bool f, bool w) override {
        return fc4_mysim_test_for_residence(Core(), GetID(), Tok(b), f, w);
    }
    bool ChangeJob(cISC4BuildingOccupant* b) override { return fc4_mysim_change_job(Core(), GetID(), Tok(b)); }
    bool DeleteFromGame() override { return fc4_mysim_delete_from_game(Core(), GetID()); }
    uint8_t GetSlot() override { return State().slot; }
    cISC4BuildingOccupant* GetResidence() override { return Building(State().residence); }
    cISC4BuildingOccupant* GetJob() override { return Building(State().job); }
    bool SetResidence(cISC4BuildingOccupant* b) override { return fc4_mysim_set_residence(Core(), GetID(), Tok(b)); }
    bool GetLocation(float* p) override {
        if (!p) return false;
        const auto s = State();
        std::memcpy(p, s.home_position, sizeof(s.home_position));
        return s.residence != 0;
    }
    void GetVehicleKey(cGZPersistResourceKey& k) override { const auto s = State(); std::memcpy(&k, &s.vehicle_key, 12); }
    uint8_t GetSign() override { return State().sign; }
    uint8_t GetAge() override { return State().age; }
    bool IsMale() override { return State().is_male != 0; }
    uint32_t GetWealth() override { return State().wealth; }
    bool IsActive() override { return State().state != 0; }
    SC4String* GetBio() override { return Text(0, bio_); }
    uint32_t GetBitmapID() override { return State().bitmap_id; }
    SC4String* GetBitmapFile() override { return Text(1, path_); }
    bool GetBitmapBuffer(cIGZBuffer** out) override { if (out) *out = nullptr; return false; }
    // SPEC-GAP: localized UI labels and portrait-file decoding are unspecified; empty labels/buffer.
    SC4String* GetHomeName() override { return &empty_; }
    SC4String* GetJobName() override { return &empty_; }
    void GetBuildingJobTitle(cISC4BuildingOccupant*, cIGZString& out) override { out.FromChar(""); }
    SC4String* GetJobTitle() override { return &empty_; }
    bool GetEQAsText(cIGZString& out) override { return Number(out, State().eq); }
    bool GetHQAsText(cIGZString& out) override { return Number(out, State().hq); }
    // mysims §0 delegates camera, pylons, audio and balloon rendering to UI owners.
    void Find() override {}
    void ShowResidence() override {}
    void HideResidence() override {}
    void ShowLocation() override {}
    void HideLocation() override {}
    void SetPylonVisibility(bool) override {}
    void UpdateResidence() override { HCommand("update_residence"); }
    void LeaveForWork() override { HCommand("leave_for_work"); }
    void ReturnHome() override { HCommand("return_home"); }
    void AnnounceMoveIn() override { fc4_mysim_announce_move_in(Core(), GetID()); }
    void DoScriptCommand(uint32_t cmd) override { fc4_mysim_do_script_command(Core(), GetID(), cmd); }
    void DoBuildingAdded(cISC4BuildingOccupant* b) override { Notify(42, Tok(b)); }
    void DoBuildingChanged(cISC4BuildingOccupant* b) override { Notify(43, Tok(b)); }
    void DoBuildingRemoved(cISC4BuildingOccupant* b, uint32_t c) override { Notify(44, Tok(b), c); }
    void DoFireAdded(float x, float z) override { Notify(45, 0, 0, x, z); }
    void DoPipeBurst(float x, float z) override { Notify(46, 0, 0, x, z); }
    void DoTornado(float x, float z) override { Notify(47, 0, 0, x, z); }
    void DoDisaster(uint32_t t, int32_t x, int32_t z) override { Notify(48, 0, t, static_cast<float>(x), static_cast<float>(z)); }
    void DoNeighborConnectionRemoved(int32_t, int32_t) override { Notify(49); }
    uint32_t DistanceToClosest(uint32_t g) override { return fc4_mysim_distance_to_closest(Core(), GetID(), g); }
    uint32_t DistanceToClosestSpecial(uint32_t g) override { return fc4_mysim_distance_to_closest_special(Core(), GetID(), g); }
    SC4Percentage* LocalSchoolGrade(SC4Percentage* out, uint32_t g) override {
        out->percentage = fc4_mysim_local_grade(Core(), GetID(), g, false); return out;
    }
    SC4Percentage* LocalHospitalGrade(SC4Percentage* out, uint32_t g) override {
        out->percentage = fc4_mysim_local_grade(Core(), GetID(), g, true); return out;
    }
    void PlayNightSound() override {}
    bool GetCurrentScreenLocationXY(int32_t*, int32_t*) override { return false; }
    bool RequestToPlaySound() override { return false; }
    bool SetCar(const cISCResExemplar*) override { return false; }
    bool SetPedestrian(const cISCResExemplar*) override { return false; }
    uint32_t GetModelButtonBitmapID(bool) override { return GetBitmapID(); }
    void GetNextBalloonAdvice() override {}
    void GetNextBalloonEventAdvice() override {}
    uint32_t GetCurrentBalloonAdvice() override { return 0; }
    uint32_t GetCurrentBalloonEventAdvice() override { return 0; }
    bool CanUDriveIt() override { return false; }
    bool EnterUDriveIt() override { return false; }
    bool ExitUDriveIt() override { return false; }
    bool CreateDispatchTool() override { return false; }
    bool ActivateLuaContext() override;
private:
    static const uint8_t* Bytes(cIGZString& s) { return reinterpret_cast<const uint8_t*>(s.ToChar()); }
    static fc4_advisors* Core() { return fc4::CurrentAdvisorCore(); }
    static uint64_t Tok(cISC4BuildingOccupant* p) { return fc4::TokenOf(p); }
    static cISC4BuildingOccupant* Building(uint64_t t) { return reinterpret_cast<cISC4BuildingOccupant*>(fc4::FromToken(t)); }
    fc4_mysim_state State() { fc4_mysim_state s{}; Pin h(*this); if (h) fc4_adv_mysim_get_state(h.get(), &s); return s; }
    SC4String* Text(uint32_t which, SC4String& out) {
        Pin h(*this);
        const size_t n = h ? fc4_adv_mysim_text(h.get(), which, nullptr, 0) : 0;
        std::string text(n + 1, '\0');
        if (h) fc4_adv_mysim_text(h.get(), which, &text[0], text.size());
        out.FromChar(text.data(), static_cast<uint32_t>(n));
        return &out;
    }
    static bool Number(cIGZString& out, float v) { char b[32]; std::snprintf(b, sizeof b, "%d", static_cast<int>(v)); out.FromChar(b); return true; }
    void Notify(uint32_t slot, uint64_t b = 0, uint32_t d = 0, float x = 0, float z = 0) {
        Pin h(*this); if (h) fc4_adv_mysim_notify(h.get(), slot, b, d, x, z);
    }
    void HCommand(const char* command);
    SC4String bio_, path_, empty_;
};

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
    fc4_mysim_state saved{};
    if (MySimState(sim, &saved) && saved.dispatch != 0) {
        auto* u = fc4::FromToken(saved.dispatch);
        if (auto* d = cFC4MySimDispatch::FromUnknown(u)) {
            FC4_ADDREF(static_cast<cFC4IMySimDispatch*>(d));
            d->SetOwner(core, sim);
            d->Init();
            Dispatches()[sim] = d;
            return;
        }
    }
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
    fc4_mysim_set_dispatch(core, sim, fc4::TokenOf(static_cast<cFC4IMySimDispatch*>(d)));
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

void MySimFront::HCommand(const char* command) { HDispatchCommand(nullptr, GetID(), command); }
bool MySimFront::ActivateLuaContext() { return fc4::ActivateMySimLua(GetID()); }

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
    // Keep the handle until the director's QI takes the first reference.
    if (front != nullptr) static_cast<cFC4Advisor*>(static_cast<cFC4ISC4Advisor*>(front))->HoldForFactory(h);
    else fc4_adv_obj_destroy(h);
    return front;
}

}  // namespace

namespace fc4 {

uint64_t CreateMySimFront(fc4_adv_obj* h) { return (new MySimFront(h))->Token(); }

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
    FillMySimLuaHost(h);
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

std::string MySimScriptAction(bool moveIn) {
    fc4_advisors* core = CurrentAdvisorCore();
    if (!core) return "FAILED: no FC4 advisor core";
    bool moved = false;
    uint32_t active = 0;
    for (int32_t id = 1; id <= 5; ++id) {
        fc4_adv_obj* handle = fc4_advisors_advisor_object(core, id);
        cIGZUnknown* front = handle ? FromToken(fc4_adv_obj_token(handle)) : nullptr;
        void* iface = nullptr;
        if (!front || !front->QueryInterface(0x0A1DA42F, &iface)) {
            if (handle) fc4_adv_obj_destroy(handle);
            return "FAILED: missing cISC4MySim";
        }
        auto* sim = static_cast<MySimPublic*>(iface);
        if (moveIn && !moved && !sim->IsActive()) {
            fc4_mysim_host host{};
            FillMySimGameServices(&host);
            const int64_t n = host.all_buildings(nullptr, nullptr, 0);
            std::vector<uint64_t> buildings(n > 0 ? static_cast<size_t>(n) : 0);
            if (!buildings.empty()) host.all_buildings(nullptr, buildings.data(), buildings.size());
            for (uint64_t token : buildings) {
                auto* b = reinterpret_cast<cISC4BuildingOccupant*>(FromToken(token));
                if (!sim->TestForResidence(b, true, false)) continue;
                SC4String name("FC4 Persist"), bio("replace-mysims27"), path("");
                sim->MoveIn(3, name, true, 4, bio, 0, path);
                moved = sim->SetResidence(b);
                break;
            }
        }
        if (sim->IsActive()) {
            ++active;
            char name[256]{};
            fc4_adv_advisor_text(handle, 1, name, sizeof name);
            fc4_mysim_state s{};
            fc4_adv_mysim_get_state(handle, &s);
            Log("mysims persisted: id=%d name=%s slot=%u state=%u age=%u wealth=%u generation=%u "
                "home=%u,%u bio=%s portrait=%u", id, name, s.slot, s.state, s.age, s.wealth,
                s.generation, s.home_cell_x, s.home_cell_z, sim->GetBio()->ToChar(), sim->GetBitmapID());
        }
        sim->Release();
        fc4_adv_obj_destroy(handle);
    }
    if (moveIn && !moved) return "FAILED: no available MySim/residence";
    return "active=" + std::to_string(active) + (moved ? " moved=1" : "");
}

}  // namespace fc4

cIGZUnknown* CreateFC4MySim() { return CreateMySimOf(kFC4_CLSID_cSC4MySim); }
cIGZUnknown* CreateFC4MyStreetSim() { return CreateMySimOf(kFC4_CLSID_cSC4MyStreetSim); }
