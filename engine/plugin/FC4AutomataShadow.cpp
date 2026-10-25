// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR
// PURPOSE. See the GNU General Public License for more details. You should have
// received a copy of the GNU General Public License along with this program; if
// not, see <https://www.gnu.org/licenses/>.

// The automata layer shadow (FC4AutomataShadow.h; docs/specs/automata.md).
//
// The two streams below carry a reference as its **host token** and class ID,
// which for this shadow is the capture's 1-based object number: the automata
// core writes `token` back unchanged (fc4.h: "A core object is written with
// the token given by fc4_automata_object_set_token"), and the original's own
// Write writes that same object number (FC4WriteCapture.h). A shared stream
// that used an object pointer as the token would round-trip the wrong number.
#include "FC4AutomataShadow.h"

#include <utility>

#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Streams.h"  // TokenOf
#include "FC4WriteCapture.h"
#include "FC4Interfaces.h"

#include "cIGZSerializable.h"
#include "cIGZUnknown.h"
#include "cISC4City.h"
#include "cISC4Automaton.h"
#include "cS3DVector3.h"
#include "GZServPtrs.h"
#include "cRZCOMDllDirector.h"
#include "cIGZCOM.h"

#include <cinttypes>
#include <cmath>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <unordered_set>
#include <vector>

#include "fc4.h"

// Private static-link transports, deliberately absent from the public C ABI.
extern "C" bool fc4_watercraft_shadow_record(const fc4_istream*, const fc4_ostream*,
    const fc4_automata_ref_host*, float*, uint32_t*);
extern "C" uint64_t fc4_watercraft_shadow_probe(uint32_t);

