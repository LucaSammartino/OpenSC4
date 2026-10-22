// SPDX-License-Identifier: GPL-3.0-or-later
// feature-manager.md §§1-7: app slot32 is borrowed. No native Init, Shutdown,
// static reset, state writes, factory, or Release-to-zero is permitted here.
#include "FC4FeatureShadow.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "compat/FC4Compat.h"
#include "fc4.h"
#include "GZServPtrs.h"
#include "cRZCOMDllDirector.h"
#include "cIGZCOM.h"
#include "cIGZFrameWork.h"
#include "cIGZApp.h"
#include "cISC4App.h"
#include "cISC4FeatureManager.h"
#include "cGZPersistResourceKey.h"
#include "cIGZPersistResourceManager.h"
#include "cIGZPersistDBRecord.h"
#include "cIGZPersistDBSerialRecord.h"
#include "cIGZDBSerialRecordAsIGZFile.h"
#include "cIGZFile.h"
#include <array>
#include <set>
#include <vector>
#include <cstdint>

namespace fc4 {
namespace {
cISC4FeatureManager* g_owned = nullptr;
fc4_feature_snapshot* g_snapshot = nullptr;
uint32_t g_exact = 0, g_diff = 0, g_refusal = 0, g_lifetime = 0, g_lifeDiff = 0, g_samples = 0;
bool g_attempted = false;
std::set<uint32_t> g_ids;

void Compare(uint32_t kind, uint32_t arg, uint32_t native) {
    const uint32_t rust = fc4_feature_snapshot_query(g_snapshot, kind, arg);
    if (rust == native) ++g_exact;
    else {
        ++g_diff;
        if (g_diff <= 12) Log("feature-shadow: DIFF kind=%u arg=%08X native=%u rust=%u", kind, arg, native, rust);
    }
}

void Queries() {
    if (!g_owned || !g_snapshot) return;
    ++g_samples;
    for (uint32_t id : g_ids) Compare(0, id, g_owned->IsFeatureEnabled(id));
    // All 32 shift residues at zero and large unsigned/signed boundaries.
    for (uint32_t base : {0u, 32u, 0x7fffffe0u, 0x80000000u, 0xffffffe0u})
        for (uint32_t i = 0; i < 32; ++i) Compare(1, base + i, g_owned->IsExpansionPackInstalled(base + i));
    Compare(2, 0, g_owned->GetCurrentExpansionPack());
}

// Public SDK raw record + serial/file adapter, matching the stages published
// in §4.1.1. Bytes are captured once; no native feature/pack answer feeds Rust.
// Complete-byte domain only: failed file Open/Read/oversize is a refusal.
bool Capture(std::array<std::vector<uint8_t>, 32>& bytes, std::array<fc4_feature_file, 32>& files) {
    cIGZPersistResourceManagerPtr rm;
    cIGZCOM* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    if (!rm || !com) return false;
    for (uint32_t k = 0; k < 32; ++k) {
        cGZPersistResourceKey key(0, 0x8A5971C5, 0x2B563701 + k);
        cIGZPersistDBRecord* rec = nullptr;
        (void)rm->OpenDBRecord(key, &rec, false);
        if (!rec) {
            Log("feature-input: pack=%u key=00000000/8A5971C5/%08X acquired=0 bytes=0", k + 1, 0x2B563701 + k);
            continue;
        }
        // OpenDBRecord owns the record until CloseDBRecord (not our AddRef).
        cIGZPersistDBSerialRecord* serial = nullptr;
        cIGZFile* file = nullptr;
        cIGZDBSerialRecordAsIGZFile* binding = nullptr;
        const bool serialOK = rec->QueryInterface(GZIID_cIGZPersistDBSerialRecord, reinterpret_cast<void**>(&serial));
        if (serial) FC4_ACQUIRED(serial, "feature serial QI");
        bool acquired = serialOK && serial && com->GetClassObject(GZCLSID_cIGZDBSerialRecordAsIGZFile,
            GZIID_cIGZFile, reinterpret_cast<void**>(&file)) && file;
        if (file) FC4_ACQUIRED(file, "feature file adapter");
        acquired = acquired && file->QueryInterface(GZIID_cIGZDBSerialRecordAsIGZFile,
            reinterpret_cast<void**>(&binding)) && binding;
        if (binding) {
            FC4_ACQUIRED(binding, "feature binding QI");
            (void)binding->SetDBSerialRecord(serial); // helper ignores this return (§4.1.1).
            FC4_RELEASE(binding); // file reference still keeps this temporary alive.
        }
        bool complete = true;
        if (acquired) {
            files[k].present = 1;
            complete = file->IsOpen() || file->Open(cIGZFile::AccessMode::Read,
                cIGZFile::CreationMode::OpenExisting, cIGZFile::ShareMode::Read);
            const uint32_t n = file->Length();
            complete = complete && n <= 16u * 1024u * 1024u && file->SeekToBegin() == 0;
            if (complete) {
                bytes[k].resize(n);
                complete = n == 0 || file->Read(bytes[k].data(), n);
                files[k].data = bytes[k].data();
                files[k].len = n;
            }
            if (file->IsOpen()) complete = file->Close() && complete;
        }
        if (file) FC4_RELEASE_CREATED(file);
        if (serial) FC4_RELEASE(serial); // record remains open until CloseDBRecord.
        rm->CloseDBRecord(key, &rec);
        // A located record whose acquisition stages fail is not a proven miss.
        // Refuse rather than infer the helper's success from record existence.
        if (!acquired || !complete) return false;
        uint32_t hash = 2166136261u;
        for (uint8_t b : bytes[k]) hash = (hash ^ b) * 16777619u;
        Log("feature-input: pack=%u key=00000000/8A5971C5/%08X acquired=1 bytes=%u fnv=%08X",
            k + 1, 0x2B563701 + k, static_cast<uint32_t>(bytes[k].size()), hash);
    }
    return true;
}

void Lifetime() {
    if (!g_snapshot) return;
    // One QI-owned hold plus the app hold always remain during these probes.
    const uint32_t up = FC4_ADDREF(g_owned);
    const uint32_t rustUp = fc4_feature_snapshot_lifetime(g_snapshot, 0, 0);
    const uint32_t down = FC4_RELEASE(g_owned);
    const uint32_t rustDown = fc4_feature_snapshot_lifetime(g_snapshot, 1, 0);
    g_lifetime += 2;
    if (up != rustUp) ++g_lifeDiff;
    if (down != rustDown || down == 0) ++g_lifeDiff;
    for (uint32_t iid : {1u, 0x0B562673u}) {
        cISC4FeatureManager* out = nullptr;
        const bool ok = g_owned->QueryInterface(iid, reinterpret_cast<void**>(&out));
        ++g_lifetime;
        const uint32_t bits = static_cast<uint32_t>(ok) | (static_cast<uint32_t>(out == g_owned) << 1);
        if (bits != fc4_feature_snapshot_lifetime(g_snapshot, 2, iid)) ++g_lifeDiff;
        if (ok && out) {
            FC4_ACQUIRED(out, "feature supported QI");
            const uint32_t native = FC4_RELEASE(out);
            const uint32_t rust = fc4_feature_snapshot_lifetime(g_snapshot, 1, 0);
            ++g_lifetime;
            if (native != rust) ++g_lifeDiff;
        }
    }
    void* out = reinterpret_cast<void*>(1);
    const bool ok = g_owned->QueryInterface(0x12345678u, &out);
    ++g_lifetime;
    const uint32_t bits = static_cast<uint32_t>(ok) | (static_cast<uint32_t>(out == reinterpret_cast<void*>(1)) << 1);
    if (bits != fc4_feature_snapshot_lifetime(g_snapshot, 2, 0x12345678u)) ++g_lifeDiff;
    Log("feature-lifetime: probe=%u exact=%u diff=%u native_add=%u rust_add=%u native_release=%u rust_release=%u",
        g_samples, g_lifetime - g_lifeDiff, g_lifeDiff, up, rustUp, down, rustDown);
}
}

void FeatureShadowLoaded() {
    if (SettingInt("feature_shadow", "FC4_FEATURE_SHADOW", 0) == 0 || g_attempted) return;
    g_attempted = true;
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* app = fw ? fw->Application() : nullptr;
    cISC4App* sc4 = nullptr;
    if (!app || !app->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&sc4)) || !sc4) {
        ++g_refusal; Log("feature-shadow: REFUSED app unavailable"); return;
    }
    FC4_ACQUIRED(sc4, "feature app QI");
    cISC4FeatureManager* borrowed = sc4->GetFeatureManager(); // no AddRef, never Release this borrow.
    cISC4FeatureManager* candidate = nullptr;
    if (compat::FeatureManagerSupported(borrowed) && borrowed->QueryInterface(0x0B562673u,
        reinterpret_cast<void**>(&candidate)) && candidate) {
        FC4_ACQUIRED(candidate, "feature manager QI owned hold");
        if (candidate == borrowed) g_owned = candidate;
        else FC4_RELEASE(candidate);
    }
    FC4_RELEASE(sc4);
    if (!g_owned) { ++g_refusal; Log("feature-shadow: REFUSED native ABI/ownership"); return; }
    std::array<std::vector<uint8_t>, 32> bytes;
    std::array<fc4_feature_file, 32> files{};
    if (!Capture(bytes, files) || !(g_snapshot = fc4_feature_snapshot_create(files.data()))) {
        ++g_refusal; Log("feature-shadow: REFUSED incomplete input capture (FM3 streaming excluded)"); return;
    }
    g_ids = {0, 1, 0x10, 0x14, 0x1A, 0x30, 0x40, 0x50, 0x60, 0x70, 0x99, 0xABC,
        0x00BC614E, 0x12345678, 0x7fffffff, 0x80000000, 0xffffffff};
    for (uint32_t i = 0, n = fc4_feature_snapshot_query(g_snapshot, 3, 0); i < n; ++i)
        g_ids.insert(fc4_feature_snapshot_query(g_snapshot, 4, i));
    Lifetime();
    Queries();
    Log("feature-shadow: captured-once ids=%u native-query-only FM1/FM2=excluded FM3=failure-injection-excluded FM4=core-only",
        static_cast<uint32_t>(g_ids.size()));
}

void FeatureShadowMonth() { Queries(); }

void FeatureShadowShutdown() {
    if (!g_attempted) return;
    Queries();
    if (g_owned) Lifetime();
    uint32_t remaining = 0;
    if (g_owned) {
        remaining = FC4_RELEASE(g_owned); g_owned = nullptr; ++g_lifetime;
        if (!remaining || (g_snapshot && remaining != fc4_feature_snapshot_lifetime(g_snapshot, 1, 0))) ++g_lifeDiff;
    }
    fc4_feature_snapshot_destroy(g_snapshot); g_snapshot = nullptr;
    Log("feature-shadow: totals samples=%u exact=%u diff=%u refusal=%u resync=0 lifetime_exact=%u lifetime_diff=%u app_hold_remaining=%u",
        g_samples, g_exact, g_diff, g_refusal, g_lifetime - g_lifeDiff, g_lifeDiff, remaining);
}
}
