// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The ABI surface the window layer stands on: the `cISC4View3DWin` slot
//! table (ui-windows.md §3.3), the `cIGZWin` subset the window layer actually
//! calls (§2.2, §13), and the `cSC4View3DWin` object layout (§3.2).
//!
//! These tables are **data**, not declarations of game calls. The ret channel
//! of every slot is recorded here exactly as §3.3 records it, so that a later
//! binding can be checked against it and so that a caller never reads a value
//! from a slot that does not produce one (HANDOFF §5.2: declaring
//! `float`/`double` where the game returns nothing leaves values on the x87
//! stack).
//!
//! Section references (`§n`) point at `docs/specs/ui-windows.md`.

/// A game method's return channel, as `docs/abi/vtables.tsv` and
/// `docs/abi/README.md` use the term.
///
/// `None` is the channel ui-windows.md §3.3 calls `none (void)`: the method
/// produces no value at all and a caller must not read EAX (conformance test
/// 3). It is distinct from `EaxOrVoid`, which is "EAX is not a usable status"
/// — a method that may leave something in EAX but whose callers ignore it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RetChannel {
    /// A bool in AL only; the upper bits of EAX are undefined.
    ///
    /// Reading this slot as anything wider than a bool is wrong, and a
    /// replacement that returns a full EAX bool is fine because a caller
    /// masks to AL.
    Al,
    /// A value in EAX that callers use (a count, a pointer, an int).
    Eax,
    /// EAX is not a usable status: a value, a pointer, a bool or nothing.
    ///
    /// `docs/abi/README.md`: "does not tell a pointer from a bool or a value:
    /// the spec decides that."
    EaxOrVoid,
    /// Nothing at all. A caller must not read EAX.
    None,
    /// `float`/`double` in ST(0).
    X87,
}

impl RetChannel {
    /// The `docs/abi/vtables.tsv` spelling of this channel.
    pub fn as_str(self) -> &'static str {
        match self {
            RetChannel::Al => "al",
            RetChannel::Eax => "eax",
            RetChannel::EaxOrVoid => "eax-or-void",
            RetChannel::None => "none",
            RetChannel::X87 => "x87",
        }
    }

    /// True when a caller may legitimately read a value out of this slot.
    ///
    /// `Al`, `Eax` and `X87` produce something. `EaxOrVoid` does not tell us
    /// whether it does, so this returns false and a caller that needs a value
    /// cites the spec instead.
    pub fn produces_value(self) -> bool {
        matches!(self, RetChannel::Al | RetChannel::Eax | RetChannel::X87)
    }
}

/// One row of a vtable as ui-windows.md records it: the slot index, its byte
/// offset in the vtable, the original target address, the method name, how
/// many argument dwords the callee pops (exclusive of `this`), how many
/// arguments that is, and the return channel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SlotInfo {
    pub slot: u32,
    /// Byte offset of the slot in the vtable: `slot * 4` on the 32-bit game.
    pub offset: u32,
    /// The original's method address, for reference. Not used at run time.
    pub target: u32,
    pub method: &'static str,
    /// Argument dwords the callee pops (`ret N`), exclusive of `this`.
    pub pops: u32,
    /// Argument count (an `int64`/`double` counts as 2).
    pub args: u32,
    pub ret: RetChannel,
}

// ---------------------------------------------------------------------------
// cSC4View3DWin / cISC4View3DWin
// ---------------------------------------------------------------------------

/// `cSC4View3DWin`'s class ID (ui-windows.md §3.1).
pub const VIEW3D_CLASS_ID: u32 = 0x9A47_B417;

/// The primary vtable of `cSC4View3DWin`, `cISC4View3DWin` (§3.1).
pub const VIEW3D_VTABLE: u32 = 0x00AB_CE40;

/// The `cIGZWin` vtable the window sub-object of a `cSC4View3DWin` carries
/// (`cGZWinGen`, §2.1).
pub const WIN_GEN_VTABLE: u32 = 0x00AD_C678;