namespace fc4 {
namespace {

bool g_enabled = false;
bool g_ownersEnabled = false;
bool g_helicopterEnabled = false;
bool g_watercraftEnabled = false;
uint64_t g_watercraftLines = 0, g_watercraftClean = 0;
int32_t g_helicopterMonth = 0;
uint64_t g_helicopterLines = 0, g_helicopterClean = 0;
uint64_t g_ownerLines = 0;
uint64_t g_ownerCleanLines = 0;
bool g_loggedNoRecord = false;
cISC4City* g_city = nullptr;  // borrowed while the city is up

// Per-component tallies, summed for the shutdown line.
uint64_t g_lines = 0;
uint64_t g_cleanLines = 0;
uint64_t g_mismatches = 0;
uint64_t g_componentsCompared = 0;
uint64_t g_bytesCompared = 0;

void AppendF(std::string& out, const char* fmt, ...) {
    char buf[512];
    va_list ap;
    va_start(ap, fmt);
    _vsnprintf_s(buf, sizeof(buf), _TRUNCATE, fmt, ap);
    va_end(ap);
    out += buf;
}

std::string DateText(int32_t julian) {
    char buf[16] = "?";
    if (julian > 0) {
        fc4_date d{};
        fc4_date_from_julian(julian, &d);
        _snprintf_s(buf, sizeof(buf), _TRUNCATE, "%04d-%02d-%02d", d.year, d.month, d.day_of_month);
    }
    return buf;
}

// ---- the two streams, and the shadow cores' reference host -----------------
//
// `user` of every stream and host callback below is the compare's
// `WriteCapture`, which is alive for as long as the shadow core is (the core
// is created and destroyed inside one ComparePart).

void HostAddRef(void* user, uint64_t token) {
    if (user != nullptr) static_cast<WriteCapture*>(user)->AcquireReplayReference(token);
}

void HostRelease(void* user, uint64_t token) {
    if (user != nullptr && token != 0 &&
        !static_cast<WriteCapture*>(user)->ReleaseReplayReference(token))
        Log("automataowners shadow: rejected unacquired replay release token=%llu", token);
}

// The controller manager's host also reads a controller's flags (§3.2: flag 8
// keeps a controller out of the record). A captured record carries no live
// controller to ask, and a missing value reads as 0 (never transient).
uint32_t HostControllerFlags(void* /*user*/, uint64_t /*controller*/) { return 0; }

void HostLog(void* /*user*/, const char* text) { Log("automata shadow: %s", text); }

fc4_automata_ref_host RefHost(const WriteCapture* cap) {
    fc4_automata_ref_host h{};
    h.struct_size = sizeof(h);
    h.user = const_cast<WriteCapture*>(cap);
    h.add_ref = &HostAddRef;
    h.release = &HostRelease;
    return h;
}

fc4_automata_cm_host CmHost(const WriteCapture* cap) {
    fc4_automata_cm_host h{};
    h.struct_size = sizeof(h);
    h.user = const_cast<WriteCapture*>(cap);
    h.log = &HostLog;
    h.add_ref = &HostAddRef;
    h.release = &HostRelease;
    h.controller_flags = &HostControllerFlags;
    return h;
}

// An fc4_istream over a captured record: a reference is the capture's object
// number as the token (so a write puts the same number back) and comes with
// one reference for the reader (fc4_istream's contract; the core drops it
// through the host table).
class CaptureIn {
public:
    explicit CaptureIn(const WriteCapture& cap) : cap_(cap) {
        s_.struct_size = sizeof(s_);
        s_.user = this;
        s_.get_u8 = &GetU8;
        s_.get_u16 = &GetU16;
        s_.get_u32 = &GetU32;
        s_.get_u64 = &GetU64;
        s_.get_f32 = &GetF32;
        s_.get_f64 = &GetF64;
        s_.get_bytes = &GetBytes;
        s_.get_str = &GetStr;
        s_.get_ref = &GetRef;
        s_.skip = &Skip;
        s_.error = &Error;
    }
    const fc4_istream* Get() const { return &s_; }
    bool atEnd() const { return pos_ == cap_.Bytes().size(); }

private:
    bool Take(void* out, size_t n) {
        const std::vector<uint8_t>& b = cap_.Bytes();
        if (bad_ || n > b.size() - pos_) {
            bad_ = true;
            return false;
        }
        if (out != nullptr && n > 0) std::memcpy(out, b.data() + pos_, n);
        pos_ += n;
        return true;
    }
    static CaptureIn* Self(void* u) { return static_cast<CaptureIn*>(u); }
    static bool GetU8(void* u, uint8_t* o) { return Self(u)->Take(o, 1); }
    static bool GetU16(void* u, uint16_t* o) { return Self(u)->Take(o, 2); }
    static bool GetU32(void* u, uint32_t* o) { return Self(u)->Take(o, 4); }
    static bool GetU64(void* u, uint64_t* o) { return Self(u)->Take(o, 8); }
    static bool GetF32(void* u, float* o) { return Self(u)->Take(o, 4); }
    static bool GetF64(void* u, double* o) { return Self(u)->Take(o, 8); }
    static bool GetBytes(void* u, uint8_t* o, size_t n) { return Self(u)->Take(o, n); }
    static bool Skip(void* u, size_t n) { return Self(u)->Take(nullptr, n); }
    static bool GetStr(void* u, fc4_bytes_sink sink, void* ctx) {
        CaptureIn* s = Self(u);
        uint32_t n = 0;
        if (!s->Take(&n, 4)) return false;
        const size_t off = s->pos_;
        if (!s->Take(nullptr, n)) return false;
        if (sink != nullptr) sink(ctx, s->cap_.Bytes().data() + off, n);
        return true;
    }
    static bool GetRef(void* u, uint32_t iid, fc4_ref* out) {
        CaptureIn* s = Self(u);
        uint32_t id = 0, cls = 0;
        if (!s->Take(&id, 4)) return false;
        if (id != 0 && !s->Take(&cls, 4)) return false;
        fc4_ref r{};
        r.class_id = cls;
        if (id != 0) {
            cIGZSerializable* o = s->cap_.SerializableAt(id);
            if (o == nullptr || s->cap_.ClassOf(id) != cls) {
                s->bad_ = true;
                return false;
            }
            if (!const_cast<WriteCapture&>(s->cap_).AcquireReplayInterface(
                    id, iid == 0 ? GZIID_cIGZUnknown : iid)) {
                // The original wrote a reference to something that does not
                // answer what the reader asks: the same null the original's
                // own Read stores (automata.md §1).
                Log("automataowners shadow: reference interface unavailable offset=%u id=%u class=0x%08X iid=0x%08X",
                    static_cast<unsigned>(s->pos_ - 8), id, cls, iid);
                if (out != nullptr) *out = fc4_ref{};
                return true;
            }
            // LR13: the helper released its temporary Serializable; the field
            // owns this successful QI until HostRelease, independently of the
            // observer pin. Tokens identify objects, never interface addresses.
            r.token = id;
        }
        if (out != nullptr) *out = r;
        return true;
    }
    static int32_t Error(void* u) { return Self(u)->bad_ ? 1 : 0; }

    const WriteCapture& cap_;
    size_t pos_ = 0;
    bool bad_ = false;
    fc4_istream s_{};
};

// An fc4_ostream that records bytes and writes a reference as its token and
// class ID (a `str` as its length and bytes): the original's own layout.
class RecordOut {
public:
    RecordOut() {
        s_.struct_size = sizeof(s_);
        s_.user = this;
        s_.put_u8 = &PutU8;
        s_.put_u16 = &PutU16;
        s_.put_u32 = &PutU32;
        s_.put_u64 = &PutU64;
        s_.put_f32 = &PutF32;
        s_.put_f64 = &PutF64;
        s_.put_bytes = &PutBytes;
        s_.put_str = &PutStr;
        s_.put_ref = &PutRef;
        s_.error = &Error;
    }
    const fc4_ostream* Get() const { return &s_; }
    std::vector<uint8_t> bytes;

private:
    bool Put(const void* p, size_t n) {
        const uint8_t* b = static_cast<const uint8_t*>(p);
        bytes.insert(bytes.end(), b, b + n);
        return true;
    }
    static RecordOut* Self(void* u) { return static_cast<RecordOut*>(u); }
    static bool PutU8(void* u, uint8_t v) { return Self(u)->Put(&v, 1); }
    static bool PutU16(void* u, uint16_t v) { return Self(u)->Put(&v, 2); }
    static bool PutU32(void* u, uint32_t v) { return Self(u)->Put(&v, 4); }
    static bool PutU64(void* u, uint64_t v) { return Self(u)->Put(&v, 8); }
    static bool PutF32(void* u, float v) { return Self(u)->Put(&v, 4); }
    static bool PutF64(void* u, double v) { return Self(u)->Put(&v, 8); }
    static bool PutBytes(void* u, const uint8_t* p, size_t n) { return Self(u)->Put(p, n); }
    static bool PutStr(void* u, const uint8_t* p, size_t n) {
        const uint32_t len = static_cast<uint32_t>(n);
        return Self(u)->Put(&len, 4) && (n == 0 || Self(u)->Put(p, n));
    }
    static bool PutRef(void* u, const fc4_ref* r) {
        const uint32_t id = r == nullptr ? 0u : static_cast<uint32_t>(r->token);
        if (!Self(u)->Put(&id, 4)) return false;
        return id == 0 || Self(u)->Put(&r->class_id, 4);
    }
    static int32_t Error(void*) { return 0; }

