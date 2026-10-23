// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors
// Record-only differential, advisors.md section 11. Never Init the system,
// simulate, install a factory, or call Read on a game object.
#include "FC4AdvisorsShadow.h"
#include "FC4Config.h"
#include "FC4CoreShadow.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"
#include "FC4WriteCapture.h"
#include "cFC4AdvisorObjects.h"
#include "cIGZSerializable.h"
#include "cISC4City.h"
#include "fc4.h"

#include <cstdio>
#include <map>
#include <memory>
#include <vector>

namespace fc4 {
namespace {
bool g_enabled = false;
cISC4City* g_city = nullptr;  // borrowed until city shutdown
uint32_t g_lines = 0, g_clean = 0;

void AddRef(void*, uint64_t token) { if (token != 0) FC4_ADDREF(FromToken(token)); }
void Release(void*, uint64_t token) { if (token != 0) FC4_RELEASE(FromToken(token)); }
uint64_t ResolveRef(void*, uint64_t token, uint32_t iid) {
    void* p = nullptr;
    if (token == 0 || !FromToken(token)->QueryInterface(iid, &p) || p == nullptr) return 0;
    auto* q = static_cast<cIGZUnknown*>(p);
    FC4_ACQUIRED(q, "QueryInterface");
    return TokenOf(q);
}

struct Node {
    cIGZSerializable* original = nullptr;  // borrowed for this synchronous snapshot
    fc4_adv_obj* core = nullptr;
    WriteCapture capture;
    bool captured = false, read = false;
    bool identityOnly = false;
};

struct Snapshot {
    fc4_advisors* system = nullptr;
    std::vector<std::unique_ptr<Node>> nodes;
    std::map<cIGZSerializable*, Node*> byObject;
    fc4_adv_advice_entry entry{};

