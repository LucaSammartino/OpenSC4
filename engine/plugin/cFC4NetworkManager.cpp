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

#include "cFC4NetworkManager.h"

#include "FC4Diag.h"
#include "FC4Fpu.h"
#include "FC4Log.h"
#include "FC4HistClock.h"
#include "FC4NetMgrHost.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "cFC4Message2Standard.h"
#include "cFC4NetworkTool.h"

#include "GZServPtrs.h"
#include "SC4List.h"
#include "SC4Vector.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cIGZMessage2Standard.h"
#include "cIGZMessageServer2.h"
#include "cISC4App.h"
#include "cISC4City.h"
#include "cISC4NetworkOccupant.h"
#include "cISC4Occupant.h"
#include "cISC4OccupantManager.h"
#include "cRZCOMDllDirector.h"

#include <vector>
#include <new>

namespace {

// §3.1: two of the 11 messages whose payload the class unpacks by hand.
constexpr uint32_t kMsgOccupantInserted = 0x99EF1142;
constexpr uint32_t kMsgOccupantRemoved = 0x99EF1143;
constexpr uint32_t kMsgViewChanged = 0xA6B79602;
// §6.1: the occupant refresh the terrain rule posts.
constexpr uint32_t kMsgOccupantRefresh = 0x99EF1145;

// cIGZMessageTarget2 IID: the SDK header declares no constant (network-manager.md 1).
constexpr uint32_t kIID_cIGZMessageTarget2 = 0x452294AA;

// The host's user is the manager's owner block (FC4NetMgrHost.h NetMgrOwner:
// the shared read callbacks read the user as the NetMgrContext* at its offset
// 0, the write callbacks reach the manager through `mgr`).
cFC4NetworkManager* Self(void* u) {
    return static_cast<cFC4NetworkManager*>(static_cast<fc4::NetMgrOwner*>(u)->mgr);
}

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

cISC4NetworkOccupant* NetOf(uint64_t t) {
    return reinterpret_cast<cISC4NetworkOccupant*>(fc4::FromToken(t));
}
cISC4Occupant* OccOf(uint64_t t) { return reinterpret_cast<cISC4Occupant*>(fc4::FromToken(t)); }

// ---- the write side (the read side is FC4NetMgrHost.cpp) -----------------------------------------------
//
// Wired here (§13's simulation-visible list): the message subscriptions, the
// city, occupant removal and refresh, the retrofit, the crews, the lit flag,
// the occupant flags and the night flag. NOT wired (NULL, so the core reads 0 /
// false / none and says so at Init): the 13 tools and their five callbacks
// (§13: tools are their own subsystem, network-tools.md), the underground
// refit's three tool callbacks, the whole-city `network_occupants` walk and
// `view_rotation_changed` (presentation only).

bool AcquireCity(void* u) {
    cFC4NetworkManager* s = Self(u);
    if (s->City() != nullptr) return true;
    if (cISC4App* app = App()) {
        s->SetCity(app->GetCity());
        FC4_RELEASE(app);
    }
    return s->City() != nullptr;
}

void SubscribeCb(void* u, uint32_t t) {
    cFC4NetworkManager* s = Self(u);
    if (cIGZMessageServer2* ms = s->Server()) ms->AddNotification(s->Target(), t);
}
void UnsubscribeCb(void* u, uint32_t t) {
    cFC4NetworkManager* s = Self(u);
    if (cIGZMessageServer2* ms = s->Server()) ms->RemoveNotification(s->Target(), t);
}

bool UkPaths(void*) {
    // §2.1 / §7.11: the original sets this in Init from its ini resource; the
    // remap tables are presentation, so the replacement reports the default
    // [choice]: the UK path remapping is off.
    return false;
}

bool IsNight(void*) {
    // §6.1 night lighting. The clock is another subsystem's (city slot 31);
    // reporting day keeps SetLit on the day rule [choice].
    return false;
}

void ViewRotationChanged(void*) {
    // §7.11: presentation only; nothing in the simulation reads it.
}

bool OccupantFlagSet(void*, uint64_t occupant, uint32_t flag) {
    return occupant != 0 && OccOf(occupant)->IsFlagSet(flag);  // cISC4Occupant slot 32
}

void SetLit(void*, uint64_t n, bool lit) {
    if (n != 0) NetOf(n)->SetLit(lit);  // slot 61
}

bool CheckForRetrofit(void*, uint64_t n) {
    return n != 0 && NetOf(n)->CheckForRetrofit();  // slot 6
}

void Retrofit(void*, uint64_t n) {
    if (n == 0) return;
    // §7.10: the model work is the renderer's; the flag is what the simulation
    // sees (slot 7).
    NetOf(n)->SetRetrofitFlag(true);
}

uint64_t CreationNumber(void*, uint64_t occupant) {
    // §7.3: the removal queue's order key. The original's is the occupant's
    // creation number; the interfaces the core reaches do not expose one, so
    // the token stands in [choice] (the core needs only a stable total order).
    return occupant;
}

bool RemoveOccupant(void* u, uint64_t occupant) {
    if (occupant == 0) return false;
    if (!AcquireCity(u)) return false;
    cISC4OccupantManager* om = Self(u)->City() ? Self(u)->City()->GetOccupantManager() : nullptr;
    if (om == nullptr) return false;
    const bool ok = om->RemoveOccupant(OccOf(occupant), false, 0);  // slot 21
    FC4_X87_CHECK("netmgr RemoveOccupant");
    return ok;
}

void PostRefresh(void* u, uint64_t n) {
    // §6.1: posts 0x99EF1145 with the occupant.
    cFC4NetworkManager* s = Self(u);
    cIGZMessageServer2* ms = s->Server();
    if (ms == nullptr || n == 0) return;
    cFC4Message2Standard* msg = FC4MakeMessage(kMsgOccupantRefresh, 0, 0, 0);
    msg->SetIGZUnknown(reinterpret_cast<cIGZUnknown*>(fc4::FromToken(n)));
    ms->MessagePost(msg, false);
    FC4_X87_CHECK("netmgr MessagePost");
    msg->Release();
}

void CrewShutdown(void*, uint64_t crew) {
    if (crew == 0) return;
    // SPEC-GAP: crew creation/Init remain unspecified; registration accepts opaque existing crews.
    // §7.12 and vtables.tsv: Shutdown is slot 5, with no stack arguments.
    // Its return value is unspecified and ignored. Do not infer the unknown
    // six-argument Init contract merely to declare the preceding slots.
    void* object = fc4::FromToken(crew);
    auto** slots = *static_cast<void***>(object);
    using ShutdownFn = void(__thiscall*)(void*);
    reinterpret_cast<ShutdownFn>(slots[5])(object);
    // Shutdown may unregister and destroy the crew; never touch it afterward.
    FC4_X87_CHECK("netmgr crew Shutdown");
}

// ---- the tools (§3.1 step 5, §7.1) ---------------------------------------------------------------------
//
// The original's Init holds 13 real tools, one per network type, and other
// components take them from slot 5 without a null test (the game's zone
// developer dereferences the null at PostCityInit; runs glue24-rep-1..3). The
// game's own tool classes are constructed by the original manager's own code
// (§7.1: the factory 0x00610907-0x00610910 calls the constructors directly);
// no GZCOM class ID for them is stated in the specs, so the replacement
// cannot create the game's tools (question NM11). Without `enable = nettool`
// the class is glue24's stand-in (cFC4NetworkTool.h): it answers the SDK's
// cISC4NetworkTool interface and does nothing [choice] — the runs only need a
// non-null tool (PostCityInit takes tools 3, 11 and 0, AddRefs them and Inits
// them, zones.md §5.2); placement, the destroy pipeline and the underground
// refit are not reached in an auto-run, and the core still counts its
// null-callback groups. With `enable = nettool` (glue29) the same class runs
// the fc4_nettool core: the manager's cache then holds the real replacement
// tools, and slot 5 hands them to the game's callers.

cFC4NetworkTool* ToolOf(uint64_t t) {
    return t != 0 && fc4::IsOurObject(fc4::FromToken(t))
               ? static_cast<cFC4NetworkTool*>(fc4::FromToken(t))
               : nullptr;
}

cFC4NetworkTool* NewTool(void* u, uint32_t type) {
    auto* tool = new (std::nothrow) cFC4NetworkTool(type, NetToolRegistry::Enabled());
    if (tool == nullptr) return nullptr;
    // SPEC-GAP: context transport is unspecified; borrow the owning manager's city interface.
    tool->SetCity(Self(u)->City());
    tool->SetMessageServer(Self(u)->Server());
    return tool;
}

uint64_t CreateTool(void* u, uint32_t type) {
    // §3.1 step 5 / §7.1: the tool is created with one reference for the core
    // (the original's cache holds it; the core's create/release callbacks own
    // it here). The tool starts at 0 like the original's (§7.1: the factory
    // returns it with no AddRef) - the host holds the creation reference by
    // AddRef'ing after the new. glue29: the tool is live (the fc4_nettool
    // core) exactly when the nettool key is on; otherwise the stand-in
    // glue24's runs were verified with.
    auto* tool = NewTool(u, type);
    if (tool == nullptr) return 0;
    tool->AddRef();
    return fc4::TokenOf(static_cast<cIGZUnknown*>(tool));
}

void ToolInit(void*, uint64_t tool) {
    if (cFC4NetworkTool* t = ToolOf(tool)) t->Init();  // tool slot 3
}

void ToolShutdown(void*, uint64_t tool) {
    if (cFC4NetworkTool* t = ToolOf(tool)) t->Shutdown();  // tool slot 4
}

void ToolDestroyOccupants(void*, uint64_t tool, const uint64_t* occupants, size_t count) {
    // §7.2 step 2.2: the tool's slot 50 with the collected occupants. The
    // stand-in's slot 50 destroys nothing (glue29's live tool runs the §10
    // pipeline through its core); the tokens travel as the
    // cISC4NetworkOccupant pointers the core collected.
    cFC4NetworkTool* t = ToolOf(tool);
    if (t == nullptr || occupants == nullptr || count == 0) return;
    SC4Vector<cISC4NetworkOccupant*> list;
    for (size_t i = 0; i < count; ++i) {
        if (occupants[i] == 0) continue;
        list.push_back(reinterpret_cast<cISC4NetworkOccupant*>(fc4::FromToken(occupants[i])));
    }
    t->DestroyOccupantsAndResolve(list);  // tool slot 50, pop 4
}

void ToolUpdateUndergroundHeights(void*, uint64_t tool, const int32_t*, const uint64_t*, size_t, uint32_t) {
    // §7.6 tail: the underground tool's UpdateUndergroundOccupantTerrain-
    // Heights (network-tools.md §11, a direct call at 0x006588D0, a hash set
    // passed by value). The stand-in has no underground state and the live
    // tool's refit runs through its own core (fc4_nettool_update_underground_
    // heights); the refit's Init / Shutdown bracket still runs here
    // (ToolInit / ToolShutdown).
    (void)tool;
}

void ToolCellInfo(void*, uint64_t, uint32_t) {}

// ---- host assembly -----------------------------------------------------------------------------------

fc4_netmgr_host MakeHost(cFC4NetworkManager* self, fc4::NetMgrOwner* owner) {
    owner->mgr = self;  // the write callbacks reach the manager through it
    fc4_netmgr_host h = fc4::MakeNetMgrReadOnlyHost(&owner->ctx);
    h.user = owner;
    h.subscribe = &SubscribeCb;
    h.unsubscribe = &UnsubscribeCb;
    h.uk_paths = &UkPaths;
    h.is_night = &IsNight;
    h.view_rotation_changed = &ViewRotationChanged;
    h.occupant_flag_set = &OccupantFlagSet;
    h.set_lit = &SetLit;
    h.check_for_retrofit = &CheckForRetrofit;
    h.retrofit = &Retrofit;
    h.creation_number = &CreationNumber;
    h.remove_occupant = &RemoveOccupant;
    h.post_refresh = &PostRefresh;
    h.crew_shutdown = &CrewShutdown;
    h.create_tool = &CreateTool;
    h.tool_init = &ToolInit;
    h.tool_shutdown = &ToolShutdown;
    h.tool_destroy_occupants = &ToolDestroyOccupants;
    h.tool_update_underground_heights = &ToolUpdateUndergroundHeights;
    // NOT wired (NULL): tool_cell_info and tool_reinsert_piece (§7.4's
    // intersection re-insert needs the tool's GetCellInfo 0x00633220 and its
    // slot 72; a NULL cell info reads as 0 and the re-insert is skipped
    // [choice], counted by the core), and network_occupants. Question NM11.
    return h;
}

}  // namespace