    fc4_ostream s_{};
};

// AF11: this is intentionally a metadata projection, not a record roundtrip
// masquerading as an independent simulation. Native outputs never seed Rust.
void HelicopterCompare(int32_t julian) {
    if (!g_helicopterEnabled || !g_city) return;
    fc4_date date{};
    // PublishedJulian is not populated yet at PostCityInit. Read the date
    // through the specified simulator interface for the load observation.
    if (julian <= 0) {
        auto* simulator = AsGame(g_city->GetSimulator());
        if (simulator) julian = simulator->GetSimDateNumber();
    }
    if (julian <= 0) return;
    fc4_date_from_julian(julian, &date);
    const int32_t month = date.year * 12 + date.month;
    if (month == g_helicopterMonth) return;
    g_helicopterMonth = month;
    RecordOut expected;
    const uint32_t cls = fc4_helicopter_shadow_fixture(expected.Get());
    uint32_t bad = (cls == 0 || expected.bytes.size() < 8) ? 1u : 0u;
    uint32_t samples = 0, fields = 0, reachable = 0, fixtures = 0;
    auto compare = [&](cIGZSerializable* native) {
        ++samples;
        ++fields;
        bad += native->GetGZCLSID() != cls;
        WriteCapture record(false, true);
        const bool captured = record.Capture(native);
        // Count each unavailable or different version separately, never just
        // one whole-record boolean. The remaining bytes are NOT compared.
        for (size_t offset = 0; offset < 8; offset += 2) {
            ++fields;
            if (!captured || record.Bytes().size() < offset + 2 ||
                expected.bytes.size() < offset + 2 ||
                std::memcmp(record.Bytes().data() + offset, expected.bytes.data() + offset, 2) != 0)
                ++bad;
        }
    };
    WriteCapture manager(false, true);
    if (!manager.Capture(reinterpret_cast<cIGZUnknown*>(g_city->GetAircraftManager()))) {
        ++bad;
    } else {
        std::unordered_set<const void*> seen;
        for (uint32_t id = 1; manager.SerializableAt(id); ++id) {
            auto* object = manager.SerializableAt(id);
            if (manager.ClassOf(id) == 0x8BA98EDA && seen.insert(object).second) {
                ++reachable;
                compare(object);
            }
        }
    }
    if (reachable == 0) {
        // AF11: no specified live-spawn scenario/group contract. A private
        // factory object is safe for identity/Write only; never Init or tick it,
        // add it to a manager, or claim it is a spawned city helicopter.
        cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
        cIGZSerializable* object = nullptr;
        if (com && com->GetClassObject(0x8BA98EDA, 0xE4FDA3D4,
                                      reinterpret_cast<void**>(&object)) && object) {
            FC4_ACQUIRED(object, "helicopter metadata factory reference");
            ++fixtures;
            compare(object);
            FC4_RELEASE_CREATED(object);
        } else {
            ++bad;  // An empty sample must never be reported as exact.
        }
    }
    Log("helicopter shadow %s: partial=AF11 reachable=%u fixtures=%u samples=%u fields=%u "
        "uncompared=parameters,home,goals,physics,lifecycle,enumeration[AF2-AF6/AF9/AF11] mismatches=%u",
        DateText(julian).c_str(), reachable, fixtures, samples, fields, bad);
    ++g_helicopterLines;
    if (!bad) ++g_helicopterClean;
}

// Each successful whole-record comparison covers the inherited base record as
// well. These are record checks, not independent movement/lifecycle simulations.
struct OwnerCount {
    size_t records = 0;
    size_t bytes = 0;
    uint32_t mismatches = 0;
    void Add(size_t n, uint32_t bad) {
        ++records;
        bytes += n;
        mismatches += bad;
    }
};

struct OwnerScan {
    OwnerCount manager, traffic, automaton, controller, construction;
    std::unordered_set<const void*> seen;
    uint32_t diagnostics = 0;