/// `cIGZWin`'s IID (§2.1).
pub const IID_IGZ_WIN: u32 = 0x22BA_0121;

/// `cIGZWinGen`'s IID (§2.1). Queried from a scripted window to reach the
/// generator interface (§4.3).
pub const IID_IGZ_WIN_GEN: u32 = 0x5386_D516;

/// The interface `CloseTertiaryMenu` queries out of the tertiary window to
/// read the catalog's two dwords of state (§4.1). Unidentified — see
/// ui-windows.md UIW-1.
pub const IID_CATALOG_STATE: u32 = 0x8A2C_AD8B;

/// The view manager interface `GetViewManager` resolves (§3.2, `+0x300`).
pub const IID_VIEW_MANAGER: u32 = 0x67C5_BEDE;

/// The UI script **group** every scripted window is built from
/// (INVENTORY-M6 §4.1; §4.3, §6.1).
pub const UI_SCRIPT_GROUP: u32 = 0x96A0_06B0;

/// The UI script service id `SC4CreateWindowFromScript` looks up (§5.1).
pub const UI_SCRIPT_SERVICE: u32 = 0x5A35_6E15;

/// The UI script service interface id (§5.1). One less than the class id
/// `0xFA3562FB` of `cGZUIScriptService`.
pub const UI_SCRIPT_IFACE: u32 = 0xFA35_62FA;

/// The View3D save record's group/type/instance (§3.3a).
pub const VIEW3D_SAVE_GROUP: u32 = 0xA9C8_647C;
pub const VIEW3D_SAVE_TYPE: u32 = 0xE9C8_647E;
pub const VIEW3D_SAVE_INSTANCE: u32 = 1;

/// The extra integer pushed alongside the record header (§3.3a). *[inferred]*
/// a record version; the header helper's body was not read.
pub const VIEW3D_SAVE_VERSION: u32 = 4;

/// The `cSC4ModelMaker` class id `DoLoad` resolves the record's second
/// reference by (§3.3a).
pub const CLASS_MODEL_MAKER: u32 = 0xA9C7_41B3;

// --- object layout (§3.2) ---------------------------------------------------

/// `cISC4View3DWin` primary vptr.
pub const VIEW3D_OFF_PRIMARY_VPTR: u32 = 0x00;
/// A `cIGZWinMessageFilter` sub-object, inline (not a pointer). *[inferred]*
pub const VIEW3D_OFF_MSG_FILTER: u32 = 0x08;
/// **The `cIGZWin` / `cGZWinGen` sub-object.** `AsIGZWin()` returns `this +
/// 0x0C` (measured, `0x005F8400`).
pub const VIEW3D_OFF_WIN: u32 = 0x0C;
/// Secondary catalog-menu window, `cIGZWin*` or null.
pub const VIEW3D_OFF_SECONDARY_WIN: u32 = 0x1F8;
/// Tertiary catalog-menu window, `cIGZWin*` or null.
pub const VIEW3D_OFF_TERTIARY_WIN: u32 = 0x1FC;
/// Secondary menu kind.
pub const VIEW3D_OFF_SECONDARY_KIND: u32 = 0x200;
/// Tertiary menu kind.
pub const VIEW3D_OFF_TERTIARY_KIND: u32 = 0x204;
/// Pointer to a two-dword catalog state, or null (only the tertiary path
/// writes it). Quirk 5: the close path never validates the pointer.
pub const VIEW3D_OFF_CATALOG_STATE: u32 = 0x208;
/// The view manager object (single-site reading, §3.2).
pub const VIEW3D_OFF_VIEW_MANAGER: u32 = 0x300;

/// `AsIGZWin` (`0x005F8400`, slot 3): the whole body is
/// `test ecx,ecx; je null; lea eax,[ecx+0xc]; ret / xor eax,eax; ret`.
///
/// Returns `this + 0x0C` for any non-null `this`, and null for a null `this`.
/// Quirk 4: there is no check that the window sub-object is initialised, so a
/// caller must not treat "non-null" as "valid window".
pub fn as_igzwin(this: u32) -> u32 {
    if this == 0 {
        0
    } else {
        this.wrapping_add(VIEW3D_OFF_WIN)
    }
}

