// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Identifiers required for compatibility (spec §1, §2.2, §6, §7, §9).

/// Class ID of the simulator object (spec §1).
pub const CLSID_SIMULATOR: u32 = 0x2990_C1E5;
/// `cISC4Simulator` interface ID (spec §1).
pub const IID_ISC4_SIMULATOR: u32 = 0x8695_664E;
/// System service ID and priority (spec §1, [compat]).
pub const SERVICE_ID_SIMULATOR: u32 = 0x4695_6659;
pub const SERVICE_PRIORITY_SIMULATOR: i32 = -2_000_000;

/// Number of agent levels (spec §2.2).
pub const LEVEL_COUNT: usize = 11;

/// Agent levels (spec §2.2).
pub mod level {
    pub const ONE_SHOT: u32 = 0;
    pub const EMERGENCY: u32 = 1;
    pub const MINUTE: u32 = 2;
    pub const HOUR: u32 = 3;
    pub const DAY: u32 = 4;
    pub const WEEK: u32 = 5;
    pub const MONTH: u32 = 6;
    pub const YEAR: u32 = 7;
    pub const SPECIFIC_DAY: u32 = 8;
    pub const SPECIFIC_TIME: u32 = 9;
    pub const IDLE: u32 = 10;
}

/// Agent registration flags (spec §4).
pub mod agent_flags {
    pub const ENABLED_UNESTABLISHED: u32 = 1;
    pub const ENABLED_ESTABLISHED: u32 = 2;
}

/// Message types (spec §2.2, §5, §6, §9).
pub mod msg {
    /// Scheduler message for level L is `SIM_LEVEL_BASE + L`.
    pub const SIM_LEVEL_BASE: u32 = 0x6695_6810;
    pub const SIM_BEGIN: u32 = 0x6695_6810;
    pub const SIM_NEW_DAY: u32 = 0x6695_6814;
    pub const SIM_NEW_WEEK: u32 = 0x6695_6815;
    pub const SIM_NEW_MONTH: u32 = 0x6695_6816;
    pub const SIM_NEW_YEAR: u32 = 0x6695_6817;
    pub const SIM_IDLE: u32 = 0x6695_681A;

    pub const PAUSE_CHANGED: u32 = 0xAA7F_B7E0;
    pub const HIDDEN_PAUSE_CHANGED: u32 = 0x4A7F_B7E2;
    pub const EMERGENCY_PAUSE_CHANGED: u32 = 0x4A7F_B807;
    pub const SPEED_CHANGED: u32 = 0x4A7F_B7E9;

    /// kSC4MessagePostCityInit: posted once the city has finished `Init`
    /// (spec §9 erratum: it is not a shutdown request).
    pub const POST_CITY_INIT: u32 = 0x26D3_1EC1;
    pub const CMD_GET_SIMULATION_DATE: u32 = 0x8BAC_4B43;
    pub const CHEAT_ISSUED: u32 = 0x230E_27AC;
}

/// Cheat IDs (spec §9).
pub mod cheat {
    pub const FPS: u32 = 0x0AC0_7E24;
    pub const SIM_DATE: u32 = 0x8A78_BEFF;
    pub const FPS_TEXT: &str = "FPS";
    pub const SIM_DATE_TEXT: &str = "SimDate";
}

/// Tuning exemplar (spec §7).
pub mod tuning {
    pub const EXEMPLAR_TYPE: u32 = 0x6534_284A;
    pub const EXEMPLAR_GROUP: u32 = 0xE7E2_C2DB;
    pub const EXEMPLAR_INSTANCE: u32 = 0xE959_0E52;
    pub const PROP_MS_PER_DAY: u32 = 0x8959_0E6D;
    pub const PROP_ANIMATION_DILATION: u32 = 0x8959_0E6E;
}

/// Name of the GetSimulationDate command (spec §9).
pub const CMD_GET_SIMULATION_DATE_NAME: &str = "GetSimulationDate";