    void References(const WriteCapture& parent) {
        // Capture IDs are contiguous, 1-based (FC4WriteCapture.h). Objects are
        // pinned by the parent capture through the entire recursive scan.
        for (uint32_t id = 1; parent.SerializableAt(id) != nullptr; ++id) {
            cIGZSerializable* object = parent.SerializableAt(id);
            if (!seen.insert(object).second) continue;
            const uint32_t cls = parent.ClassOf(id);
            const bool isConstruction = cls == 0xA97F909E;  // lot-records §9
            fc4_automata_object* core = isConstruction ? nullptr : fc4_automata_object_create(cls);
            if (!isConstruction && core == nullptr) {
                // Other saved occupants are outside these owners. An unknown
                // automaton/controller is a failure, never a silent coverage gap.
                for (uint32_t iid : {0xA9B40F05u, 0x6A1857F8u}) {
                    void* iface = nullptr;
                    if (object->QueryInterface(iid, &iface) && iface != nullptr) {
                        auto* acquired = reinterpret_cast<cIGZUnknown*>(iface);
                        FC4_ACQUIRED(acquired, "automata owner record classification QI");
                        FC4_RELEASE(acquired);
                        (iid == 0xA9B40F05u ? automaton : controller).Add(0, 1);
                        if (diagnostics++ < 8)
                            Log("automataowners shadow: unsupported record class=0x%08X iid=0x%08X", cls, iid);
                        break;
                    }
                }
                continue;
            }
            OwnerCount& count = isConstruction ? construction :
                (fc4_automata_object_is_controller(core) ? controller : automaton);
            WriteCapture capture(false, true);
            const bool captured = capture.Capture(object);
            uint32_t bad = 1;
            if (captured) {
                CaptureIn input(capture);
                RecordOut output;
                const fc4_automata_ref_host host = RefHost(&capture);
                if (isConstruction) {
                    bad = !(fc4_automata_construction_roundtrip(input.Get(), output.Get(), &host) &&
                            input.atEnd() && output.bytes == capture.Bytes());
                } else {
                    const bool read = fc4_automata_object_read(core, input.Get(), &host);
                    bad = !(read && input.atEnd() && fc4_automata_object_write(core, output.Get()) &&
                            output.bytes == capture.Bytes());
                    // Object records keep host tokens, not owning references.
                    // Failed reads release themselves; successful ones are ours.
                    if (read) {
                        std::vector<uint64_t> refs(fc4_automata_object_refs(core, nullptr, 0));
                        fc4_automata_object_refs(core, refs.data(), refs.size());
                        for (uint64_t token : refs) HostRelease(const_cast<WriteCapture*>(&capture), token);
                    }
                }
                if (bad != 0 && diagnostics++ < 8) {
                    size_t first = 0;
                    const auto& original = capture.Bytes();
                    while (first < original.size() && first < output.bytes.size() &&
                           original[first] == output.bytes[first]) ++first;
                    Log("automataowners shadow: record class=0x%08X bytes=%u/%u roundtrip FAILED "
                        "(consumed=%u first=%u)", cls, static_cast<unsigned>(original.size()),
                        static_cast<unsigned>(output.bytes.size()), input.atEnd() ? 1u : 0u,
                        static_cast<unsigned>(first));
                }
                References(capture);
            } else {
                if (diagnostics++ < 8) Log("automataowners shadow: record class=0x%08X capture FAILED", cls);
            }
            count.Add(capture.Bytes().size(), bad);
            if (core != nullptr) fc4_automata_object_destroy(core);
        }
    }