/// The 61-slot `cISC4View3DWin` table (vtable `0x00ABCE40`, slots 0-60).
///
/// Ordered by slot, which is the verified declaration order. The SDK lists
/// some overload pairs in reverse declaration order (the `SetCostIndicator`
/// pair at slots 42/43 is one), so a binding must declare through a
/// no-overload class in **this** order rather than the SDK's.
pub const VIEW3D_SLOTS: [SlotInfo; 61] = [
    SlotInfo { slot: 0, offset: 0x00, target: 0x007E_5630, method: "QueryInterface", pops: 8, args: 2, ret: RetChannel::Al },
    SlotInfo { slot: 1, offset: 0x04, target: 0x007E_56D0, method: "AddRef", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 2, offset: 0x08, target: 0x007E_56E0, method: "Release", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 3, offset: 0x0C, target: 0x005F_8400, method: "AsIGZWin", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 4, offset: 0x10, target: 0x0043_FD00, method: "GetRenderer", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 5, offset: 0x14, target: 0x007F_0FE0, method: "GetModelMaker", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 6, offset: 0x18, target: 0x007E_60B0, method: "GetViewManager", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 7, offset: 0x1C, target: 0x007F_1C10, method: "ProcessCommand", pops: 12, args: 3, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 8, offset: 0x20, target: 0x007E_60E0, method: "SetZoom", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 9, offset: 0x24, target: 0x007E_62C0, method: "ZoomIn", pops: 0, args: 0, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 10, offset: 0x28, target: 0x007E_6320, method: "ZoomOut", pops: 0, args: 0, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 11, offset: 0x2C, target: 0x007E_6150, method: "SetZoomLimits", pops: 8, args: 2, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 12, offset: 0x30, target: 0x007E_5780, method: "CanZoomIn", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 13, offset: 0x34, target: 0x007E_57A0, method: "CanZoomOut", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 14, offset: 0x38, target: 0x007E_61D0, method: "SetRotation", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 15, offset: 0x3C, target: 0x007E_6220, method: "RotateLeft", pops: 0, args: 0, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 16, offset: 0x40, target: 0x007E_6270, method: "RotateRight", pops: 0, args: 0, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 17, offset: 0x44, target: 0x007E_6380, method: "SetZoomAndRotation", pops: 8, args: 2, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 18, offset: 0x48, target: 0x007E_57C0, method: "GetCurrentViewFilterInfo", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 19, offset: 0x4C, target: 0x007E_63F0, method: "SetCurrentViewFilterInfo", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 20, offset: 0x50, target: 0x007E_57E0, method: "GetCurrentViewLevelInfo", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 21, offset: 0x54, target: 0x007E_6500, method: "SetCurrentViewLevelInfo", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 22, offset: 0x58, target: 0x007E_57F0, method: "GetCurrentViewLevel", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 23, offset: 0x5C, target: 0x007E_5800, method: "SetCurrentViewLevel", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 24, offset: 0x60, target: 0x007E_65E0, method: "GetCurrentViewInputControl", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 25, offset: 0x64, target: 0x007E_BD70, method: "SetCurrentViewInputControl", pops: 8, args: 2, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 26, offset: 0x68, target: 0x007E_9390, method: "RemoveCurrentViewInputControl", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 27, offset: 0x6C, target: 0x007E_5820, method: "RemoveAllViewInputControls", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 28, offset: 0x70, target: 0x007E_BEA0, method: "AddPersistentViewInputControl", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 29, offset: 0x74, target: 0x007E_7FB0, method: "RemovePersistentViewInputControl", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 30, offset: 0x78, target: 0x007E_8030, method: "RemoveAllPersistentViewInputControls", pops: 0, args: 0, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 31, offset: 0x7C, target: 0x007E_56F0, method: "GetKeyAccelerator", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 32, offset: 0x80, target: 0x007E_5700, method: "SetKeyAccelerator", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 33, offset: 0x84, target: 0x007F_64F0, method: "SetScrolling", pops: 12, args: 3, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 34, offset: 0x88, target: 0x007E_5740, method: "ScrollStop", pops: 0, args: 0, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 35, offset: 0x8C, target: 0x007E_7F90, method: "KillKeyboardScrolling", pops: 0, args: 0, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 36, offset: 0x90, target: 0x007E_5840, method: "PickTerrain", pops: 16, args: 4, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 37, offset: 0x94, target: 0x007E_5890, method: "PickOccupant", pops: 12, args: 3, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 38, offset: 0x98, target: 0x006F_7DC0, method: "DisplayTerrainPickDebugString", pops: 8, args: 2, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 39, offset: 0x9C, target: 0x007E_5900, method: "GetTerrainQueryEnabled", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 40, offset: 0xA0, target: 0x007F_5CF0, method: "SetCursorText", pops: 20, args: 5, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 41, offset: 0xA4, target: 0x007E_F7A0, method: "ClearCursorText", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 42, offset: 0xA8, target: 0x007E_C800, method: "SetCostIndicator2", pops: 8, args: 2, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 43, offset: 0xAC, target: 0x007F_0FF0, method: "SetCostIndicator1", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 44, offset: 0xB0, target: 0x007E_C900, method: "ClearCostIndicator", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 45, offset: 0xB4, target: 0x0064_8F10, method: "DebugDumpHook", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 46, offset: 0xB8, target: 0x007E_B850, method: "GetGlobalEffectTriggerState", pops: 4, args: 1, ret: RetChannel::Eax },
    SlotInfo { slot: 47, offset: 0xBC, target: 0x007E_CCE0, method: "AssertGlobalEffectTrigger", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 48, offset: 0xC0, target: 0x007E_B8A0, method: "RetractGlobalEffectTrigger", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 49, offset: 0xC4, target: 0x007F_1010, method: "GetCoverageHelper", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 50, offset: 0xC8, target: 0x007E_90F0, method: "SetCursorTextPosition3D", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 51, offset: 0xCC, target: 0x007F_1060, method: "GetLastSelectedBuilding", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 52, offset: 0xD0, target: 0x007F_1080, method: "GetLastSelectedLot", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 53, offset: 0xD4, target: 0x007F_1020, method: "SetLastSelectedBuilding", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 54, offset: 0xD8, target: 0x007F_1040, method: "SetLastSelectedLot", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 55, offset: 0xDC, target: 0x007E_91E0, method: "MinimizeUI", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 56, offset: 0xE0, target: 0x007E_CF60, method: "ShowMissionStatusPanel", pops: 4, args: 1, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 57, offset: 0xE4, target: 0x007E_77F0, method: "HideMissionStatusPanel", pops: 0, args: 0, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 58, offset: 0xE8, target: 0x007E_B970, method: "UpdateMissionStatusPanel", pops: 0, args: 0, ret: RetChannel::EaxOrVoid },
    // Slots 59/60: ui-windows.md §3.3 corrects vtables.tsv, which records
    // slot 60 as `via-call`/low confidence. The body at 0x007E5E90 ends in a
    // plain `ret` after `pop ecx; pop esi`, so the channel is none (void),
    // confidence high. Conformance test 3 pins this: a caller must not read
    // EAX from either.
    SlotInfo { slot: 59, offset: 0xEC, target: 0x007E_5D20, method: "CloseSecondaryMenu", pops: 0, args: 0, ret: RetChannel::None },
    SlotInfo { slot: 60, offset: 0xF0, target: 0x007E_5E90, method: "CloseTertiaryMenu", pops: 0, args: 0, ret: RetChannel::None },
];

