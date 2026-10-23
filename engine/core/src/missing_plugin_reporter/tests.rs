// SPDX-License-Identifier: GPL-3.0-or-later
// Literal public MPR-T01–15 expectations recorded before production code.
// No expected trace is computed by a reporter/verifier implementation.
const T01: &[&str] = &["selector"];
const T02: &[&str] = &["selector", "dispatch R31", "set zero", "set init 100", "context", "manager", "set clear", "set buckets free"];
const T03: &[&str] = &["set zero", "set init 100", "context", "read C12", "read C16", "write F12 73", "write F16 A6", "manager", "set clear", "set buckets free"];
const T04: &[&str] = &["set zero", "set init 100", "context", "manager", "cache load", "alloc A17 12", "reset N7", "query M63 F51 max", "next N7", "reset N7", "cache load", "free A17 N7", "set clear", "set buckets free"];
const T05: &[&str] = &["occupant N23", "holder O17", "lookup H117 6A871B82", "release O17", "next N23"];
const T06: &[&str] = &["occupant N61", "holder O92", "lookup H192 6A871B82", "variant P512", "convert V612", "release O92", "next N61"];
const T07: &[&str] = &["occupant N23", "holder O17", "lookup H117 6A871B82", "variant P256", "convert V356", "capacity 1", "insert A15E0037", "release O17", "next N23", "permission"];
const T08: &[&str] = &["capacity 1", "insert 0", "release O92", "next N61", "permission", "dialog alloc 8", "dialog init D83 vptr A8ECE0 ref 0", "addref D83", "dialog D83", "save AL 00", "release D83"];
const T09: &[&str] = &["capacity 1", "insert FFFFFFFF", "release O17", "next N23", "permission", "dialog alloc 8", "dialog init D83 vptr A8ECE0 ref 0", "addref D83", "dialog D83", "save AL 01", "release D83"];
const T10: &[&str] = &["occupant N61", "holder O92", "lookup H192 6A871B82", "variant P512", "convert V612", "capacity 1", "insert A15E0037", "release O92", "next N61", "occupant N23", "holder O17", "lookup H117 6A871B82", "variant P256", "convert V356", "capacity 2", "insert D209", "release O17", "next N23", "permission", "dialog alloc 8", "dialog init D83 vptr A8ECE0 ref 0", "addref D83", "dialog D83", "save AL 80", "release D83"];
const T11: &[&str] = &["capacity 1", "insert A15E0037", "release O17", "next N23", "occupant N61", "holder O92", "lookup H192 6A871B82", "release O92", "next N61", "occupant N89", "holder O44", "lookup H144 6A871B82", "variant P768", "convert V868", "capacity 2", "insert A15E0037", "release O44", "next N89", "permission"];
const T12: &[&str] = &["save AL 00", "release D83", "next N7", "next N61", "cache load", "free A17 N61", "reset N7", "cache load", "free A17 N7", "set node free D209", "set clear", "set buckets free", "selector", "dispatch R31", "set zero", "set init 100"];
const T13: &[&str] = &["cache load", "framework", "lookup W39 3AE4BEA3 3AE4BEAB", "cache store A17", "release A17", "cache load", "alloc A17 12"];
const T14_NO_FRAMEWORK: &[&str] = &["set zero", "set init 100", "context", "manager", "cache load", "framework", "cache store null", "cache load"];
const T14_NULL_OUTPUT: &[&str] = &["set zero", "set init 100", "context", "manager", "cache load", "framework", "lookup W39 3AE4BEA3 3AE4BEAB", "cache store null", "cache load"];
const T15: &[&str] = &["capacity 1", "insert D209", "release O92", "next N61", "permission", "dialog alloc 8"];