// ---- the class ---------------------------------------------------------------------------------------

cFC4NetworkManager::cFC4NetworkManager() {
    {
        cIGZMessageServer2Ptr ms;
        if (ms) {
            SetServer(ms);
            FC4_ADDREF(msgServer_);
        }
    }
    host_ = MakeHost(this, &owner_);
    core_ = fc4_netmgr_create(&host_, 0);
    if (core_ == nullptr) fc4::Log("cFC4NetworkManager: core creation failed");
    fc4::Log("cFC4NetworkManager(%p) created, core %p", static_cast<void*>(Primary()),
             static_cast<void*>(core_));
}

cFC4NetworkManager::~cFC4NetworkManager() {
    fc4_netmgr_destroy(core_);
    core_ = nullptr;
    SetCity(nullptr);
    FC4_RELEASE(msgServer_);
    msgServer_ = nullptr;
    fc4::Log("cFC4NetworkManager destroyed");
}

bool cFC4NetworkManager::QueryInterface(uint32_t riid, void** ppvObj) {
    if (ppvObj == nullptr) return false;
    if (!fc4_netmgr_query_interface(riid)) return false;
    if (riid == 1u || riid == kFC4_IID_cISC4NetworkManager) {
        *ppvObj = Primary();
    } else if (riid == kIID_cIGZMessageTarget2) {
        *ppvObj = static_cast<cIGZMessageTarget2*>(this);
    } else if (riid == GZIID_cIGZSerializable) {
        *ppvObj = static_cast<cIGZSerializable*>(this);
    } else {
        return false;
    }
    AddRef();
    return true;
}

