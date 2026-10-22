// SPDX-License-Identifier: GPL-3.0-or-later
// Clean-room drawing-fonts.md sections 1, 12 and 12.1; SDK virtual calls are MSVC thiscall.
#include "FC4FontSysVerify.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "compat/FC4Compat.h"
#include "cIGZCOM.h"
#include "cIGZFontSys.h"
#include "cRZCOMDllDirector.h"
#include "SC4String.h"
#include <windows.h>
#include <algorithm>
#include <array>
#include <cstdint>
#include <cstring>
#include <string>
#include <vector>

struct FontSysMetadata { char path[260]; char family[128]; uint32_t style, kind; };
static_assert(sizeof(FontSysMetadata) == 396, "FontSys raw provider metadata");
extern "C" {
void* fc4_fontsys_verify_new(const FontSysMetadata*, uint32_t);
uint32_t fc4_fontsys_verify_call(void*, uint32_t, uint32_t, uint32_t, uint32_t, uint32_t,
                               const char*, const char*, uint8_t*);
void fc4_fontsys_verify_free(void*);
}

namespace fc4 {
namespace {
// Use the published slot numbers for overloaded SDK methods. MSVC groups/reverses
// overload declarations; their source order is not a native-vtable contract.
template<class Result, class... Args>
Result FontSysSlot(cIGZFontSys* self, unsigned slot, Args... args) {
    using Function = Result(__fastcall*)(cIGZFontSys*, void*, Args...);
    return reinterpret_cast<Function>((*reinterpret_cast<void***>(self))[slot])(self, nullptr, args...);
}
void PutString(uint8_t* dst, size_t capacity, const char* text) {
    std::memset(dst, 0, capacity);
    std::memcpy(dst, text, (std::min)(std::strlen(text), capacity - 1));
}
// SetRenderer uses only the three IUnknown slots (section 12). This sentinel
// is never passed to a font or draw call; the stack owner keeps reference 1.
struct LifetimeRenderer final : cIGZUnknown {
    uint32_t refs = 1;
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return --refs; }
};
struct Pair {
    cIGZFontSys* native;
    void* rust;
    std::vector<cIGZFont*> fonts;
    std::vector<uint32_t> callers;
    LifetimeRenderer renderer;
    uint32_t exact = 0, diff = 0, refusal = 0, positive = 0, negative = 0;
    explicit Pair(cIGZFontSys* n, const std::vector<FontSysMetadata>& metadata)
        : native(n), rust(fc4_fontsys_verify_new(metadata.data(), uint32_t(metadata.size()))) {}
    ~Pair() {
        // Drop every retained caller reference while the owned catalogue still exists.
        for (size_t i = 0; i < fonts.size(); ++i)
            while (callers[i]) { FC4_RELEASE_CREATED(fonts[i]); --callers[i]; }
        fc4_fontsys_verify_free(rust);
    }
    uint32_t Intern(cIGZFont* font, bool acquired) {
        if (!font) return 0;
        auto it = std::find(fonts.begin(), fonts.end(), font);
        uint32_t id = uint32_t(it - fonts.begin());
        if (it == fonts.end()) { fonts.push_back(font); callers.push_back(0); }
        if (acquired) { ++callers[id]; FC4_ACQUIRED(font, "FontSys owned FontAcquire"); }
        return id + 1;
    }
    void InvalidateUnowned() {
        for (size_t i = 0; i < fonts.size(); ++i) if (!callers[i]) fonts[i] = nullptr;
    }
    uint32_t Native(uint32_t op, uint32_t a, uint32_t b, uint32_t c, uint32_t d,
                    const char* s, const char* t, uint8_t* out) {
        SC4String first(s), second(t);
        auto* f = a && a <= fonts.size() ? fonts[a-1] : nullptr;
        using Style = cIGZFont::Style;
        switch (op) {
        case 0: return native->Init();
        case 1: { bool ok = native->Shutdown(); InvalidateUnowned(); return ok; }
        case 2: return FontSysSlot<bool>(native, 14, static_cast<const cIGZString*>(&first));
        case 3: return native->EnumerateFontInfo(reinterpret_cast<cIGZFontSys::FontInfo*>(out), a);
        case 4: return native->IsFontAvailable(first, b, Style(a));
        case 5: return native->AddSubstitution(first, second);
        case 6: {
            SC4String strings[4] = {SC4String("sentinel"), SC4String("sentinel"), SC4String("sentinel"), SC4String("sentinel")};
            cIGZString* slots[4] = {};
            for (uint32_t i = 0; i < a; ++i) if (!(b & (1 << i))) slots[i] = &strings[i];
            uint32_t n = native->EnumerateFontSubstitutions(first, slots, a);
            for (uint32_t i = 0; i < a; ++i) if (slots[i]) PutString(out+i*128, 128, strings[i].ToChar());
            return n;
        }
        case 7: {
            SC4String result("sentinel");
            bool ok = native->IsSubstitutionAvailable(first, result); // section 12.1: second arg is output.
            PutString(out, 128, result.ToChar()); return ok;
        }
        case 8: native->SetDefaultUnantialiasedSizeRange(int32_t(a), int32_t(b), c ? &first : nullptr); return 0;
        case 9: native->SetDefaultCodePage(int32_t(a)); return 0; // SDK discards the published true return.
        case 10: native->SetDefaultFontAntialiasing(cIGZFont::AntialiasingMode(a)); return 0;
        case 11: return Intern(FontSysSlot<cIGZFont*>(native, 10, static_cast<const cIGZString*>(&first), a, b, c != 0), true);
        case 12: return Intern(FontSysSlot<cIGZFont*>(native, 11, static_cast<const cIGZString*>(&first), d, a, b, c != 0), true);
        case 13: return Intern(FontSysSlot<cIGZFont*>(native, 9, d, a, b, uint32_t(0x12345678), c != 0), true);
        case 14: {
            if (!f) return UINT32_MAX;
            SC4String path, family;
            f->GetFilePath(path); f->GetName(family);
            PutString(out, 260, path.ToChar()); PutString(out+260, 128, family.ToChar());
            int32_t cp = 0; f->GetCodePage(cp);
            // Published section 4.1 slot 7 is the no-argument GetStyles. Do not infer
            // its slot from the textual order of overloaded SDK declarations.
            using GetStyles = uint32_t(__fastcall*)(void*, void*);
            uint32_t style = reinterpret_cast<GetStyles>((*reinterpret_cast<void***>(f))[7])(f, nullptr);
            FC4_ADDREF(f); uint32_t refs = FC4_RELEASE_CREATED(f);
            using GetU32 = uint32_t(__fastcall*)(void*, void*);
            const uint32_t size = reinterpret_cast<GetU32>((*reinterpret_cast<void***>(f))[10])(f, nullptr);
            const uint32_t values[] = {size, style, uint32_t(f->GetType()), uint32_t(f->GetAntialiasing()),
                                      uint32_t(cp), f->GetFontColor(), f->GetFontBackgroundColor(), refs};
            std::memcpy(out+388, values, sizeof(values)); return 1;
        }
        case 15: {
            if (!f || !callers[a-1]) return UINT32_MAX;
            const auto before = RefBefore(f);
            uint32_t refs = native->FontRelease(f);
            RefNote(before, -1, long(refs), "FontSys FontRelease caller", FC4_SITE, true, true);
            --callers[a-1]; if (!refs) fonts[a-1] = nullptr;
            return refs;
        }
        case 16: {
            cIGZFont* cached[8] = {};
            uint32_t n = native->EnumerateManagedFonts(cached, a);
            for (uint32_t i = 0; i < (std::min)(n,a); ++i) {
                uint32_t id = Intern(cached[i], false); std::memcpy(out+i*4, &id, 4);
            }
            return n;
        }
        case 17: { bool ok = native->RemoveFontByName(first); InvalidateUnowned(); return ok; }
        case 18: return FontSysSlot<bool>(native, 13, f);
        case 19: return native->RemoveFontByFile(first);
        case 20: return FC4_ADDREF(native);
        case 21: return FC4_RELEASE_CREATED(native);
        case 22: {
            void* view = reinterpret_cast<void*>(uintptr_t(0x1234));
            bool ok = native->QueryInterface(a, &view);
            if (!ok) return view == reinterpret_cast<void*>(uintptr_t(0x1234)) ? 0 : UINT32_MAX;
            auto* p = static_cast<cIGZUnknown*>(view); FC4_ACQUIRED(p, "FontSys paired QI");
            FC4_RELEASE_CREATED(p); return a == 0x287FB697 ? 2 : 1;
        }
        case 23: native->SetDefaultFontRenderer(nullptr); return 0;
        case 24: {
            if (!f || !callers[a-1]) return UINT32_MAX;
            uint32_t refs = FC4_RELEASE_CREATED(f); --callers[a-1];
            if (!refs) fonts[a-1] = nullptr; return refs;
        }
        case 25: return FontSysSlot<uint32_t>(native,15,static_cast<const cIGZString*>(&first));
        case 26: native->SetDefaultFontRenderer(reinterpret_cast<cIGZFontRenderer*>(&renderer)); return 0;
        case 27: return renderer.refs;
        default: return UINT32_MAX;
        }
    }
    uint32_t Call(uint32_t op, uint32_t a=0, uint32_t b=0, uint32_t c=0, uint32_t d=0,
                  const char* s="", const char* t="") {
        if (!rust) { ++refusal; return 0; }
        alignas(4) std::array<uint8_t,1024> n, r; n.fill(0xa5); r.fill(0xa5);
        uint32_t nr = Native(op,a,b,c,d,s,t,n.data());
        uint32_t rr = fc4_fontsys_verify_call(rust,op,a,b,c,d,s,t,r.data());
        if (nr == UINT32_MAX || rr == UINT32_MAX) { ++refusal; return nr; }
        if (nr == rr && n == r) ++exact;
        else {
            if (diff < 12) {
                size_t offset = 0; while (offset < n.size() && n[offset] == r[offset]) ++offset;
                Log("fontsys difference op=%u a=%u b=%u c=%u d=%u text='%s' result=%u/%u offset=%u byte=%u/%u",
                    op,a,b,c,d,s,nr,rr,uint32_t(offset),offset<n.size()?n[offset]:0,offset<r.size()?r[offset]:0);
            }
            ++diff;
        }
        nr ? ++positive : ++negative;
        return nr;
    }
};

void Compare(cIGZFontSys* owned, const char* phase) {
    char windows[MAX_PATH] = {};
    GetWindowsDirectoryA(windows, MAX_PATH);
    std::vector<std::string> paths;
    const std::string directory = std::string(windows) + "\\Fonts\\";
    for (const char* pattern : {"*.tt?", "*.pfb", "*.mxf", "*.otf"}) {
        WIN32_FIND_DATAA found = {};
        HANDLE search = FindFirstFileA((directory + pattern).c_str(), &found);
        if (search == INVALID_HANDLE_VALUE) continue;
        do {
            if (found.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) continue;
            std::string path = directory + found.cFileName;
            SC4String input(path);
            if (FontSysSlot<bool>(owned, 14, static_cast<const cIGZString*>(&input))) paths.push_back(path);
        } while (FindNextFileA(search, &found));
        FindClose(search);
    }
    std::sort(paths.begin(), paths.end());
    std::vector<cIGZFontSys::FontInfo> raw(paths.size());
    uint32_t count = owned->EnumerateFontInfo(raw.data(), uint32_t(raw.size()));
    if (paths.empty() || count != paths.size()) {
        Log("fontsys verify phase=%s refusal=1 reason=raw-provider-catalogue count=%u paths=%u",phase,count,uint32_t(paths.size()));
        owned->Shutdown(); return;
    }
    std::vector<FontSysMetadata> metadata(paths.size());
    for (size_t i = 0; i < paths.size(); ++i) {
        auto& m = metadata[i]; std::memcpy(m.path, paths[i].c_str(), paths[i].size()+1);
        std::memcpy(m.family,raw[i].name,128); m.style=uint32_t(raw[i].style); m.kind=uint32_t(raw[i].type);
    }
    // This is the one raw independent provider input. Query/cache/selection answers below are
    // never fed into Rust. Both catalogues start empty after the original's owned Shutdown.
    owned->Shutdown();
    Pair p(owned,metadata);
    p.Call(0); p.Call(0); p.Call(3); p.Call(16);
    p.Call(22,1); p.Call(22,0x00673960); p.Call(22,0x287FB697); p.Call(22,0); p.Call(22,0xffffffff);
    p.Call(20); p.Call(21); p.Call(23);
    p.Call(27); p.Call(26); p.Call(27); p.Call(26); p.Call(27); p.Call(23); p.Call(27);
    p.Call(26); p.Call(1); p.Call(27); p.Call(0);
    p.Call(25,0,0,0,0,"FC4_nonexistent_font_directory\\");
    p.Call(25,0,0,0,0,directory.c_str()); p.Call(3); p.Call(25,0,0,0,0,directory.c_str());
    p.Call(1); p.Call(0);
    p.Call(11,12,0,1,0,"FC4_missing_font.ttf");
    p.Call(12,12,0,1,1,"missing"); p.Call(13,12,0,1,2);
    p.Call(18); p.Call(19,0,0,0,0,"missing");
    p.Call(7,0,0,0,0,"missing"); p.Call(6,0,0,0,0,"missing");
    p.Call(9,1252); p.Call(10,3);
    // Reverse insertion proves path ordering independently of raw metadata insertion order.
    for (auto it=paths.rbegin();it!=paths.rend();++it) p.Call(2,0,0,0,0,it->c_str());
    p.Call(2,0,0,0,0,paths[0].c_str());
    p.Call(2,0,0,0,0,"FC4_missing_font.ttf");
    p.Call(3); p.Call(3,1); p.Call(3,4);
    for (const auto& m: metadata) {
        for (uint32_t style: {0u,1u,2u,3u,4u,5u,0xffffffffu}) p.Call(4,style,123,0,0,m.family);
    }
    p.Call(4,0,0,0,0,"missing");
    p.Call(5,0,0,0,0,"FC4_ALIAS", "missing");
    p.Call(5,0,0,0,0,"FC4_ALIAS",metadata[0].family);
    p.Call(5,0,0,0,0,"FC4_ALIAS",metadata[0].family);
    p.Call(6,0,0,0,0,"FC4_ALIAS"); p.Call(6,3,2,0,0,"FC4_ALIAS");
    p.Call(6,1,0,0,0,"fc4_alias"); p.Call(7,0,0,0,0,"FC4_ALIAS");
    p.Call(7,0,0,0,0,"fc4_alias");
    uint32_t cached=p.Call(11,12,4,1,0,paths[0].c_str());
    if(cached) {
        p.Call(14,cached); p.Call(11,12,4,1,0,paths[0].c_str()); p.Call(14,cached);
        p.Call(15,cached); p.Call(14,cached); p.Call(18,cached); p.Call(16); p.Call(16,8);
        p.Call(9,932); p.Call(10,2); p.Call(11,12,4,1,0,paths[0].c_str()); p.Call(14,cached); p.Call(15,cached);
        p.Call(15,cached); p.Call(16);
    }
    // Pre-Init family is empty: named-family range must not select monochrome; empty key does.
    p.Call(8,10,20,1,0,metadata[0].family);
    uint32_t uncached=p.Call(11,16,0,0,0,paths[0].c_str());
    if(uncached) {p.Call(14,uncached); p.Call(24,uncached);}
    p.Call(8,10,20,1,0,""); p.Call(8,1,2,1,0,"");
    uncached=p.Call(11,16,0,0,0,paths[0].c_str());
    if(uncached) {p.Call(14,uncached); p.Call(24,uncached);}
    p.Call(8,20,10,1,0,""); p.Call(8,12,16,0);
    uncached=p.Call(11,16,0,0,0,paths[0].c_str());
    if(uncached) {p.Call(14,uncached); p.Call(24,uncached);}
    p.Call(8,0,0,0);
    for(const char* name: {"", "missing", "Arial,Tahoma", "Arial,,Tahoma", " Arial , Tahoma ", ",Arial", "\"Arial\",Tahoma", "FC4_ALIAS"}) {
        uint32_t id=p.Call(12,12,5,1,1,name);
        if(id) {p.Call(14,id); p.Call(15,id);}
    }
    for(uint32_t kind: {0u,1u,2u,3u}) {
        uint32_t id=p.Call(13,14,0,1,kind);
        if(id) {p.Call(14,id); p.Call(15,id);}
    }
    // A caller survives cache removal and owned Shutdown. Reinit keeps code page/ranges.
    uint32_t retained=p.Call(11,18,0,1,0,paths.back().c_str());
    p.Call(17,0,0,0,0,metadata.back().family); p.Call(3); p.Call(4,0,0,0,0,metadata.back().family);
    if(retained) p.Call(14,retained);
    p.Call(1); p.Call(1); p.Call(0); p.Call(3); p.Call(16); p.Call(7,0,0,0,0,"FC4_ALIAS");
    if(retained) {p.Call(14,retained); p.Call(24,retained);}
    // Cached acquisition creates an entry absent from the catalogue (FN14).
    retained=p.Call(11,12,0,1,0,paths[0].c_str());
    p.Call(3,4); p.Call(16,8); p.Call(4,0,0,0,0,metadata[0].family);
    if(retained) {p.Call(14,retained); p.Call(15,retained);}
    Log("fontsys production phase=%s constructed exact=%u diff=%u refusal=%u fallback=0 resync=0 natural=0 metadata_once=%u positive=%u zero=%u",
        phase,p.exact,p.diff,p.refusal,uint32_t(metadata.size()),p.positive,p.negative);
}
}
void FontSysVerify(const char* phase) {
    if (!SettingInt("fontsys_verify", "FC4_FONTSYS_VERIFY", 0)) return;
    uint32_t exact = 0, diff = 0, refusal = 0;
    auto check = [&](bool value) { value ? ++exact : ++diff; };
    auto* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    void* service = nullptr;
    if (!compat::IdentifiedVersion() || !com ||
        !com->GetClassObject(0x20673960, 0x287FB697, &service) || !service) {
        Log("fontsys verify phase=%s constructed exact=0 diff=0 refusal=1 fallback=0 resync=0 natural=0 reason=factory", phase);
        return;
    }
    auto* ownedService = static_cast<cIGZUnknown*>(service);
    FC4_ACQUIRED(ownedService, "FontSys owned factory service view");
    void* primary = nullptr;
    if (ownedService->QueryInterface(0x00673960, &primary) && primary) {
        auto* owned = static_cast<cIGZFontSys*>(primary);
        FC4_ACQUIRED(owned, "FontSys owned primary QI");
        if (*reinterpret_cast<uintptr_t*>(owned) != reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr)) + 0x006c5488) {
            FC4_RELEASE_CREATED(owned); FC4_RELEASE_CREATED(ownedService);
            Log("fontsys verify phase=%s refusal=1 reason=original-vtable",phase); return;
        }
        check(reinterpret_cast<uintptr_t>(service) == reinterpret_cast<uintptr_t>(primary) + 4);
        // Release only the created instance's views; never the framework's FontSys service.
        check(FC4_RELEASE_CREATED(ownedService) == 1);
        ownedService = nullptr;
        check(owned->Init());
        check(owned->Init());
        for (uint32_t iid : {uint32_t(1), uint32_t(0x00673960), uint32_t(0x287FB697), uint32_t(0), uint32_t(0xffffffff)}) {
            void* out = reinterpret_cast<void*>(uintptr_t(0x1234));
            const bool ok = owned->QueryInterface(iid, &out);
            check(ok == (iid == 1 || iid == 0x00673960 || iid == 0x287FB697));
            if (ok && out) {
                auto* view = static_cast<cIGZUnknown*>(out);
                FC4_ACQUIRED(view, "FontSys owned QI fixture");
                check(out == (iid == 0x287FB697 ? service : primary));
                check(FC4_RELEASE_CREATED(view) == 1);
            } else check(out == reinterpret_cast<void*>(uintptr_t(0x1234)));
        }
        check(owned->EnumerateFontInfo(nullptr, 0) == 0);
        check(owned->EnumerateManagedFonts(nullptr, 0) == 0);
        check(owned->Shutdown());
        check(owned->Shutdown());
        check(owned->Init());
        Compare(owned, phase);
        check(FC4_RELEASE_CREATED(owned) == 0);
    } else ++refusal;
    if (ownedService) FC4_RELEASE_CREATED(ownedService);
    Log("fontsys verify phase=%s constructed exact=%u diff=%u refusal=%u fallback=0 resync=0 natural=0 route=owned-factory-QI", phase, exact, diff, refusal);
}
}
