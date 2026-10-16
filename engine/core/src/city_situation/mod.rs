// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The city situation manager (`cSC4CitySituationManager`, class
//! `0x0BB14381`), `docs/specs/city-situation.md`. Section references (`§n`)
//! point at that spec.
//!
//! The component runs the Rush Hour "U-Drive-It" missions. The spec is a
//! short form (§0): it describes the record, the lifecycle, the indicator
//! tracking, the mission state machine on the animation tick, the outcome
//! (money, aura, success count, popup) and the entry points. The mission
//! gameplay itself (targets, `CheckForSuccess`, pursuit, coverage cells, the
//! player-drive camera, the scripted windows and effects) is out of the
//! spec's scope and is the host's: the core calls it through
//! [`CitySituationHost`] at the points the spec names.
//!
//! # Structure
//!
//! - `manager.rs`: the state of §2, `Init` / `Shutdown` (§4.1), triggered
//!   and untriggered situations (§4.2), starting a mission and the popup
//!   (§4.3), the animation tick (§4.4), the outcome (§4.5), the debug and
//!   script entry points (§4.6), the messages (§3), the slots of §8 and the
//!   record of §7.
//! - `tests.rs`: a fake game and the §12 tests.
//!
//! # Tokens and references
//!
//! Objects are opaque host tokens. The core holds one reference on the
//! active advice, the held window and the four automata managers (returned
//! held by `acquire_automata_managers`). Advices in the track list, tracked
//! vehicles, the active automaton and the current target are borrowed (the
//! spec lists them without "held").
//!
//! # Choices (`[choice]`) and literal readings (`[question]`)
//!
//! - **Max CSI** (§2) is process-wide in the original; here it is a field of
//!   the manager (constructor value 0) [choice]. The glue keeps one manager
//!   per city, and the automata manager sets it through `SetMaxCSI`.
//! - **The tick** (§4.4, CS4): step 4 runs on every tick with a mission
//!   held, at any status, unless steps 1–3 ended the tick (a countdown that
//!   stays > 0, or a positive `endDelay`); it does run in the tick in which
//!   the countdown expires. `CheckForSuccess` is split: the host does its
//!   steps 2–4 (the evade list, the built-in checks, the Lua check), the
//!   core steps 1, 5 and 6 (the mission clock at status 3, in `f32`; a
//!   time limit ≥ 1.0 that the clock reached gives status 2). The 30 s
//!   warning compares the elapsed time before this tick's clock step; with
//!   no active automaton (the original has no null test) it plays nothing
//!   [choice].
//! - **`InitiateSituation`** (§4.3, CS5) follows the spec's eight steps:
//!   false for the advice already active (null included) or a null advice,
//!   and for "could not start" (status 5); without a briefing a started
//!   mission also gets status 5 and the post, but answers true *[quirk]*.
//! - **`DeactivateSituation` and `EndSituation`** (§4.3.1, CS6) are two
//!   calls. Deactivate leaves the time limit, the clock, the countdown,
//!   `endDelay` and the indicators alone; End releases the `+0xA9C` effect,
//!   zeroes the time limit, stops the countdown effect (the countdown value
//!   runs on and unfreezes the automata later), untracks the active vehicle
//!   through the entry that holds it and clears the advice's entry, then
//!   deactivates, and re-shows the indicators (when the view is held).
//! - **The track list** (§4.2, CS7) is re-sorted ascending by advice
//!   priority (stable) on each insert [question CS15: the priority compares
//!   unsigned]. The take-over at max CSI scans the entries after the new
//!   one for a strictly lower priority, which the sort rules out: not done.
//! - **The payouts** deposit only non-zero amounts (CS8); the reward list
//!   is saved only when a GUID was added.
//! - **The outcome position** (§4.5, CS9): success at the active
//!   automaton's position, else (0, 0) *[quirk]*; failure (only for the
//!   active advice) at the active automaton's, else the target's, and with
//!   neither it skips the effect, the aura and the money.
//! - **Slots 17, 18** answer the success aura and the success money of the
//!   active advice; **slots 19, 20** answer whether the automaton is tracked.
//!   **Slot 22** `(occupant, advice)` follows §8 (CS2). **Slots 21, 23–25**
//!   follow §8 (CS11): slot 21 tries the drivable automata of the group (or
//!   the advice's groups) in set (token) order, then a created one; slot 23
//!   starts the first entry accepting the group through slot 21, or a
//!   joyride (the automaton is held for the call and released after
//!   `StartJoyride` [choice]); slot 24 sends `0xABBBCA64` with no advice;
//!   slot 25 sets the group and runs `HandleInsertAutomaton`.
//! - **Automaton notifications** (§4.7, CS10): 2 fails a running mission of
//!   the active automaton; 9 calls `CheckForSuccess(0)` for the active
//!   automaton or the target and does not apply its result [question CS12];
//!   0xB, and 0xC with data3 = 0x14, mark the automaton disabled, end or
//!   advance the mission (the host picks the next target; the index and
//!   count stay [question CS14]) and drop it from tracking. A removed
//!   occupant is dropped from tracking whether or not a mission is active
//!   [choice]; a removed active vehicle or target is forgotten (the tokens
//!   are borrowed) after the failure.
//! - **Occupant inserted** (`0x99EF1142`) and slot 25 run
//!   `HandleInsertAutomaton`: the first entry without a vehicle whose advice
//!   accepts the automaton (advice slot 53), while fewer than max CSI
//!   entries hold one [question CS13].
//! - **The popup buttons** (§4.3): close on a finished mission (status 1
//!   or 2) posts `0x6BB350E4`; otherwise it cancels with
//!   `DeactivateSituation` [choice]. The popup kind follows the status
//!   (1 and 5 success; 2 failure; else the briefing); a null advice counts
//!   as 5.
//! - **`SetCSIVisible`** (slot 29) also re-shows the indicators when the
//!   view is held [choice].
//! - **Float arithmetic** (§4.4, CS4): `t × 0.001f` and the sums are `f32`
//!   operations (the 24-bit x87 rounds each one to `float`).
//! - **The generator** (§5): `RzRandomSuper` seeded with
//!   `derive_seed(S, 0x0BB14381)` (the spec's *[choice]*); the core never
//!   draws by itself; the host's mission code draws through
//!   [`CitySituationManager::random_uniform_u32`] and
//!   [`CitySituationManager::random_range_i32`].
//! - **`Read`** (§7) follows the spec literally: a version above 2 fails
//!   before anything changes; otherwise `loadedFromSave` is set, the map is
//!   cleared and filled entry by entry (a stream error leaves what was read).
//! - **The write order** of the success map (§7: hash-map order, any order
//!   valid on read) is ascending GUID [choice].
//! - **Threading** (foundations §19.3): the state is a `RefCell`; the core is
//!   main-thread only. No borrow is held across a host call; host callbacks
//!   must not call back into the manager, except the generator draws.

