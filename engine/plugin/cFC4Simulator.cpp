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

#include "cFC4Simulator.h"

#include "FC4Log.h"

#include "GZServPtrs.h"
#include "SC4String.h"
#include "SC4Vector.h"
#include "cGZPersistResourceKey.h"
#include "cIGZApp.h"
#include "cIGZCheatCodeManager.h"
#include "cIGZCommandServer.h"
#include "cIGZFrameWork.h"
#include "cIGZIStream.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZOStream.h"
#include "cIGZPersistResourceManager.h"
#include "cIGZString.h"
#include "cIGZVariant.h"
#include "cISC4App.h"
#include "cISCProperty.h"
#include "cISCPropertyHolder.h"
#include "cISCResExemplar.h"
#include "cRZBaseString.h"
#include "cFC4Message2Standard.h"
#include "cRZCOMDllDirector.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <new>
#include <string>

namespace {

// Interface IDs (spec §1 and the gzcom-dll SDK).
constexpr uint32_t kIID_cISC4Simulator = 0x8695664E;
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;  // GZCLSID::kcIGZMessageTarget2 in the SDK
constexpr uint32_t kIID_cISC4App = 0x26CE01C0;            // as used by the SDK examples
constexpr uint32_t kIID_cIGZCheatCodeManager = 0xA1085722;  // as used by the SDK examples

// Service (spec §1).
constexpr uint32_t kServiceID = 0x46956659;
constexpr int32_t kServicePriority = -2000000;

// Messages (spec §9).
constexpr uint32_t kMsgPostCityInit = 0x26D31EC1;  // kSC4MessagePostCityInit (spec §9 erratum)
constexpr uint32_t kCmdGetSimulationDate = 0x8BAC4B43;
constexpr uint32_t kMsgCheatIssued = 0x230E27AC;
constexpr uint32_t kMsgSimNewMonth = 0x66956816;

// Cheats (spec §9).
constexpr uint32_t kCheatFPS = 0x0AC07E24;
constexpr uint32_t kCheatSimDate = 0x8A78BEFF;

// Tuning (spec §7).
constexpr uint32_t kTuningType = 0x6534284A;
constexpr uint32_t kTuningGroup = 0xE7E2C2DB;
constexpr uint32_t kTuningInstance = 0xE9590E52;
constexpr uint32_t kPropMsPerDay = 0x89590E6D;
constexpr uint32_t kPropAnimationDilation = 0x89590E6E;

cIGZMessageTarget2* AgentPtr(fc4_agent a) { return reinterpret_cast<cIGZMessageTarget2*>(static_cast<uintptr_t>(a)); }
fc4_agent AgentHandle(cIGZMessageTarget2* p) { return static_cast<fc4_agent>(reinterpret_cast<uintptr_t>(p)); }

std::string ToStd(cIGZString const& s) {
    const char* p = s.ToChar();
    return p ? std::string(p, s.Strlen()) : std::string();
}

// A fresh standard message (foundations §5.1), already AddRef'd once.
cFC4Message2Standard* MakeMessage(const fc4_message& src) {
    return FC4MakeMessage(src.type, static_cast<intptr_t>(src.data1), static_cast<intptr_t>(src.data2),
                          static_cast<intptr_t>(src.data3));
}

}  // namespace

// ---- construction -------------------------------------------------------------

cFC4Simulator::cFC4Simulator() : serviceId_(kServiceID) {
    fc4_host host{};
    host.struct_size = sizeof(fc4_host);
    host.user = this;
    host.now_ms = &cFC4Simulator::HostNowMs;
    host.deliver = &cFC4Simulator::HostDeliver;
    host.add_ref = &cFC4Simulator::HostAddRef;
    host.release = &cFC4Simulator::HostRelease;
    host.post = &cFC4Simulator::HostPost;
    sim_ = fc4_sim_create(&host);
    if (sim_ == nullptr || fc4_abi_version() != FC4_ABI_VERSION) {
        fc4::Log("cFC4Simulator: core creation failed (abi %u, header %u)", fc4_abi_version(), FC4_ABI_VERSION);
    }
}