/// `cISC4View3DWin`'s slot count: slots 0 through 60 (§3.1).
pub const VIEW3D_SLOT_COUNT: u32 = 61;

/// Slot numbers the window layer names, so call sites do not hard-code them.
pub mod view3d_slot {
    pub const QUERY_INTERFACE: u32 = 0;
    pub const ADD_REF: u32 = 1;
    pub const RELEASE: u32 = 2;
    pub const AS_IGZ_WIN: u32 = 3;
    pub const GET_RENDERER: u32 = 4;
    pub const GET_MODEL_MAKER: u32 = 5;
    pub const GET_VIEW_MANAGER: u32 = 6;
    pub const PROCESS_COMMAND: u32 = 7;
    pub const REMOVE_ALL_VIEW_INPUT_CONTROLS: u32 = 27;
    /// `CloseSecondaryMenu` — the `endTutorial` teardown (tutorial.md TU2).
    pub const CLOSE_SECONDARY_MENU: u32 = 59;
    /// `CloseTertiaryMenu`. §4.2: `CloseSecondaryMenu` calls this first.
    pub const CLOSE_TERTIARY_MENU: u32 = 60;
}

// ---------------------------------------------------------------------------
// cIGZWin (the window layer's subset)
// ---------------------------------------------------------------------------

