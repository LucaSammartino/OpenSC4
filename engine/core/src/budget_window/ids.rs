// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Identities of the budget window family (budget-window.md §1-§7).

/// §1: window-generator class and interface.
pub const GENERATOR_CLSID: u32 = 0x4386_D516;
pub const GENERATOR_IID: u32 = 0x5386_D516;
/// §1: the dialog window's ID.
pub const DIALOG_WIN_ID: u32 = 0x0423_278F;
/// §1: `cSC4SubBudgetWindow` allocation size.
pub const SUB_BUDGET_WINDOW_SIZE: usize = 0x140;
/// §1: the budget panel's two tab windows.
pub const PANEL_TAB_0: u32 = 0xAA3A_C000;
pub const PANEL_TAB_1: u32 = 0xAA3A_C001;

/// §4 messages.
pub const MSG_INIT: u32 = 0xA2BF_8AD0;
pub const MSG_SHUTDOWN: u32 = 0xA2BF_8AD1;
pub const MSG_BUTTON: u32 = 0x2872_59F6;
pub const MSG_SLIDER: u32 = 0x8871_13A4;
pub const MSG_COMBO: u32 = 0xE871_1255;
pub const KEY_ESC: u32 = 0x1B;
pub const KEY_ENTER: u32 = 0x0D;
pub const KEY_0X73: u32 = 0x73;
/// §4: modifier bit that enables the Enter / `0x73` close.
pub const KEY_CLOSE_MODIFIER: u32 = 0x4;

/// §4 selectors.
pub const SELECTOR_ORDINANCES: u32 = 1;
pub const SELECTOR_NEIGHBORS: u32 = 2;

/// §4 background images (type, group, instance).
pub const BG_TYPE: u32 = 0x856D_DBAC;
pub const BG_GROUP: u32 = 0x46A0_06B0;
pub const BG_ORDINANCES: u32 = 0x1401_55F2;
pub const BG_NEIGHBORS: u32 = 0x1401_55D2;

/// §4.4: slider IDs below this set occupant funding level 0, else level 1.
pub const OCCUPANT_LEVEL1_SLIDER_MIN: u32 = 0x0ABC_E500;

/// §5 buttons.
pub const BTN_OK: u32 = 0x67;
pub const BTN_OK_ALT: u32 = 0x42B7_C353;
pub const BTN_CANCEL: u32 = 0x6D;
pub const BTN_CLOSE_CHILD: u32 = 0x68;
pub const BTN_CLOSE_CHILD_ALT: u32 = 0x384;
pub const BTN_END: u32 = 0xCC;
pub const BTN_END_ALT: u32 = 0x385;
pub const BTN_ORDINANCE_APPLY: u32 = 0xCD;
pub const BTN_CE: u32 = 0xCE;
pub const BTN_CF: u32 = 0xCF;
pub const BTN_LIST_A_UP: u32 = 0x451;
pub const BTN_LIST_A_DOWN: u32 = 0x452;
pub const BTN_LIST_B_UP: u32 = 0x453;
pub const BTN_LIST_B_DOWN: u32 = 0x454;
pub const BTN_NESTED_ROW_FIRST: u32 = 0x0ABC_E600;
pub const BTN_NESTED_ROW_LAST: u32 = 0x0ABC_E6FF;
pub const BTN_CAMERA_JUMP_FIRST: u32 = 0x0ABC_DE00;
pub const BTN_CAMERA_JUMP_LAST: u32 = 0x0ABC_DEFF;
/// §5: scroll step and page, in rows.
pub const SCROLL_STEP: u32 = 2;
pub const SCROLL_PAGE: u32 = 9;

/// §3 generator flags in `CreateSubBudgetWindow` order.
pub const CREATE_GENERATOR_FLAGS: [(u32, bool); 8] = [
    (0x1, true),
    (0x2, false),
    (0x1_0000, false),
    (0x20, false),
    (0x40, false),
    (0x1000, true),
    (0x2000, false),
    (0x4000, false),
];
/// §3 window flags in `CreateSubBudgetWindow` order.
pub const CREATE_WIN_FLAGS: [(u32, bool); 4] = [(0x100, true), (0x1_0000, false), (0x4, true), (0x800, false)];
/// §6: the window flag `end_dialog` tests (modal).
pub const WIN_FLAG_MODAL: u32 = 0x1000;

/// §7 panel messages.
pub const PANEL_MSG_MONEY_A: u32 = 0x08DA_C3E7;
pub const PANEL_MSG_MONEY_B: u32 = 0x08DA_C3E8;
pub const PANEL_MSG_TICK: u32 = 0x86AD_10EE;

