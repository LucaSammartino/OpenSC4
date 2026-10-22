// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
// Included once. Opt-in ORIGINAL sets, public SDK calls only. No command
// registration/execution/replacement, layouts, hooks or native-result seeding.
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "compat/FC4Compat.h"
#include "cIGZCommandServer.h"
#include "cIGZCommandParameterSet.h"
#include "cIGZVariant.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "cIGZFrameWork.h"
#include <cstring>
#include <string>

namespace fc4 { namespace commands_verify {
struct Snapshot {
    uint32_t count, first, next, present, kind, elements, bits, len;
    unsigned char text[512];
    uint32_t logical_elements;
};
static_assert(sizeof(Snapshot) == 548, "private transport layout");
extern "C" void* fc4_commands_verify_new();
extern "C" void fc4_commands_verify_release(void*);
extern "C" uint32_t fc4_commands_verify_mutate(void*, uint32_t, uint32_t, uint32_t,
    uint32_t, const unsigned char*, uint32_t, void*);
extern "C" uint32_t fc4_commands_verify_snapshot(void*, uint32_t, uint32_t, Snapshot*);
extern "C" uint32_t fc4_commands_verify_parse(const unsigned char*, uint32_t, uint32_t, uint32_t*, void**);

struct Pair {
    cIGZCommandParameterSet* native = nullptr;
    void* core = nullptr;
    ~Pair() {
        if (native) FC4_RELEASE_CREATED(native);
        if (core) fc4_commands_verify_release(core);
    }
    bool AcquireOriginal(cIGZCommandServer* server) {
        // Section 5: factory returns reference count 1 in EAX, not a bool.
        // Discard that result; validate the documented out pointer instead.
        server->CreateCommandParameterSet(&native);
        if (!native) return false;
        FC4_ACQUIRED(native, "commands verify: new ORIGINAL set");
        if (IsOurObject(native)) return false;
        core = fc4_commands_verify_new();
        return core != nullptr;
    }
};

struct Compare {
    unsigned checked = 0, mismatches = 0, refused = 0, elementMismatches = 0;
    unsigned logicalCountDifferences = 0;
    const char* phase = "sets";
    void Eq(const char* field, uint32_t id, uint32_t a, uint32_t b) {
        ++checked;
        if (a != b) {
            if (std::strcmp(field, "elements") == 0) ++elementMismatches;
            if (++mismatches <= 12)
                Log("commands_verify: DIFF phase=%s field=%s id=%08x native=%08x core=%08x", phase, field, id, a, b);
        }
    }
    void Check(Pair& p, uint32_t id, bool fallback = false) {
        Snapshot a{}, b{};
        a.count = p.native->GetParameterCount();
        a.first = p.native->GetFirstParameterID();
        a.next = p.native->GetNextParameterID(id);
        cIGZVariant* v = fallback ? (*p.native)[id] : p.native->GetParameter(id);
        a.present = v ? 1 : 0;
        a.kind = v ? v->GetType() : 0xffffffffu;
        if (v) {
            a.elements = v->GetCount();
            switch (a.kind) {
                case 0: break;
                case 1: a.bits = v->GetValBool() ? 1 : 0; break;
                case 6: a.bits = v->GetValUint32(); break;
                case 7: a.bits = static_cast<uint32_t>(v->GetValSint32()); break;
                case 10: { float f = v->GetValFloat32(); std::memcpy(&a.bits, &f, sizeof(f)); break; }
                case 0x800e: {
                    cRZBaseString s;
                    if (!v->GetValString(s) || s.Strlen() > sizeof(a.text)) { ++refused; return; }
                    a.len = s.Strlen();
                    std::memcpy(a.text, s.ToChar(), a.len);
                    break;
                }
                default: ++refused; return;
            }
        }
        if (!fc4_commands_verify_snapshot(p.core, id, fallback, &b)) { ++refused; return; }
        // Preserve the historical raw-native versus logical DTO diagnostic.
        // All original assertions below now use the production native projection.
        if (a.elements != b.logical_elements) ++logicalCountDifferences;
        Eq("count", id, a.count, b.count); Eq("first", id, a.first, b.first);
        Eq("next", id, a.next, b.next); Eq("present", id, a.present, b.present);
        Eq("type", id, a.kind, b.kind); Eq("elements", id, a.elements, b.elements);
        Eq("value", id, a.bits, b.bits); Eq("length", id, a.len, b.len);
        Eq("text", id, 0, a.len == b.len && std::memcmp(a.text, b.text, a.len) == 0 ? 0 : 1);
    }
    void Survey(Pair& p) {
        const uint32_t ids[] = {0, 1, 2, 5, 6, 11, 12, 0x2b999d2b, 0x2b999d2c, 0x80000000, 0xffffffff};
        for (uint32_t id : ids) Check(p, id);
        Check(p, 123456, true);
        // ORIGINAL order selects read queries only. First/next results are
        // compared independently; native traversal never mutates core state.
        uint32_t id = p.native->GetFirstParameterID();
        for (unsigned n = 0; id != 0xffffffff && n < 300; ++n) {
            Check(p, id); id = p.native->GetNextParameterID(id);
        }
    }
    void Mutate(Pair& p, Pair& scratch, uint32_t op, uint32_t id = 0,
                uint32_t kind = 0, uint32_t bits = 0, const std::string& text = {}) {
        cIGZVariant* v = scratch.native->GetParameter(0);
        if (!v) { ++refused; return; }
        // ORIGINAL input variant, SDK string is only an input carrier.
        switch (kind) {
            case 0: v->Erase(); break;
            case 1: v->SetValBool(bits != 0); break;
            case 6: v->SetValUint32(bits); break;
            case 7: v->SetValSint32(static_cast<int32_t>(bits)); break;
            case 10: { float f; std::memcpy(&f, &bits, sizeof(f)); v->SetValFloat32(f); break; }
            case 0x800e: { cRZBaseString s(text.c_str()); v->SetValString(s); break; }
            default: ++refused; return;
        }
        switch (op) {
            case 1: p.native->SetParameter(id, *v); break;
            case 2: p.native->AppendParameter(*v); break;
            case 3: p.native->SetParameterCount(bits); break;
            case 4: p.native->RemoveParameter(id); break;
            case 5: p.native->SetStatusParameterValue(static_cast<int32_t>(bits)); break;
            // [abi-ret] CopyFrom return is discarded; only stored value is measured.
            case 6: (void)(*p.native)[id]->CopyFrom(*v); break;
            default: ++refused; return;
        }
        if (!fc4_commands_verify_mutate(p.core, op, id, kind, bits,
            reinterpret_cast<const unsigned char*>(text.data()), static_cast<uint32_t>(text.size()), nullptr)) ++refused;
        v->SetValSint32(0x12345678); // Check the copy after changing the input.
        Survey(p);
    }
};

inline void Run() {
    if (SettingInt("commands_verify", "FC4_COMMANDS_VERIFY", 0) == 0) return;
    if (!compat::IdentifiedVersion()) { Log("commands_verify: REFUSED unsupported version"); return; }
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZCommandServer* server = nullptr;
    if (!fw || !fw->GetSystemService(0xeb903a32, 0xcb90253a, reinterpret_cast<void**>(&server)) || !server) {
        Log("commands_verify: REFUSED unavailable service"); return;
    }
    FC4_ACQUIRED(server, "commands verify: app-owned service");
    if (IsOurObject(server)) { Log("commands_verify: REFUSED service is ours"); FC4_RELEASE(server); return; }
    Compare c;
    {
        Pair p, scratch, copied;
        if (!p.AcquireOriginal(server) || !scratch.AcquireOriginal(server) || !copied.AcquireOriginal(server)) ++c.refused;
        else {
            scratch.native->SetParameterCount(1);
            if (!fc4_commands_verify_mutate(scratch.core, 3, 0, 0, 1, nullptr, 0, nullptr)) ++c.refused;
            c.Survey(p);
            c.Mutate(p, scratch, 1, 5, 0x800e, 0, "MiXeD text");
            c.Mutate(p, scratch, 1, 0, 7, 37);
            c.Mutate(p, scratch, 1, 11, 7, static_cast<uint32_t>(-1234567));
            c.Mutate(p, scratch, 2, 0, 1, 1);
            c.Mutate(p, scratch, 1, 1, 1, 0);
            c.Mutate(p, scratch, 1, 2, 6, 0xfedcba98);
            c.Mutate(p, scratch, 1, 0x80000000, 10, 0xc0200000); // -2.5
            c.Mutate(p, scratch, 1, 6, 10, 0x40600000); // +3.5
            c.Mutate(p, scratch, 1, 5, 0x800e, 0, "replacement");
            c.Mutate(p, scratch, 1, 6, 0x800e, 0, "");
            c.Mutate(p, scratch, 5, 0, 0, static_cast<uint32_t>(-17));
            c.Mutate(p, scratch, 2, 0, 0x800e, 0, "after highest");
            c.Mutate(p, scratch, 4, 11); c.Mutate(p, scratch, 4, 999);
            c.Mutate(p, scratch, 6, 123456, 7, static_cast<uint32_t>(-99));
            c.Mutate(p, scratch, 3, 0, 0, 3); c.Mutate(p, scratch, 3, 0, 0, 7);
            c.Mutate(p, scratch, 3, 0, 0, 0);
            c.Mutate(p, scratch, 1, 0xffffffff, 7, 91);
            c.Mutate(p, scratch, 2, 0, 7, static_cast<uint32_t>(-91));
            c.Mutate(p, scratch, 3, 0, 0, 0);
            c.Mutate(p, scratch, 1, 0, 0x800e, 0, "owned copy");
            copied.native->SetParameter(5, *p.native->GetParameter(0));
            if (!fc4_commands_verify_mutate(copied.core, 7, 5, 0, 0, nullptr, 0, p.core)) ++c.refused;
            c.Mutate(p, scratch, 1, 0, 0x800e, 0, "changed source");
            c.Survey(copied);
        }
    }
    unsigned sets = c.checked;
    c.phase = "parser";
    cRZBaseString name("SetDebugLevel");
    const uint32_t known = server->GetCommandID(name);
    c.Eq("documented_command_id", 0, known, 0x2bb40833);
    if (known != 0x2bb40833) ++c.refused;
    else {
        std::string lines[] = {"", " \t  ", "FC4_verify_unregistered_6f43 x", "SetDebugLevel", "setdebuglevel -123",
            "SetDebugLevel 37 -37 0 4294967295 -2.5 true false", " SetDebugLevel   one\ttwo  ",
            "SetDebugLevel \"hello world\" \"\" y", "SetDebugLevel a\"b c\"d \"c\"d", "SetDebugLevel \"abc def",
            "\"SetDebugLevel\" \"a\tb\" c", "SetDebugLevel a\nb", "SetDebugLevel 1 ignored", "SetDebugLevel"};
        for (unsigned i = 0; i < 270; ++i) lines[13] += " x";
        for (unsigned i = 0; i < sizeof(lines) / sizeof(lines[0]); ++i) {
            Pair parsed;
            uint32_t a = 0xcccccccc, b = 0xcccccccc;
            const uint32_t length = i == 12 ? 15u : (i % 2 ? 0xffffffffu : static_cast<uint32_t>(lines[i].size()));
            // [abi-ret] Section 3.4 returns AL; section 5 slot 22 forwards it unchanged.
            const bool native = server->ConvertStringToCommand(lines[i].c_str(), length, a, parsed.native);
            if (parsed.native) FC4_ACQUIRED(parsed.native, "commands verify: parsed ORIGINAL set");
            const uint32_t core = fc4_commands_verify_parse(reinterpret_cast<const unsigned char*>(lines[i].c_str()),
                static_cast<uint32_t>(lines[i].size() + 1), length, &b, &parsed.core);
            c.Eq("parse_return", i, native ? 1 : 0, core); c.Eq("parse_id", i, a, b);
            c.Eq("parse_pointer", i, parsed.native ? 1 : 0, parsed.core ? 1 : 0);
            if (parsed.native && parsed.core && !IsOurObject(parsed.native)) c.Survey(parsed);
            else if (parsed.native || parsed.core) ++c.refused;
        }
    }
    FC4_RELEASE(server);
    Log("commands_verify: checked=%u exact=%u mismatches=%u refused=%u sets=%u parser=%u elements_diffs=%u other_diffs=%u historical_logical_count_diffs=%u scope=owned-original-production-methods GC1-GC7=excluded",
        c.checked, c.checked - c.mismatches, c.mismatches, c.refused, sets, c.checked - sets,
        c.elementMismatches, c.mismatches - c.elementMismatches, c.logicalCountDifferences);
}
} }