/// `cIGZWin` slot numbers this layer uses (§2.2 and §13's replaceability
/// list: slots 0-3, 11, 14-18, 23, 34, 63-70, 113, 116-117, 143, 146-147).
///
/// §2.2's measured table below ([`WIN_SLOTS`]) is a subset of this set: it
/// lists the slots whose Windows bodies were re-checked for this spec. The
/// `63-70` range and `146-147` are named by §13 as what a replacement needs
/// but their bodies were not re-read here, so their rows carry the ABI
/// table's channel and are marked accordingly.
pub mod win_slot {
    pub const QUERY_INTERFACE: u32 = 0;
    pub const ADD_REF: u32 = 1;
    pub const RELEASE: u32 = 2;
    /// `DoMessage` (§6.5). Returns bool in AL only.
    pub const DO_MESSAGE: u32 = 3;
    pub const GET_PARENT_WIN: u32 = 11;
    pub const CHILD_ADD: u32 = 14;
    pub const CHILD_REMOVE: u32 = 15;
    /// `ChildDelete` — a four-instruction tail forward to the manager's
    /// `DestroyWindow` (§2.3). It **destroys** the child and is not
    /// overridable on the window.
    pub const CHILD_DELETE: u32 = 16;
    pub const PULL_TO_FRONT: u32 = 23;
    pub const GET_CHILD_WINDOW_FROM_ID: u32 = 34;
    /// `GetID` — the first argument `nSC4UI::AlignWindowToWindow` is passed
    /// (§4.3). In the `63-70` range §13 asks a replacement to implement.
    pub const GET_ID: u32 = 63;
    /// `SetID` (§5.1: what UIW-3 would settle is whether the create call
    /// sets the window's ID from the instance argument).
    pub const SET_ID: u32 = 64;
    pub const GET_FLAG: u32 = 67;
    pub const SET_FLAG: u32 = 68;
    pub const SHOW_WINDOW: u32 = 69;
    pub const HIDE_WINDOW: u32 = 70;
    pub const GET_PARAM: u32 = 113;
    pub const ADD_MESSAGE_FILTER: u32 = 116;
    pub const REMOVE_MESSAGE_FILTER: u32 = 117;
    pub const GZ_ON_COMMAND: u32 = 143;
    /// `SendMsg` / `PostMsg` occupy slots 144-147 (§6.5): the two forms are
    /// `(cIGZWin* target, type, d1, d2, d3)` (5 dwords) and
    /// `(cIGZWin* target, cGZMessage const&)` (2).
    ///
    /// `docs/abi/interfaces.tsv` names **146 and 147 both `PostMsg`**: 146 is
    /// the 5-argument form and forwards to 147; 147 is the `cGZMessage const&`
    /// form and returns the window manager's slot 15 result, not a status.
    /// The 144/145 split between the two forms of `SendMsg` is not measured
    /// here (§13 names only 146-147). *[choice]*
    pub const SEND_MSG_5: u32 = 144;
    pub const SEND_MSG_MSG: u32 = 145;
    /// `PostMsg(target, type, d1, d2, d3)` — forwards to [`POST_MSG`].
    pub const POST_MSG_5: u32 = 146;
    /// `PostMsg(target, cGZMessage const&)` → window manager slot 15.
    pub const POST_MSG: u32 = 147;
}