mod manager;

#[cfg(test)]
mod tests;

pub use manager::{query_interface, CitySituationManager, CitySituationState, TrackEntry};

/// Class ID and interface ID (§1).
pub const CLSID_CITY_SITUATION: u32 = 0x0BB1_4381;
pub const IID_CITY_SITUATION: u32 = 0x6BB1_437A;
pub const IID_UNKNOWN: u32 = 0x0000_0001;
pub const IID_SERIALIZABLE: u32 = 0xE4FD_A3D4;
pub const IID_MESSAGE_TARGET2: u32 = 0x4522_94AA;
pub const IID_MESSAGE_TARGET: u32 = 0xE98B_2F57;
/// A city-situation advice (`QueryInterface`, §3).
pub const IID_SITUATION_ADVICE: u32 = 0x6BEB_1E8A;
/// An automaton (`QueryInterface`, §3).
pub const IID_AUTOMATON: u32 = 0xA9B4_0F05;

/// City component and getter slot (§1).
pub const CITY_COMPONENT: u32 = 52;
pub const CITY_GETTER_SLOT: u32 = 49;
/// The CitySituations advisor (§0).
pub const ADVISOR: u32 = 29;

/// Messages (§3).
pub mod msg {
    pub const OCCUPANT_INSERTED: u32 = 0x99EF_1142;
    pub const OCCUPANT_REMOVED: u32 = 0x99EF_1143;
    pub const ADVICE_ADDED: u32 = 0x8A09_FA64;
    pub const ADVICE_REMOVED: u32 = 0x8A09_FA65;
    pub const ADVICE_REMOVED_ALL: u32 = 0x8A09_FA67;
    /// Posted by its own popup's close button after a mission ended.
    pub const POPUP_CLOSED: u32 = 0x6BB3_50E4;
    /// Show the situation popup (posted only by the manager itself).
    pub const SHOW_POPUP: u32 = 0xEBB6_7EF7;
    pub const WINDOW: u32 = 0xCC0D_905F;
    /// `InitiateSituationByGuid(data1, data2 ≠ 0, data3)`.
    pub const INITIATE_BY_GUID: u32 = 0xAC10_45EF;
    pub const POST_CITY_INIT: u32 = 0x26D3_1EC1;
    /// The ten subscriptions of `Init` (§3), in table order.
    pub const SUBSCRIPTIONS: [u32; 10] = [
        OCCUPANT_INSERTED,
        OCCUPANT_REMOVED,
        ADVICE_ADDED,
        ADVICE_REMOVED,
        ADVICE_REMOVED_ALL,
        POPUP_CLOSED,
        SHOW_POPUP,
        WINDOW,
        INITIATE_BY_GUID,
        POST_CITY_INIT,
    ];
    /// The animation tick: data3 = the elapsed milliseconds as float bits.
    pub const ANIMATION_TICK: u32 = 0x86AD_10EE;
    /// Automaton notifications (data1 = the automaton, data2 = subtype).
    pub const AUTOMATON_NOTIFY: u32 = 0x09D9_2FC3;
    /// The player left the vehicle.
    pub const PLAYER_LEFT_VEHICLE: u32 = 0x0BB3_24BD;
    /// The cheat code manager.
    pub const CHEAT: u32 = 0x230E_27AC;
    /// The popup with no advice (no poster in the executable).
    pub const POPUP_NO_ADVICE: u32 = 0x2C16_BDE7;
    /// Sent: a mission becomes active (data1 = 1) or is deactivated (0).
    pub const ACTIVATION: u32 = 0xABBB_CA64;
    /// Sent: a target in a target sequence is reached (data1 = index).
    pub const TARGET_REACHED: u32 = 0xAC08_3D7D;
}