    Snapshot() {
        fc4_advisor_host host{};
        host.struct_size = sizeof(host);
        host.user = this;
        host.add_ref = &AddRef;
        host.release = &Release;
        host.resolve_ref = &ResolveRef;
        host.advice_entry = [](void* u, uint32_t, fc4_adv_advice_entry* out) {
            *out = static_cast<Snapshot*>(u)->entry;
            return true;
        };
        // MySims are identities in the system map only (advisors.md section
        // 2.1 explicitly puts their records out of scope). No MySim Init.
        fc4_mysim_host mysim{};
        mysim.struct_size = sizeof(mysim);
        system = fc4_advisors_create(&host, &mysim, 0);
    }
    ~Snapshot() {
        for (auto& n : nodes) fc4_adv_obj_destroy(n->core);
        fc4_advisors_destroy(system);
    }
    Node* Add(cIGZSerializable* original) {
        const auto found = byObject.find(original);
        if (found != byObject.end()) return found->second;
        const uint32_t cls = original->GetGZCLSID();
        fc4_adv_obj* core = fc4_advisors_create_object(system, cls);
        if (core == nullptr) return nullptr;  // external subject object: opaque reference
        auto n = std::make_unique<Node>();
        n->original = original;
        n->core = core;
        n->identityOnly = cls == 0x4A1DBBBF || cls == 0x0BF606A5;
        fc4_adv_obj_set_token(core, TokenOf(original));
        Node* result = n.get();
        byObject.emplace(original, result);
        nodes.push_back(std::move(n));
        return result;
    }
    void CaptureAll() {
        for (size_t i = 0; i < nodes.size(); ++i) {
            Node& n = *nodes[i];
            if (n.identityOnly) continue;
            n.captured = n.capture.Capture(n.original);
            if (!n.captured) continue;
            for (uint32_t id = 1; n.capture.Object(id) != nullptr; ++id) Add(n.capture.SerializableAt(id));
        }
    }
    void SeedEvent(Node& n) {
        // eventID is script state, not in an advice record (sections 2.2,
        // 11.3), but an advisor's record writes it. Supply the original's
        // getter as Init input before Read restores the saved fields. No Lua
        // callbacks or game writes; effects start empty and are read once.
        void* p = nullptr;
        if (!n.original->QueryInterface(kFC4_IID_cISC4Advice, &p) || p == nullptr) return;
        auto* advice = static_cast<cFC4ISC4Advice*>(p);
        FC4_ACQUIRED(advice, "QueryInterface");
        entry = {};
        entry.guid = 1;
        entry.class_id = n.original->GetGZCLSID();
        entry.event = static_cast<uint32_t>(advice->GetEventID());
        FC4_RELEASE(advice);
        fc4_adv_advice_init(n.core, 1);
    }
};

// Delegate scalar decoding to CaptureIStream, but resolve family references
// to our core objects. The capture's per-record object IDs are never global.
class Input {
public:
    Input(Snapshot& snapshot, Node& node, ShadowOStream& out)
        : snapshot_(snapshot), node_(node), input_(node.capture), out_(out) {
        api_.struct_size = sizeof(api_);
        api_.user = this;
        api_.get_u8 = [](void* u, uint8_t* v) { auto& s = Self(u); return s.Base()->get_u8(s.Base()->user, v); };
        api_.get_u16 = [](void* u, uint16_t* v) { auto& s = Self(u); return s.Base()->get_u16(s.Base()->user, v); };
        api_.get_u32 = [](void* u, uint32_t* v) { auto& s = Self(u); return s.Base()->get_u32(s.Base()->user, v); };
        api_.get_u64 = [](void* u, uint64_t* v) { auto& s = Self(u); return s.Base()->get_u64(s.Base()->user, v); };
        api_.get_f32 = [](void* u, float* v) { auto& s = Self(u); return s.Base()->get_f32(s.Base()->user, v); };
        api_.get_f64 = [](void* u, double* v) { auto& s = Self(u); return s.Base()->get_f64(s.Base()->user, v); };
        api_.get_bytes = [](void* u, uint8_t* v, size_t n) { auto& s = Self(u); return s.Base()->get_bytes(s.Base()->user, v, n); };
        api_.get_str = [](void* u, fc4_bytes_sink cb, void* ctx) { auto& s = Self(u); return s.Base()->get_str(s.Base()->user, cb, ctx); };
        api_.skip = [](void* u, size_t n) { auto& s = Self(u); return s.Base()->skip(s.Base()->user, n); };
        api_.error = [](void* u) { auto& s = Self(u); return s.bad_ ? 1 : s.Base()->error(s.Base()->user); };
        api_.get_ref = [](void* u, uint32_t iid, fc4_ref* r) { return Self(u).Ref(iid, r); };
    }
    const fc4_istream* Get() const { return &api_; }
    bool AtEnd() const { return !bad_ && input_.good() && input_.atEnd(); }
private:
    static Input& Self(void* u) { return *static_cast<Input*>(u); }
    const fc4_istream* Base() const { return input_.Get(); }
    bool Ref(uint32_t iid, fc4_ref* out) {
        uint32_t id = 0, cls = 0;
        if (!Base()->get_u32(Base()->user, &id)) return false;
        if (id != 0 && !Base()->get_u32(Base()->user, &cls)) return false;
        fc4_ref r{};
        r.class_id = cls;
        if (id != 0) {
            auto* original = node_.capture.SerializableAt(id);
            if (original == nullptr) { bad_ = true; return false; }
            void* p = nullptr;
            if (original->QueryInterface(iid == 0 ? GZIID_cIGZUnknown : iid, &p) && p != nullptr) {
                auto* q = static_cast<cIGZUnknown*>(p);
                FC4_ACQUIRED(q, "QueryInterface");
                const auto found = snapshot_.byObject.find(original);
                if (found != snapshot_.byObject.end() && iid != GZIID_cIGZUnknown) {
                    r.core_handle = found->second->core;
                    r.token = TokenOf(original);
                } else {
                    // Borrowed token: host.resolve_ref takes the subject's
                    // reference, just as IStreamAdapter does for our objects.
                    r.token = TokenOf(q);
                }
                FC4_RELEASE(q);
                out_.AddToken(r.token, id, cls);
            }
        }
        if (out != nullptr) *out = r;
        return true;
    }
    Snapshot& snapshot_;
    Node& node_;
    CaptureIStream input_;
    ShadowOStream& out_;
    bool bad_ = false;
    fc4_istream api_{};
};

struct Result {
    uint32_t mismatches = 0, records = 0, identities = 0;
    size_t bytes = 0;
};

Result RoundTrip(cIGZUnknown* original) {
    Snapshot s;
    void* p = nullptr;
    Node* root = nullptr;
    if (original != nullptr && original->QueryInterface(GZIID_cIGZSerializable, &p) && p != nullptr) {
        auto* ser = static_cast<cIGZSerializable*>(p);
        FC4_ACQUIRED(ser, "QueryInterface");
        root = s.Add(ser);
        FC4_RELEASE(ser);
    }
    s.CaptureAll();
    Result result;
    auto& mismatches = result.mismatches;
    auto& records = result.records;
    auto& identities = result.identities;
    auto& bytes = result.bytes;
    mismatches = root == nullptr ? 1 : 0;
    // Advice flags must be loaded before an advisor writes CheckFlags(2).
    // Subjects need no prior state to be referenced. System reads last.
    const uint32_t kinds[] = {FC4_ADV_OBJ_SUBJECT, FC4_ADV_OBJ_ADVICE, FC4_ADV_OBJ_ADVISOR, FC4_ADV_OBJ_SYSTEM};
    std::map<Node*, std::unique_ptr<ShadowOStream>> outputs;
    for (uint32_t kind : kinds) {
        for (auto& ptr : s.nodes) {
            Node& n = *ptr;
            if (n.identityOnly || !n.captured || fc4_adv_obj_kind(n.core) != kind) continue;
            if (kind == FC4_ADV_OBJ_ADVICE) s.SeedEvent(n);
            auto out = std::make_unique<ShadowOStream>();
            Input in(s, n, *out);
            n.read = fc4_adv_obj_read(n.core, in.Get()) && in.AtEnd();
            outputs.emplace(&n, std::move(out));
        }
    }
    for (auto& ptr : s.nodes) {
        Node& n = *ptr;
        if (n.identityOnly) { ++identities; continue; }
        ++records;
        bytes += n.capture.Bytes().size();
        const auto found = outputs.find(&n);
        const bool written = found != outputs.end() && n.read && fc4_adv_obj_write(n.core, found->second->Get());
        const bool exact = written && found->second->UnknownRefs() == 0 && found->second->Bytes() == n.capture.Bytes();
        if (!exact) {
            ++mismatches;
            size_t first = 0;
            if (written) {
                const auto& a = n.capture.Bytes();
                const auto& b = found->second->Bytes();
                while (first < a.size() && first < b.size() && a[first] == b[first]) ++first;
            }
            // Diagnostic uses a separate prefix: only the dated aggregate is
            // a shadow line, and it counts every failed record.
            if (mismatches <= 3) {
                Log("advisors record diff: class=0x%08X captured=%d read=%d write=%d bytes=%u first=%u",
                    n.original->GetGZCLSID(), n.captured, n.read, written,
                    static_cast<unsigned>(n.capture.Bytes().size()), static_cast<unsigned>(first));
            }
        }
    }
    return result;
}

void Compare(const char* event, int32_t julian) {
    if (!g_enabled || g_city == nullptr) return;
    const Result result = RoundTrip(reinterpret_cast<cIGZUnknown*>(g_city->GetAdvisorSystem()));  // borrowed
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    Log("advisors shadow %04d-%02d-%02d %s: records=%u bytes=%u MySim_identities=%u; mismatches=%u",
        date.year, date.month, date.day_of_month, event, result.records, static_cast<unsigned>(result.bytes), result.identities, result.mismatches);
    ++g_lines;
    if (result.mismatches == 0) ++g_clean;
}
}  // namespace

void AdvisorsShadowStartup() {
    g_enabled = SettingInt("advisors_shadow", nullptr, 0) != 0;
    if (g_enabled) Log("advisors shadow: ON; record-only system/advisor/advice/subject roundtrip at load and month; MySim records out of scope (identities only); no simulation or replacement");
}
uint32_t AdvisorsShadowTest(cIGZUnknown* original) { return RoundTrip(original).mismatches; }
void AdvisorsShadowCityLoaded(cISC4City* city, int32_t julian) {
    g_city = city;
    g_lines = g_clean = 0;
    Compare("load", julian);
}
void AdvisorsShadowMonth(int32_t julian) { Compare("month", julian); }
void AdvisorsShadowCityShutdown() {
    if (g_enabled && g_city != nullptr) Log("advisors shadow: city shutdown: %u lines, %u with mismatches=0", g_lines, g_clean);
    g_city = nullptr;
}
}  // namespace fc4