/// The `cIGZWin` slot rows this layer calls (§2.2), re-checked for this spec.
pub const WIN_SLOTS: [SlotInfo; 18] = [
    SlotInfo { slot: 0, offset: 0x00, target: 0x0099_8F83, method: "QueryInterface", pops: 8, args: 2, ret: RetChannel::Al },
    SlotInfo { slot: 1, offset: 0x04, target: 0x008F_B729, method: "AddRef", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 2, offset: 0x08, target: 0x008F_B72E, method: "Release", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 3, offset: 0x0C, target: 0x0099_A276, method: "DoMessage", pops: 4, args: 1, ret: RetChannel::Al },
    SlotInfo { slot: 11, offset: 0x2C, target: 0x0099_B7EE, method: "GetParentWin", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 14, offset: 0x38, target: 0x0099_EA66, method: "ChildAdd", pops: 4, args: 1, ret: RetChannel::Al },
    SlotInfo { slot: 15, offset: 0x3C, target: 0x0099_EA6B, method: "ChildRemove", pops: 4, args: 1, ret: RetChannel::Al },
    SlotInfo { slot: 16, offset: 0x40, target: 0x0099_B833, method: "ChildDelete", pops: 4, args: 1, ret: RetChannel::None },
    SlotInfo { slot: 23, offset: 0x5C, target: 0x0099_B856, method: "PullToFront", pops: 0, args: 0, ret: RetChannel::None },
    SlotInfo { slot: 34, offset: 0x88, target: 0x0099_DE74, method: "GetChildWindowFromID", pops: 4, args: 1, ret: RetChannel::Eax },
    SlotInfo { slot: 67, offset: 0x10C, target: 0x0099_BDBB, method: "GetFlag", pops: 4, args: 1, ret: RetChannel::Eax },
    SlotInfo { slot: 68, offset: 0x110, target: 0x0099_DB6B, method: "SetFlag", pops: 8, args: 2, ret: RetChannel::Eax },
    SlotInfo { slot: 69, offset: 0x114, target: 0x0099_D1AA, method: "ShowWindow", pops: 0, args: 0, ret: RetChannel::None },
    SlotInfo { slot: 70, offset: 0x118, target: 0x0099_D1EA, method: "HideWindow", pops: 0, args: 0, ret: RetChannel::None },
    SlotInfo { slot: 113, offset: 0x1C4, target: 0x0099_D388, method: "GetParam", pops: 8, args: 2, ret: RetChannel::Al },
    SlotInfo { slot: 116, offset: 0x1D0, target: 0x0099_D832, method: "AddMessageFilter", pops: 4, args: 1, ret: RetChannel::None },
    SlotInfo { slot: 117, offset: 0x1D4, target: 0x0099_D865, method: "RemoveMessageFilter", pops: 4, args: 1, ret: RetChannel::None },
    SlotInfo { slot: 143, offset: 0x23C, target: 0x0099_9896, method: "GZOnCommand", pops: 8, args: 2, ret: RetChannel::None },
];

