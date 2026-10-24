// SPDX-License-Identifier: GPL-3.0-or-later
// LR12 T58-T65 inventory/failure discriminators. Codec fields are additionally
// exercised against the real Rust reader in lotrec_lr12.rs.
#include "FC4LotRecordObservation.h"
#include <cstdio>

int main() {
    int failures = 0, a = 0, b = 0;
    auto check = [&](bool ok, const char* name) {
        if (!ok) { ++failures; std::printf("FAIL %s\n", name); }
    };
    using fc4::LotRecordObservation;
    { // T58: one count per lot, independent of covered cells.
        LotRecordObservation s; s.expected = 2;
        s.Visit(&a); s.Record(true, true, true);
        s.Visit(&b); s.Record(true, true, true); s.Finish(true, 2);
        check(s.visited == 2 && s.captures == 2 && s.exact == 2 && s.Mismatches() == 0, "T58");
    }
    { // T59: references/occupants never enter the master inventory.
        LotRecordObservation s; s.expected = 1;
        s.Visit(&a); s.Record(true, true, true); s.Finish(true, 1);
        check(s.seen.count(&b) == 0 && s.exact == 1 && s.Mismatches() == 0, "T59");
    }
    { // T60: cyclic/aliased references do not increase denominator.
        LotRecordObservation s; s.expected = 2;
        s.Visit(&a); s.Record(true, true, true);
        s.Visit(&b); s.Record(true, true, true); s.Finish(true, 2);
        check(s.expected == 2 && s.exact == 2 && s.Mismatches() == 0, "T60");
    }
    { // T61: available empty manager vs failed discovery.
        LotRecordObservation s; s.Finish(true, 0);
        check(s.Mismatches() == 0 && s.exact == 0, "T61 empty");
        s.Finish(false, 0); check(s.Mismatches() != 0, "T61 unavailable");
    }
    { // T62: failed partial Write must remain in denominator.
        LotRecordObservation s; s.expected = 2;
        s.Visit(&a); s.Record(false, false, false);
        s.Visit(&b); s.Record(true, true, true); s.Finish(true, 2);
        check(s.visited == 2 && s.captures == 1 && s.exact == 1 && s.Mismatches() != 0, "T62");
    }
    { // T63: a captured field mismatch is distinct from capture failure.
        LotRecordObservation s; s.expected = 1;
        s.Visit(&a); s.Record(true, true, false); s.Finish(true, 1);
        check(s.captures == 1 && s.exact == 0 && s.differences == 1, "T63");
    }
    { // T64: count drift, duplicate, null.
        LotRecordObservation s; s.expected = 2;
        s.Visit(&a); check(!s.Visit(&a) && !s.Visit(nullptr), "T64 values");
        s.Finish(true, 3); check(s.errors == 3, "T64 count");
    }
    { // T65: rejected/trailing payload and unequal reference identities fail.
        LotRecordObservation s; s.expected = 2;
        s.Visit(&a); s.Record(true, false, false);
        s.Visit(&b); s.Record(true, true, false); s.Finish(true, 2);
        check(s.captures == 2 && s.exact == 0 && s.errors == 1 && s.differences == 1, "T65");
    }
    { // T80/T81: shared unit edges deduplicate; null adds no payload.
        LotRecordObservation masters, units;
        masters.expected = 3;
        int c = 0;
        void* lots[] = {&a, &b, &c};
        void* getters[] = {&a, &a, nullptr};
        for (unsigned i = 0; i < 3; ++i) {
            masters.Visit(lots[i]);
            if (getters[i] && !units.seen.count(getters[i])) {
                ++units.expected; units.Visit(getters[i]); units.Record(true, true, true);
            }
        }
        masters.Finish(true, 3);
        check(masters.Mismatches() == 0 && units.expected == 1 && units.exact == 1, "T80 rooted unit union");
        masters.Finish(false, 3);
        check(masters.Mismatches() != 0, "T81 incomplete master walk invalidates units");
    }
    { // T82/T83/T86: registered list completeness, empty/failure, malformed values.
        struct Node { Node* next; Node* prev; void* value; };
        Node root{}, first{}, second{};
        root = {&first, &second, nullptr}; first = {&second, &root, &a}; second = {&root, &first, &b};
        auto observe = [&](bool queried, LotRecordObservation& s) {
            return fc4::ObserveRegisteredList(&root, queried, s,
                [&](void*) { s.Record(true, true, true); });
        };
        LotRecordObservation valid;
        check(observe(true, valid) && valid.exact == 2 && valid.Mismatches() == 0, "T82 distinct registered occupants");
        second.value = &a;
        LotRecordObservation duplicate;
        observe(true, duplicate);
        check(duplicate.expected == 2 && duplicate.exact == 1 && duplicate.Mismatches() != 0, "T86 duplicate stays error");
        second.value = nullptr;
        LotRecordObservation nullValue;
        observe(true, nullValue);
        check(nullValue.expected == 2 && nullValue.Mismatches() != 0, "T86 null stays error");
        second.next = &first;
        LotRecordObservation cycle;
        check(!observe(true, cycle) && cycle.Mismatches() != 0, "T86 detached cycle");
        root.next = root.prev = &root;
        LotRecordObservation empty, failed;
        check(observe(true, empty) && empty.expected == 0 && empty.Mismatches() == 0, "T83 successful empty");
        check(!observe(false, failed) && failed.Mismatches() != 0, "T83 failed is not empty success");
    }
    { // T85/T87: failure/QI/unsupported class remains in selected denominator.
        LotRecordObservation s; s.expected = 3;
        int c = 0;
        s.Visit(&a); s.Record(false, false, false);
        s.Visit(&b); s.Record(true, false, false);
        s.Visit(&c); s.Record(true, true, false);
        check(s.expected == 3 && s.exact == 0 && s.Mismatches() == 3, "T85/T87 failures retained");
    }
    return failures ? 1 : 0;
}