cFC4Simulator::~cFC4Simulator() {
    fc4::Log("cFC4Simulator destroyed");
    fc4_sim_destroy(sim_);  // releases any agents still registered
    sim_ = nullptr;
}

// ---- cIGZUnknown ----------------------------------------------------------------

bool cFC4Simulator::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    switch (riid) {
        case kIID_cISC4Simulator:
            *ppvObj = static_cast<cISC4Simulator*>(this);
            break;
        case GZIID_cIGZSerializable:
            *ppvObj = static_cast<cIGZSerializable*>(this);
            break;
        case GZIID_cIGZSystemService:
            *ppvObj = static_cast<cIGZSystemService*>(this);
            break;
        case kIID_cIGZMessageTarget2:
            *ppvObj = static_cast<cIGZMessageTarget2*>(this);
            break;
        case GZIID_cIGZUnknown:
            *ppvObj = static_cast<cIGZUnknown*>(static_cast<cISC4Simulator*>(this));
            break;
        default:
            *ppvObj = nullptr;
            return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4Simulator::AddRef() { return ++refCount_; }

uint32_t cFC4Simulator::Release() {
    if (refCount_ > 0) --refCount_;
    if (refCount_ == 0) {
        delete this;
        return 0;
    }
    return refCount_;
}

// ---- host callbacks ------------------------------------------------------------------

uint32_t cFC4Simulator::HostNowMs(void*) {
    static LARGE_INTEGER freq = [] {
        LARGE_INTEGER f;
        QueryPerformanceFrequency(&f);
        return f;
    }();
    LARGE_INTEGER now;
    QueryPerformanceCounter(&now);
    return static_cast<uint32_t>((static_cast<unsigned long long>(now.QuadPart) * 1000ull) /
                                 static_cast<unsigned long long>(freq.QuadPart));
}

bool cFC4Simulator::HostDeliver(void*, fc4_sim*, fc4_agent agent, const fc4_message* msg) {
    cIGZMessageTarget2* target = AgentPtr(agent);
    if (target == nullptr || msg == nullptr) return true;
    // Synchronous delivery (spec §4) with the simulator's pattern from
    // foundations §5.1: create, AddRef, set, send, Release. An agent that
    // keeps the message (AddRef) keeps it alive.
    cFC4Message2Standard* m = MakeMessage(*msg);
    const bool finished = target->DoMessage(m);
    m->Release();
    return finished;
}

void cFC4Simulator::HostAddRef(void*, fc4_agent agent) {
    if (cIGZMessageTarget2* t = AgentPtr(agent)) t->AddRef();
}

void cFC4Simulator::HostRelease(void*, fc4_agent agent) {
    if (cIGZMessageTarget2* t = AgentPtr(agent)) t->Release();
}

void cFC4Simulator::HostPost(void* user, const fc4_message* msg) {
    if (user != nullptr && msg != nullptr) static_cast<cFC4Simulator*>(user)->Post(*msg);
}

void cFC4Simulator::Post(const fc4_message& msg) {
    if (msg.type == kMsgSimNewMonth && daysLogged_ < 1000) {
        ++daysLogged_;
        fc4_date d{};
        fc4_date_from_julian(static_cast<int32_t>(msg.data1), &d);
        fc4::Log("SimNewMonth %04d-%02d-%02d (julian %d)", d.year, d.month, d.day_of_month, d.julian);
    }
    if (msgServer_ == nullptr) return;
    // Queued broadcast (spec §5.2): the queue holds its own reference (foundations §5.1).
    cFC4Message2Standard* m = MakeMessage(msg);
    msgServer_->MessagePost(m, false);
    m->Release();
}

// ---- lifecycle (spec §9) ---------------------------------------------------------------

fc4_tuning cFC4Simulator::ReadTuning(uint32_t (&msPerDay)[3], size_t& msCount, float (&dilation)[5], size_t& dilCount) {
    fc4_tuning t{};
    msCount = 0;
    dilCount = 0;
    cIGZPersistResourceManagerPtr rm;
    if (!rm) {
        fc4::Log("tuning: no resource manager; using defaults");
        return t;
    }
    cGZPersistResourceKey key(kTuningType, kTuningGroup, kTuningInstance);
    cISCResExemplar* ex = nullptr;
    if (!rm->GetResource(key, GZIID_cISCResExemplar, reinterpret_cast<void**>(&ex), 0, nullptr) || ex == nullptr) {
        fc4::Log("tuning: exemplar %08X-%08X-%08X not found; using defaults", kTuningType, kTuningGroup, kTuningInstance);
        return t;
    }
    const cISCPropertyHolder* ph = ex->AsISCPropertyHolder();
    if (ph != nullptr) {
        if (cISCProperty* p = ph->GetProperty(kPropMsPerDay)) {
            const cIGZVariant* v = p->GetPropertyValue();
            const uint16_t type = v ? v->GetType() : 0;
            const uint32_t n = v ? v->GetCount() : 0;
            const uint32_t* src = nullptr;
            if (type == cIGZVariant::Uint32Array) src = v->RefUint32();
            if (type == cIGZVariant::Sint32Array) src = reinterpret_cast<const uint32_t*>(v->RefSint32());
            // Spec §7: used only with exactly 3 values (any other count keeps
            // the defaults, which is what passing "missing" does).
            if (src != nullptr && n == 3) {
                for (uint32_t i = 0; i < 3; ++i) msPerDay[i] = src[i];
                msCount = 3;
                t.ms_per_day = msPerDay;
                t.ms_per_day_count = 3;
            }
            fc4::Log("tuning: msPerDay property type 0x%04X count %u", type, n);
        }
        if (cISCProperty* p = ph->GetProperty(kPropAnimationDilation)) {
            const cIGZVariant* v = p->GetPropertyValue();
            const uint16_t type = v ? v->GetType() : 0;
            const uint32_t n = v ? v->GetCount() : 0;
            // [choice] used only with exactly 5 values.
            if (type == cIGZVariant::Float32Array && v->RefFloat32() != nullptr && n == 5) {
                for (uint32_t i = 0; i < n; ++i) dilation[i] = v->RefFloat32()[i];
                dilCount = n;
                t.animation_dilation = dilation;
                t.animation_dilation_count = n;
            }
            fc4::Log("tuning: dilation property type 0x%04X count %u", type, n);
        }
    }
    ex->Release();
    return t;
}

bool cFC4Simulator::Init() {
    if (sim_ == nullptr) return false;
    if (fc4_sim_is_initialized(sim_)) return true;

    // Step 3: notifications. (The message server is acquired first so the
    // SpeedChanged post of step 6 can be broadcast.)
    cIGZMessageServer2Ptr ms;
    if (ms) {
        msgServer_ = ms;
        msgServer_->AddRef();
        msgServer_->AddNotification(static_cast<cIGZMessageTarget2*>(this), kMsgPostCityInit);
        postCityInitSubscribed_ = true;
        msgServer_->AddNotification(static_cast<cIGZMessageTarget2*>(this), kCmdGetSimulationDate);
    } else {
        fc4::Log("Init: no message server");
    }

    // Step 4: command registration. [choice] empty parameter description.
    cIGZCommandServerPtr cs;
    if (cs) commandRegistered_ = cs->RegisterCommand(kCmdGetSimulationDate, "GetSimulationDate", "");

    // Steps 1, 2, 5, 6, 7 (core).
    uint32_t msPerDay[3] = {};
    float dilation[5] = {};
    size_t msCount = 0, dilCount = 0;
    const fc4_tuning tuning = ReadTuning(msPerDay, msCount, dilation, dilCount);
    const bool ok = fc4_sim_init(sim_, &tuning);

    // Step 8: framework registration.
    if (cIGZFrameWork* fw = RZGetFrameWork()) {
        fw->AddSystemService(static_cast<cIGZSystemService*>(this));
        fw->AddToTick(static_cast<cIGZSystemService*>(this));
        addedToFramework_ = true;
    }
    serviceRunning_ = true;

    // Step 9: cheats.
    RegisterCheats();

    fc4_state st{};
    st.struct_size = sizeof(st);
    fc4_sim_get_state(sim_, &st);
    fc4::Log("Init: ok=%d speed=%d msPerDay=%u fps=%d julian=%d table={%u,%u,%u} cheats=%s command=%s", ok ? 1 : 0,
             st.sim_speed, st.ms_per_day, st.target_fps, st.julian_day, st.ms_per_day_table[0], st.ms_per_day_table[1],
             st.ms_per_day_table[2], cheatMgr_ ? "yes" : "no", commandRegistered_ ? "yes" : "no");
    return ok;
}

bool cFC4Simulator::Shutdown() {
    if (sim_ == nullptr || !fc4_sim_is_initialized(sim_)) return true;
    AddRef();  // the framework may drop its reference while we unregister

    UnregisterCheats();          // step 1
    fc4_sim_shutdown(sim_);      // steps 2, 3, 7

    if (msgServer_ != nullptr) {  // step 4
        if (postCityInitSubscribed_) {
            msgServer_->RemoveNotification(static_cast<cIGZMessageTarget2*>(this), kMsgPostCityInit);
            postCityInitSubscribed_ = false;
        }
        msgServer_->RemoveNotification(static_cast<cIGZMessageTarget2*>(this), kCmdGetSimulationDate);
        msgServer_->Release();
        msgServer_ = nullptr;
    }
    if (commandRegistered_) {  // step 5
        cIGZCommandServerPtr cs;
        if (cs) cs->UnregisterCommand(kCmdGetSimulationDate);
        commandRegistered_ = false;
    }
    if (addedToFramework_) {  // step 6
        if (cIGZFrameWork* fw = RZGetFrameWork()) {
            fw->RemoveFromTick(static_cast<cIGZSystemService*>(this));
            fw->RemoveSystemService(static_cast<cIGZSystemService*>(this));
        }
        addedToFramework_ = false;
    }
    serviceRunning_ = false;
    fc4::Log("Shutdown");
    Release();  // may delete this; nothing may touch members afterwards
    return true;
}

void cFC4Simulator::RegisterCheats() {
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (app == nullptr || !app->QueryInterface(kIID_cISC4App, reinterpret_cast<void**>(&sc4)) || sc4 == nullptr) return;
    cIGZCheatCodeManager* cm = sc4->GetCheatCodeManager();
    if (cm != nullptr && cm->QueryInterface(kIID_cIGZCheatCodeManager, reinterpret_cast<void**>(&cheatMgr_))) {
        cheatMgr_->RegisterCheatCode(kCheatFPS, cRZBaseString("FPS"));
        if (sc4->GetDebugFunctionalityEnabled()) {
            cheatMgr_->RegisterCheatCode(kCheatSimDate, cRZBaseString("SimDate"));
            simDateCheatRegistered_ = true;
        }
        // [choice] One subscription covers both cheats: the SDK documents the
        // message-ID argument as unused, so subscribing twice could double-deliver.
        cheatMgr_->AddNotification2(static_cast<cIGZMessageTarget2*>(this), 0);
    }
    sc4->Release();
}

void cFC4Simulator::UnregisterCheats() {
    if (cheatMgr_ == nullptr) return;
    cheatMgr_->UnregisterCheatCode(kCheatFPS);
    if (simDateCheatRegistered_) cheatMgr_->UnregisterCheatCode(kCheatSimDate);
    simDateCheatRegistered_ = false;
    // [choice] Also unsubscribe so the manager never calls a dead object.
    cheatMgr_->RemoveNotification2(static_cast<cIGZMessageTarget2*>(this), 0);
    cheatMgr_->Release();
    cheatMgr_ = nullptr;
}

// ---- date -----------------------------------------------------------------------------------

bool cFC4Simulator::GetSimStartDate(cIGZDate& sDate) {
    // [choice] The spec does not define the start date; we report the
    // initial simulation date, 2000-01-01.
    return sDate.Set(1u, 1u, 2000u);
}

cIGZDate* cFC4Simulator::GetSimDate() {
    // [choice] Returns an object owned by the simulator (no AddRef), refreshed
    // on every call.
    date_.SetJulian(fc4_sim_get_julian_day(sim_));
    return &date_;
}

void cFC4Simulator::GetSimDate(uint32_t* year, uint32_t* month, uint32_t* day, uint32_t* dayOfYear, uint32_t* weekDay) {
    fc4_date d{};
    fc4_sim_get_date(sim_, &d);
    if (year) *year = static_cast<uint32_t>(d.year);
    if (month) *month = static_cast<uint32_t>(d.month);
    if (day) *day = static_cast<uint32_t>(d.day_of_month);
    if (dayOfYear) *dayOfYear = static_cast<uint32_t>(d.day_of_year);
    if (weekDay) *weekDay = static_cast<uint32_t>(d.weekday);
}

int32_t cFC4Simulator::GetSimDateNumber() { return fc4_sim_get_julian_day(sim_); }

// ---- pause and speed (spec §6) -------------------------------------------------------------

bool cFC4Simulator::Pause() { return fc4_sim_pause(sim_); }
bool cFC4Simulator::HiddenPause() { return fc4_sim_hidden_pause(sim_); }
bool cFC4Simulator::EmergencyPause() { return fc4_sim_emergency_pause(sim_); }
bool cFC4Simulator::Resume() { return fc4_sim_resume(sim_); }
bool cFC4Simulator::HiddenResume() { return fc4_sim_hidden_resume(sim_); }
bool cFC4Simulator::EmergencyResume() { return fc4_sim_emergency_resume(sim_); }
bool cFC4Simulator::IsPaused() { return fc4_sim_is_paused(sim_); }
bool cFC4Simulator::IsHiddenPaused() { return fc4_sim_is_hidden_paused(sim_); }
bool cFC4Simulator::IsEmergencyPaused() { return fc4_sim_is_emergency_paused(sim_); }
bool cFC4Simulator::IsAnyPaused() { return fc4_sim_is_any_paused(sim_); }
int32_t cFC4Simulator::GetSimSpeed() { return fc4_sim_get_sim_speed(sim_); }

bool cFC4Simulator::SetSimSpeed(int32_t lSpeed) {
    const bool ok = fc4_sim_set_sim_speed(sim_, lSpeed);
    fc4::Log("SetSimSpeed(%d) -> %d", lSpeed, ok ? 1 : 0);
    return ok;
}

int32_t cFC4Simulator::GetSimTime() { return fc4_sim_get_sim_time(sim_); }
bool cFC4Simulator::SetSimTime(int32_t lTime) { return fc4_sim_set_sim_time(sim_, lTime); }
bool cFC4Simulator::SetMaxMillisecondsPerTick(uint32_t dwTime) { return fc4_sim_set_max_milliseconds_per_tick(sim_, dwTime); }
float cFC4Simulator::GetAnimationTimeDilation() { return fc4_sim_get_animation_time_dilation(sim_); }
bool cFC4Simulator::SetCityEstablished(bool bEstablished) { return fc4_sim_set_city_established(sim_, bEstablished); }

// ---- agents (spec §4) --------------------------------------------------------------------------

bool cFC4Simulator::AddAgent(cIGZMessageTarget2* pAgent, eAgentType agentType, cIGZString const& szAgentName,
                             eAgentFlags flags) {
    if (pAgent == nullptr) return false;  // [choice]
    const std::string name = ToStd(szAgentName);
    return fc4_sim_add_agent(sim_, AgentHandle(pAgent), static_cast<uint32_t>(agentType), name.data(), name.size(),
                             static_cast<uint32_t>(flags));
}

bool cFC4Simulator::RemoveAgent(cIGZMessageTarget2* pAgent, eAgentType agentType) {
    return fc4_sim_remove_agent(sim_, AgentHandle(pAgent), static_cast<uint32_t>(agentType));
}

bool cFC4Simulator::RemoveAgent(cIGZMessageTarget2* pAgent) {
    return fc4_sim_remove_agent_all_levels(sim_, AgentHandle(pAgent));
}

bool cFC4Simulator::RemoveAllAgents() { return fc4_sim_remove_all_agents(sim_); }

bool cFC4Simulator::RemoveAllAgents(eAgentType agentType) {
    return fc4_sim_remove_all_agents_at(sim_, static_cast<uint32_t>(agentType));
}

namespace {
void AppendName(void* ctx, const char* name, size_t len) {
    auto* v = static_cast<SC4Vector<SC4String>*>(ctx);
    // SC4Vector::push_back copies the object bitwise, so build the string in
    // raw storage and hand its ownership to the vector (no destructor here).
    alignas(SC4String) unsigned char storage[sizeof(SC4String)];
    SC4String* s = new (storage) SC4String(name, static_cast<uint32_t>(len));
    v->push_back(*s);
}
}  // namespace

bool cFC4Simulator::EnumerateAgentsByName(SC4Vector<SC4String>& sAgents) {
    fc4_sim_enumerate_agents(sim_, &AppendName, &sAgents);
    return true;
}

bool cFC4Simulator::GetAgentEnabled(cIGZString const& szAgentName) {
    const std::string name = ToStd(szAgentName);
    return fc4_sim_get_agent_enabled(sim_, name.data(), name.size());
}

bool cFC4Simulator::SetAgentEnabled(cIGZString const& szAgentName, bool bEnabled) {
    const std::string name = ToStd(szAgentName);
    return fc4_sim_set_agent_enabled(sim_, name.data(), name.size(), bEnabled);
}

// ---- persistence (spec §8) -------------------------------------------------------------------------

bool cFC4Simulator::Write(cIGZOStream& gzOut) {
    uint8_t buf[256];
    const size_t size = fc4_sim_save(sim_, buf, sizeof(buf));
    const uint8_t* widths = nullptr;
    const size_t count = fc4_record_layout(static_cast<uint16_t>(buf[0] | (buf[1] << 8)), &widths);
    if (size > sizeof(buf) || count == 0) return false;
    size_t pos = 0;
    for (size_t i = 0; i < count && pos + widths[i] <= size; ++i) {
        switch (widths[i]) {
            case 1: gzOut.SetUint8(buf[pos]); break;
            case 2: gzOut.SetUint16(static_cast<uint16_t>(buf[pos] | (buf[pos + 1] << 8))); break;
            case 4:
                gzOut.SetUint32(static_cast<uint32_t>(buf[pos]) | (static_cast<uint32_t>(buf[pos + 1]) << 8) |
                                (static_cast<uint32_t>(buf[pos + 2]) << 16) | (static_cast<uint32_t>(buf[pos + 3]) << 24));
                break;
            default: return false;
        }
        pos += widths[i];
    }
    const bool ok = pos == size && gzOut.GetError() == 0;
    fc4::Log("Write: %u bytes, ok=%d", static_cast<unsigned>(size), ok ? 1 : 0);
    return ok;
}

bool cFC4Simulator::Read(cIGZIStream& gzIn) {
    uint16_t version = 0;
    if (!gzIn.GetUint16(version)) return false;
    const uint8_t* widths = nullptr;
    const size_t count = fc4_record_layout(version, &widths);
    if (count == 0) {
        fc4::Log("Read: unsupported simulator record version %u", version);
        return false;
    }
    uint8_t buf[256];
    size_t pos = 0;
    buf[pos++] = static_cast<uint8_t>(version & 0xFF);
    buf[pos++] = static_cast<uint8_t>(version >> 8);
    for (size_t i = 1; i < count; ++i) {
        uint32_t v = 0;
        switch (widths[i]) {
            case 1: { uint8_t x = 0; if (!gzIn.GetUint8(x)) return false; v = x; break; }
            case 2: { uint16_t x = 0; if (!gzIn.GetUint16(x)) return false; v = x; break; }
            case 4: { if (!gzIn.GetUint32(v)) return false; break; }
            default: return false;
        }
        for (uint8_t b = 0; b < widths[i]; ++b) buf[pos++] = static_cast<uint8_t>(v >> (8 * b));
    }
    const int32_t r = fc4_sim_load(sim_, buf, pos);
    fc4::Log("Read: version %u, %u bytes, result %d, julian %d", version, static_cast<unsigned>(pos), r,
             fc4_sim_get_julian_day(sim_));
    return r == FC4_OK && gzIn.GetError() == 0;
}

uint32_t cFC4Simulator::GetGZCLSID() { return kFC4_CLSID_cSC4Simulator; }

// ---- cIGZSystemService ---------------------------------------------------------------------------------

uint32_t cFC4Simulator::GetServiceID() { return serviceId_; }

cIGZSystemService* cFC4Simulator::SetServiceID(uint32_t dwServiceId) {
    serviceId_ = dwServiceId;
    return static_cast<cIGZSystemService*>(this);
}

int32_t cFC4Simulator::GetServicePriority() { return kServicePriority; }
bool cFC4Simulator::IsServiceRunning() { return serviceRunning_; }

cIGZSystemService* cFC4Simulator::SetServiceRunning(bool bRunning) {
    serviceRunning_ = bRunning;
    return static_cast<cIGZSystemService*>(this);
}

bool cFC4Simulator::OnTick(uint32_t) { return fc4_sim_on_tick(sim_); }
bool cFC4Simulator::OnIdle(uint32_t) { return true; }  // [choice] no idle-tick work
int32_t cFC4Simulator::GetServiceTickPriority() { return kServicePriority; }  // [choice]

// ---- cIGZMessageTarget2 (spec §9) -----------------------------------------------------------------------

bool cFC4Simulator::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr) return true;
    switch (pMessage->GetType()) {
        case kMsgPostCityInit:
            // Spec §9 (erratum): the city finished Init. Unsubscribe and
            // release the hidden pause set by Init so the clock starts.
            // [choice] A copy that was already queued when we unsubscribed is
            // ignored, so the hidden pause is released only once per Init.
            if (postCityInitSubscribed_) {
                postCityInitSubscribed_ = false;
                if (msgServer_ != nullptr) {
                    msgServer_->RemoveNotification(static_cast<cIGZMessageTarget2*>(this), kMsgPostCityInit);
                }
                fc4_sim_on_post_city_init(sim_);
                fc4::Log("PostCityInit: hidden pause released (hidden=%d)", fc4_sim_is_hidden_paused(sim_) ? 1 : 0);
            }
            break;

        case kCmdGetSimulationDate:
            // TODO(spec §9): fill the command's output parameters "year", "month",
            // "day of month", "day of week", "day of year" and "number". The SDK
            // does not describe how a command message carries its parameter
            // sets, nor how parameter names map to cIGZCommandParameterSet IDs,
            // so the values (fc4_sim_get_date) cannot be returned yet.
            break;

        case kMsgCheatIssued: {
            cIGZMessage2Standard* m = nullptr;
            if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) && m != nullptr) {
                const uint32_t id = static_cast<uint32_t>(m->GetData1());
                const cIGZString* text = static_cast<const cIGZString*>(m->GetVoid2());
                const std::string s = text ? ToStd(*text) : std::string();
                if (id == kCheatFPS || id == kCheatSimDate) {
                    fc4_sim_handle_cheat(sim_, id, s.data(), s.size());
                    fc4::Log("cheat %08X '%s'", id, s.c_str());
                }
                m->Release();
            }
            break;
        }

        default:
            break;
    }
    return true;
}
