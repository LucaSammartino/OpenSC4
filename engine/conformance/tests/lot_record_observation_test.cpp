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
    return failures ? 1 : 0;
}