/// Automaton notification subtypes (§3).
pub mod notify {
    pub const REMOVED: u32 = 2;
    pub const MOVED: u32 = 9;
    pub const DISABLED_B: u32 = 0xB;
    pub const DISABLED_C: u32 = 0xC;
    pub const DISABLED_C_DATA3: u32 = 0x14;
}

/// The cheat (§4.6).
pub const CHEAT_ID: u32 = 0x8C41_36DB;
pub const CHEAT_NAME: &str = "csitrigger";

/// The status values (§2).
pub mod status {
    pub const SUCCESS: u32 = 1;
    pub const FAILURE: u32 = 2;
    pub const RUNNING: u32 = 3;
    pub const IDLE: u32 = 4;
    pub const COULD_NOT_START: u32 = 5;
}

/// Mission types (§4.3).
pub const MISSION_TYPE_NO_TARGET: u32 = 2;
pub const MISSION_TYPE_COVERAGE: u32 = 5;

/// Advice situation flags (§3, §4.3, §4.5).
pub const SITUATION_FLAG_WINDOW: u32 = 0x8;
pub const SITUATION_FLAG_MYSIM_WINDOW: u32 = 0x20;
pub const SITUATION_FLAG_REWARD: u32 = 0x40;

/// Advice types whose disabled target moves on to the next one (§4.7).
pub const MISSION_TYPES_NEXT_TARGET: [u32; 2] = [1, 3];
/// The automaton flag a disabled automaton gets (§4.7).
pub const AUTO_FLAG_DISABLED: u32 = 0x1000_0000;
/// The smallest time limit that fires (§4.4, `CheckForSuccess` step 5).
pub const MIN_TIME_LIMIT: f32 = 1.0;
/// `t × 0.001f` (the `float` constant at `0x00A867A4`).
pub const MS_TO_SECONDS: f32 = 0.001;

/// Automaton flags and types of `CanPlayerDrive` (§4.2).
pub const AUTO_FLAG_DRIVABLE: u32 = 0x2;
pub const AUTO_FLAGS_BLOCKING: u32 = 0x1 | 0x0800_0000 | 0x1000_0000;
pub const AUTO_FLAG_LONG_END: u32 = 0x0800_0000;
pub const AUTO_TYPES_NOT_DRIVABLE: [u32; 2] = [0x34, 0x14];