/// The `63-70` and `146-147` `cIGZWin` slots §13 asks a replacement to
/// implement beyond [`WIN_SLOTS`]. Their Windows bodies were not re-read for
/// ui-windows.md, so the method names come from `docs/abi/interfaces.tsv`
/// and the channels are the ABI table's.
///
/// `GetID` (63) is load-bearing here: it is the first argument
/// `nSC4UI::AlignWindowToWindow` is passed on a catalog-menu create (§4.3),
/// and conformance test 19 reads it.
pub const WIN_EXTRA_SLOTS: [SlotInfo; 6] = [
    SlotInfo { slot: 63, offset: 0xFC, target: 0, method: "GetID", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 64, offset: 0x100, target: 0, method: "SetID", pops: 4, args: 1, ret: RetChannel::None },
    SlotInfo { slot: 65, offset: 0x104, target: 0, method: "GetUserData", pops: 0, args: 0, ret: RetChannel::Eax },
    SlotInfo { slot: 66, offset: 0x108, target: 0, method: "SetUserData", pops: 4, args: 1, ret: RetChannel::None },
    // 146/147 are both named PostMsg in interfaces.tsv: 146 is the
    // 5-argument form and forwards to 147, which is the
    // `cGZMessage const&` form.
    SlotInfo { slot: 146, offset: 0x248, target: 0, method: "PostMsg5", pops: 20, args: 5, ret: RetChannel::EaxOrVoid },
    SlotInfo { slot: 147, offset: 0x24C, target: 0, method: "PostMsg", pops: 8, args: 2, ret: RetChannel::EaxOrVoid },
];

/// `cIGZWinMgr` slots this layer names (§2.3, §6.5, modal-dialogs.md).
pub mod mgr_slot {
    /// `DestroyWindow` — the destination of `ChildDelete`'s tail forward.
    /// Deferred to the condemned list while the manager's busy count
    /// (slots 59/60) is non-zero (§2.3); otherwise destroyed at once by
    /// slot 56 `DestroyWindowAbsolute`.
    pub const DESTROY_WINDOW: u32 = 23;
    /// `IsWindowValid` (§3.2, quirk 4).
    ///
    /// Returns bool in **AL only** (upper EAX is garbage). It looks the
    /// pointer *value* up in the manager's window set without dereferencing
    /// it, so it is safe on a stale pointer — which is exactly why quirk 4
    /// says to use it rather than trust a non-null `AsIGZWin`.
    pub const IS_WINDOW_VALID: u32 = 24;
    /// `DoModalWin` (modal-dialogs.md §2.2).
    pub const DO_MODAL_WIN: u32 = 41;
    /// `EndModal(win, result)` = `GZWinMsgPost(0xF, win, result)` (slot 9).
    pub const END_MODAL: u32 = 54;
    /// `DestroyWindowAbsolute` — what slot 23 calls when the manager is idle.
    pub const DESTROY_WINDOW_ABSOLUTE: u32 = 56;
    /// `DestroyCondemnedWindows`.
    pub const DESTROY_CONDEMNED: u32 = 57;
    /// `IncrementBusyCount` — one half of the busy count slot 23 consults.
    pub const INCREMENT_BUSY: u32 = 59;
    /// `DecrementBusyCount`.
    pub const DECREMENT_BUSY: u32 = 60;
    /// `ModalMessageLoop` (modal-dialogs.md §2.3).
    pub const MODAL_MESSAGE_LOOP: u32 = 63;
}

/// `cIGZWinMgr`'s IID and the framework service that answers for it (§2.1).
pub const IID_IGZ_WIN_MGR: u32 = 0x5A4;
pub const SERVICE_WIN_MGR: u32 = 0xA417_445E;

/// The field at `cIGZWin + 4` that `ChildDelete` forwards through (§2.3).
///
/// *[inferred]* it is the window manager pointer. The forward and its
/// destination slot are measured; the field's identity is not (ui-windows.md
/// UIW-7). A replacement stores its manager handle here and re-derives
/// nothing.
pub const WIN_OFF_MANAGER: u32 = 0x04;

