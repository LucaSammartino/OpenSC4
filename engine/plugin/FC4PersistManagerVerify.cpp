// SPDX-License-Identifier: GPL-3.0-or-later
// persistence-manager.md sections 2 and 6.1. Public SDK calls only. The
// application manager is borrowed through its owning SDK service-reference
// helper, never constructed privately, reset, shut down or released to zero.
#include "FC4PersistManagerVerify.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "fc4.h"
#include "GZServPtrs.h"
#include "cIGZPersistResourceManager.h"
#include "cIGZPersistResource.h"
#include "cIGZPersistResourceFactory.h"
#include "cGZPersistResourceKey.h"
#include <array>
#include <atomic>
#include <cstring>
#include <new>

namespace fc4 {
namespace {
// Raw foreign fixture, not a persistence manager or cache algorithm. One
// creator reference per resource. Cached keys stay immutable during this probe.
class Resource final : public cIGZPersistResource {
public:
    explicit Resource(cGZPersistResourceKey key) : key_(key) {}
    bool QueryInterface(uint32_t iid, void** out) override {
        if (iid != GZIID_cIGZPersistResource && iid != 1) return false;
        *out = static_cast<cIGZPersistResource*>(this); AddRef(); return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override {
        const uint32_t remaining = --refs_;
        if (!remaining) delete this;
        return remaining;
    }
    void GetKey(cGZPersistResourceKey& key) const override { key = key_; }
    bool SetKey(cGZPersistResourceKey const& key) override { key_ = key; return true; }
    uint32_t GetType() const override { return key_.type; }
    uint32_t GetGroup() const override { return key_.group; }
    uint32_t GetInstance() const override { return key_.instance; }
    bool SetType(uint32_t value) override { key_.type = value; return true; }
    bool SetGroup(uint32_t value) override { key_.group = value; return true; }
    bool SetInstance(uint32_t value) override { key_.instance = value; return true; }
    uint32_t GetRefCount() const override { return refs_.load(); }
private:
    std::atomic<uint32_t> refs_{1};
    cGZPersistResourceKey key_;
};
struct Step { uint32_t op, resource, key, iid; };
constexpr std::array<fc4_pm_key, 3> kKeys{{
    {0xF4C400E1, 0xF4C4B441, 0xD00D4001},
    {0xF4C400E1, 0xF4C4B441, 0xD00D4002},
    {0xF4C400E1, 0xF4C4B441, 0xD00D4003},
}};
// Independent literal inputs shared by both executions, not native outputs.
constexpr Step kSteps[] = {
    {2, 0, 0, 0}, {3, 0, 0, 0}, {0, 3, 0, 0}, // absence, absent removal, null registration
    {6, 0, 0, 0x456B8F1D}, // independently absent factory: uncached load miss
    {0, 0, 0, 0}, {2, 0, 0, 0}, {4, 0, 0, 0x456B8F1D},
    {4, 0, 0, 0xDEADBEEF}, // failed QI preserves independently chosen sentinel
    {6, 0, 0, 0x456B8F1D}, // private load bypasses the present cached resource
    {1, 1, 0, 0}, {4, 0, 0, 0x456B8F1D}, // duplicate must keep resource 0
    {2, 0, 1, 0}, {1, 1, 1, 0}, {1, 2, 2, 0},
    {2, 0, 2, 0}, {3, 0, 0, 0}, {2, 0, 0, 0}, {3, 0, 0, 0},
    {4, 1, 1, 0x456B8F1D}, {3, 0, 1, 0}, {3, 0, 2, 0},
    {2, 0, 1, 0}, {5, 0, 0, 0},
};
cGZPersistResourceKey NativeKey(uint32_t index) {
    const auto& k = kKeys[index]; return {k.ty, k.group, k.instance};
}
void Refused(const char* reason, uint32_t fallback = 0) {
    Log("persist-manager-verify: totals reason=%s constructed=0 natural=0 executed=0 exact=0 diff=0 positive=0 negative=0 refusal=1 fallback=%u resync=0 cleanup=0/0 creator_refs=0/0 before_city=1", reason, fallback);
}
}

void PersistManagerVerify() {
    static bool attempted = false;
    if (SettingInt("persist_manager_verify", "FC4_PERSIST_MANAGER_VERIFY", 0) == 0 || attempted) return;
    attempted = true;
    // SDK GZServPtrs uses the published IID/service ID and owns exactly its
    // GetSystemService reference. Its destructor balances that reference only.
    cIGZPersistResourceManagerPtr manager;
    if (!manager) {
        Refused("acquisition");
        return;
    }
    Log("persist-manager-verify: acquired published SDK service hold; scoped preflight before city load");
    // Published typed route already used in FC4ExemplarFactory. No guessed
    // service ID, vtable cast or factory registration. A collision refuses.
    constexpr auto findByType = static_cast<bool (cIGZPersistResourceManager::*)(
        uint32_t, cIGZPersistResourceFactory**)>(&cIGZPersistResourceManager::FindObjectFactory);
    cIGZPersistResourceFactory* factory = nullptr;
    cIGZPersistResourceManager* borrowed = manager;
    const bool found = (borrowed->*findByType)(kKeys[0].ty, &factory);
    if (found || factory) {
        if (found && factory) { FC4_ACQUIRED(factory, "persistence type collision lookup"); FC4_RELEASE(factory); }
        Refused("factory-type-collision"); return;
    }
    for (uint32_t i = 0; i < 3; ++i) {
        const auto key = NativeKey(i);
        if (manager->HasRegisteredResource(key) || manager->TestForKey(key)) {
            Log("persist-manager-verify: collision key=%u", i);
            Refused("resource-key-collision");
            return; // No mutation occurred. Existing global state is out of scope.
        }
    }
    auto* model = fc4_pm_create(kKeys.data());
    if (!model) { Refused("Rust-allocation"); return; }
    std::array<fc4_pm_result, std::size(kSteps)> rust{};
    uint32_t refusal = 0, unexpected = 0;
    // Execute Rust first, before original operations. Only literal inputs flow in.
    for (size_t i = 0; i < std::size(kSteps); ++i) {
        const auto& s = kSteps[i];
        if (!fc4_pm_step(model, s.op, s.resource, kKeys[s.key], s.iid, &rust[i])) ++refusal;
        unexpected = rust[i].unexpected;
    }
    if (refusal || unexpected) {
        fc4_pm_destroy(model);
        Refused("Rust-transport", unexpected);
        return;
    }
    std::array<Resource*, 3> resources{{new (std::nothrow) Resource(NativeKey(0)), new (std::nothrow) Resource(NativeKey(1)), new (std::nothrow) Resource(NativeKey(2))}};
    if (!resources[0] || !resources[1] || !resources[2]) {
        for (auto* resource : resources) if (resource) resource->Release();
        fc4_pm_destroy(model); Refused("resource-allocation"); return;
    }
    uint32_t exact = 0, diff = 0, positive = 0, negative = 0, executed = 0;
    for (size_t i = 0; i < std::size(kSteps); ++i) {
        const auto& s = kSteps[i]; const auto key = NativeKey(s.key);
        fc4_pm_result native{};
        // MSVC reverses each overload group (published SDK convention also
        // documented by FC4ExemplarFactory): resource-only is slot 7, key+ref 8.
        switch (s.op) {
        case 0: native.status = manager->RegisterResource(s.resource < 3 ? resources[s.resource] : nullptr); break;
        case 1: native.status = manager->RegisterResource(key, *resources[s.resource]); break;
        case 2: native.status = manager->HasRegisteredResource(key); break;
        case 3: native.status = manager->UnregisterResource(key); break;
        case 4:
        case 6: {
            // Never turn an insertion failure into an uncontrolled factory load.
            if (s.op == 4 && !manager->HasRegisteredResource(key)) { ++refusal; break; }
            void* out = static_cast<cIGZPersistResource*>(resources[2]);
            native.status = s.op == 4 ? manager->GetResource(key, s.iid, &out, 0, nullptr)
                                     : manager->GetPrivateResource(key, s.iid, &out, 0, nullptr);
            for (uint32_t j = 0; j < 3; ++j)
                if (out == static_cast<cIGZPersistResource*>(resources[j])) native.identity = j + 1;
            if (native.status && native.identity) static_cast<cIGZPersistResource*>(out)->Release();
            else if (native.status) { ++refusal; } // Foreign pointer is never blindly released.
            break;
        }
        default: native.status = 1; break;
        }
        if (refusal) break;
        for (uint32_t j = 0; j < 3; ++j) {
            native.references[j] = resources[j]->GetRefCount();
            cGZPersistResourceKey current;
            resources[j]->GetKey(current);
            native.current_keys[j] = {current.type, current.group, current.instance};
        }
        ++executed;
        if (native.status) ++positive; else ++negative;
        if (std::memcmp(&native, &rust[i], sizeof(native)) == 0) ++exact;
        else {
            ++diff;
            Log("persist-manager-verify: DIFF step=%u op=%u status=%u/%u identity=%u/%u refs=%u,%u,%u/%u,%u,%u",
                static_cast<uint32_t>(i), s.op, native.status, rust[i].status, native.identity, rust[i].identity,
                native.references[0], native.references[1], native.references[2], rust[i].references[0], rust[i].references[1], rust[i].references[2]);
        }
    }
    // Unconditionally remove our entire preflight-empty key domain, even on a
    // difference/refusal. No factory, segment or global cache registry was changed.
    uint32_t cleanup = 0, refs = 0;
    for (uint32_t i = 0; i < 3; ++i) {
        manager->UnregisterResource(NativeKey(i));
        if (!manager->HasRegisteredResource(NativeKey(i))) ++cleanup;
        if (resources[i]->GetRefCount() == 1) ++refs;
        resources[i]->Release(); // Balances our own created fixture reference.
    }
    fc4_pm_destroy(model);
    if (cleanup != 3 || refs != 3) ++refusal;
    Log("persist-manager-verify: totals constructed=1 natural=0 planned=%u executed=%u exact=%u diff=%u positive=%u negative=%u refusal=%u fallback=%u resync=0 cleanup=%u/3 creator_refs=%u/3 before_city=1 factory=excluded-PR6 cache_binding=excluded-PR7 PR4=choice-preserved PR5=excluded-PR7",
        static_cast<uint32_t>(std::size(kSteps)), executed, exact, diff, positive, negative, refusal, unexpected, cleanup, refs);
}
}