/// The indicator kind of `TrackOccupant` / `ShowAllIndicators` (§4.1, §4.2).
pub const INDICATOR_KIND: u32 = 4;
/// The countdown (§4.4 step 1), milliseconds.
pub const COUNTDOWN_MS: f32 = 4000.0;
/// The 30-second warning (§4.4 step 4).
pub const WARNING_SECONDS: f32 = 30.0;
pub const WARNING_SOUND: u32 = 0x6C32_871F;
/// The default end delay (§4.4).
pub const DEFAULT_END_DELAY: f32 = 1.0;
/// The preview-occupant property (§3; presence).
pub const PROP_PREVIEW_OCCUPANT: u32 = 0x0292_44DB;
/// The preferences' reward list (§4.1 step 8).
pub const REWARD_SLOTS: usize = 64;
/// A success count saturates (§4.5 step 1).
pub const MAX_SUCCESS_COUNT: u16 = 0xFFFF;

/// The record version (§7).
pub const RECORD_VERSION: u16 = 2;

/// What the core needs to know about a city-situation advice (its Lua
/// table, §6).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AdviceInfo {
    pub guid: u32,
    /// The situation flags (8: remember a window; 0x40: a reward).
    pub flags: u32,
    /// 2: no target; 5: coverage; others: target missions.
    pub mission_type: u32,
    /// Seconds, used when the vehicle has flag `0x8000000` and it is > 1.0.
    pub end_delay: f32,
    pub success_money: i32,
    pub failure_money: i32,
    /// (radius, magnitude); radius > 0 adds an aura.
    pub success_aura: (f32, f32),
    pub failure_aura: (f32, f32),
}

/// The scripts of an advice (§4.3, §4.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Script {
    Start,
    Success,
    Failure,
}

/// The situation popup's window (§4.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Popup {
    /// `0x2BB16D50` / `0xEBB16D71`.
    Briefing,
    /// `0x0BBC06B6` / `0xEBBC081E`: success, and "could not start".
    Success,
    /// `0x4BBC080F` / `0xEBBC081E`.
    Failure,
}

/// Popup buttons (§4.3, data2 of `0x287259F6`).
pub mod button {
    pub const CLOSE: u32 = 1;
    pub const START: u32 = 6;
    pub const JOYRIDE: u32 = 7;
}

/// A message as `DoMessage` sees it (§3).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SituationMessage {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    /// IGZUnknown (an advice, a window) or the occupant / automaton.
    pub object: Option<u64>,
    /// void1 (the vehicle of `0xEBB67EF7`).
    pub void1: Option<u64>,
    /// The cheat's text (`0x230E27AC`).
    pub text: Option<String>,
}

/// Everything the city situation manager reads from, or does to, the rest
/// of the game. Re-entrancy: callbacks must not call back into the manager
/// (the generator draws excepted).
pub trait CitySituationHost {
    // ---- lifecycle (§4.1) ----
    /// `cISC4App` slot 21: the debug functionality flag.
    fn debug_enabled(&self) -> bool;
    /// With a cheat code manager: register +4 as a cheat target and the
    /// cheat `csitrigger` (`0x8C4136DB`); false without one.
    fn register_cheat(&self) -> bool;
    fn unregister_cheat(&self);
    fn subscribe(&self, msg_type: u32);
    fn unsubscribe(&self, msg_type: u32);
    /// Hold the dispatch manager's indicator view (slot 22); false = none.
    fn acquire_indicator_view(&self) -> bool;
    fn release_indicator_view(&self);
    /// Register / unregister +4 with the animation tick manager (priority
    /// `0xFF`).
    fn register_animation_tick(&self);
    fn unregister_animation_tick(&self);
    /// The four automata managers (city slots 43, 45, 44, 46, in that
    /// order), each held for the core (absent ones are skipped).
    fn acquire_automata_managers(&self) -> Vec<u64>;
    fn acquire_lot_manager(&self) -> bool;
    fn release_lot_manager(&self);
    fn acquire_mysim(&self) -> bool;
    fn release_mysim(&self);
    fn acquire_lua(&self) -> bool;
    fn release_lua(&self);
    /// Creates / removes the Lua table `sitmgr` (§4.1 step 5).
    fn create_lua_table(&self);
    fn remove_lua_table(&self);
    /// `Shutdown`: the effect handles and the coverage cells.
    fn release_mission_resources(&self);
    /// `indicatorView.ShowAllIndicators(kind, visible)`.
    fn show_all_indicators(&self, kind: u32, visible: bool);
    /// The user preferences' 64-entry reward list (`+0x1334`).
    fn preferences_rewards(&self) -> Vec<u32>;
    fn set_preferences_reward(&self, index: usize, guid: u32);
    /// `cISC4App` slot 17: save the preferences.
    fn save_preferences(&self);

    // ---- references ----
    fn add_ref(&self, token: u64);
    fn release(&self, token: u64);