/// §2.1 department-group selectors.
pub const GROUP_CIVIC: u32 = 0x0A5A_72D1;
pub const GROUP_TRANSPORTATION: u32 = 0xAA36_9059;
pub const GROUP_PROTECTION: u32 = 0x4A35_7B40;
pub const GROUP_EDUCATION_HEALTH: u32 = 0x6A35_7B7F;
pub const GROUP_UTILITIES: u32 = 0x4A35_7EAF;
pub const GROUP_PARKS_LANDMARKS: u32 = 0x6A35_7B96;
pub const GROUP_CIVIC2: u32 = 0xEA59_7195;

/// §5, §6: the code every `end_dialog` call passes (the modal result).
pub const END_CODE: u32 = 0x42B7_C353;
/// §4.1: group-mode background instance (the neighbour image).
pub const BG_GROUP_MODE: u32 = 0x1401_55D2;
/// §4.1: rows visible per list, and the income line-item type.
pub const LIST_ROWS_VISIBLE: u32 = 9;
pub const LINE_ITEM_INCOME: u32 = 1;
/// §4.1: line-item display flag bit 0 (ShowLineItem) and the ID bound.
pub const LINE_ITEM_SHOW: u32 = 0x1;
pub const LINE_ITEM_ALWAYS_ID_ABOVE: u32 = 100;
/// §5 ordinance check boxes and info buttons.
pub const BTN_ORDINANCE_CHECK_FIRST: u32 = 300;
pub const BTN_ORDINANCE_CHECK_LAST: u32 = 499;
pub const BTN_ORDINANCE_INFO_FIRST: u32 = 500;
pub const BTN_ORDINANCE_INFO_LAST: u32 = 699;
/// §5.4 deal flags.
pub const DEAL_PROPOSED_BIT: u32 = 0x1;
pub const DEAL_ACTIVE: u32 = 0x2;
pub const DEAL_CANCELLED: u32 = 0x4;
/// §5.5 departments (`+0x24`) and service kinds (`+0x134`).
pub const DEPT_HEALTH: u32 = 0x0918_8F42;
pub const DEPT_HEALTH_B: u32 = 0xAA53_8CB3;
pub const DEPT_EDUCATION: u32 = 0x0918_8F4C;
pub const DEPT_SCHOOLS: u32 = 0x4A53_8CC6;
pub const DEPT_POLICE: u32 = 0xA296_3983;
pub const DEPT_JAIL: u32 = 0xA296_3984;
pub const DEPT_FIRE: u32 = 0x28F5_5A9F;
pub const DEPT_POWER: u32 = 0x8910_BC8A;
pub const KIND_HOSPITAL: u32 = 0xCA56_5486;
pub const KIND_SCHOOL: u32 = 0xEA56_54B6;
pub const KIND_POLICE: u32 = 0x0A56_7BAA;
pub const KIND_FIRE: u32 = 0xEA56_7BC3;
pub const KIND_POWER: u32 = 0xCA58_E540;
/// §4.4, §5.5: the f32 constant at `0x00A823A4` (`0x3C23D70A`).
pub const FUNDING_SCALE: f32 = 0.01;
/// §5.7: the f32 global `0x00B4DFCC` that pre-fills the camera vector;
/// uninitialised data, 0.0 unless written (nothing in the spec writes it).
pub const CAMERA_JUMP_FILL: f32 = 0.0;
/// §2.1: `ShowWindow`'s flag cleared on the shown tab.
pub const PANEL_TAB_SHOWN_FLAG: u32 = 0x800;
/// §4 (BW10a): the content window `+0x10`'s ID.
pub const CONTENT_WIN_ID: u32 = 0x0423_278E;
/// §4 (BW10a): content window generator flags (generator slot 6), in order.
pub const CONTENT_GENERATOR_FLAGS: [(u32, bool); 8] = [
    (1, false),
    (2, false),
    (0x10000, false),
    (0x20, false),
    (0x40, false),
    (0x1000, true),
    (0x2000, true),
    (0x4000, true),
];
/// §4 (BW10a): content window `SetFlag` calls (slot 68), in order.
pub const CONTENT_WIN_FLAGS: [(u32, bool); 6] = [
    (0x100, true),
    (0x10000, true),
    (0x20_0000, false),
    (0x800, false),
    (0x8000, false),
    (4, true),
];
/// §5.3 (BW10c): `add_image_button2` image instances of the scroll arrows.
pub const ARROW_UP_IMAGE: u32 = 0x1401_55CB;
pub const ARROW_DOWN_IMAGE: u32 = 0x1401_55CC;