uint32_t cFC4NetworkManager::AddRef() { return ++refCount_; }

uint32_t cFC4NetworkManager::Release() {
    if (refCount_ > 1) return --refCount_;
    refCount_ = 0;
    if (destroying_) return 0;
    destroying_ = true;
    if (core_ != nullptr) {
        fc4_netmgr_state st{};
        st.struct_size = sizeof(st);
        if (fc4_netmgr_get_state(core_, &st) && st.initialized != 0) Shutdown();
    }
    delete this;
    return 0;
}

void cFC4NetworkManager::SetCity(cISC4City* c) {
    if (c == city_) return;
    if (c != nullptr) FC4_ADDREF(c);
    if (city_ != nullptr) FC4_RELEASE(city_);
    city_ = c;
    owner_.ctx.city = c;
}

bool cFC4NetworkManager::Init() {
    FC4_TRACE("NetworkManager::Init");
    SetWritable(true);
    AcquireCity(&owner_);
    const bool ok = fc4_netmgr_init(core_);
    fc4_netmgr_state st{};
    st.struct_size = sizeof(st);
    fc4_netmgr_get_state(core_, &st);
    fc4::Log("netmgr Init: ok=%d managed=%u pending=%u retrofit=%u crews=%u tools=%u", ok ? 1 : 0,
             st.managed, st.pending, st.retrofit, st.crews, st.tools);
    return true;
}