    // ---- advices ----
    /// `QueryInterface(0x6BEB1E8A)` succeeds.
    fn is_situation_advice(&self, object: u64) -> bool;
    /// The advices of advisor 29.
    fn advisor_advices(&self) -> Vec<u64>;
    /// Advisor 29's advice with that GUID.
    fn find_advice(&self, guid: u32) -> Option<u64>;
    fn advice_info(&self, advice: u64) -> AdviceInfo;
    /// `RetireAdvice` (advisor slot 20).
    fn retire_advice(&self, advice: u64);
    fn run_script(&self, advice: u64, script: Script);
    /// The script's `time_limit` (seconds, 0 = none), read at activation.
    fn time_limit(&self, advice: u64) -> f32;
    /// Binds the mission script to the vehicle (§4.3).
    fn bind_mission_script(&self, advice: u64, vehicle: Option<u64>);
    /// A coverage mission builds its coverage cells (`0x0052C4E0`): false
    /// when that fails (§4.3 step 5).
    fn build_coverage_cells(&self, advice: u64) -> bool;
    /// A target mission finds its first target, or creates one (advice flag
    /// `0x100`; or flag 1 when none is found) (§4.3 step 5).
    fn first_target(&self, advice: u64) -> Option<u64>;
    /// The number of targets of a target mission: 1, or the size of the
    /// advice's target list when it has one (advice slots 17 and 16).
    fn target_count(&self, advice: u64) -> u32;
    /// Chooses the next target of a target mission (§4.7, a disabled target
    /// of advice type 1 or 3): the new target, or `None` when none is left.
    fn next_target(&self, advice: u64) -> Option<u64>;
    /// Slot 16: the current target's name.
    fn target_name(&self, advice: Option<u64>, target: Option<u64>) -> String;
    /// Advice slot 53 (`+0xD4`): the advice accepts the automaton (§8 slot 22).
    fn advice_accepts_automaton(&self, advice: u64, automaton: u64) -> bool;
    /// Advice slot 54 (`+0xD8`): the advice accepts the one-group list
    /// (§8 slot 23).
    fn advice_accepts_group(&self, advice: u64, group: u32) -> bool;
    /// Advice slot 12: the advice's automata groups (§8 slot 21).
    fn advice_groups(&self, advice: u64) -> Vec<u32>;
    /// `cISC4Advice` slot 7: the priority `trackList` is sorted by (§4.2).
    fn advice_priority(&self, advice: u64) -> u32;

    // ---- automata ----
    /// `QueryInterface(0xA9B40F05)` succeeds.
    fn is_automaton(&self, object: u64) -> bool;
    /// The occupant has the property `id` (any value).
    fn occupant_has_property(&self, object: u64, id: u32) -> bool;
    /// The automata of the advice's automata groups.
    fn candidate_vehicles(&self, advice: u64) -> Vec<u64>;
    /// The automata of one group (`0x00525640`: each of the four automata
    /// managers whose slot 30 accepts the group, slot 32), borrowed.
    fn group_automata(&self, group: u32) -> Vec<u64>;
    /// `CreateAutomatonForPlayerDrive` (`0x005292F0`) for the advice (if
    /// any) and the group (0 = the advice's own): one reference for the
    /// core, or `None`.
    fn create_automaton_for_player_drive(&self, advice: Option<u64>, group: u32) -> Option<u64>;
    /// Automaton slot 13 `Remove(0)`.
    fn remove_automaton(&self, automaton: u64);
    /// Automaton slot 30 `SetFlag(flag, 1)`.
    fn set_automaton_flag(&self, automaton: u64, flag: u32);
    /// Sets the group on the automaton's slot-5 object (its slot 23, §8
    /// slot 25).
    fn set_automaton_group(&self, automaton: u64, group: u32);
    fn automaton_flags(&self, automaton: u64) -> u32;
    fn automaton_type(&self, automaton: u64) -> u32;
    /// `TrackOccupant(a, 4, …)` on the indicator view: the handle.
    fn track_indicator(&self, automaton: u64) -> u32;
    fn untrack_indicator(&self, handle: u32);
    /// The manager becomes / stops being the vehicle's listener.
    fn add_listener(&self, automaton: u64);
    fn remove_listener(&self, automaton: u64);
    /// Slot 9 (`on`) / slot 10 on an automata manager.
    fn freeze_automata(&self, manager: u64, on: bool);
    fn play_sound(&self, automaton: u64, sound: u32);
    /// An object's position in x and z.
    fn position(&self, object: u64) -> (f32, f32);

