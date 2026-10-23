// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent literal transcription of public VP-T traces, before comparison.
//! These are portable test inputs, NOT executed native vectors.

pub const INITIAL_BYTES: [(usize, u8); 13] = [
    (0x0c, 0),
    (0x0d, 7),
    (0x0e, 8),
    (0x0f, 9),
    (0x10, 10),
    (0x11, 11),
    (0x12, 12),
    (0x13, 13),
    (0x14, 14),
    (0x15, 15),
    (0x16, 16),
    (0x18, 18),
    (0x28, 28),
];
pub const INITIAL_DWORDS: [(usize, u32); 3] = [(0x1c, 1111), (0x20, 777), (0x24, 24)];
pub const INITIAL_WORDS: [u16; 6] = [1, 1, 32, 1, 1, 1];
pub const T4_BYTES: [u8; 13] = [0, 2, 3, 1, 3, 1, 0, 1, 0, 1, 2, 0, 1];
pub const T4_DWORDS: [u32; 3] = [1024, 768, 32];
pub const T4_CHECKED: [(u32, u8); 25] = [
    (0x6a57da49, 0),
    (0x6a57da4a, 1),
    (0x6a57da4b, 0),
    (0x6a57da4c, 0),
    (0x6a57da4d, 0),
    (0x6a57da4e, 255),
    (0x6a57da4f, 1),
    (0x6a57da50, 0),
    (0x6a57da51, 0),
    (0x2ab6c373, 0),
    (0x2ab6c374, 2),
    (0x2ab6c375, 0),
    (0xab81ded5, 1),
    (0x0b85cf92, 0),
    (0x2b85cf9f, 1),
    (0x2b85cfaa, 1),
    (0x6a57da57, 1),
    (0x6a57da58, 0),
    (0x6a57da55, 0),
    (0x6a57da56, 1),
    (0x6a57da60, 0),
    (0x6a57da5a, 0),
    (0x6a57da5b, 1),
    (0x6a57da5c, 0),
    (0x6a57da5d, 0),
];
pub const T4_DEPTH: [(u32, u8); 2] = [(0x2ab6c376, 0), (0x2ab6c377, 1)];

// T1..T3 stop after row four. A scoped prefix, not a full-function oracle.
pub const PREFIX: [([Option<u8>; 4], [u8; 2]); 3] = [
    ([Some(1), Some(0), Some(0), Some(0)], [1, 9]),
    ([Some(1), Some(2), Some(255), Some(1)], [3, 1]),
    ([None, Some(0), None, Some(0)], [7, 9]),
];

// Independently enumerated full failing-lookup order (utilities not lookups).
pub const ALL_LOOKUPS: [u32; 29] = [
    0x6a57da49, 0x6a57da4a, 0x6a57da4b, 0x6a57da4c, 0x6a57da4d, 0x6a57da4e, 0x6a57da4f, 0x6a57da50, 0x6a57da51,
    0x2ab6c373, 0x2ab6c374, 0x2ab6c375, 0xab81ded5, 0xab81ded6, 0xab81ded7, 0x0b85cf92, 0x2b85cf9f, 0x2b85cfaa,
    0x6a57da57, 0x6a57da58, 0x6a57da55, 0x6a57da56, 0x6a57da60, 0x6a57da5a, 0x6a57da5b, 0x6a57da5c, 0x6a57da5d,
    0x2ab6c376, 0x2ab6c377,
];

// T8..T24 literal admission/action inputs and portable assertions, §4.2.
pub const MESSAGES: [(u32, u32, u32); 10] = [
    (2, 0x287259f6, 0x6a57da48), // T8 false AL, root/source only
    (3, 0, 0x6a57da48),          // T9 true AL, root/source only
    (3, 0x287259f6, 0x6a57da52), // T10 true AL, gap
    (3, 0x287259f6, 0x6a57da48), // T11 guarded EndDialog
    (3, 0x287259f6, 0xea57da59), // T12 C=0 save/apply/notification
    (3, 0x287259f6, 0xea57da59), // T13 C=1 raw 83/84 requests
    (3, 0x287259f6, 0xea57da6f), // T14 C=1 unconditional save
    (3, 0x287259f6, 0xea5e99d9), // T15 A=null
    (3, 0x287259f6, 0xea5e99d9), // T16 A valid, P=null
    (3, 0x287259f6, 0xab81ded6), // T17 clear/select/clear, repeated
];
pub const T17_REQUESTS: [(u32, u16); 3] = [(0xab81ded5, 0x1c), (0xab81ded6, 0x18), (0xab81ded7, 0x1c)];
pub const T19_REQUESTS: [(u32, u16); 2] = [(0x6a57da4a, 0x18), (0x6a57da4b, 0x1c)];
pub const T20: (u32, u64, bool) = (2, 0x12345678, false);
pub const T21: (u32, u32) = (3, 2);
pub const T23_T24: [(u32, u8, u8, bool); 2] = [(1, 1, 0, false), (1, 1, 1, true)];
// T18 null P clears first-redraw 255 -> 0; T22 null H returns true, no calls.
