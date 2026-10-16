// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

// The glue15 lane: the tutorial system (docs/specs/tutorial.md, plugin/
// cFC4Tutorial.h) through `enable = tutorial`, without a framework: the
// class, its QueryInterface table, the 11-byte record (§3: version 2, the
// GUID mapping of a read), IsTutorial and the getters, Init / Shutdown with no
// message server or cheat manager, and the last Release. Included by
// plugin_load_test.cpp.

namespace glue15 {

constexpr uint32_t kCLSID_Tutorial = 0xAA3836FB;
constexpr uint32_t kIID_Tutorial = 0xAA3836F6;
constexpr uint32_t kIID_MessageTarget2 = 0x452294AA;
constexpr uint32_t kIID_MessageTarget = 0xE98B2F57;

std::vector<uint8_t> Record(uint16_t version, uint8_t isTutorial, int32_t task, uint32_t guid) {
    std::vector<uint8_t> b;
    auto put = [&b](const void* p, size_t n) {
        const uint8_t* q = static_cast<const uint8_t*>(p);
        b.insert(b.end(), q, q + n);
    };
    put(&version, 2);
    put(&isTutorial, 1);
    put(&task, 4);
    put(&guid, 4);
    return b;
}

}  // namespace glue15

int TutorialEnableTest(const std::string& dll, const std::string& ini, const std::string& log) {
    using namespace glue15;
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    WriteTextFile(ini, "[subsystems]\nenable = tutorial\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    SetGamePrecision();
    if (director != nullptr) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(std::count(classes.begin(), classes.end(), kCLSID_Tutorial) == 1);
        void* obj = nullptr;
        CHECK(director->GetClassObject(kCLSID_Tutorial, kIID_Tutorial, &obj));
        auto* t = static_cast<cISC4TutorialSystem*>(obj);
        if (t != nullptr) {
            // §1: 0xAA3836F6 and 1 at +0; the message target, the cheat
            // handler and the serializable elsewhere; nothing else.
            void* p = nullptr;
            CHECK(t->QueryInterface(GZIID_cIGZUnknown, &p) && p == obj);
            if (p != nullptr) static_cast<cIGZUnknown*>(p)->Release();
            p = nullptr;
            CHECK(t->QueryInterface(kIID_Tutorial, &p) && p == obj);
            if (p != nullptr) static_cast<cIGZUnknown*>(p)->Release();
            p = nullptr;
            CHECK(t->QueryInterface(kIID_MessageTarget2, &p) && p != nullptr && p != obj);
            if (p != nullptr) static_cast<cIGZUnknown*>(p)->Release();
            p = nullptr;
            CHECK(t->QueryInterface(kIID_MessageTarget, &p) && p != nullptr && p != obj);
            if (p != nullptr) static_cast<cIGZUnknown*>(p)->Release();
            p = reinterpret_cast<void*>(0x1234);
            CHECK(!t->QueryInterface(0x12345678, &p) && p == reinterpret_cast<void*>(0x1234));
            cIGZSerializable* ser = nullptr;
            CHECK(t->QueryInterface(GZIID_cIGZSerializable, reinterpret_cast<void**>(&ser)) && ser != nullptr);
            // §2: constructed with no tutorial, task 0, no GUID.
            CHECK(!t->IsTutorial());
            CHECK(t->GetTutorialTaskNumber() == 0);
            CHECK(static_cast<uint32_t>(t->GetTutorialGUID()) == 0xFFFFFFFFu);
            if (ser != nullptr) {
                CHECK(ser->GetGZCLSID() == kCLSID_Tutorial);
                // §3: version 2; GUID 0 reads as 0x8A5B7A6C.
                LandfillIStream in(Record(2, 1, 3, 0));
                CHECK(ser->Read(in));
                CHECK(t->IsTutorial());
                CHECK(t->GetTutorialTaskNumber() == 3);
                CHECK(static_cast<uint32_t>(t->GetTutorialGUID()) == 0x8A5B7A6Cu);
                LandfillOStream out;
                CHECK(ser->Write(out));
                CHECK(out.bytes == Record(2, 1, 3, 0x8A5B7A6Cu));
                // Another version: false, nothing changed.
                LandfillIStream v1(Record(1, 0, 9, 1));
                CHECK(!ser->Read(v1));
                CHECK(t->IsTutorial() && t->GetTutorialTaskNumber() == 3);
                // GUID 1 reads as 0xEA5D6DC8.
                LandfillIStream g1(Record(2, 0, 4, 1));
                CHECK(ser->Read(g1));
                CHECK(!t->IsTutorial() && static_cast<uint32_t>(t->GetTutorialGUID()) == 0xEA5D6DC8u);
                ser->Release();
            }
            // Init / Shutdown with no message server and no cheat manager.
            CHECK(t->Init());
            CHECK(t->Shutdown());
            CHECK(t->Release() == 0);
        }
    }
    FreeLibrary(hd);
    const std::string text = ReadFile(log);
    const auto npos = std::string::npos;
    CHECK(text.find("subsystem 'tutorial': enabled (1 class replaced) by FreeCity4.ini (enable=)") != npos);
    CHECK(text.find("tutorial Read -> 1 (is_tutorial=1 task=3 guid=0x8A5B7A6C)") != npos);
    CHECK(text.find("tutorial Read -> 0") != npos);
    CHECK(text.find("tutorial Write -> 1 (is_tutorial=1 task=3 guid=0x8A5B7A6C)") != npos);
    CHECK(text.find("tutorial Init: is_tutorial=0 task=4 guid=0xEA5D6DC8") != npos);
    CHECK(text.find("tutorial Shutdown:") != npos);
    CHECK(text.find("cFC4TutorialSystem destroyed") != npos);
    CHECK(text.find("WARNING: released last reference") == npos && text.find("WARNING: over-release") == npos);
    if (g_failures == 0) std::printf("plugin tutorial replacement test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}