    // ---- the mission (out of the spec's scope) ----
    /// Steps 2–4 of `CheckForSuccess(t)` (§4.4): from the current status,
    /// the evade-list check (advice flag 2), the built-in checks (only at
    /// status 3, without a Lua check) and the Lua check (any status). The
    /// core does steps 1, 5 (the mission clock and the time limit) and 6.
    fn check_for_success(&self, advice: u64, t_ms: f32, status: u32) -> u32;
    /// The pursuit logic.
    fn run_pursuit(&self, advice: u64, t_ms: f32);
    fn start_countdown_effect(&self);
    fn end_countdown_effect(&self);
    /// The success / failure effect (advice slot 47 / 50) at (x, z) (§4.5).
    fn outcome_effect(&self, advice: u64, success: bool, x: f32, z: f32);
    /// Slot 35 `InitiatePlayerDriveMode(on)`: on, `EnterPlayerDriveView`
    /// with the active automaton; off, leave the drive control.
    fn player_drive_mode(&self, vehicle: Option<u64>, on: bool);
    /// The popup's joyride button (§4.3).
    fn start_joyride(&self, vehicle: Option<u64>);
    /// `cSC4AutomataManager::EnterPlayerDriveView(automaton, target)` (§8
    /// slot 24).
    fn enter_player_drive_view(&self, automaton: u64, target: Option<u64>);
    /// `DeactivateSituation`'s mission part (§4.3.1 steps 1–2): free the
    /// coverage cells, clear the occupant set and the script references.
    fn clear_mission_state(&self, advice: u64);
    /// `EndSituation` step 1: release the effect at `+0xA9C`.
    fn release_situation_effect(&self);
    /// With advice flag `0x20` (§4.3 step 4): a window from the MySim
    /// simulator (slot 12), held for the core, or `None`.
    fn acquire_mysim_window(&self) -> Option<u64>;

    // ---- other components ----
    /// `aura.AddTransientEffect(x, z, radius, magnitude)` (aura.md §8.2).
    fn add_transient_effect(&self, x: f32, z: f32, radius: f32, magnitude: f32);
    /// `budget.DepositFunds(amount)` (budget.md §6).
    fn deposit_funds(&self, amount: i64);

    // ---- messages ----
    /// `MessageSend` of `0xABBBCA64` (data1 = 1 / 0, IGZUnknown = advice;
    /// `None` for a joyride, §8 slot 24).
    fn send_activation(&self, advice: Option<u64>, active: bool);
    /// `MessagePost` of `0xEBB67EF7` to itself (IGZUnknown = advice,
    /// void1 = vehicle or 0).
    fn post_show_popup(&self, advice: Option<u64>, vehicle: Option<u64>);
    /// `MessagePost` of `0x6BB350E4` (IGZUnknown = advice).
    fn post_popup_closed(&self, advice: u64);
    /// `MessagePost` of `0xAC1045EF` (data1 = GUID, data2 = show briefing,
    /// data3 = 0).
    fn post_initiate(&self, guid: u32, show_briefing: bool);

    // ---- the popup (§4.3) ----
    /// 3D view slot 57 `HideMissionStatusPanel`.
    fn hide_mission_status_panel(&self);
    /// Ends an open MySim dialog.
    fn end_mysim_dialog(&self);
    /// Shows the popup modally (with an auto game pause); the button that
    /// closed it (0 = none).
    fn show_popup(&self, popup: Popup, advice: Option<u64>, vehicle: Option<u64>) -> u32;
}

/// `CanPlayerDrive` (§4.2) from the automaton's flags and type.
pub fn can_player_drive(flags: u32, automaton_type: u32) -> bool {
    flags & AUTO_FLAG_DRIVABLE != 0
        && flags & AUTO_FLAGS_BLOCKING == 0
        && !AUTO_TYPES_NOT_DRIVABLE.contains(&automaton_type)
}

/// Parses the cheat's argument: `csitrigger <hex GUID>` (§4.6).
pub fn parse_cheat(text: &str) -> Option<u32> {
    let mut parts = text.split_whitespace();
    if !parts.next()?.eq_ignore_ascii_case(CHEAT_NAME) {
        return None;
    }
    let g = parts.next()?;
    let g = g.strip_prefix("0x").or_else(|| g.strip_prefix("0X")).unwrap_or(g);
    u32::from_str_radix(g, 16).ok()
}