/// `ChildDelete`'s four-instruction body (§2.3):
///
/// ```text
/// 0099b833  mov ecx, dword ptr [ecx + 4]    ; ECX = this+4
/// 0099b836  mov eax, dword ptr [ecx]        ; its vtable
/// 0099b838  jmp dword ptr [eax + 0x5c]      ; tail-call slot 23
/// ```
///
/// It is a **tail call**, not a normal call: the window's own slot 16 body
/// never runs a return, so a `cIGZWin` subclass cannot intercept its own
/// child's destruction by overriding it (quirk 2). The destination is
/// `cIGZWinMgr::DestroyWindow`.
pub const CHILD_DELETE_FORWARDS_TO: u32 = mgr_slot::DESTROY_WINDOW;

/// `cGZUIScriptService`'s factory slot: the one `SC4CreateWindowFromScript`
/// calls (§5.1).
///
/// Quirk 10 / the name-collision trap: `CreateWindowFromScript` names two
/// functions and **neither is slot 4**. `0x0094B08B` is a 5-argument QI
/// wrapper that returns bool in AL and **releases the window it just made**;
/// `0x0094B210` takes a struct. The function that returns the window in EAX
/// is the unnamed slot 4, three arguments, forwarded unchanged.
pub const SCRIPT_SERVICE_CREATE_SLOT: u32 = 4;

/// The measured signature of the QI wrapper at `0x0094B08B`, recorded so a
/// binding cannot confuse it with slot 4 (§5.1).
pub struct QIWrapperSignature;

impl QIWrapperSignature {
    /// Argument dwords it pops (`ret 0x14`).
    pub const POPS: u32 = 20;
    /// Its return channel: bool in AL only.
    pub const RET: RetChannel = RetChannel::Al;
    /// What it does to the window it created: releases it.
    pub const RELEASES_WINDOW: bool = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view3d_table_is_61_slots_in_order() {
        assert_eq!(VIEW3D_SLOTS.len(), 61);
        assert_eq!(VIEW3D_SLOT_COUNT, 61);
        for (i, s) in VIEW3D_SLOTS.iter().enumerate() {
            assert_eq!(s.slot, i as u32, "slot {} out of order", i);
            assert_eq!(s.offset, s.slot * 4, "slot {} offset", i);
        }
    }

    #[test]
    fn as_igzwin_is_this_plus_0x0c() {
        // The load-bearing measurement: AsIGZWin at 0x005F8400 returns
        // this + 0x0C, and null for a null this (conformance test 1).
        assert_eq!(as_igzwin(0), 0);
        assert_eq!(as_igzwin(0x1000), 0x100C);
        assert_eq!(as_igzwin(0xFFFF_FFF8u32), 0x4); // wrapping, as LEA is
    }

    #[test]
    fn slots_59_and_60_return_nothing() {
        // Conformance test 3: neither CloseSecondaryMenu nor
        // CloseTertiaryMenu produces a value.
        for s in &VIEW3D_SLOTS {
            if s.slot == 59 || s.slot == 60 {
                assert_eq!(s.ret, RetChannel::None, "slot {}", s.slot);
                assert!(!s.ret.produces_value());
            }
        }
        assert_eq!(VIEW3D_SLOTS[59].method, "CloseSecondaryMenu");
        assert_eq!(VIEW3D_SLOTS[60].method, "CloseTertiaryMenu");
    }

    #[test]
    fn child_delete_is_a_forward() {
        assert_eq!(CHILD_DELETE_FORWARDS_TO, mgr_slot::DESTROY_WINDOW);
        let child_delete = WIN_SLOTS.iter().find(|s| s.slot == 16).unwrap();
        assert_eq!(child_delete.ret, RetChannel::None);
    }

    #[test]
    fn qi_wrapper_is_not_slot_4() {
        assert_eq!(QIWrapperSignature::POPS, 20);
        assert_eq!(QIWrapperSignature::RET, RetChannel::Al);
        assert!(QIWrapperSignature::RELEASES_WINDOW);
        assert_eq!(SCRIPT_SERVICE_CREATE_SLOT, 4);
    }
}
