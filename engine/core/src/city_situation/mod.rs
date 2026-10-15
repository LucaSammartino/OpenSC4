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
//! - **Step 4 of the tick** (§4.4) runs only while the status is 3
//!   (running), and the elapsed time advances there by `t × 0.001` s; the
//!   time limit (when > 0) gives status 2 once the elapsed time reaches it
//!   and the host's `CheckForSuccess` still answers 3; the 30-second warning
//!   fires once when the remaining time is ≤ 30 s [question CS4].
//! - **`InitiateSituation`** returns false when the mission could not start
//!   (status 5) and true otherwise [question CS5].
//! - **`DeactivateSituation` and `EndSituation`** (slots 38, 39) are not
//!   described: deactivate (when a mission is active) sends `0xABBBCA64`
//!   (data1 = 0), releases the advice and resets the mission fields
//!   (status 4; a running countdown is ended and the automata unfrozen);
//!   `EndSituation` is the same call (its callers leave the player-drive
//!   mode first) [question CS6].
//! - **Taking over another entry's vehicle** when max CSI is reached (§4.2
//!   "may take over") is not done [question CS7].
//! - **The failure payout** deposits `failure_money` only when it is not 0,
//!   like the success [question CS8]; the reward list is saved only when a
//!   GUID was added.
//! - **The outcome position** (§4.5 step 4) is the active vehicle's, else
//!   the current target's; without either no aura is added [question CS9].
//! - **Slots 17, 18** answer the success aura and the success money of the
//!   active advice; **slots 19, 20** answer whether the automaton is tracked;
//!   **slot 22** starts the situation whose entry tracks the occupant, with
//!   a briefing; **slots 23, 25** do nothing and slot 23 answers false
//!   (CS2: the argument meanings are not known).
//! - **Automaton notifications** (`0x09D92FC3`, §3): 9 re-checks success of
//!   a running mission's vehicle, 2 fails it; 0xB, and 0xC with data3 =
//!   0x14, drop the automaton from tracking like a removed occupant
//!   [question CS10]. A removed occupant is dropped from tracking whether
//!   or not a mission is active [choice]. A removed active vehicle or
//!   target is forgotten (the tokens are borrowed) after the failure.
//! - **Occupant inserted** (§3 "note it as a candidate"): entries without
//!   a vehicle look for one again (the host lists the candidates)
//!   [choice].
//! - **The popup buttons** (§4.3): close on a finished mission (status 1
//!   or 2) posts `0x6BB350E4`; otherwise it cancels with
//!   `DeactivateSituation` [choice]. The popup kind follows the status
//!   (1 success; 2 failure; 5 "could not start"; else the briefing).
//! - **`InitiateSituationByGuid`** (slot 21): the advisor's advice with
//!   that GUID, the vehicle its entry tracks (if any); without a briefing
//!   it activates the mission at once; data3 is ignored.
//! - **`SetCSIVisible`** (slot 29) also re-shows the indicators when the
//!   view is held [choice].
//! - **Float arithmetic** (§4.4): `t × 0.001` is computed in double and
//!   stored to the float fields (x87 [choice]).
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

/// Advice situation flags (§3, §4.5).
pub const SITUATION_FLAG_WINDOW: u32 = 0x8;
pub const SITUATION_FLAG_REWARD: u32 = 0x40;

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
    /// A coverage mission builds its coverage cells.
    fn build_coverage_cells(&self, advice: u64);
    /// A target mission finds or creates its first target.
    fn first_target(&self, advice: u64) -> Option<u64>;
    /// The number of targets of a target mission (slot 11).
    fn target_count(&self, advice: u64) -> u32;
    /// Slot 16: the current target's name.
    fn target_name(&self, advice: Option<u64>, target: Option<u64>) -> String;

    // ---- automata ----
    /// `QueryInterface(0xA9B40F05)` succeeds.
    fn is_automaton(&self, object: u64) -> bool;
    /// The occupant has the property `id` (any value).
    fn occupant_has_property(&self, object: u64, id: u32) -> bool;
    /// The automata of the advice's automata groups.
    fn candidate_vehicles(&self, advice: u64) -> Vec<u64>;
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
    /// `CheckForSuccess(t)`: the new status.
    fn check_for_success(&self, advice: u64, t_ms: f32) -> u32;
    /// The pursuit logic.
    fn run_pursuit(&self, advice: u64, t_ms: f32);
    fn start_countdown_effect(&self);
    fn end_countdown_effect(&self);
    /// The success / failure effect at the vehicle's position.
    fn outcome_effect(&self, advice: u64, success: bool);
    /// Slot 35 `InitiatePlayerDriveMode(on)`.
    fn player_drive_mode(&self, vehicle: Option<u64>, on: bool);
    /// Slot 24 `StartJoyride`.
    fn start_joyride(&self, vehicle: Option<u64>);

    // ---- other components ----
    /// `aura.AddTransientEffect(x, z, radius, magnitude)` (aura.md §8.2).
    fn add_transient_effect(&self, x: f32, z: f32, radius: f32, magnitude: f32);
    /// `budget.DepositFunds(amount)` (budget.md §6).
    fn deposit_funds(&self, amount: i64);

    // ---- messages ----
    /// `MessageSend` of `0xABBBCA64` (data1 = 1 / 0, IGZUnknown = advice).
    fn send_activation(&self, advice: u64, active: bool);
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
