// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
// Ephemeral, read-only process probe. No command-line ownership or mutators.
#include "FC4Config.h"
#include "FC4Diag.h"
#include "FC4RefAudit.h"
#include "cIGZFrameWork.h"
#include "cIGZCmdLine.h"
#include "cRZBaseString.h"
#include <windows.h>
#include <algorithm>
#include <cstring>
#include <string>
#include <vector>

extern "C" {
void* fc4_cmdline_probe_create(const char* raw);
void fc4_cmdline_probe_destroy(void* handle);
int32_t fc4_cmdline_probe_query(const void* handle, uint32_t op, const char* key,
    int32_t param, char* output, uint32_t capacity, uint32_t* length, int32_t* scalar);
}

namespace fc4 {
inline void CmdLineProbe(cIGZFrameWork* framework) {
    static bool ran = false;
    if (ran || !SettingInt("cmdline_probe", nullptr, 0)) return;
    ran = true;
    if (!framework) { Log("CMDLINE native acquisition failure: no framework"); return; }
    // The director's initialized public framework remains alive for this scope.
    // Take our own audited hold; CommandLine() is borrowed and NEVER released.
    struct Hold {
        cIGZFrameWork* p;
        explicit Hold(cIGZFrameWork* f) : p(f) { FC4_ADDREF(p); }
        ~Hold() { FC4_RELEASE(p); }
    } hold(framework);
    const cIGZCmdLine* native = framework->CommandLine();
    if (!native) { Log("CMDLINE native acquisition failure: null borrowed command line"); return; }
    // command-line.md section 5: Windows NT supplies the FULL GetCommandLineA.
    // This independent capture is the ONLY parser input, never native getters.
    const std::string raw(GetCommandLineA());
    void* rust = fc4_cmdline_probe_create(raw.c_str());
    if (!rust) { Log("CMDLINE Rust construction failure"); return; }
    uint32_t total = 0, exact = 0, failures = 0;
    uint32_t calls[10] = {}, matches[10] = {}, present[10] = {}, absent[10] = {};
    auto query = [&](uint32_t op, const std::string& key, int32_t param) {
        cRZBaseString k(key), value("__fc4_unchanged__");
        int32_t n = 0;
        // Public typed SDK dispatch, including MSVC's overloaded virtual layout.
        // No hand-written slots, addresses, private storage, or private slot 16.
        switch (op) {
        case 0: n = static_cast<int32_t>(native->argc()); break;
        case 1: value.FromChar(native->argv(param)); break;
        case 2: n = native->GetCommandLineText(value); break;
        case 3: n = native->Find(k, param != 0); break;
        case 4: n = native->FindSubString(k, param != 0); break;
        case 5: n = native->IsSwitchPresent(k); break;
        case 6: n = native->IsSwitchPresent(k, value, param != 0); break;
        case 7: n = native->IsSwitchPresent(key[0], value, param != 0); break;
        case 8: n = native->GetIndexOfSwitch(k, param); break;
        case 9: n = native->GetIndexOfSwitch(key[0], param); break;
        }
        std::vector<char> out(raw.size() + 64);
        uint32_t len = 0;
        int32_t r = 0;
        const int32_t status = fc4_cmdline_probe_query(rust, op, key.c_str(), param,
            out.data(), static_cast<uint32_t>(out.size()), &len, &r);
        ++total;
        const bool ok = status == 0 && n == r && len == value.Strlen()
            && std::memcmp(out.data(), value.ToChar(), len) == 0;
        exact += ok ? 1u : 0u;
        ++calls[op];
        matches[op] += ok ? 1u : 0u;
        if (op >= 3) {
            const bool found = (op == 3 || op == 4 || op == 8 || op == 9) ? n >= 0 : n != 0;
            ++(found ? present[op] : absent[op]);
        }
        if (op <= 2) Log("CMDLINE natural op=%u index=%d scalar=%d text='%s' exact=%d",
            op, param, n, value.ToChar(), ok ? 1 : 0);
        if (status) ++failures;
        if (!ok) Log("CMDLINE difference op=%u key='%s' param=%d native=%d rust=%d native_text='%s' rust_text='%.*s' status=%d",
            op, key.c_str(), param, n, r, value.ToChar(), static_cast<int>(len), out.data(), status);
    };
    query(0, "", 0);
    query(2, "", 0);
    // Bounds derived from independently parsed raw input, not native answers.
    uint32_t length = 0;
    int32_t argc = 0;
    char scratch[32];
    const int32_t countStatus = fc4_cmdline_probe_query(rust, 0, "", 0,
        scratch, sizeof scratch, &length, &argc);
    if (countStatus == 0 && argc >= 0) {
        for (int32_t i = 0; i <= argc; ++i) query(1, "", i);
    } else ++failures;
    // Known ASCII process switches and a negative name; no empty-name CL2 or
    // high-byte CL3 queries. Values/order/text still compare genuine process data.
    const char* names[] = {"intro", "CPUCount", "CustomResolution", "r", "d", "f", "w", "__fc4_absent_7F91__"};
    for (const char* name : names) {
        query(5, name, 0);
        for (int32_t flag : {0, 1}) query(6, name, flag);
        for (int32_t start : {-1, 0, argc}) query(8, name, start);
        std::string upper(name);
        for (char& c : upper) if (c >= 'a' && c <= 'z') c -= 'a' - 'A';
        query(5, upper, 0);
    }
    for (char c : std::string("iCrdfwZ")) {
        const std::string key(1, c);
        for (int32_t flag : {0, 1}) query(7, key, flag);
        for (int32_t start : {-1, 0, argc}) query(9, key, start);
    }
    // Natural full-argument Find and substring queries: raw-derived Rust argv,
    // never native outputs. Only ASCII nonempty keys enter query comparisons.
    for (int32_t i = 0; i < argc; ++i) {
        std::vector<char> arg(raw.size() + 64);
        int32_t ignored = 0;
        uint32_t size = 0;
        if (fc4_cmdline_probe_query(rust, 1, "", i, arg.data(),
                static_cast<uint32_t>(arg.size()), &size, &ignored)) { ++failures; continue; }
        const std::string key(arg.data(), size);
        if (key.empty() || std::any_of(key.begin(), key.end(), [](unsigned char c) { return c >= 128; })) continue;
        for (int32_t flag : {0, 1}) {
            query(3, key, flag);
            query(4, key.substr(0, std::min<size_t>(3, key.size())), flag);
        }
    }
    for (int32_t flag : {0, 1}) {
        query(3, "__fc4_absent_7F91__", flag);
        query(4, "__fc4_absent_7F91__", flag);
    }
    fc4_cmdline_probe_destroy(rust);
    for (uint32_t op = 0; op < 10; ++op)
        Log("CMDLINE method op=%u total=%u exact=%u present=%u absent=%u",
            op, calls[op], matches[op], present[op], absent[op]);
    Log("CMDLINE native readonly total=%u exact=%u differences=%u failures=%u raw_bytes=%u argc=%d borrowed_release=0 mutations=0 constructed_native=0 CL4=open",
        total, exact, total - exact, failures, static_cast<uint32_t>(raw.size()), argc);
}
} // namespace fc4