bool cFC4NetworkManager::Shutdown() {
    FC4_TRACE("NetworkManager::Shutdown");
    if (!destroying_) AddRef();
    fc4_netmgr_shutdown(core_);
    fc4_netmgr_state st{};
    st.struct_size = sizeof(st);
    fc4_netmgr_get_state(core_, &st);
    fc4::Log("netmgr Shutdown: managed=%u pending=%u", st.managed, st.pending);
    SetWritable(false);
    fc4::NetMgrReleaseHeld(&owner_.ctx);
    if (!destroying_) Release();
    return true;
}

void* cFC4NetworkManager::GetNetworkTool(int32_t type, bool unique) {
    if (unique) {
        if (static_cast<uint32_t>(type) >= 13) return nullptr;
        // network-manager.md 7.1: native callers take the first reference;
        // the core host's create_tool callback instead owns a cache reference.
        AcquireCity(&owner_);
        return static_cast<cIGZUnknown*>(NewTool(&owner_, static_cast<uint32_t>(type)));
    }
    return fc4::FromToken(fc4_netmgr_get_network_tool(core_, static_cast<uint32_t>(type), false));
}

void cFC4NetworkManager::DestroyNetworkOccupants(SC4List<cISC4Occupant*> const& list) {
    std::vector<uint64_t> tokens;
    for (cISC4Occupant* o : list) tokens.push_back(fc4::TokenOf(o));
    fc4_netmgr_destroy_network_occupants(core_, tokens.data(), tokens.size());
}

