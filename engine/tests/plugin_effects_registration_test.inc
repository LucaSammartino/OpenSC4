// SPDX-License-Identifier: GPL-3.0-or-later
// effects.md section 1: registration and factory agree on the city manager.
int EffectsRegistrationTest(const std::string& dll, const std::string& ini,
                            const std::string& log, const std::string& mode) {
    const bool enabled = mode == "--effects-enabled";
    const bool disabled = mode == "--effects-disabled";
    const bool guarded = mode == "--effects-guarded";
    SetEnvironmentVariableA("FC4_ENABLE", nullptr);
    SetEnvironmentVariableA("FC4_DISABLE", nullptr);
    SetEnvironmentVariableA("FC4_EFFECTS_REPLACE", nullptr);
    SetEnvironmentVariableA("FC4_EFFECTS_SHADOW", nullptr);
    WriteTextFile(ini, disabled ? "[subsystems]\nenable = effects\ndisable = effects\n[tools]\neffects_replace = 1\n" :
                       enabled ? "[subsystems]\nenable = effects\n[tools]\neffects_replace = 1\n" :
                       guarded ? "[subsystems]\nenable = effects\n[tools]\neffects_shadow = 1\n" : "[subsystems]\n");
    HMODULE hd = LoadLibraryA(dll.c_str());
    CHECK(hd != nullptr);
    if (hd == nullptr) return 1;
    auto getDirector = reinterpret_cast<cIGZCOMDirector* (*)()>(GetProcAddress(hd, "GZDllGetGZCOMDirector"));
    cIGZCOMDirector* director = getDirector ? getDirector() : nullptr;
    CHECK(director != nullptr);
    if (director != nullptr) {
        std::vector<uint32_t> classes;
        director->EnumClassObjects(&CollectClass, &classes);
        CHECK(std::count(classes.begin(), classes.end(), 0x49822F75u) == (enabled ? 1 : 0));
        void* obj = nullptr;
        CHECK(director->GetClassObject(0x49822F75u, 0xC97CF5CDu, &obj) == enabled);
        CHECK((obj != nullptr) == enabled);
        if (obj != nullptr) {
            // The manager interface starts with cIGZUnknown; GetClassObject owns this reference.
            auto* manager = static_cast<cIGZUnknown*>(obj);
            // Section 18.2 rejects unknown IIDs without changing the caller's
            // output, and neither failure path may acquire a reference.
            void* rejected = obj;
            CHECK(!manager->QueryInterface(0xFFFFFFFFu, &rejected));
            CHECK(rejected == obj);
            CHECK(!manager->QueryInterface(GZIID_cIGZUnknown, nullptr));
            CHECK(manager->AddRef() == 2);
            CHECK(manager->Release() == 1);
            void* unknown = nullptr;
            CHECK(manager->QueryInterface(GZIID_cIGZUnknown, &unknown));
            CHECK(unknown == obj);
            if (unknown != nullptr) static_cast<cIGZUnknown*>(unknown)->Release();
            void* target = nullptr;
            CHECK(manager->QueryInterface(0x452294AAu, &target));
            CHECK(static_cast<char*>(target) == static_cast<char*>(obj) + 8);
            if (target != nullptr) {
                auto* messages = static_cast<cIGZMessageTarget2*>(target);
                CHECK(messages->DoMessage(nullptr));
                // The adjusted message-target pointer must round-trip to the
                // same controlling identity and share its reference count.
                void* primary = nullptr;
                CHECK(messages->QueryInterface(0xC97CF5CDu, &primary));
                CHECK(primary == obj);
                if (primary != nullptr) CHECK(static_cast<cIGZUnknown*>(primary)->Release() == 2);
                rejected = obj;
                CHECK(!messages->QueryInterface(0xFFFFFFFFu, &rejected));
                CHECK(rejected == obj);
                CHECK(messages->Release() == 1);
            }
            CHECK(manager->Release() == 0);
        }
    }
    FreeLibrary(hd);
    DeleteFileA(ini.c_str());
    const std::string text = ReadFile(log);
    CHECK((text.find("AddCls skipped") != std::string::npos) == guarded);
    CHECK(text.find("SEH") == std::string::npos);
    if (enabled) CHECK(text.find("AddCls(0x49822F75) registered") != std::string::npos);
    if (guarded) CHECK(text.find("effects shadow: enabled native record-only names") != std::string::npos);
    return g_failures == 0 ? 0 : 1;
}
