// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once

#include "cIGZAllocatorService.h"
#include "cIGZFrameWork.h"
#include <cstdlib>

// The SDK's SC4String caches allocate through the host even before advisor
// Init. Supply that service without starting any application or simulators.
class MySimTestAllocator final : public cIGZAllocatorService {
public:
    bool QueryInterface(uint32_t, void** out) override { *out = this; return true; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    void* Allocate(uint32_t size) override { return std::malloc(size ? size : 1); }
    bool Deallocate(void* p) override { std::free(p); return true; }
    void* Reallocate(void* p, uint32_t size) override { return std::realloc(p, size ? size : 1); }
};

class MySimTestFramework final : public cIGZFrameWork {
public:
    MySimTestAllocator allocator;
    bool QueryInterface(uint32_t, void** out) override { *out = nullptr; return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    bool AddSystemService(cIGZSystemService*) override { return false; }
    bool RemoveSystemService(cIGZSystemService*) override { return false; }
    bool GetSystemService(uint32_t service, uint32_t iid, void** out) override {
        *out = nullptr;
        if (service != 988069539u || iid != 988069547u) return false;
        *out = &allocator;
        allocator.AddRef();
        return true;
    }
    bool EnumSystemServices(void*, cIGZUnknown*, uint32_t) override { return false; }
    bool AddHook(cIGZFrameWorkHooks*) override { return false; }
    bool RemoveHook(cIGZFrameWorkHooks*) override { return false; }
    bool AddToTick(cIGZSystemService*) override { return false; }
    bool RemoveFromTick(cIGZSystemService*) override { return false; }
    bool AddToOnIdle(cIGZSystemService*) override { return false; }
    bool RemoveFromOnIdle(cIGZSystemService*) override { return false; }
    int32_t GetOnIdleInterval() override { return 0; }
    bool SetOnIdleInterval(int32_t) override { return false; }
    bool OnTick(uint32_t) override { return false; }
    bool OnIdle(uint32_t) override { return false; }
    bool IsTickEnabled() override { return false; }
    void ToggleTick(bool) override {}
    void Quit(int32_t) override {}
    void AbortiveQuit(int32_t) override {}
    cIGZCmdLine* CommandLine() override { return nullptr; }
    bool IsInstall() override { return false; }
    cIGZCOM* GetCOMObject() override { return nullptr; }
    FrameworkState GetState() override { return kStatePreAppInit; }
    void* GetDebugStream() override { return nullptr; }
    int32_t DefaultDebugStream() override { return 0; }
    int32_t DebugStream() override { return 0; }
    bool SetDebugStream(void*) override { return false; }
    bool SetDebugLevel(int32_t) override { return false; }
    int32_t GetDebugLevel() override { return 0; }
    cIGZOStream* StdOut() override { return nullptr; }
    cIGZOStream* StdErr() override { return nullptr; }
    cIGZIStream* StdIn() override { return nullptr; }
    bool GetStream(tStreamType, uint32_t, void**) override { return false; }
    bool SetStream(tStreamType, cIGZUnknown*) override { return false; }
    bool SetApplication(cIGZApp* const) override { return false; }
    cIGZApp* const Application() override { return nullptr; }
    void ReportException(char const*) override {}
    cIGZExceptionNotification* ExceptionNotificationObj() override { return nullptr; }
};

class MySimTestCOM final : public cIGZCOM {
public:
    cIGZCOMDirector* director = nullptr;
    MySimTestFramework framework;
    bool QueryInterface(uint32_t, void** out) override { *out = nullptr; return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    bool GetClassObject(uint32_t cls, uint32_t iid, void** out) override {
        return director && director->GetClassObject(cls, iid, out);
    }
    cIGZFrameWork* FrameWork() override { return &framework; }
    bool AddLibrary(const cIGZString&) override { return false; }
    void FreeUnusedLibraries() override {}
    bool CreateGuid(uint32_t*) override { return false; }
    bool CreateGuids(uint32_t[], uint32_t) override { return false; }
    bool RealInit() override { return true; }
    void RealShutdown() override {}
    cIGZSystemService* SetServiceRunning(bool) override { return nullptr; }
    uint32_t EnumerateLibraries(cIGZCOMLibrary*[], uint32_t& n) override { n = 0; return 0; }
    void EnumerateLibraries(GZCOMLibraryIterationFunction, void*) override {}
};