void cFC4NetworkManager::QueueRemovals(bool queue) { fc4_netmgr_queue_removals(core_, queue); }

bool cFC4NetworkManager::RemoveOrReplaceOccupant(void* tool, cISC4Occupant* occ, bool& replaced) {
    bool r = false;
    const bool ok = fc4_netmgr_remove_or_replace_occupant(core_, fc4::TokenOf(tool), fc4::TokenOf(occ), &r);
    if (ok) replaced = r;
    return ok;
}

bool cFC4NetworkManager::IsOccupantSubwayStation(cISC4Occupant* occ) {
    // §7.5: a subway network occupant that is a station. The station bit is
    // the piece's flag 0x200000 (the connection node bit, §5).
    if (occ == nullptr) return false;
    cISC4NetworkOccupant* n = nullptr;
    if (!occ->QueryInterface(GZIID_cISC4NetworkOccupant, reinterpret_cast<void**>(&n)) || n == nullptr) {
        return false;
    }
    FC4_ACQUIRED(n, "QueryInterface");
    const bool station = n->IsOfType(cISC4NetworkOccupant::Subway) && (n->GetNetworkFlag() & 0x200000u) != 0;
    FC4_RELEASE(n);
    return station;  // [abi-ret] interfaces.tsv slot 9 is eax-or-void
}

intptr_t cFC4NetworkManager::FindPipeOccupant(intptr_t, int32_t, intptr_t) {
    return 0;  // §7.5: the pipe walk is the plumbing subsystem's
}

intptr_t cFC4NetworkManager::FindPipeConnectionOccupant(intptr_t, int32_t, intptr_t) { return 0; }

intptr_t cFC4NetworkManager::GetAllIntersectionPieces(intptr_t, intptr_t, intptr_t, intptr_t) {
    return 0;  // §7.9: the intersection piece list (the renderer's)
}

// §13 [choice]: the three presentation slots are identity mappings until a
// renderer needs the ini resource's remap tables.
uint32_t cFC4NetworkManager::MapPreviewTexture(uint32_t a, uint8_t&) { return a; }
uint32_t cFC4NetworkManager::MapShadowTexture(uint32_t a, uint8_t&, uint32_t, uint32_t) { return a; }
uint32_t cFC4NetworkManager::MapPath(uint32_t a) { return a; }

void cFC4NetworkManager::RegisterConstructionCrew(intptr_t crew) {
    fc4_netmgr_register_construction_crew(core_, static_cast<uint64_t>(crew));
}

intptr_t cFC4NetworkManager::UnregisterConstructionCrew(intptr_t crew) {
    fc4_netmgr_unregister_construction_crew(core_, static_cast<uint64_t>(crew));
    return 1;  // [abi-ret] interfaces.tsv slot 17 is eax-or-void
}

void* cFC4NetworkManager::GetWaterOccupantFilter() {
    return nullptr;  // §7.5: the occupant manager's shared type filter
}
void* cFC4NetworkManager::GetSubwayOccupantFilter() { return nullptr; }

