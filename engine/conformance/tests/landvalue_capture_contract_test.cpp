// SPDX-License-Identifier: GPL-3.0-or-later
// landvalue.md 16.26-31: deterministic host acceptance fixtures, not an
// interceptor or a claim that the plugin supplies this host. Admission here is
// BEFORE destination fetch. A counter inside a real forwarder is insufficient.
#include <cstdint>
#include <cstdlib>
#include <iostream>
#include <vector>

namespace {
void Check(bool value) { if (!value) std::abort(); }

struct Delivery {
    unsigned generation;
    bool mayRead;
    bool nativeResult;
    unsigned forwards = 0;
    unsigned beforeCopies = 0;
    unsigned afterCopies = 0;
    int budget;
    std::vector<int16_t> developer;
};

// Reference host with explicit pre-fetch admission and retained-target leases.
// These leases model the exclusion LV8 requires, not an API the game exposes.
class Host {
public:
    bool excluded = false;
    bool route = false;
    bool ownsRoute = false;
    bool initialized = false;
    bool reading = false;
    bool startupComplete = false;
    unsigned generation = 0;
    unsigned admitted = 0;
    unsigned retainedTargets = 0;
    unsigned captures = 0;
    unsigned privateReads = 0;
    unsigned references = 0;
    uintptr_t ownerAddress = 0;
    std::vector<int16_t> actualD;