    void Append(std::string& line, const char* name, const OwnerCount& count) const {
        AppendF(line, " %s=%u/%u bytes=%u mismatches=%u;", name,
                static_cast<unsigned>(count.records - count.mismatches), static_cast<unsigned>(count.records),
                static_cast<unsigned>(count.bytes), count.mismatches);
    }
};

// ---- one component: the original's record against the shadow core's --------

// The three shapes the six components come in (§2, §3, §4-§5).
enum class Kind { Scripts, Cm, Manager };

struct Part {
    const char* name;  // the log's short name
    uint32_t slot;     // the city getter (§0's table)
    uint32_t classId;  // the class ID of §0
    Kind kind;
    void* core = nullptr;
};

// §0's table: 40 the script system, 41 the controller manager, 43-46 the
// managers. The city getters are slots 48, 47, 43, 44, 45, 46.
Part g_parts[] = {
    {"script", 48, 0x4A1B664E, Kind::Scripts, nullptr},      // 40
    {"cm", 47, 0xCA14CA03, Kind::Cm, nullptr},               // 41
    {"vehicle", 43, 0xA98F9D3B, Kind::Manager, nullptr},     // 43
    {"pedestrian", 44, 0xC98F9D17, Kind::Manager, nullptr},  // 44
    {"aircraft", 45, 0xC98F9D4A, Kind::Manager, nullptr},    // 45
    {"watercraft", 46, 0xA98F9D28, Kind::Manager, nullptr},  // 46
};

constexpr size_t kParts = sizeof(g_parts) / sizeof(g_parts[0]);

cIGZUnknown* UnknownOf(cISC4City* city, uint32_t slot) {
    if (city == nullptr) return nullptr;
    // The city getters return the interface pointer (interfaces.tsv slots
    // 43-48); the object's cIGZUnknown is its first sub-object.
    switch (slot) {
        case 43:
            return reinterpret_cast<cIGZUnknown*>(city->GetVehicleManager());
        case 44:
            return reinterpret_cast<cIGZUnknown*>(city->GetPedestrianManager());
        case 45:
            return reinterpret_cast<cIGZUnknown*>(city->GetAircraftManager());
        case 46:
            return reinterpret_cast<cIGZUnknown*>(city->GetWatercraftManager());
        case 47:
            return reinterpret_cast<cIGZUnknown*>(city->GetAutomataControllerManager());
        case 48:
            return reinterpret_cast<cIGZUnknown*>(city->GetAutomataScriptSystem());
        default:
            return nullptr;
    }
}

void DropCore(Part& p) {
    if (p.core == nullptr) return;
    switch (p.kind) {
        case Kind::Scripts:
            fc4_automata_scripts_destroy(static_cast<fc4_automata_scripts*>(p.core));
            break;
        case Kind::Cm:
            fc4_automata_cm_destroy(static_cast<fc4_automata_cm*>(p.core));
            break;
        case Kind::Manager:
            fc4_automata_manager_destroy(static_cast<fc4_automata_manager*>(p.core));
            break;
    }
    p.core = nullptr;
}

bool MakeCore(Part& p, const WriteCapture* cap) {
    DropCore(p);
    switch (p.kind) {
        case Kind::Scripts:
            p.core = fc4_automata_scripts_create();
            break;
        case Kind::Cm: {
            const fc4_automata_cm_host h = CmHost(cap);
            p.core = fc4_automata_cm_create(&h);
            break;
        }
        case Kind::Manager: {
            const fc4_automata_ref_host h = RefHost(cap);
            p.core = fc4_automata_manager_create(p.classId, &h);
            break;
        }
    }
    return p.core != nullptr;
}

bool WriteCore(Part& p, RecordOut& out) {
    switch (p.kind) {
        case Kind::Scripts:
            return fc4_automata_scripts_write(static_cast<fc4_automata_scripts*>(p.core), out.Get());
        case Kind::Cm:
            return fc4_automata_cm_write(static_cast<fc4_automata_cm*>(p.core), out.Get());
        case Kind::Manager:
            return fc4_automata_manager_write(static_cast<fc4_automata_manager*>(p.core), out.Get());
    }
    return false;
}

bool ReadCore(Part& p, CaptureIn& in) {
    switch (p.kind) {
        case Kind::Scripts:
            return fc4_automata_scripts_read(static_cast<fc4_automata_scripts*>(p.core), in.Get());
        case Kind::Cm:
            return fc4_automata_cm_read(static_cast<fc4_automata_cm*>(p.core), in.Get());
        case Kind::Manager:
            return fc4_automata_manager_read(static_cast<fc4_automata_manager*>(p.core), in.Get());
    }
    return false;
}

// A short form of the two byte sequences' first difference.
void FirstDifference(const std::vector<uint8_t>& a, const std::vector<uint8_t>& b, char* out, size_t cap) {
    const size_t n = a.size() < b.size() ? a.size() : b.size();
    for (size_t i = 0; i < n; ++i) {
        if (a[i] != b[i]) {
            _snprintf_s(out, cap, _TRUNCATE, "first difference at byte %u (ours 0x%02X, original 0x%02X)",
                        static_cast<unsigned>(i), a[i], b[i]);
            return;
        }
    }
    _snprintf_s(out, cap, _TRUNCATE, "lengths differ (ours %u, original %u)", static_cast<unsigned>(a.size()),
                static_cast<unsigned>(b.size()));
}

// One component: the original's own Write against the shadow core's.
// Returns the mismatch count (0 = the records are identical).
uint32_t ComparePart(Part& p, std::string& out, OwnerScan* owners) {
    cIGZUnknown* obj = UnknownOf(g_city, p.slot);
    if (obj == nullptr) {
        AppendF(out, " %s: no component", p.name);
        return 0;
    }
    WriteCapture cap(false, true);
    bool answered = false;
    const bool captured = cap.Capture(obj, &answered);
    if (!captured) {
        AppendF(out, " %s: the original's record %s", p.name, answered ? "failed" : "cannot be read");
        if (!g_loggedNoRecord) {
            g_loggedNoRecord = true;
            Log("automata shadow: the original's record (its own Write) %s; nothing compared",
                answered ? "failed" : "cannot be read (no cIGZSerializable)");
        }
        return 1;
    }
    const std::vector<uint8_t> original = cap.Bytes();
    uint32_t mismatches = 0;
    if (!MakeCore(p, &cap)) {
        AppendF(out, " %s: %u bytes, the shadow core could not be created", p.name,
                static_cast<unsigned>(original.size()));
        return 1;
    }
    {
        CaptureIn in(cap);
        const bool read = ReadCore(p, in);
        RecordOut again;
        const bool written = read && in.atEnd() && WriteCore(p, again);
        if (!read) {
            AppendF(out, " %s: %u bytes, read FAILED", p.name, static_cast<unsigned>(original.size()));
            mismatches = 1;
        } else if (!in.atEnd()) {
            AppendF(out, " %s: %u bytes, read did not consume the record", p.name,
                    static_cast<unsigned>(original.size()));
            mismatches = 1;
        } else if (!written) {
            AppendF(out, " %s: %u bytes, written back FAILED", p.name, static_cast<unsigned>(original.size()));
            mismatches = 1;
        } else if (again.bytes == original) {
            AppendF(out, " %s: %u bytes identical", p.name, static_cast<unsigned>(original.size()));
        } else {
            char diff[128] = "";
            FirstDifference(again.bytes, original, diff, sizeof(diff));
            AppendF(out, " %s: %u bytes DIFFERENT (%s)", p.name, static_cast<unsigned>(original.size()), diff);
            mismatches = 1;
        }
    }
    // The core goes before the capture: its references are dropped while the
    // objects the capture named are still reachable (HANDOFF §5.1b).
    DropCore(p);
    if (owners != nullptr) {
        if (p.kind == Kind::Manager) {
            owners->manager.Add(original.size(), mismatches);
            if (p.slot == 43 || p.slot == 44) owners->traffic.Add(original.size(), mismatches);
        }
        owners->References(cap);
    }
    g_bytesCompared += original.size();
    return mismatches;
}

// Watercraft §§1/3.1 and automata §7.4: record/getter observations are
// read-only on live boats. Only the private factory fixture receives probes.
void WatercraftCompare(const char* what, int32_t julian) {
    if (!g_watercraftEnabled || !g_city) return;
    uint32_t bad = 0, samples = 0, reachable = 0, fixtures = 0, probes = 0;
    uint64_t bytes = 0;
    auto compare = [&](cIGZSerializable* native, uint32_t step, bool fixture) {
        ++samples;
        bad += native->GetGZCLSID() != 0x096E75F5;
        WriteCapture capture(false, true);
        if (!capture.Capture(native)) { ++bad; return; }
        CaptureIn input(capture);
        RecordOut output;
        const auto host = RefHost(&capture);
        float vectors[9]{};
        uint32_t states[2]{};
        const bool decoded = fc4_watercraft_shadow_record(input.Get(), output.Get(), &host, vectors, states);
        bytes += capture.Bytes().size();
        if (!decoded || !input.atEnd() || output.bytes != capture.Bytes()) {
            ++bad;
            Log("watercraft shadow detail: record differs decoded=%u input=%u output=%u",
                decoded ? 1u : 0u, static_cast<unsigned>(capture.Bytes().size()),
                static_cast<unsigned>(output.bytes.size()));
        }
        if (!decoded) return;
        cISC4Automaton* boat = nullptr;
        if (!native->QueryInterface(GZIID_cISC4Automaton, reinterpret_cast<void**>(&boat)) || !boat) {
            ++bad; return;
        }
        FC4_ACQUIRED(boat, "watercraft shadow getters");
        cS3DVector3 actual[3];
        boat->GetPosition(actual[0]);
        boat->GetDestination(actual[1]);
        boat->GetOrientation(actual[2]);
        for (uint32_t i = 0; i < 3; ++i) {
            const float values[] = {actual[i].fX, actual[i].fY, actual[i].fZ};
            for (uint32_t axis = 0; axis < 3; ++axis)
                bad += !(std::fabs(values[axis] - vectors[i * 3 + axis]) <= 1e-5f);
        }
        auto* top = boat->GetState();
        bad += (top == nullptr) != (states[0] == 0);
        if (top) bad += boat->FindState(states[1]) != top;
        if (fixture) {
            const uint64_t expected = fc4_watercraft_shadow_probe(step);
            const uint64_t actualState = (static_cast<uint64_t>(states[0]) << 32) | states[1];
            if (actualState != expected) {
                ++bad;
                Log("watercraft shadow detail: probe=%u states=%u top=%u expected=%llu",
                    step, states[0], states[1], static_cast<unsigned long long>(expected));
            }
        }
        FC4_RELEASE(boat);
    };
    WriteCapture manager(false, true);
    if (!manager.Capture(reinterpret_cast<cIGZUnknown*>(g_city->GetWatercraftManager()))) {
        ++bad;
    } else {
        std::unordered_set<const void*> seen;
        for (uint32_t id = 1; manager.SerializableAt(id); ++id) {
            auto* object = manager.SerializableAt(id);
            if (manager.ClassOf(id) == 0x096E75F5 && seen.insert(object).second) {
                ++reachable;
                compare(object, 0, false);
            }
        }
    }
    // Always exercise the inherited stack, even when the city has live boats.
    // SPEC-GAP: no callable native watercraft path-step interface is published;
    // keep AI/physics/ferry ticking unmeasured rather than guessing signatures.
    cIGZSerializable* object = nullptr;
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    if (com && com->GetClassObject(0x096E75F5, 0xE4FDA3D4, reinterpret_cast<void**>(&object)) && object) {
        FC4_ACQUIRED(object, "watercraft shadow factory reference");
        ++fixtures;
        compare(object, 0, true);
        cISC4Automaton* boat = nullptr;
        if (object->QueryInterface(GZIID_cISC4Automaton, reinterpret_cast<void**>(&boat)) && boat) {
            FC4_ACQUIRED(boat, "watercraft shadow probe reference");
            // [abi-ret] watercraft §3.1 preserves the inherited boolean in AL.
            bad += !boat->SetState(2, nullptr, -1.0f, false, 0, 0, nullptr);
            ++probes;
            compare(object, 1, true);
            // [abi-ret] watercraft §3.1 preserves the inherited boolean in AL.
            bad += !boat->PushState(3, nullptr, -1.0f, false, 0, 0, nullptr);
            ++probes;
            compare(object, 2, true);
            // [abi-ret] watercraft §3.1 preserves the inherited boolean in AL.
            bad += !boat->PopState();
            ++probes;
            compare(object, 3, true);
            FC4_RELEASE(boat);
        } else ++bad;
        FC4_RELEASE_CREATED(object);
    } else ++bad;
    Log("watercraft shadow %s %s: partial=record-getters-inherited-stack reachable=%u fixtures=%u "
        "samples=%u probes=%u bytes=%llu path_steps=unmeasured mismatches=%u",
        DateText(julian).c_str(), what, reachable, fixtures, samples, probes,
        static_cast<unsigned long long>(bytes), bad);
    ++g_watercraftLines;
    if (!bad) ++g_watercraftClean;
}

void RunCompare(const char* what, int32_t julian) {
    WatercraftCompare(what, julian);
    HelicopterCompare(julian);
    if (!g_enabled || g_city == nullptr) return;
    std::string line;
    OwnerScan owners;
    uint32_t mismatches = 0;
    for (size_t i = 0; i < kParts; ++i) mismatches += ComparePart(g_parts[i], line, g_ownersEnabled ? &owners : nullptr);
    AppendF(line, "; mismatches=%u", mismatches);
    const std::string d = DateText(julian);
    Log("automata shadow %s %s:%s", d.c_str(), what, line.c_str());
    if (g_ownersEnabled) {
        // The occupant manager's saved references include constructions without
        // controllers too (occupant-manager §13); no guessed native enumerator.
        WriteCapture occupants(false, true);
        if (!occupants.Capture(reinterpret_cast<cIGZUnknown*>(g_city->GetOccupantManager()))) {
            owners.construction.Add(0, 1);
        } else {
            owners.References(occupants);
        }
        std::string detail;
        owners.Append(detail, "cSC4AutomataManager", owners.manager);
        owners.Append(detail, "cSC4TrafficAutomataManager", owners.traffic);
        owners.Append(detail, "cSC4Automaton", owners.automaton);
        owners.Append(detail, "cSC4AutomataController", owners.controller);
        owners.Append(detail, "cSC4ConstructionOccupant", owners.construction);
        const uint32_t bad = mismatches + owners.automaton.mismatches + owners.controller.mismatches +
                             owners.construction.mismatches + (owners.manager.records != 4 ? 1u : 0u) +
                             (owners.traffic.records != 2 ? 1u : 0u);
        Log("automataowners shadow %s %s:%s mismatches=%u", d.c_str(), what, detail.c_str(), bad);
        ++g_ownerLines;
        if (bad == 0) ++g_ownerCleanLines;
    }
    ++g_lines;
    if (mismatches == 0) {
        ++g_cleanLines;
    } else {
        g_mismatches += mismatches;
    }
    g_componentsCompared += kParts;
}

}  // namespace

void AutomataShadowStartup(bool automataReplaced) {
    const char* src = "ini";
    g_enabled = SettingInt("automata_shadow", "FC4_AUTOMATA_SHADOW", 0) != 0;
    g_ownersEnabled = SettingInt("automata_owners_shadow", "FC4_AUTOMATA_OWNERS_SHADOW", 0) != 0;
    g_helicopterEnabled = SettingInt("helicopter_shadow", "FC4_HELICOPTER_SHADOW", 0) != 0 && !automataReplaced;
    g_watercraftEnabled = SettingInt("watercraft_shadow", "FC4_WATERCRAFT_SHADOW", 0) != 0 && !automataReplaced;
    if (g_watercraftEnabled) Log("watercraft shadow: ON; partial records/getters/inherited-stack; "
        "private factory probes; AI/physics/ferry path stepping unmeasured");
    if (g_helicopterEnabled) Log("helicopter shadow: ON; partial metadata only [AF11], "
        "class and four record versions independently from a fresh Rust owner; no flight claim");
    g_enabled = g_enabled || g_ownersEnabled;
    if (g_enabled) Setting("automata_shadow", "FC4_AUTOMATA_SHADOW", &src);
    if (!g_enabled) return;
    if (automataReplaced) {
        Log("automata shadow: requested (automata_shadow from %s) but INACTIVE: it compares against the ORIGINAL "
            "automata components, and `automata` is enabled (ours)",
            src);
        g_enabled = false;
        g_ownersEnabled = false;
        return;
    }
    Log("automata shadow: ON (automata_shadow from %s): the original's six component records (40 the script system, "
        "41 the controller manager, 43-46 the four managers) and every object their records reference are read by a "
        "shadow core (FreeCity4's automata layer) and written back, and the two byte sequences are compared at each "
        "city load and each month. Success is mismatches=0 on every line.",
        src);
    if (g_ownersEnabled) Log("automataowners shadow: ON (automata_owners_shadow): complete inherited manager "
        "records, reachable automaton/controller records and complete construction records (all 15 fields, "
        "inherited holder, all reference identities, nulls, membership and order); same-input codec "
        "comparison only, crane presence seeded from captured record; no lifecycle/movement claim");
}

void AutomataShadowStart() {}

void AutomataShadowStop() {
    g_enabled = false;
    g_ownersEnabled = false;
    g_helicopterEnabled = false;
    g_watercraftEnabled = false;
    g_city = nullptr;
}

void AutomataShadowCityLoaded(cISC4City* city, int32_t julian) {
    g_city = city;
    g_watercraftLines = g_watercraftClean = 0;
    g_helicopterMonth = 0;
    if (city == nullptr) return;
    g_loggedNoRecord = false;
    RunCompare("load", julian);
}

void AutomataShadowCityShutdown() {
    if (g_watercraftLines) Log("watercraft shadow: shutdown: %llu lines, %llu with mismatches=0; partial records/getters/inherited-stack",
        static_cast<unsigned long long>(g_watercraftLines), static_cast<unsigned long long>(g_watercraftClean));
    if (g_helicopterLines) Log("helicopter shadow: shutdown: %llu lines, %llu with mismatches=0; partial AF11",
        static_cast<unsigned long long>(g_helicopterLines), static_cast<unsigned long long>(g_helicopterClean));
    g_helicopterLines = g_helicopterClean = 0;
    g_helicopterMonth = 0;
    if (g_lines == 0) {
        g_city = nullptr;
        return;
    }
    Log("automata shadow: city shutdown: %llu lines, %llu with mismatches=0; components compared %llu, bytes "
        "compared %llu, mismatches %llu",
        static_cast<unsigned long long>(g_lines), static_cast<unsigned long long>(g_cleanLines),
        static_cast<unsigned long long>(g_componentsCompared), static_cast<unsigned long long>(g_bytesCompared),
        static_cast<unsigned long long>(g_mismatches));
    if (g_ownersEnabled) Log("automataowners shadow: city shutdown: %llu lines, %llu with mismatches=0",
        static_cast<unsigned long long>(g_ownerLines), static_cast<unsigned long long>(g_ownerCleanLines));
    g_ownerLines = 0;
    g_ownerCleanLines = 0;
    for (size_t i = 0; i < kParts; ++i) DropCore(g_parts[i]);
    g_city = nullptr;
    g_lines = 0;
    g_cleanLines = 0;
    g_mismatches = 0;
    g_componentsCompared = 0;
    g_bytesCompared = 0;
}

void AutomataShadowMonth(int32_t julian) { RunCompare("month", julian); }

// Native regression seam: replay the supplied reference-only writer through
// the exact capture/GetRef/host path used by the owners shadow.
intptr_t AutomataCaptureTest(cIGZUnknown* object, uint32_t iid) {
    WriteCapture capture(false, true);
    {
        WriteCapture original(false, true);
        if (!original.Capture(object)) return -1;
        WriteCapture moved(std::move(original));
        capture = std::move(moved);
    }  // Moved-from captures must not release the replay's retained references.
    // Unmatched cleanup must not consume the capture pin. Exercise repeated
    // acquisitions independently of the references handed out by GetRef.
    if (capture.ReleaseReplayReference(1) ||
        !capture.AcquireReplayReference(1) || !capture.AcquireReplayReference(1) ||
        !capture.ReleaseReplayReference(1) || !capture.ReleaseReplayReference(1) ||
        capture.ReleaseReplayReference(1) || capture.AcquireReplayReference(UINT64_MAX)) return -2;
    CaptureIn input(capture);
    while (!input.atEnd()) {
        fc4_ref ref{};
        if (!input.Get()->get_ref(input.Get()->user, iid, &ref)) return 0;
        if (ref.token != 0) HostRelease(&capture, ref.token);
    }
    return 1;
}

// T66-T73 use exactly the production construction capture/codec/replay path.
// No native Read, Init, or mutation is performed by this test seam.
intptr_t ConstructionCaptureTest(cIGZUnknown* object) {
    WriteCapture capture(false, true);
    if (!capture.Capture(object)) return -1;
    CaptureIn input(capture);
    RecordOut output;
    const auto host = RefHost(&capture);
    return fc4_automata_construction_roundtrip(input.Get(), output.Get(), &host) &&
           input.atEnd() && output.bytes == capture.Bytes() ? 1 : 0;
}

}  // namespace fc4

void AutomataShadowStartup(bool automataReplaced) { fc4::AutomataShadowStartup(automataReplaced); }
void AutomataShadowStart() { fc4::AutomataShadowStart(); }
void AutomataShadowStop() { fc4::AutomataShadowStop(); }
void AutomataShadowCityLoaded(cISC4City* city, int32_t julian) {
    fc4::AutomataShadowCityLoaded(city, julian);
}
void AutomataShadowCityShutdown() { fc4::AutomataShadowCityShutdown(); }
void AutomataShadowMonth(int32_t julian) { fc4::AutomataShadowMonth(julian); }