bool cFC4NetworkManager::CreateConnectionNode(cISC4Occupant*& out, int32_t, SC4Point<int> const&, uint32_t,
                                              uint32_t, uint32_t, uint8_t) {
    // §7.7: the node is a network occupant the tools create. §13 leaves the
    // tools to their own subsystem, so this replacement places none.
    out = nullptr;
    return false;
}

void cFC4NetworkManager::AutoconnectLotNodes(cISC4Lot*, SC4List<cISC4Occupant*> const&) {
    // §7.8: the same geometry as CreateConnectionNode.
}

const char* cFC4NetworkManager::GetEnglishNetworkName(int32_t type) {
    // §7.5 slot 22: the table at 0x00AA845C has no bounds check; ours answers
    // the 13 types of §3.1 and the empty string outside them.
    static const char* const kNames[13] = {"Road",      "Rail",       "Highway",  "Street",  "Pipe",
                                           "PowerLine", "Avenue",     "Subway",   "LightRail", "Monorail",
                                           "OneWayRoad", "DirtRoad",  "GroundHighway"};
    return (type >= 0 && type < 13) ? kNames[type] : "";
}

void cFC4NetworkManager::AddToRetrofitList(intptr_t networkOccupant) {
    fc4_netmgr_add_to_retrofit_list(core_, static_cast<uint64_t>(networkOccupant));
}

// §3 / §6.1: the 11 subscribed messages. The view-change message is
// presentation (§6.1); the core still takes it.
bool cFC4NetworkManager::DoMessage(cIGZMessage2* pMessage) {
    if (pMessage == nullptr || core_ == nullptr) return true;
    FC4_X87_SCOPE("netmgr DoMessage", pMessage->GetType());
    const uint32_t type = pMessage->GetType();
    uint32_t d1 = 0, d2 = 0, d3 = 0, d4 = 0;
    uint64_t void1 = 0;
    uint32_t viewOld = 0, viewNew = 0;
    cIGZMessage2Standard* m = nullptr;
    if (pMessage->QueryInterface(GZIID_cIGZMessage2Standard, reinterpret_cast<void**>(&m)) && m != nullptr) {
        FC4_ACQUIRED(m, "QueryInterface");
        d1 = static_cast<uint32_t>(m->GetData1());
        d2 = static_cast<uint32_t>(m->GetData2());
        d3 = static_cast<uint32_t>(m->GetData3());
        d4 = static_cast<uint32_t>(m->GetData4());
        if (type == kMsgOccupantInserted || type == kMsgOccupantRemoved) void1 = fc4::TokenOf(m->GetVoid1());
        if (type == kMsgViewChanged) {
            viewOld = d1;
            viewNew = d2;
        }
        FC4_RELEASE(m);
    }
    return fc4_netmgr_do_message(core_, type, d1, d2, d3, d4, void1, viewOld, viewNew);
}

bool cFC4NetworkManager::Write(cIGZOStream& s) {
    FC4_TRACE("NetworkManager::Write");
    fc4::OStreamAdapter a(s);
    fc4::MemOStream mem;
    const bool ok = fc4_netmgr_write(core_, a.Get());
    fc4_netmgr_write(core_, mem.Get());
    // §4.1: the record read back by a scratch core and written again — the
    // save's bytes must survive a load (this lane's round-trip check).
    bool same = false;
    {
        fc4_netmgr_host h{};
        h.struct_size = sizeof(h);
        fc4_netmgr* scratch = fc4_netmgr_create(&h, 0);
        fc4::MemIStream in(mem.bytes);
        fc4::MemOStream again;
        if (scratch != nullptr && fc4_netmgr_read(scratch, in.Get()) && in.Remaining() == 0 &&
            fc4_netmgr_write(scratch, again.Get())) {
            same = again.bytes == mem.bytes;
        }
        fc4_netmgr_destroy(scratch);
    }
    fc4::Log("netmgr Write -> %d (%u bytes, record53 round trip %s)", ok ? 1 : 0,
             static_cast<unsigned>(mem.bytes.size()), same ? "same" : "DIFFERENT");
    return ok;
}

