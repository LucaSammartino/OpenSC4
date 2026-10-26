// SPDX-License-Identifier: GPL-3.0-or-later
// automata.md 7.6.5: supplied native construction events, not worker AI.
// event, roof getter/setter pair, expected y word after event
{"D0 SetLot", true, 0x40E00831},
{"D0 animation q250", true, 0x41000000}, // 8
{"D0 daily before deadline", false, 0x41000000},
{"D0 animation q500", true, 0x41100000}, // 9
{"D1 daily phase1", false, 0x41100000},
{"D1 animation q250", true, 0x41280000}, // 10.5
{"D1 animation q500", true, 0x41400000}, // 12
{"D2 daily phase2 and animation", false, 0x41400000},
{"D3 daily before deadline", false, 0x41400000},
{"D4 daily phase1", false, 0x41400000},
{"D4 animation q250", true, 0x41580000}, // 13.5
{"D4 animation q500", true, 0x41700000}, // 15
{"D5 daily phase2", false, 0x41700000},
{"D7 daily phase3", false, 0x41700000},
{"D7 animation q250", true, 0x41300000}, // 11
{"D7 animation q500", true, 0x40E00000}, // 7