    void Exclude() { excluded = true; }
    bool Drained() const { return excluded && admitted == 0; }
    bool Install(bool validIdentity) {
        if (!Drained() || !validIdentity) return false;
        route = ownsRoute = true;
        return true;
    }
    bool Restore() {
        if (!Drained() || !ownsRoute) return false;
        route = ownsRoute = false;
        return true;
    }
    bool CanReclaim() const {
        return Drained() && !route && retainedTargets == 0;
    }
    void Open() { excluded = false; }
    void BeginInit(uintptr_t address) {
        Check(Drained() && !initialized && references == 0);
        ownerAddress = address;
        ++generation;
        reading = false;
        startupComplete = route;
        captures = 0;
        actualD.clear();
    }
    void EndInit() {
        Check(Drained());
        initialized = true;
        reading = route;
    }
    bool Fetch() {
        if (excluded) return false;
        ++admitted;
        return true;
    }
    Delivery Enter(int budget, bool nativeResult, bool developer = false) {
        Check(admitted != 0);
        Delivery d{generation, initialized && reading && route, nativeResult,
                   0, 0, 0, budget, {}};
        if (d.mayRead) {
            ++d.beforeCopies;
            ++privateReads;
            if (developer) d.developer = actualD; // actual pre-call map, never a best fit
        }
        return d;
    }
    bool Native(Delivery& d) {
        Check(d.forwards++ == 0);
        return d.nativeResult;
    }
    void Finish(Delivery& d) {
        Check(d.forwards == 1 && admitted != 0);
        if (d.mayRead) {
            Check(initialized && d.generation == generation);
            ++d.afterCopies;
            ++privateReads;
            ++captures;
        }
        --admitted;
    }
    void StopOwnerReads() { reading = false; }
    bool Shutdown() {
        if (!Drained() || reading) return false;
        initialized = startupComplete = false;
        ownerAddress = 0;
        actualD.clear();
        return true;
    }
};

void LifecycleOnly26() {
    Host h;
    // Pre-init/QI, ordinary pause, HiddenPause and queued post-init provide no
    // pre-fetch exclusion. Naming one of those callbacks cannot install a route.
    for (int boundary = 0; boundary != 4; ++boundary) {
        Check(!h.Install(true));
        Check(!h.route && h.privateReads == 0);
        Check(h.Fetch());
        auto d = h.Enter(5, boundary % 2 == 0);
        Check(h.Native(d) == d.nativeResult);
        h.Finish(d);
        Check(d.forwards == 1 && d.beforeCopies == 0 && d.afterCopies == 0);
    }
}

void Startup27() {
    for (bool loaded : {false, true}) {
        Host h;
        h.Exclude();
        Check(h.Install(true));
        h.BeginInit(0x1234);
        // Registration inside Init does not permit an initialized private read.
        Check(!h.Fetch() && h.privateReads == 0 && !h.reading);
        h.EndInit();
        h.actualD = loaded ? std::vector<int16_t>{-7, 42} : std::vector<int16_t>{0, 0};
        h.Open();
        Check(h.Fetch());
        auto d = h.Enter(5, true, true);
        const auto preCall = h.actualD;
        h.actualD = {99, 99};
        Check(h.Native(d));
        h.Finish(d);
        Check(d.developer == preCall && d.budget == 5 && d.forwards == 1);
        Check(d.beforeCopies == 1 && d.afterCopies == 1 && h.startupComplete);
    }
    Host late;
    late.Exclude(); late.BeginInit(0x1234); late.EndInit(); late.Open();
    Check(late.Fetch());
    auto first = late.Enter(5, false);
    Check(!late.Native(first)); late.Finish(first);
    late.Exclude(); Check(late.Install(true)); late.Open();
    Check(!late.startupComplete && late.privateReads == 0);
    // A loaded-owner object route is absent when a later new owner is created.
    late.Exclude(); late.StopOwnerReads(); Check(late.Shutdown());
    Check(late.Restore()); late.BeginInit(0x5678); late.EndInit();
    Check(!late.startupComplete);
}

void FetchedDestination28() {
    Host h;
    h.Exclude(); Check(h.Install(true));
    h.BeginInit(0x1234); h.EndInit(); h.Open();
    Check(h.Fetch()); // caller A has fetched a destination but has not entered
    Check(h.captures == 0);
    h.Exclude(); // caller B requests restoration; caller C must be excluded
    Check(!h.Fetch() && !h.Restore() && !h.CanReclaim());
    auto a = h.Enter(73, false);
    Check(!h.Native(a)); h.Finish(a);
    Check(a.budget == 73 && a.forwards == 1);
    Check(h.Restore() && h.CanReclaim());
}

void RetainedNotification29() {
    Host h;
    h.Exclude(); Check(h.Install(true));
    h.BeginInit(0x1234); h.EndInit(); h.Open();
    h.retainedTargets = 2; // server snapshots A and B, then releases its lock
    const bool subscribedB = false; // A unsubscribes B; snapshot still owns B
    Check(!subscribedB && h.retainedTargets == 2);
    h.Exclude(); h.StopOwnerReads(); Check(h.Shutdown());
    Check(h.Restore() && !h.CanReclaim());
    // A late retained callback forwards once but reads no retired owner data.
    h.Open(); Check(h.Fetch());
    auto b = h.Enter(5, true);
    Check(h.Native(b)); h.Finish(b);
    Check(b.forwards == 1 && b.beforeCopies == 0 && h.privateReads == 0);
    h.Exclude(); h.retainedTargets = 0;
    Check(h.CanReclaim());
}

void ShutdownReuse30() {
    Host h;
    h.Exclude(); Check(h.Install(true));
    h.BeginInit(0x1234); h.EndInit(); h.references = 2; h.Open();
    Check(h.Fetch()); auto d = h.Enter(5, true); Check(h.Native(d));
    h.Exclude(); h.StopOwnerReads();
    Check(!h.Shutdown()); // after-copy still admitted; no wait inside d itself
    h.Finish(d); Check(h.Shutdown());
    Check(h.references == 2 && h.actualD.empty() && h.ownerAddress == 0);
    h.Open(); Check(h.Fetch()); auto late = h.Enter(5, false);
    Check(!h.Native(late)); h.Finish(late);
    Check(late.beforeCopies == 0 && late.afterCopies == 0);
    h.Exclude(); h.references = 0;
    h.BeginInit(0x1234); h.EndInit(); // same address, distinct city generation
    Check(h.generation != d.generation && h.captures == 0 && h.actualD.empty());
    Check(h.route && h.startupComplete); // resident global route, no restoration
    h.Open(); Check(h.Fetch()); auto next = h.Enter(5, true);
    Check(h.Native(next)); h.Finish(next);
    Check(next.beforeCopies == 1 && next.afterCopies == 1 && h.captures == 1);
}

void OwnershipFailure31() {
    Host h;
    h.Exclude(); Check(!h.Install(false));
    Check(!h.route && !h.ownsRoute && h.privateReads == 0);
    Check(h.Install(true));
    h.ownsRoute = false; // foreign installer replaces the route
    Check(!h.Restore() && h.route && !h.CanReclaim());
    // Foreign-route ownership and former-route reachability are separate facts.
    h.retainedTargets = 1;
    Check(!h.CanReclaim() && h.privateReads == 0);
}
}

int main() {
    LifecycleOnly26(); Startup27(); FetchedDestination28();
    RetainedNotification29(); ShutdownReuse30(); OwnershipFailure31();
    std::cout << "LV8 host acceptance fixtures 26-31 passed; native capture not installed\n";
}