// §4.1: Read runs before Init (foundations §3.7); nothing here is sized from
// the city. The core appends to the managed list and never clears it.
bool cFC4NetworkManager::Read(cIGZIStream& s) {
    FC4_TRACE("NetworkManager::Read");
    fc4::IStreamAdapter a(s);
    const bool ok = fc4_netmgr_read(core_, a.Get());
    fc4_netmgr_state st{};
    st.struct_size = sizeof(st);
    fc4_netmgr_get_state(core_, &st);
    fc4::Log("netmgr Read -> %d (%u managed entries)", ok ? 1 : 0, st.managed);
    return ok;
}

uint32_t cFC4NetworkManager::GetGZCLSID() { return kFC4_CLSID_cSC4NetworkManager; }

cIGZUnknown* CreateFC4NetworkManager() {
    return static_cast<cIGZUnknown*>(static_cast<cFC4ISC4NetworkManager*>(new cFC4NetworkManager()));
}

intptr_t NetworkCrewOwnershipTest() {
    // A synthetic six-slot object, not a declaration of the native crew ABI.
    // Only the specified unknown prefix and Shutdown slot are ever called.
    class CrewProbe : public cIGZUnknown {
    public:
        explicit CrewProbe(cFC4NetworkManager& manager) : manager_(manager) {}
        bool QueryInterface(uint32_t, void** out) override {
            if (out) *out = nullptr;
            return false;
        }
        uint32_t AddRef() override { ++adds; return ++refs; }
        uint32_t Release() override { ++drops; return --refs; }
        virtual void Unused3() { ++unexpected; }
        virtual void Unused4() { ++unexpected; }
        virtual void Shutdown() {
            ++shutdowns;
            if (unregister) manager_.UnregisterConstructionCrew(reinterpret_cast<intptr_t>(this));  // abi: via cFC4ISC4NetworkManager
        }
        cFC4NetworkManager& manager_;
        uint32_t refs = 1, adds = 0, drops = 0, shutdowns = 0, unexpected = 0;
        bool unregister = true;
    };
    cFC4NetworkManager manager;
    CrewProbe first(manager), second(manager);
    const auto token = [](CrewProbe& c) { return reinterpret_cast<intptr_t>(&c); };
    const auto shutdown = [&]() {
        auto* message = FC4MakeMessage(0x26D31EC2, 0, 0, 0);
        const bool ok = manager.DoMessage(message);
        message->Release();
        return ok;
    };
    manager.RegisterConstructionCrew(token(first));
    manager.RegisterConstructionCrew(token(second));
    if (first.refs != 2 || second.refs != 2 || !shutdown() ||
        first.shutdowns != 1 || second.shutdowns != 1 || first.refs != 1 || second.refs != 1) return 0;
    // Reentrant removal leaves no stale token to call at the next shutdown.
    if (!shutdown() || first.shutdowns != 1 || second.shutdowns != 1) return 0;
    // Each registration owns a reference, even when a crew registers twice.
    manager.RegisterConstructionCrew(token(first));
    manager.RegisterConstructionCrew(token(first));
    if (first.refs != 3 || !shutdown() || first.shutdowns != 3 || first.refs != 1) return 0;
    manager.UnregisterConstructionCrew(token(first));  // abi: via cFC4ISC4NetworkManager; absent: no Release
    if (first.drops != 3) return 0;
    // §7.12's bounded-loop choice: a non-unregistering first crew cannot hang
    // shutdown, nor may the bridge manufacture an extra Release for it.
    first.unregister = false;
    manager.RegisterConstructionCrew(token(first));
    manager.RegisterConstructionCrew(token(second));
    if (!shutdown() || first.shutdowns != 5 || second.shutdowns != 1 ||
        first.refs != 2 || second.refs != 2) return 0;
    manager.UnregisterConstructionCrew(token(first));  // abi: via cFC4ISC4NetworkManager
    manager.UnregisterConstructionCrew(token(second));  // abi: via cFC4ISC4NetworkManager
    return first.refs == 1 && second.refs == 1 && first.adds == first.drops &&
        second.adds == second.drops && first.unexpected == 0 && second.unexpected == 0 ? 1 : 0;
}
