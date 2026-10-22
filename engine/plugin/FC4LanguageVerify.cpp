// SPDX-License-Identifier: GPL-3.0-or-later
// Constructed ORIGINAL-vs-production-core verification; never intercepts or
// modifies the manager's borrowed utility, current language, or process locale.
#include "FC4LanguageVerify.h"
#include "FC4Config.h"
#include "FC4GameList.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cIGZFrameWork.h"
#include "cIGZLanguageManager.h"
#include "cIGZLanguageUtility.h"
#include "cIGZString.h"
#include "cRZCOMDllDirector.h"
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace {
using HostFn = int32_t (*)(uint32_t, const uint8_t*, size_t, const uint8_t*, size_t,
                           double, uint8_t*, size_t);
struct Request {
    uint32_t op, a, b, c, d;
    int64_t integer;
    double value;
    const uint8_t* text;
    size_t textLen;
    const uint8_t* other;
    size_t otherLen;
    uint8_t* out;
    size_t capacity, length;
    HostFn host;
};
static_assert(offsetof(Request, integer) == 24, "Rust repr(C) request alignment");
extern "C" void* fc4_language_verify_new(uint32_t);
extern "C" int32_t fc4_language_verify_call(void*, Request*);
extern "C" void fc4_language_verify_free(void*);

// Explicit foreign LocaleHost boundary. snprintf belongs to OUR DLL's CRT.
// Its comparisons are reported separately: no assertion about private game
// CRT identity/bindings is made. No locale writes, native-result oracle, or
// calls to the utility under comparison are permitted in this callback.
int32_t Host(uint32_t op, const uint8_t* a, size_t an, const uint8_t* b, size_t bn,
             double value, uint8_t* out, size_t cap) {
    if (op == 0) {
        const std::string format(reinterpret_cast<const char*>(a), an);
        return std::snprintf(reinterpret_cast<char*>(out), cap, format.c_str(), value);
    }
    if (op == 5 || op == 6) {
        const int r = CompareStringA(LOCALE_USER_DEFAULT, op == 6 ? NORM_IGNORECASE : 0,
            reinterpret_cast<const char*>(a), static_cast<int>(an),
            reinterpret_cast<const char*>(b), static_cast<int>(bn));
        return r == 0 ? 2 : r - CSTR_EQUAL;
    }
    if (an > cap) return -1;
    if (op == 3 || op == 4) {
        std::memcpy(out, a, an);
        if (an != 0) {
            if (op == 3) CharLowerBuffA(reinterpret_cast<char*>(out), static_cast<DWORD>(an));
            else CharUpperBuffA(reinterpret_cast<char*>(out), static_cast<DWORD>(an));
        }
        return static_cast<int32_t>(an);
    }
    if (an == 0) return 0;
    const UINT from = op == 1 ? CP_UTF8 : CP_ACP;
    const UINT to = op == 1 ? CP_ACP : CP_UTF8;
    const int wn = MultiByteToWideChar(from, 0, reinterpret_cast<const char*>(a), static_cast<int>(an), nullptr, 0);
    if (wn == 0) return -1;
    std::vector<wchar_t> w(static_cast<size_t>(wn));
    MultiByteToWideChar(from, 0, reinterpret_cast<const char*>(a), static_cast<int>(an), w.data(), wn);
    const int n = WideCharToMultiByte(to, 0, w.data(), wn, reinterpret_cast<char*>(out), static_cast<int>(cap), nullptr, nullptr);
    return n == 0 ? -1 : n;
}

struct String {
    fc4::GameString storage{};
    bool good;
    explicit String(const std::string& s) : good(fc4::GameStringInit(storage, s.data(), s.size())) {}
    ~String() { if (good) fc4::GameStringDestroy(storage); }
    cIGZString& ref() { return *reinterpret_cast<cIGZString*>(&storage); }
    std::string bytes() const { return std::string(storage.data(), storage.size()); }
    String(const String&) = delete;
    String& operator=(const String&) = delete;
};

std::string Hex(const std::string& s) {
    const char* h = "0123456789abcdef";
    std::string out;
    for (unsigned char c : s) { out += h[c >> 4]; out += h[c & 15]; }
    return out;
}

struct Counts { uint32_t total = 0, exact = 0, positive = 0, negative = 0; };
struct Probe {
    cIGZLanguageUtility* native;
    void* core;
    uint32_t id;
    Counts strict, host, gap;
    uint32_t refusals = 0;
    explicit Probe(cIGZLanguageUtility* u, uint32_t language) : native(u), core(fc4_language_verify_new(language)), id(language) {}
    ~Probe() { fc4_language_verify_free(core); }

    void Check(Request r, const std::string& seed = "", const std::string& text = "",
               const std::string& other = "", uint32_t category = 0) {
        String out(seed), source(text), symbol(other);
        if (!out.good || !source.good || !symbol.good) {
            ++refusals;
            fc4::Log("language native refusal: id=%u op=%u storage", id, r.op);
            return;
        }
        uint8_t buffer[8192];
        std::memcpy(buffer, seed.data(), seed.size());
        r.out = buffer; r.capacity = sizeof(buffer); r.length = seed.size(); r.host = Host;
        r.text = reinterpret_cast<const uint8_t*>(text.data()); r.textLen = text.size();
        r.other = reinterpret_cast<const uint8_t*>(other.data()); r.otherLen = other.size();
        int32_t n = -999;
        // SDK slots 0..29, no copied executable code or private function addresses.
        switch (r.op) {
        case 0: {
            void* q = reinterpret_cast<void*>(static_cast<uintptr_t>(0x1234));
            n = native->QueryInterface(r.a, &q);
            if (n) {
                auto* owned = static_cast<cIGZUnknown*>(q);
                FC4_ACQUIRED(owned, "language verification QI");
                if (q != native) n = -996;
                FC4_RELEASE(owned);
            } else if (q != reinterpret_cast<void*>(static_cast<uintptr_t>(0x1234))) n = -995;
            break;
        }
        case 1: n = static_cast<int32_t>(FC4_ADDREF(native)); break;
        case 2: n = static_cast<int32_t>(FC4_RELEASE(native)); break;
        case 3: n = native->Init(); break;
        case 4: n = native->Shutdown(); break;
        case 5: n = native->GetDayName(static_cast<uint8_t>(r.a), out.ref()); break;
        case 6: n = native->GetAbbrDayName(static_cast<uint8_t>(r.a), out.ref()); break;
        case 7: n = native->GetMonthName(static_cast<uint8_t>(r.a), out.ref()); break;
        case 8: n = native->GetAbbrMonthName(static_cast<uint8_t>(r.a), out.ref()); break;
        case 9: n = native->MakeTimeString(static_cast<int32_t>(r.a), static_cast<int32_t>(r.b), static_cast<int32_t>(r.c), out.ref()); break;
        case 10: n = native->MakeDateString(static_cast<int32_t>(r.a), static_cast<int32_t>(r.b), r.c, out.ref(), r.d); break;
        case 11: n = native->GetCurrencySymbol(out.ref()); break;
        case 12: n = native->DoesCurrencySymbolPrecedeAmount(); break;
        case 13: n = native->IsSpaceBetweenCurrencySymbolAndAmount(); break;
        case 14: n = native->GetThousandSeparator(out.ref()); break;
        case 15: n = native->GetDecimalSeparator(out.ref()); break;
        case 16: n = native->MakeMoneyString(r.integer, out.ref(), r.a ? &symbol.ref() : nullptr); break;
        case 17: n = native->MakeFormattedMoneyString(r.value, out.ref(), source.ref(), r.a ? &symbol.ref() : nullptr); break;
        case 18: n = native->MakeNumberString(r.integer, out.ref()); break;
        case 19: n = native->MakeFormattedNumberString(r.value, out.ref(), source.ref()); break;
        case 20: n = native->ConvertToLowerCase(source.ref(), out.ref()); break;
        case 21: n = native->ConvertToUpperCase(source.ref(), out.ref()); break;
        case 22: n = native->CompareStrings(source.ref(), symbol.ref(), r.a != 0); break;
        case 23: n = native->GetCharacterSetFromLanguage(); break;
        case 24: n = native->DoesLanguageUseCharacterSet(static_cast<int32_t>(r.a)); break;
        case 25: n = native->GetLanguageRoadDrivingSide(); break;
        case 26: n = static_cast<int32_t>(native->GetMeasurementSystem()); break;
        case 27: n = native->AreFirstAndLastNamesReversed(); break;
        case 28: n = native->DoesLanguageUseInputMethodEditing(); break;
        case 29: n = native->DoesLanguageUseMultiByteCharacters(); break;
        case 31: case 32: {
            // Published ABI slots, one u32 argument; void return per §1.
            using Set = void (__thiscall*)(cIGZLanguageUtility*, uint32_t);
            const auto vt = *reinterpret_cast<void***>(native);
            reinterpret_cast<Set>(vt[r.op])(native, r.a); n = 0; break;
        }
        }
        const int32_t c = fc4_language_verify_call(core, &r);
        const std::string ours(reinterpret_cast<char*>(buffer), r.length);
        const std::string original = out.bytes();
        // A successful native QI was immediately balanced above; likewise core.
        if (r.op == 0 && c == 1) { Request release = r; release.op = 2; fc4_language_verify_call(core, &release); }
        Counts& count = category == 1 ? host : category == 2 || id == 0 ? gap : strict;
        ++count.total;
        if (n > 0) ++count.positive; else ++count.negative;
        if (n == c && original == ours) ++count.exact;
        else fc4::Log("language native DIFF: id=%u op=%u a=%u b=%u c=%u d=%u i=%lld v=%.17g category=%u seed=%s text=%s other=%s ret=%d/%d raw=%s/%s",
            id, r.op, r.a, r.b, r.c, r.d, static_cast<long long>(r.integer), r.value, category,
            Hex(seed).c_str(), Hex(text).c_str(), Hex(other).c_str(), n, c, Hex(original).c_str(), Hex(ours).c_str());
    }
};
Request Op(uint32_t op, uint32_t a = 0, uint32_t b = 0, uint32_t c = 0, uint32_t d = 0) {
    Request r{}; r.op = op; r.a = a; r.b = b; r.c = c; r.d = d; return r;
}
} // namespace

