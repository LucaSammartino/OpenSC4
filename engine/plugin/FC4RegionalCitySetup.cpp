// SPDX-License-Identifier: GPL-3.0-or-later
#include "cFC4RegionalCity.h"
#include "FC4RegionalCityHost.h"
#include "FC4Streams.h"
#include "FC4RefAudit.h"
#include "GZServPtrs.h"
#include "cIGZLanguageManager.h"
#include "cIGZPersistResourceManager.h"
#include "cGZPersistResourceKey.h"
#include "cISC4DBSegment.h"
#include "cISC4DBSegmentIStream.h"
#include "cISC4Region.h"
#include "SC4String.h"
#include "cRZBaseString.h"
#include <string>
#include <vector>
#include <windows.h>

namespace fc4::regionalcity {
namespace {
std::string DefaultName() {
    cIGZPersistResourceManagerPtr rm;
    cIGZLanguageManagerPtr lm;
    const uint32_t language = lm ? lm->GetCurrentLanguage() : 0;
    cIGZString* text = nullptr;
    if (rm) {
        auto load = [&](uint32_t g) {
            const cGZPersistResourceKey key(0x2026960B, g, 0xAA738C4E);
            return rm->GetPrivateResource(key, GZIID_cIGZString, reinterpret_cast<void**>(&text), 0, nullptr) && text;
        };
        if (!load(0x6A231EAA + language) && language) load(0x6A231EAA);
    }
    std::string out;
    if (text) {
        FC4_ACQUIRED(text, "regionalcity default name");
        out.assign(text->ToChar(), text->Strlen()); FC4_RELEASE_CREATED(text);
    }
    return out;
}
bool Exists(void*, const uint8_t* p, uint32_t n) {
    const std::string path(reinterpret_cast<const char*>(p), n);
    const auto size = MultiByteToWideChar(CP_UTF8, 0, path.data(), static_cast<int>(path.size()), nullptr, 0);
    std::wstring wide(size, L'\0');
    MultiByteToWideChar(CP_UTF8, 0, path.data(), static_cast<int>(path.size()), wide.data(), size);
    return GetFileAttributesW(wide.c_str()) != INVALID_FILE_ATTRIBUTES;
}
std::string Convert(const std::string& s, UINT from, UINT to) {
    const auto n = MultiByteToWideChar(from, 0, s.data(), static_cast<int>(s.size()), nullptr, 0);
    std::wstring wide(n, L'\0');
    MultiByteToWideChar(from, 0, s.data(), static_cast<int>(s.size()), wide.data(), n);
    const auto m = WideCharToMultiByte(to, 0, wide.data(), n, nullptr, 0, nullptr, nullptr);
    std::string out(m, '\0');
    WideCharToMultiByte(to, 0, wide.data(), n, out.data(), m, nullptr, nullptr);
    return out;
}
void Roundtrip(void*, const uint8_t* p, uint32_t n, void* sink, void (*emit)(void*, const uint8_t*, uint32_t)) {
    const auto s = Convert(Convert(std::string(reinterpret_cast<const char*>(p), n), CP_UTF8, CP_ACP), CP_ACP, CP_UTF8);
    emit(sink, reinterpret_cast<const uint8_t*>(s.data()), static_cast<uint32_t>(s.size()));
}
void Text(void* out, const uint8_t* p, uint32_t n) {
    static_cast<cIGZString*>(out)->FromChar(reinterpret_cast<const char*>(p), n);
}
}
void FinishRead(cFC4RegionalCity& record, cIGZIStream& input, uint8_t difficulty) {
    const auto name = DefaultName();
    fc4_regionalcity_owner_finish_read(record.Owner(), difficulty,
        reinterpret_cast<const uint8_t*>(name.data()), static_cast<uint32_t>(name.size()));
    cFC4ISC4DBSegmentIStream* segmentStream = nullptr;
    if (!input.QueryInterface(kFC4_IID_cISC4DBSegmentIStream, reinterpret_cast<void**>(&segmentStream)) || !segmentStream) return;
    FC4_ACQUIRED(segmentStream, "regionalcity localization stream");
    auto* segment = reinterpret_cast<cISC4DBSegment*>(segmentStream->GetSegment()); // borrowed
    cIGZLanguageManagerPtr lm;
    const uint32_t language = lm ? lm->GetCurrentLanguage() : 0;
    if (segment) for (uint32_t group : {0x6AAEE931u, 0x6AAEE932u}) {
        const cGZPersistResourceKey key(0x2026960B, group, language);
        cISC4DBSegmentIStream* stream = nullptr;
        if (segment->OpenIStream(key, &stream) && stream) {
            FC4_ACQUIRED(stream, "regionalcity localized name record");
            cRZBaseString s;
            if (stream->GetGZStr(s)) {
                if (group == 0x6AAEE931) record.SetCityName(s); else record.SetMayorName(s);
            }
            segment->CloseIStream(stream); FC4_RELEASE_CREATED(stream);
        }
    }
    FC4_RELEASE(segmentStream);
}
}
void cFC4RegionalCity::SetupCity(cISC4City* city) {
    if (!city) return;
    cRZBaseString s;
    GetMayorName(s); city->SetMayorName(s);
    GetCityName(s); city->SetCityName(s);
    city->SetDifficultyLevel(GetDifficultyLevel());
    fc4::regionalcity::App app;
    auto* region = app.p ? app.p->GetRegion() : nullptr;
    if (region && region->GetDirectoryName()) {
        const auto* directory = region->GetDirectoryName();
        std::string base(directory->ToChar(), directory->Strlen());
        if (base.size() < 2 || (base[1] != ':' && base[0] != '\\' && base[0] != '/')) {
            cRZBaseString root; app.p->GetRegionsDirectory(root);
            base = std::string(root.ToChar(), root.Strlen()) + "\\" + base;
        }
        fc4_regionalcity_owner_save_path(owner_, reinterpret_cast<const uint8_t*>(base.data()),
            static_cast<uint32_t>(base.size()), nullptr, fc4::regionalcity::Exists,
            fc4::regionalcity::Roundtrip, &s, fc4::regionalcity::Text);
        city->SetCitySaveFilePath(s);
    }
    int32_t x = 0, z = 0; GetCitySize(x, z);
    city->SetSize(static_cast<float>(x * 1024), static_cast<float>(z * 1024));
}
