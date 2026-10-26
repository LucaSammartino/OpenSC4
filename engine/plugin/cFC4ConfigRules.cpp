// SPDX-License-Identifier: GPL-3.0-or-later
#include "cFC4ConfigRules.h"
#include "FC4ConfigRulesAbi.h"
#include "FC4ConfigRulesAcquisition.h"
#include "FC4ConfigRulesSystem.h"
#include "FC4ConfigRulesTrace.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "cIGZApp.h"
#include "cIGZCOM.h"
#include "cIGZCmdLine.h"
#include "cIGZFrameWork.h"
#include "cIGZGDriver.h"
#include "cISC4App.h"
#include "cISC4RenderProperties.h"
#include "cRZBaseString.h"
#include "cRZCOMDllDirector.h"
#include "SC4Preferences.h"
#include <Windows.h>
#include <array>
#include <atomic>
#include <cctype>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <iterator>
#include <string>

namespace {
using namespace fc4::cfgrules;
template<class R, class... A> R Call(void* p, size_t slot, A... args) {
    using Fn = R(__thiscall*)(void*, A...);
    return reinterpret_cast<Fn>((*static_cast<void***>(p))[slot])(p, args...);
}
template<class T = cIGZUnknown> struct Ref {
    T* p = nullptr;
    bool created = false;
    ~Ref() { if (p) { if (created) FC4_RELEASE_CREATED(p); else FC4_RELEASE(p); } }
    bool Service(uint32_t service, uint32_t iid) {
        auto* fw = RZGetFrameWork();
        if (fw) fw->GetSystemService(service, iid, reinterpret_cast<void**>(&p));
        if (p) FC4_ACQUIRED(p, "cfgrules service");
        return p != nullptr;
    }
    bool Query(cIGZUnknown* from, uint32_t iid) {
        if (from) from->QueryInterface(iid, reinterpret_cast<void**>(&p));
        if (p) FC4_ACQUIRED(p, "cfgrules query");
        return p != nullptr;
    }
};
FC4ConfigRulesBytes Bytes(const std::string& s) {
    return {reinterpret_cast<const uint8_t*>(s.data()), s.size()};
}
std::string Text(FC4ConfigRulesBytes b) {
    return b.len ? std::string(reinterpret_cast<const char*>(b.data), b.len) : std::string();
}
struct ProbeHost {
    cIGZGDriver* driver;
    Ref<> base, ui, buffer;
    bool Begin() {
        Call<uint32_t>(driver, 50);
        if (!base.Service(0xC416025C, 0x0073283C) || !ui.Service(0xC416025C, 0xC47B747C)) return false;
        Call<void>(ui.p, 12);
        return true;
    }
    void ClearColor() { Call<void, float, float, float, float>(driver, 12, 0, 0, 0, 0); }
    void Clear() { Call<void, uint32_t>(driver, 11, 0x4000); }
    void Disable(uint32_t cap) { Call<void, uint32_t>(driver, 45, cap); }
    void Arrays(const Vertex* v) { Call<void, uint32_t, uint32_t, const void*>(driver, 5, 1, 16, v); }
    void Draw() { Call<void, uint32_t, uint32_t, uint32_t>(driver, 3, 6, 0, 256); }
    bool Copy() {
        Ref<> copy;
        if (!copy.Query(driver, 0xE69BFE2A)) return false;
        buffer.p = Call<cIGZUnknown*, int32_t, int32_t, uint32_t, uint32_t, void*>(copy.p, 3, 0, 0, 64, 64, nullptr);
        buffer.created = true;
        return true;
    }
    void Flush() { Call<void>(driver, 43); }
    void End() { Call<void>(ui.p, 12); }
    bool HasBuffer() const { return buffer.p != nullptr; }
    uint32_t Pixel(int x, int y) { return Call<uint32_t, int32_t, int32_t>(buffer.p, 21, x, y); }
};
struct StoreHost {
    cIGZUnknown* p;
    bool Get() { return Call<bool>(p, 56); }
    void Set(bool value) { Call<void, bool>(p, 57, value); }
};

struct Host {
    std::array<std::string, 6> os;
    std::array<std::string, 2> sound;
    std::string description, file;
    FILE* log = nullptr;
    ~Host() { if (log) fclose(log); }
    bool Measure(FC4ConfigRulesMeasurement& m) {
        if (!MeasureSystem(m, os)) return false;
        Ref<> gs, gs3;
        if (!gs.Service(0xC416025C, 0x0073283C) || !gs3.Query(gs.p, 0xC47B747C)) return false;
        auto* driver = Call<cIGZGDriver*>(gs3.p, 3); // borrowed current driver
        if (!driver) return false;
        Call<void, void*>(driver, 64, m.driverRecord);
        Call<void, void*>(gs.p, 10, m.screen);
        m.videoBytes = Call<uint32_t, uint32_t>(gs.p, 23, 2);
        m.textureBytes = Call<uint32_t, uint32_t>(gs.p, 23, 0);
        const char* info = driver->GetDriverInfo();
        if (!info) return false;
        description = info;
        m.driverDescription = Bytes(description);
        ProbeHost probe{driver};
        bool forced = false;
        if (!Probe(probe, m.driverRecord[0x30] != 0, forced)) return false;
        m.forcedFSAA = forced;
        // 2.4: the uninitialized renderer facade observes the shared store even
        // before the application's view exists. Never initialize this facade.
        Ref<> render;
        auto* fw = RZGetFrameWork();
        auto* com = fw ? fw->GetCOMObject() : nullptr;
        if (!com || !com->GetClassObject(0xE9C622D8, 0x681BD529, reinterpret_cast<void**>(&render.p)) || !render.p)
            return false;
        render.created = true;
        StoreHost store{render.p};
        m.backingStore = BackingStore(store);
        Ref<> audio;
        m.hasSound = audio.Service(0xE9D27715, 0x49D28666);
        if (m.hasSound) {
            cRZBaseString driverName, cardName;
            Call<void, cIGZString&, cIGZString&>(audio.p, 25, driverName, cardName);
            sound[0] = driverName.ToChar(); sound[1] = cardName.ToChar();
            m.soundNames[0] = Bytes(sound[0]); m.soundNames[1] = Bytes(sound[1]);
        }
        fc4::Log("cfgrules provider: hardware=%u fsaa=%u backing=%u sound=%u cpuMHz=%u",
                 m.driverRecord[0x30], m.forcedFSAA, m.backingStore, m.hasSound, m.cpuMHz);
        return true;
    }
    FC4ConfigRulesHost Callbacks() {
        return {this,
            [](void* p) { FC4_ADDREF(static_cast<cISC4RenderProperties*>(p)); },
            [](void* p) { FC4_RELEASE(static_cast<cISC4RenderProperties*>(p)); },
            [](void* p, uint32_t k, FC4ConfigRulesBytes b) -> int32_t {
                auto* r = static_cast<cISC4RenderProperties*>(p); const auto s = Text(b);
                return k == 0 ? r->BoolPropertyIDFromName(s.c_str()) : k == 1 ? r->IntPropertyIDFromName(s.c_str()) : r->FloatPropertyIDFromName(s.c_str());
            },
            [](void* p, uint32_t k, uint32_t i) -> uint32_t {
                auto* r = static_cast<cISC4RenderProperties*>(p);
                if (k == 0) return r->BoolValue(i);
                if (k == 1) return r->IntValue(i);
                const float f = r->FloatValue(i); uint32_t bits; memcpy(&bits, &f, 4); return bits;
            },
            [](void* p, uint32_t k, uint32_t i, uint32_t v) {
                auto* r = static_cast<cISC4RenderProperties*>(p);
                fc4::Log("cfgrules write kind=%u index=%u bits=%08x", k, i, v);
                if (k == 0) r->SetBoolValue(i, v != 0);
                else if (k == 1) r->SetIntValue(i, static_cast<int32_t>(v));
                else { float f; memcpy(&f, &v, 4); r->SetFloatValue(i, f); }
            },
            [](void* p, uint32_t k) -> uint32_t {
                auto* r = static_cast<cISC4RenderProperties*>(p);
                return k == 0 ? r->NumBoolProperties() : k == 1 ? r->NumIntProperties() : r->NumFloatProperties();
            },
            [](void* p, uint32_t k, uint32_t i) -> FC4ConfigRulesBytes {
                auto* r = static_cast<cISC4RenderProperties*>(p);
                const char* n = k == 0 ? r->NameFromBoolPropertyID(i) : k == 1 ? r->NameFromIntPropertyID(i) : r->NameFromFloatPropertyID(i);
                return {reinterpret_cast<const uint8_t*>(n), n ? strlen(n) : 0};
            },
            [](void* p, FC4ConfigRulesBytes path, FC4ConfigRulesBytes* out) -> int32_t {
                auto& h = *static_cast<Host*>(p); const auto name = Text(path);
                std::ifstream stream(name, std::ios::binary);
                if (!stream) {
                    const DWORD attributes = GetFileAttributesA(name.c_str());
                    fc4::Log("cfgrules read path=%s opened=0", name.c_str());
                    return attributes == INVALID_FILE_ATTRIBUTES && GetLastError() == ERROR_FILE_NOT_FOUND ? 0 : -1;
                }
                h.file.assign(std::istreambuf_iterator<char>(stream), {});
                *out = Bytes(h.file);
                fc4::Log("cfgrules read path=%s opened=1 bytes=%u", name.c_str(), static_cast<unsigned>(h.file.size()));
                return 1;
            },
            [](void* p, FC4ConfigRulesMeasurement* m) { return static_cast<Host*>(p)->Measure(*m); },
            [](void* p, FC4ConfigRulesBytes path) {
                auto& h = *static_cast<Host*>(p); if (h.log) fclose(h.log);
                h.log = nullptr; fopen_s(&h.log, Text(path).c_str(), "wb"); return h.log != nullptr;
            },
            [](void* p) { return static_cast<Host*>(p)->log != nullptr; },
            [](void* p, FC4ConfigRulesBytes bytes) {
                auto* f = static_cast<Host*>(p)->log;
                return f && fwrite(bytes.data, 1, bytes.len, f) == bytes.len;
            },
            [](void* p) {
                auto& h = *static_cast<Host*>(p); if (!h.log) return false;
                auto* f = h.log; h.log = nullptr; return fclose(f) == 0;
            },
            [](void*, uint8_t c) -> uint8_t { return static_cast<uint8_t>(std::tolower(c)); }};
    }
};
struct App {
    Ref<cISC4App> app;
    App() {
        auto* fw = RZGetFrameWork();
        app.Query(fw ? fw->Application() : nullptr, GZIID_cISC4App);
    }
};
struct LogInput {
    std::string directory, date, writeLog;
    std::array<uint8_t, 12> preferences{};
    FC4ConfigRulesLogInput input{};
    LogInput() {
        App a;
        cRZBaseString path;
        if (a.app.p) {
            // application 9: plain path getters copy strings and return no bool.
            Call<void, cIGZString&>(a.app.p, 51, path);
            directory = path.ToChar();
        }
        if (a.app.p && a.app.p->GetPreferences()) {
            const auto* bytes = reinterpret_cast<const uint8_t*>(a.app.p->GetPreferences());
            const unsigned offsets[] = {0xf0d,0xf0e,0xf0f,0xf11,0xf12,0xf13,0xf14,0xf18,0xf15,0xf16,0xf28,0xf10};
            for (size_t i = 0; i < preferences.size(); ++i) preferences[i] = bytes[offsets[i]];
            input.preferences = preferences.data();
        }
        auto* fw = RZGetFrameWork();
        auto* cmd = fw ? fw->CommandLine() : nullptr;
        cRZBaseString value;
        if (cmd && cmd->IsSwitchPresent(cRZBaseString("writeLog"), value, false)) writeLog = value.ToChar();
        SYSTEMTIME now{}; GetLocalTime(&now);
        char text[96]{};
        // dates 1.6: the configuration logger requests DateString format 0.
        sprintf_s(text, "%u/%u/%4u", now.wMonth, now.wDay, now.wYear);
        date = text;
        input.appDirectory = Bytes(directory); input.writeLog = Bytes(writeLog); input.date = Bytes(date);
        input.hour = static_cast<uint8_t>(now.wHour); input.minute = static_cast<uint8_t>(now.wMinute);
    }
};
class Owner final : public cFC4ISC4ConfigRules {
    Host host_;
    void* owner_;
    std::atomic<uint32_t> refs_{0};
    bool initialized_ = false;
    cISC4RenderProperties* properties_ = nullptr; // borrowed from the transport's retained reference
public:
    Owner() { const auto host = host_.Callbacks(); owner_ = fc4_cfgrules_owner_create(&host); }
    ~Owner() { fc4_cfgrules_owner_destroy(owner_); }
    bool QueryInterface(uint32_t iid, void** out) override {
        if (!out) return false;
        *out = nullptr;
        if (iid != 1 && iid != 0x0A1D8284) return false;
        *out = static_cast<cFC4ISC4ConfigRules*>(this); AddRef(); return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { const auto n = --refs_; if (!n) delete this; return n; }
    void Init(cISC4RenderProperties* properties) override {
        App app;
        cRZBaseString data, user;
        if (!owner_ || !properties || !app.app.p) {
            initialized_ = false; fc4::Log("cfgrules Init refused: application/path acquisition unavailable"); return;
        }
        // application 9: SDK directory aliases/signatures differ from the game.
        // Slot 51 copies the data path, slot 56 the user-data path; neither takes
        // a language index (that belongs to plugin path slot 53).
        Call<void, cIGZString&>(app.app.p, 51, data);
        Call<void, cIGZString&>(app.app.p, 56, user);
        if (!data.Strlen() || !user.Strlen()) {
            initialized_ = false; fc4::Log("cfgrules Init refused: empty application paths"); return;
        }
        const auto* prefs = reinterpret_cast<const uint8_t*>(app.app.p->GetPreferences());
        const int status = fc4_cfgrules_owner_init(owner_, properties, Bytes(std::string(data.ToChar())), Bytes(std::string(user.ToChar())), prefs && prefs[0xf0c]);
        initialized_ = status >= 0;
        if (initialized_) properties_ = properties;
        fc4::Log("cfgrules query Init status=%d", status);
    }
    void SetOptionsFromConfiguration() override {
        if (!initialized_) { fc4::Log("cfgrules SetOptions refused: uninitialized"); return; }
        LogInput log;
        const int status = fc4_cfgrules_owner_evaluate(owner_, &log.input);
        fc4::Log("cfgrules query SetOptions status=%d", status);
        fc4::ConfigRulesTraceProperties(properties_);
    }
    bool LogConfigurationToFile(const cIGZString& path) override {
        LogInput log; bool opened = false;
        const int status = fc4_cfgrules_owner_log(owner_, Bytes(std::string(path.ToChar())), &log.input, &opened);
        fc4::Log("cfgrules query Log path=%s status=%d opened=%u", path.ToChar(), status, opened);
        return status >= 0 && opened;
    }
    void ApplyOptionGroupSetting(uint32_t group, uint32_t option) override {
        const int status = initialized_ ? fc4_cfgrules_owner_apply(owner_, group, option) : -2;
        fc4::Log("cfgrules query Apply group=%u option=%u status=%d", group, option, status);
        if (initialized_) fc4::ConfigRulesTraceProperties(properties_);
    }
    uint32_t GetOptionGroupAutoSetting(uint32_t group) override {
        uint32_t result = 0;
        const int status = initialized_ ? fc4_cfgrules_owner_auto(owner_, group, &result) : -2;
        fc4::Log("cfgrules query Auto group=%u result=%u status=%d", group, result, status);
        return result;
    }
};
}
cIGZUnknown* CreateFC4ConfigRules() { return new Owner(); }