namespace fc4 {
void LanguageVerifyRun() {
    if (!SettingInt("language_native_verify", "FC4_LANGUAGE_NATIVE_VERIFY", 0)) return;
    auto* framework = RZGetFrameWork();
    cIGZLanguageManager* manager = nullptr;
    if (!StlNodeAllocator::Available() || !StlNodeAllocator::StringVtable() || !framework ||
        !framework->GetSystemService(0x441E5070, 0x0054B7D5, reinterpret_cast<void**>(&manager)) || !manager) {
        Log("language native REFUSED: supported storage or manager service unavailable; constructed=0 natural=0");
        return;
    }
    FC4_ACQUIRED(manager, "language verification service hold");
    const uint32_t languageBefore = manager->GetCurrentLanguage(), countryBefore = manager->GetCurrentCountry();
    cIGZLanguageUtility* borrowed = manager->GetLanguageUtility(0); // borrowed, never released/mutated
    const int driveBefore = borrowed ? borrowed->GetLanguageRoadDrivingSide() : -1;
    Log("language native BEGIN: constructed only, ids=0..45 rawbytes; production LanguageUtility via CAPI; natural=0/0 (not exercised); LU1-LU3/SL1 open; foreign CRT separate; resync=0 fallback=0");
    uint32_t objects = 0, exact = 0, total = 0, hostExact = 0, hostTotal = 0, gapExact = 0, gapTotal = 0, refused = 0;
    for (uint32_t id = 0; id <= 0x2d; ++id) {
        cIGZLanguageUtility* native = manager->GetNewLanguageUtility(id);
        if (!native || native == borrowed) {
            Log("language native REFUSED: id=%u new utility null/borrowed", id); ++refused; continue;
        }
        FC4_ACQUIRED(native, "GetNewLanguageUtility created owned reference");
        ++objects;
        Probe p(native, id);
        for (uint32_t iid : {1u, 0x1054B7D5u, 0u, 0x0054B7D5u, 0xffffffffu}) p.Check(Op(0, iid));
        p.Check(Op(1)); p.Check(Op(2)); p.Check(Op(3));
        for (uint32_t method = 5; method <= 8; ++method) {
            for (uint32_t index = 0; index < 256; ++index) {
                p.Check(Op(method, index)); p.Check(Op(method, index), "sentinel");
            }
            for (uint32_t index : {256u, 257u, 263u, 268u, 511u, 0x10001u, 0xffffffffu}) {
                p.Check(Op(method, index)); p.Check(Op(method, index), "sentinel");
            }
        }
        for (uint32_t method : {11u, 12u, 13u, 14u, 15u, 23u, 25u, 26u, 27u, 28u, 29u}) p.Check(Op(method), "sentinel");
        for (uint32_t cp : {0u, 8u, 16u, 874u, 932u, 936u, 949u, 950u, 1250u, 1251u, 1252u, 1253u, 1254u, 1255u, 1256u, 1257u, 65001u, 0xffffffffu}) p.Check(Op(24, cp));
        for (uint32_t country : {0u, 1u, 0x35u, 0x36u, 0x37u, 0xffffffffu}) { p.Check(Op(32, country)); p.Check(Op(26)); }
        p.Check(Op(32, 0x36));
        for (int64_t value : {INT64_C(0), INT64_C(1), INT64_C(-1), INT64_C(999), INT64_C(-999), INT64_C(1000), INT64_C(-1000), INT64_C(123456789), INT64_C(-123456789), INT64_MAX}) {
            Request r = Op(18); r.integer = value; p.Check(r, "old");
            r.op = 16; p.Check(r, "old"); r.a = 1; p.Check(r, "old", "", "TOK"); p.Check(r, "old", "", "");
        }
        for (uint32_t hour : {0u, 7u, 12u, 13u, 23u, 24u, 49u, 0xffffffffu})
            for (uint32_t minute : {0u, 5u, 59u, 60u, 0xffffffffu})
                for (uint32_t second : {0u, 9u, 59u, 60u, 0xffffffffu}) p.Check(Op(9, hour, minute, second), "old");
        for (uint32_t style : {0u, 1u, 2u, 3u, 4u, 5u, 6u, 0xffffffffu})
            for (uint32_t month : {0u, 1u, 2u, 3u, 12u, 13u, 257u})
                for (uint32_t year : {0u, 5u, 2032u}) p.Check(Op(10, month, 7, year, style), "old");
        p.Check(Op(10, 3, 7, 0xfffffffbu, 0), "old", "", "", 2); // LU1
        for (const char* format : {"%.2f", "%10.2f", "%.3e", "%.5g", "pre%% %.2f %%post"})
            for (double value : {0.0, -0.0, 1.25, -1.25, 1234567.5, -1234567.5}) {
                Request r = Op(17); r.value = value; p.Check(r, "old", format, "", 1);
                r.a = 1; p.Check(r, "old", format, "TOK", 1);
                // Only use valid one-double formats for the unchecked printf slot.
                r.op = 19; p.Check(r, "old", format, "", 1);
            }
        for (const char* invalid : {"plain", "%%", "%d", "%n", "%", "%q"}) p.Check(Op(17), "old", invalid);
        for (uint32_t method : {20u, 21u}) {
            // LU2 also leaves the return value unspecified for these lengths.
            p.Check(Op(method), "", "AbC", "", 2);
            p.Check(Op(method), "xyz", "AbC", "", 2);
            p.Check(Op(method), "x", "AbC", "", 2);
            p.Check(Op(method), "toolong", "AbC", "", 2); // LU2
        }
        for (uint32_t sensitive : {0u, 1u})
            for (const char* text : {"abc", "ABC", "abd", "ab"}) p.Check(Op(22, sensitive), "", text, "abc");
        // Exercise setters without changing the manager or refreshing core from native.
        p.Check(Op(31, 2)); p.Check(Op(25)); p.Check(Op(31, id)); p.Check(Op(25));
        p.Check(Op(4), "", "", "", 2); // LU3 result observed, not normative
        p.Check(Op(14)); p.Check(Op(3)); p.Check(Op(14));
        Request release = Op(2);
        uint8_t dummy = 0;
        release.text = release.other = &dummy; release.out = &dummy; release.capacity = 1; release.host = Host;
        const int32_t cr = fc4_language_verify_call(p.core, &release);
        const uint32_t nr = FC4_RELEASE_CREATED(native);
        Log("language native id=%u strict=%u/%u positive=%u negative=%u host=%u/%u gaps=%u/%u refused=%u finalRelease=%u/%d",
            id, p.strict.exact, p.strict.total, p.strict.positive, p.strict.negative,
            p.host.exact, p.host.total, p.gap.exact, p.gap.total, p.refusals, nr, cr);
        exact += p.strict.exact; total += p.strict.total; hostExact += p.host.exact; hostTotal += p.host.total;
        gapExact += p.gap.exact; gapTotal += p.gap.total; refused += p.refusals;
        if (nr != 0 || cr != 0) { ++refused; Log("language native lifetime DIFF: id=%u native=%u core=%d", id, nr, cr); }
    }
    const bool control = languageBefore == manager->GetCurrentLanguage() && countryBefore == manager->GetCurrentCountry() &&
        borrowed == manager->GetLanguageUtility(0) && (!borrowed || driveBefore == borrowed->GetLanguageRoadDrivingSide());
    Log("language native END: objects=%u strict=%u/%u host=%u/%u gaps=%u/%u refused=%u natural=0/0-not-exercised resync=0 fallback=0 managerUnchanged=%u", objects, exact, total, hostExact, hostTotal, gapExact, gapTotal, refused, control ? 1 : 0);
    FC4_RELEASE(manager);
}
} // namespace fc4
