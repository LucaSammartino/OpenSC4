// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Automaton records (§6): the automaton base record (version 8), the states
//! (§6.2), the traffic automaton (§6.3) and the concrete classes of §6.4
//! (pedestrian, vehicle, emergency vehicle, patrol car, trailer), and the
//! automaton object with the slots other specs use (§7.4). Movement and
//! animation are not modelled.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::stream::{InStream, OutStream, RefValue, ResKey, SaveObject};

use super::codec::{
    get_key3, get_n_refs, get_ref_checked, get_refs, get_vec3, put_refs, put_vec3, ref_lists_equal, refs_equal,
    OccupantBase, Vec3, OCCUPANT_FLAG_VISIBLE,
};
use super::controller::saved_controller_ref;
use super::{
    CLSID_PATROL_CAR, CLSID_PEDESTRIAN, CLSID_TRAILER, CLSID_VEHICLE, IID_AUTOMATA_CONTROLLER, IID_AUTOMATON,
    IID_MESSAGE_TARGET, IID_MODEL, IID_NETWORK_OCCUPANT, IID_OCCUPANT, IID_SERIALIZABLE, IID_UNKNOWN,
};

/// The automaton occupant type (§6.1).
pub const AUTOMATON_OCCUPANT_TYPE: u32 = 0x496E_5783;
/// The automaton base record's current version (§6.1; Read accepts ≤ 8).
pub const AUTOMATON_BASE_VERSION: u16 = 8;
/// The state base record's current version (§6.2; Read accepts ≤ 3).
pub const STATE_BASE_VERSION: u16 = 3;
/// The traffic automaton record's version (§6.3; Read requires it).
pub const TRAFFIC_VERSION: u16 = 2;
/// The versions of §6.4 (written for new objects; a read one keeps its own).
pub const PEDESTRIAN_VERSION: u16 = 5;
pub const VEHICLE_VERSION: u16 = 2;
pub const EMERGENCY_VERSION: u16 = 2;
pub const PATROL_CAR_VERSION: u16 = 2;
pub const TRAILER_VERSION: u16 = 2;
/// The vehicle's constant vector (§6.4).
pub const VEHICLE_CONSTANT: Vec3 = [-1.0, -2.0, -3.0];

/// The state types of §6.2 with a record of their own.
pub mod state_type {
    /// `DEFAULT` (Lua `automata_states`): `PushState(0, 0, …)` delegates to
    /// `SetState` (§6.2).
    pub const DEFAULT: u32 = 0;
    /// `DISAPPEAR` (1). Also `SetState(1, …)` at the riot's end (§8.1), and
    /// what `PopState` pushes instead of revealing the state below a `final`
    /// one (§12 quirk 11).
    pub const RIOT_END: u32 = 1;
    pub const RIOT_LEADER: u32 = 8;
    pub const T0B: u32 = 0x0B;
    pub const T0E: u32 = 0x0E;
    pub const T0F: u32 = 0x0F;
    pub const PLAYER_DRIVE: u32 = 0x10;
    pub const T11: u32 = 0x11;
    pub const ATTACH_ANIM: u32 = 0x13;
    pub const T14: u32 = 0x14;
    pub const T15: u32 = 0x15;
    pub const CHASE: u32 = 0x34;
    /// The state factory (automaton slot 94) knows types 8 to 0x34.
    pub const FACTORY_FIRST: u32 = 8;
    pub const FACTORY_LAST: u32 = 0x34;
}

// ---- states (§6.2) ------------------------------------------------------------------------------------------

/// The state base record (§6.2, version 3; Read accepts ≤ 3).
///
/// The field names are the object offsets' meanings (§6.2, re-integrate1):
/// `PushState` writes every one of them. `+0x24` (the behaviour template) is
/// **not** in the record — `Read` zeroes it.
#[derive(Clone, Debug)]
pub struct StateBase {
    /// The version as read (3 for a new state). Written back unchanged unless
    /// [`StateBase::extra`] is non-zero, which needs version 3 *[choice]*.
    pub version: u16,
    /// `+0x08` the controller (read with `0x6A1857F8`); written as null when
    /// it has flag 8.
    pub controller: RefValue,
    /// `+0x0C` the **stack index** (0-based, bottom = 0) *[quirk]*. Not a
    /// parameter of `PushState`: it is recomputed as `stack.len() - 1` after a
    /// push (§12 quirk 10), and a replacement must recompute it after any
    /// stack mutation rather than trust the stream.
    pub stack_index: u32,
    /// `+0x10` the **countdown in milliseconds**. `DoStateTick` subtracts `dt`
    /// and `PopState`s on expiry; ≤ 0 means "never expires".
    pub countdown_ms: f32,
    /// `+0x14` **`final`**. `SetState` refuses to run while the top state has
    /// it (§12 quirk 11), and `PopState` on a `final` state that is not already
    /// type 1 pushes DISAPPEAR instead of revealing the state below.
    pub is_final: bool,
    /// `+0x18` the SFX id (`PushState` param 6).
    pub sfx_id: u32,
    /// `+0x1C` the animation id (`PushState` param 5).
    pub anim_id: u32,
    /// `+0x20` the automata-flags snapshot (written only at version ≥ 3).
    pub flags_snapshot: u32,
}

impl Default for StateBase {
    fn default() -> StateBase {
        StateBase {
            version: STATE_BASE_VERSION,
            controller: RefValue::Null,
            stack_index: 0,
            countdown_ms: 0.0,
            is_final: false,
            sfx_id: 0,
            anim_id: 0,
            flags_snapshot: 0,
        }
    }
}

impl PartialEq for StateBase {
    fn eq(&self, o: &Self) -> bool {
        self.version == o.version
            && refs_equal(&self.controller, &o.controller)
            && self.stack_index == o.stack_index
            && self.countdown_ms == o.countdown_ms
            && self.is_final == o.is_final
            && self.sfx_id == o.sfx_id
            && self.anim_id == o.anim_id
            && self.flags_snapshot == o.flags_snapshot
    }
}

impl StateBase {
    fn written_version(&self) -> u16 {
        if self.version >= 3 || self.flags_snapshot != 0 {
            self.version.max(3)
        } else {
            self.version
        }
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        let v = self.written_version();
        out.put_u16(v);
        out.put_ref(&saved_controller_ref(&self.controller, &|_| 0));
        out.put_u32(self.stack_index);
        out.put_f32(self.countdown_ms);
        out.put_bool(self.is_final);
        out.put_u32(self.sfx_id);
        out.put_u32(self.anim_id);
        if v >= 3 {
            out.put_u32(self.flags_snapshot);
        }
        out.error() == 0
    }

    fn read(inp: &mut dyn InStream) -> Option<StateBase> {
        let version = inp.get_u16()?;
        if version > STATE_BASE_VERSION {
            return None;
        }
        Some(StateBase {
            version,
            controller: get_ref_checked(inp, IID_AUTOMATA_CONTROLLER)?,
            stack_index: inp.get_u32()?,
            countdown_ms: inp.get_f32()?,
            is_final: inp.get_bool()?,
            sfx_id: inp.get_u32()?,
            anim_id: inp.get_u32()?,
            flags_snapshot: if version >= 3 { inp.get_u32()? } else { 0 },
        })
    }
}

/// What a state's record adds after the base (§6.2 table).
#[derive(Clone, Debug)]
pub enum StateExtra {
    /// The default state and every type without a row: nothing.
    None,
    /// 8 RiotLeader: `s32`, `s32`, `u8`.
    RiotLeader { a: i32, b: i32, c: u8 },
    /// `0x0B` RandomWalk: `vec3`, `f32`, `f32`.
    T0B { v: Vec3, a: f32, b: f32 },
    /// `0x0E` Plummet (§6.2, quirk 12): five `f32` named by their **object
    /// offsets**, written in the stream order `+0x28, +0x34, +0x30, +0x2C,
    /// +0x38` — **not** offset order. The object's sixth slot `+0x3C` is a
    /// `ref` (an effect pointer) that is zeroed on Read and never written
    /// *[quirk]*; a replacement that animates may keep it, but it is not part
    /// of the record and this codec does not model it.
    T0E {
        /// `+0x28` the descent velocity.
        descent: f32,
        /// `+0x2C` the roll rate.
        roll_rate: f32,
        /// `+0x30` the roll centre (the Y-orientation centre).
        roll_centre: f32,
        /// `+0x34` the tumble countdown, milliseconds.
        tumble_ms: f32,
        /// `+0x38` the post-splash countdown, milliseconds.
        post_splash_ms: f32,
    },
    /// `0x0F` PlopJump: `f32`, `f32`.
    T0F([f32; 2]),
    /// `0x10` PlayerDrive *[inferred]*: `f32`.
    PlayerDrive(f32),
    /// `0x11` Bumped: 4 × `vec3`.
    T11([Vec3; 4]),
    /// `0x13` AttachAnim (§6.2, quirk 14): the stream is `ref`, `u16`, `bool`,
    /// while the object holds the `bool` at `+0x2C` **before** the `u16` at
    /// `+0x30`. The fields are named by their object offsets; `write` emits
    /// the stream order.
    AttachAnim {
        /// `+0x28` the model (`0xA818EDD8`).
        model: RefValue,
        /// `+0x2C` the flag (a `bool` in memory, before the selector).
        flag: bool,
        /// `+0x30` the animation selector (`u16`).
        anim_selector: u16,
    },
    /// `0x15` PullOver: `f32`.
    T15(f32),
    /// `0x34` Chase *[inferred]*: `ref` automaton.
    Chase(RefValue),
}

impl PartialEq for StateExtra {
    fn eq(&self, o: &Self) -> bool {
        use StateExtra::*;
        match (self, o) {
            (None, None) => true,
            (RiotLeader { a, b, c }, RiotLeader { a: a2, b: b2, c: c2 }) => a == a2 && b == b2 && c == c2,
            (T0B { v, a, b }, T0B { v: v2, a: a2, b: b2 }) => v == v2 && a == a2 && b == b2,
            (
                T0E {
                    descent,
                    roll_rate,
                    roll_centre,
                    tumble_ms,
                    post_splash_ms,
                },
                T0E {
                    descent: d2,
                    roll_rate: r2,
                    roll_centre: c2,
                    tumble_ms: t2,
                    post_splash_ms: p2,
                },
            ) => descent == d2 && roll_rate == r2 && roll_centre == c2 && tumble_ms == t2 && post_splash_ms == p2,
            (T0F(x), T0F(y)) => x == y,
            (PlayerDrive(x), PlayerDrive(y)) => x == y,
            (T11(x), T11(y)) => x == y,
            (
                AttachAnim {
                    model,
                    flag,
                    anim_selector,
                },
                AttachAnim {
                    model: m2,
                    flag: f2,
                    anim_selector: a2,
                },
            ) => refs_equal(model, m2) && flag == f2 && anim_selector == a2,
            (T15(x), T15(y)) => x == y,
            (Chase(x), Chase(y)) => refs_equal(x, y),
            _ => false,
        }
    }
}

impl StateExtra {
    /// The extra part a new state of `state_type` starts with (all zero).
    pub fn default_for(kind: u32) -> StateExtra {
        use self::state_type as t;
        match kind {
            t::RIOT_LEADER => StateExtra::RiotLeader { a: 0, b: 0, c: 0 },
            t::T0B => StateExtra::T0B {
                v: [0.0; 3],
                a: 0.0,
                b: 0.0,
            },
            t::T0E => StateExtra::T0E {
                descent: 0.0,
                roll_rate: 0.0,
                roll_centre: 0.0,
                tumble_ms: 0.0,
                post_splash_ms: 0.0,
            },
            t::T0F => StateExtra::T0F([0.0; 2]),
            t::PLAYER_DRIVE => StateExtra::PlayerDrive(0.0),
            t::T11 => StateExtra::T11([[0.0; 3]; 4]),
            t::ATTACH_ANIM => StateExtra::AttachAnim {
                model: RefValue::Null,
                flag: false,
                anim_selector: 0,
            },
            t::T15 => StateExtra::T15(0.0),
            t::CHASE => StateExtra::Chase(RefValue::Null),
            // 0x14 and every other type: nothing after the base.
            _ => StateExtra::None,
        }
    }

    /// The record's stream order. For Plummet this is **not** offset order
    /// (§6.2, quirk 12) and for AttachAnim the `u16` comes before the `bool`
    /// even though the object holds the `bool` first (quirk 14).
    fn write(&self, out: &mut dyn OutStream) -> bool {
        match self {
            StateExtra::None => {}
            StateExtra::RiotLeader { a, b, c } => {
                out.put_i32(*a);
                out.put_i32(*b);
                out.put_u8(*c);
            }
            StateExtra::T0B { v, a, b } => {
                put_vec3(out, v);
                out.put_f32(*a);
                out.put_f32(*b);
            }
            StateExtra::T0E {
                descent,
                roll_rate,
                roll_centre,
                tumble_ms,
                post_splash_ms,
            } => {
                // Stream: +0x28, +0x34, +0x30, +0x2C, +0x38 (not offset order).
                out.put_f32(*descent);
                out.put_f32(*tumble_ms);
                out.put_f32(*roll_centre);
                out.put_f32(*roll_rate);
                out.put_f32(*post_splash_ms);
            }
            StateExtra::T0F(f) => {
                for x in f {
                    out.put_f32(*x);
                }
            }
            StateExtra::PlayerDrive(x) | StateExtra::T15(x) => {
                out.put_f32(*x);
            }
            StateExtra::T11(v) => {
                for x in v {
                    put_vec3(out, x);
                }
            }
            StateExtra::AttachAnim {
                model,
                flag,
                anim_selector,
            } => {
                // Stream: ref, u16, bool — the bool is at +0x2C, before the
                // u16 at +0x30, in the object (§6.2, quirk 14).
                out.put_ref(model);
                out.put_u16(*anim_selector);
                out.put_bool(*flag);
            }
            StateExtra::Chase(r) => {
                out.put_ref(r);
            }
        }
        out.error() == 0
    }

    fn read(state_type: u32, inp: &mut dyn InStream) -> Option<StateExtra> {
        Some(match StateExtra::default_for(state_type) {
            StateExtra::None => StateExtra::None,
            StateExtra::RiotLeader { .. } => StateExtra::RiotLeader {
                a: inp.get_i32()?,
                b: inp.get_i32()?,
                c: inp.get_u8()?,
            },
            StateExtra::T0B { .. } => StateExtra::T0B {
                v: get_vec3(inp)?,
                a: inp.get_f32()?,
                b: inp.get_f32()?,
            },
            StateExtra::T0E { .. } => StateExtra::T0E {
                descent: inp.get_f32()?,
                tumble_ms: inp.get_f32()?,
                roll_centre: inp.get_f32()?,
                roll_rate: inp.get_f32()?,
                post_splash_ms: inp.get_f32()?,
            },
            StateExtra::T0F(_) => StateExtra::T0F([inp.get_f32()?, inp.get_f32()?]),
            StateExtra::PlayerDrive(_) => StateExtra::PlayerDrive(inp.get_f32()?),
            StateExtra::T11(_) => StateExtra::T11([get_vec3(inp)?, get_vec3(inp)?, get_vec3(inp)?, get_vec3(inp)?]),
            StateExtra::AttachAnim { .. } => StateExtra::AttachAnim {
                model: get_ref_checked(inp, IID_MODEL)?,
                anim_selector: inp.get_u16()?,
                flag: inp.get_bool()?,
            },
            StateExtra::T15(_) => StateExtra::T15(inp.get_f32()?),
            StateExtra::Chase(_) => StateExtra::Chase(get_ref_checked(inp, IID_AUTOMATON)?),
        })
    }
}

/// One entry of the state stack (§6.1 field 9): the type and its record.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub state_type: u32,
    pub base: StateBase,
    pub extra: StateExtra,
}

/// Recompute every state's stack index (`+0x0C`, §12 quirk 10) after a stack
/// **mutation**: 0-based from the bottom, which is what `PushState` derives.
/// Not applied on Read — see the note in [`AutomatonBase::read`].
fn reindex(states: &mut [State]) {
    for (i, s) in states.iter_mut().enumerate() {
        s.base.stack_index = i as u32;
    }
}

impl State {
    /// A new state of `state_type` (the state factory, automaton slot 94).
    pub fn new(state_type: u32) -> State {
        State {
            state_type,
            base: StateBase::default(),
            extra: StateExtra::default_for(state_type),
        }
    }

    /// `u32` type, then the state record.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u32(self.state_type) && self.base.write(out) && self.extra.write(out)
    }

    fn read(inp: &mut dyn InStream) -> Option<State> {
        let state_type = inp.get_u32()?;
        let base = StateBase::read(inp)?;
        let extra = StateExtra::read(state_type, inp)?;
        Some(State {
            state_type,
            base,
            extra,
        })
    }
}

// ---- the automaton base record (§6.1) -------------------------------------------------------------------------

/// The automaton base record (§6.1, version 8; Read accepts ≤ 8).
#[derive(Clone, Debug)]
pub struct AutomatonBase {
    /// The version as read (8 for a new automaton). Written back unchanged
    /// unless `tail` is non-zero, which needs version 8 *[choice]*.
    pub version: u16,
    pub occupant: OccupantBase,
    /// Field 2: 3 × `vec3`.
    pub vectors: [Vec3; 3],
    /// Field 3.
    pub f1: [f32; 2],
    /// Field 4: the automata flags and an unnamed `u32`.
    pub automata_flags: u32,
    pub u1: u32,
    /// Field 5: the controller; written as null when it has flag 8.
    pub controller: RefValue,
    /// Field 6.
    pub f2: [f32; 2],
    /// Field 7.
    pub v2: Vec3,
    /// Field 8: the leader *[inferred]* (slots 48/49).
    pub leader: RefValue,
    /// Field 9: the state stack.
    pub states: Vec<State>,
    /// Field 10: model keys.
    pub model_keys: Vec<ResKey>,
    /// Field 11: the current model.
    pub model: ResKey,
    /// Field 12.
    pub u2: u32,
    /// Field 13: the callback targets (read with `0x452294AA`). Only
    /// serializable targets can be held here, so all are written.
    pub callbacks: Vec<RefValue>,
    /// Field 14: 8 × `f32`.
    pub f3: [f32; 8],
    /// Field 15.
    pub b1: u8,
    /// Field 16 (version 8 only).
    pub tail: u32,
}

impl AutomatonBase {
    /// Shared slot 51 contract, also used by non-traffic owners.
    pub fn set_state(&mut self, state_type: u32, args: SetStateArgs) -> bool {
        if self.states.last().map(|s| s.base.is_final).unwrap_or(false) {
            return false;
        }
        self.states = vec![args.to_state(state_type, RefValue::Null)];
        reindex(&mut self.states);
        true
    }

    /// Shared slot 52 contract. A null-controller DEFAULT replaces the stack.
    pub fn push_state(&mut self, state_type: u32, args: SetStateArgs) -> bool {
        if state_type == state_type::DEFAULT && args.controller == 0 {
            return self.set_state(state_type, args);
        }
        self.states.push(args.to_state(state_type, RefValue::Null));
        reindex(&mut self.states);
        true
    }

    /// Shared slot 53 contract, including final-state DISAPPEAR insertion.
    pub fn pop_state(&mut self) -> bool {
        let Some(top) = self.states.last() else {
            return false;
        };
        if top.base.is_final && top.state_type != state_type::RIOT_END {
            self.states
                .push(SetStateArgs::default().to_state(state_type::RIOT_END, RefValue::Null));
        } else {
            self.states.pop();
        }
        reindex(&mut self.states);
        true
    }

    /// The base of a new automaton: occupant type `0x496E5783`, visible, no
    /// states, version 8.
    pub fn new() -> AutomatonBase {
        AutomatonBase {
            version: AUTOMATON_BASE_VERSION,
            occupant: OccupantBase::new(AUTOMATON_OCCUPANT_TYPE),
            vectors: [[0.0; 3]; 3],
            f1: [0.0; 2],
            automata_flags: 0,
            u1: 0,
            controller: RefValue::Null,
            f2: [0.0; 2],
            v2: [0.0; 3],
            leader: RefValue::Null,
            states: Vec::new(),
            model_keys: Vec::new(),
            model: ResKey::default(),
            u2: 0,
            callbacks: Vec::new(),
            f3: [0.0; 8],
            b1: 0,
            tail: 0,
        }
    }

    fn written_version(&self) -> u16 {
        if self.version >= 8 || self.tail != 0 {
            self.version.max(8)
        } else {
            self.version
        }
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        let v = self.written_version();
        out.put_u16(v);
        if !self.occupant.write(out) {
            return false;
        }
        for x in &self.vectors {
            put_vec3(out, x);
        }
        out.put_f32(self.f1[0]);
        out.put_f32(self.f1[1]);
        out.put_u32(self.automata_flags);
        out.put_u32(self.u1);
        out.put_ref(&saved_controller_ref(&self.controller, &|_| 0));
        out.put_f32(self.f2[0]);
        out.put_f32(self.f2[1]);
        put_vec3(out, &self.v2);
        out.put_ref(&self.leader);
        out.put_u32(self.states.len() as u32);
        for s in &self.states {
            if !s.write(out) {
                return false;
            }
        }
        out.put_u32(self.model_keys.len() as u32);
        for k in &self.model_keys {
            out.put_res_key(*k);
        }
        out.put_res_key(self.model);
        out.put_u32(self.u2);
        put_refs(out, &self.callbacks);
        for x in &self.f3 {
            out.put_f32(*x);
        }
        out.put_u8(self.b1);
        if v >= 8 {
            out.put_u32(self.tail);
        }
        out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<AutomatonBase> {
        let version = inp.get_u16()?;
        if version > AUTOMATON_BASE_VERSION {
            return None;
        }
        let occupant = OccupantBase::read(inp)?;
        let vectors = [get_vec3(inp)?, get_vec3(inp)?, get_vec3(inp)?];
        let f1 = [inp.get_f32()?, inp.get_f32()?];
        let automata_flags = inp.get_u32()?;
        let u1 = inp.get_u32()?;
        let controller = get_ref_checked(inp, IID_AUTOMATA_CONTROLLER)?;
        let f2 = [inp.get_f32()?, inp.get_f32()?];
        let v2 = get_vec3(inp)?;
        let leader = get_ref_checked(inp, IID_AUTOMATON)?;
        let n = inp.get_u32()?;
        let mut states = Vec::new();
        for _ in 0..n {
            states.push(State::read(inp)?);
        }
        // The stack index (`+0x0C`) is kept as read: a re-save of the
        // original's save must be byte-identical (§14 test 4), and the corpus
        // carries values this reader would have to change to recompute
        // (measured: byte 174 of a 342-byte pedestrian record, saved 1 against
        // a recomputed 0; glue19 question A1). re-integrate1's §14 test 29 asks
        // for recompute-on-read as well; it is applied after every stack
        // **mutation** (below) but not here. See [`reindex`].
        let m = inp.get_u32()?;
        let mut model_keys = Vec::new();
        for _ in 0..m {
            model_keys.push(get_key3(inp)?);
        }
        let model = get_key3(inp)?;
        let u2 = inp.get_u32()?;
        let callbacks = get_refs(inp, IID_MESSAGE_TARGET)?;
        let mut f3 = [0.0; 8];
        for x in &mut f3 {
            *x = inp.get_f32()?;
        }
        let b1 = inp.get_u8()?;
        let tail = if version >= 8 { inp.get_u32()? } else { 0 };
        Some(AutomatonBase {
            version,
            occupant,
            vectors,
            f1,
            automata_flags,
            u1,
            controller,
            f2,
            v2,
            leader,
            states,
            model_keys,
            model,
            u2,
            callbacks,
            f3,
            b1,
            tail,
        })
    }
}

impl Default for AutomatonBase {
    fn default() -> Self {
        AutomatonBase::new()
    }
}

impl PartialEq for AutomatonBase {
    fn eq(&self, o: &Self) -> bool {
        self.version == o.version
            && self.occupant == o.occupant
            && self.vectors == o.vectors
            && self.f1 == o.f1
            && self.automata_flags == o.automata_flags
            && self.u1 == o.u1
            && refs_equal(&self.controller, &o.controller)
            && self.f2 == o.f2
            && self.v2 == o.v2
            && refs_equal(&self.leader, &o.leader)
            && self.states == o.states
            && self.model_keys == o.model_keys
            && self.model == o.model
            && self.u2 == o.u2
            && ref_lists_equal(&self.callbacks, &o.callbacks)
            && self.f3 == o.f3
            && self.b1 == o.b1
            && self.tail == o.tail
    }
}

// ---- the traffic automaton (§6.3) -----------------------------------------------------------------------------

/// The traffic automaton record (§6.3, version 2, required).
#[derive(Clone, Debug)]
pub struct TrafficRecord {
    pub base: AutomatonBase,
    /// `ref` automaton (`0xA9B40F05`).
    pub automaton: RefValue,
    pub u1: u32,
    pub b1: u8,
    /// `ref` network occupant (`0xA821EF94`).
    pub network: RefValue,
    pub u2: u32,
    pub u3: u32,
    /// The route steps (`u8`, `u8`, `u8`).
    pub route: Vec<[u8; 3]>,
}

impl PartialEq for TrafficRecord {
    fn eq(&self, o: &Self) -> bool {
        self.base == o.base
            && refs_equal(&self.automaton, &o.automaton)
            && self.u1 == o.u1
            && self.b1 == o.b1
            && refs_equal(&self.network, &o.network)
            && self.u2 == o.u2
            && self.u3 == o.u3
            && self.route == o.route
    }
}

impl Default for TrafficRecord {
    fn default() -> Self {
        TrafficRecord {
            base: AutomatonBase::new(),
            automaton: RefValue::Null,
            u1: 0,
            b1: 0,
            network: RefValue::Null,
            u2: 0,
            u3: 0,
            route: Vec::new(),
        }
    }
}

impl TrafficRecord {
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(TRAFFIC_VERSION);
        if !self.base.write(out) {
            return false;
        }
        out.put_ref(&self.automaton);
        out.put_u32(self.u1);
        out.put_u8(self.b1);
        out.put_ref(&self.network);
        out.put_u32(self.u2);
        out.put_u32(self.u3);
        out.put_u32(self.route.len() as u32);
        for s in &self.route {
            out.put_u8(s[0]);
            out.put_u8(s[1]);
            out.put_u8(s[2]);
        }
        out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<TrafficRecord> {
        if inp.get_u16()? != TRAFFIC_VERSION {
            return None;
        }
        let base = AutomatonBase::read(inp)?;
        let automaton = get_ref_checked(inp, IID_AUTOMATON)?;
        let u1 = inp.get_u32()?;
        let b1 = inp.get_u8()?;
        let network = get_ref_checked(inp, IID_NETWORK_OCCUPANT)?;
        let u2 = inp.get_u32()?;
        let u3 = inp.get_u32()?;
        let n = inp.get_u32()?;
        let mut route = Vec::new();
        for _ in 0..n {
            route.push([inp.get_u8()?, inp.get_u8()?, inp.get_u8()?]);
        }
        Some(TrafficRecord {
            base,
            automaton,
            u1,
            b1,
            network,
            u2,
            u3,
            route,
        })
    }
}

// ---- the concrete classes (§6.4) ------------------------------------------------------------------------------

/// Pedestrian (`0x896E75AF`): `u16` 5, traffic record, `bool`, `vec3`, `f32`, `u32`.
#[derive(Clone, Debug, PartialEq)]
pub struct PedestrianRecord {
    /// The version as read (the spec gives no read check; written back
    /// unchanged *[choice]*).
    pub version: u16,
    pub traffic: TrafficRecord,
    pub flag: bool,
    pub v: Vec3,
    pub f: f32,
    pub u: u32,
}

/// Vehicle (`0x896E714A`): `u16` 2, traffic record, path entries, `u32` 0,
/// `vec3`, the constant `vec3`, `f32`.
#[derive(Clone, Debug, PartialEq)]
pub struct VehicleRecord {
    pub version: u16,
    pub traffic: TrafficRecord,
    /// The path entries (`u32`, `s32`, `s32`). The original's Read discards
    /// them; they are kept as read so a re-save is byte-identical *[choice]*
    /// (a new vehicle writes an empty list, which §6.4 allows).
    pub path: Vec<(u32, i32, i32)>,
    /// The `u32` written as 0; kept as read *[choice]*.
    pub zero: u32,
    pub v: Vec3,
    /// The constant vector (−1, −2, −3); the original's Read discards it. Kept
    /// as read *[choice]*.
    pub constant: Vec3,
    pub f: f32,
}

/// Emergency vehicle (class `0x896E714A`, §12 quirk 3): `u16` 2, vehicle
/// record, `vec3`, `ref` occupant, `u8`, `u16`, `u32`. Never saved by the
/// managers *[choice]*.
#[derive(Clone, Debug)]
pub struct EmergencyRecord {
    pub version: u16,
    pub vehicle: VehicleRecord,
    pub v: Vec3,
    pub occupant: RefValue,
    pub b: u8,
    pub w: u16,
    pub u: u32,
}

impl PartialEq for EmergencyRecord {
    fn eq(&self, o: &Self) -> bool {
        self.version == o.version
            && self.vehicle == o.vehicle
            && self.v == o.v
            && refs_equal(&self.occupant, &o.occupant)
            && self.b == o.b
            && self.w == o.w
            && self.u == o.u
    }
}

/// Patrol car (`0x4A222B98`): `u16` 2, emergency record, `ref` automaton.
#[derive(Clone, Debug)]
pub struct PatrolCarRecord {
    pub version: u16,
    pub emergency: EmergencyRecord,
    pub target: RefValue,
}

impl PartialEq for PatrolCarRecord {
    fn eq(&self, o: &Self) -> bool {
        self.version == o.version && self.emergency == o.emergency && refs_equal(&self.target, &o.target)
    }
}

/// Trailer vehicle (`0x4A5B69ED`): `u16` 2, vehicle record, `u32` n + n ×
/// `ref` vehicle, `u32` m + m × `u32`, `vec3`, `ref` network occupant, `u32`.
#[derive(Clone, Debug)]
pub struct TrailerRecord {
    pub version: u16,
    pub vehicle: VehicleRecord,
    /// Read with the automaton interface `0xA9B40F05` *[choice]* (§6.4 names
    /// no IID, AM10).
    pub trailers: Vec<RefValue>,
    pub values: Vec<u32>,
    pub v: Vec3,
    pub network: RefValue,
    pub u: u32,
}

impl PartialEq for TrailerRecord {
    fn eq(&self, o: &Self) -> bool {
        self.version == o.version
            && self.vehicle == o.vehicle
            && ref_lists_equal(&self.trailers, &o.trailers)
            && self.values == o.values
            && self.v == o.v
            && refs_equal(&self.network, &o.network)
            && self.u == o.u
    }
}

impl PedestrianRecord {
    pub fn new() -> PedestrianRecord {
        PedestrianRecord {
            version: PEDESTRIAN_VERSION,
            traffic: TrafficRecord::default(),
            flag: false,
            v: [0.0; 3],
            f: 0.0,
            u: 0,
        }
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(self.version);
        if !self.traffic.write(out) {
            return false;
        }
        out.put_bool(self.flag);
        put_vec3(out, &self.v);
        out.put_f32(self.f);
        out.put_u32(self.u);
        out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<PedestrianRecord> {
        Some(PedestrianRecord {
            version: inp.get_u16()?,
            traffic: TrafficRecord::read(inp)?,
            flag: inp.get_bool()?,
            v: get_vec3(inp)?,
            f: inp.get_f32()?,
            u: inp.get_u32()?,
        })
    }
}

impl Default for PedestrianRecord {
    fn default() -> Self {
        PedestrianRecord::new()
    }
}

impl VehicleRecord {
    pub fn new() -> VehicleRecord {
        VehicleRecord {
            version: VEHICLE_VERSION,
            traffic: TrafficRecord::default(),
            path: Vec::new(),
            zero: 0,
            v: [0.0; 3],
            constant: VEHICLE_CONSTANT,
            f: 0.0,
        }
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(self.version);
        if !self.traffic.write(out) {
            return false;
        }
        out.put_u32(self.path.len() as u32);
        for &(a, b, c) in &self.path {
            out.put_u32(a);
            out.put_i32(b);
            out.put_i32(c);
        }
        out.put_u32(self.zero);
        put_vec3(out, &self.v);
        put_vec3(out, &self.constant);
        out.put_f32(self.f);
        out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<VehicleRecord> {
        let version = inp.get_u16()?;
        if version != 2 {
            return None;
        }
        let traffic = TrafficRecord::read(inp)?;
        let n = inp.get_u32()?;
        let mut path = Vec::new();
        for _ in 0..n {
            path.push((inp.get_u32()?, inp.get_i32()?, inp.get_i32()?));
        }
        Some(VehicleRecord {
            version,
            traffic,
            path,
            zero: inp.get_u32()?,
            v: get_vec3(inp)?,
            constant: get_vec3(inp)?,
            f: inp.get_f32()?,
        })
    }
}

impl Default for VehicleRecord {
    fn default() -> Self {
        VehicleRecord::new()
    }
}

impl EmergencyRecord {
    pub fn new() -> EmergencyRecord {
        EmergencyRecord {
            version: EMERGENCY_VERSION,
            vehicle: VehicleRecord::new(),
            v: [0.0; 3],
            occupant: RefValue::Null,
            b: 0,
            w: 0,
            u: 0,
        }
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(self.version);
        if !self.vehicle.write(out) {
            return false;
        }
        put_vec3(out, &self.v);
        out.put_ref(&self.occupant);
        out.put_u8(self.b);
        out.put_u16(self.w);
        out.put_u32(self.u);
        out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<EmergencyRecord> {
        Some(EmergencyRecord {
            version: inp.get_u16()?,
            vehicle: VehicleRecord::read(inp)?,
            v: get_vec3(inp)?,
            occupant: get_ref_checked(inp, IID_OCCUPANT)?,
            b: inp.get_u8()?,
            w: inp.get_u16()?,
            u: inp.get_u32()?,
        })
    }
}

impl Default for EmergencyRecord {
    fn default() -> Self {
        EmergencyRecord::new()
    }
}

impl PatrolCarRecord {
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(self.version);
        if !self.emergency.write(out) {
            return false;
        }
        out.put_ref(&self.target);
        out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<PatrolCarRecord> {
        Some(PatrolCarRecord {
            version: inp.get_u16()?,
            emergency: EmergencyRecord::read(inp)?,
            target: get_ref_checked(inp, IID_AUTOMATON)?,
        })
    }
}

impl TrailerRecord {
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(self.version);
        if !self.vehicle.write(out) {
            return false;
        }
        put_refs(out, &self.trailers);
        out.put_u32(self.values.len() as u32);
        for v in &self.values {
            out.put_u32(*v);
        }
        put_vec3(out, &self.v);
        out.put_ref(&self.network);
        out.put_u32(self.u);
        out.error() == 0
    }

    pub fn read(inp: &mut dyn InStream) -> Option<TrailerRecord> {
        let version = inp.get_u16()?;
        let vehicle = VehicleRecord::read(inp)?;
        let n = inp.get_u32()? as usize;
        let trailers = get_n_refs(inp, n, IID_AUTOMATON)?;
        let m = inp.get_u32()?;
        let mut values = Vec::new();
        for _ in 0..m {
            values.push(inp.get_u32()?);
        }
        Some(TrailerRecord {
            version,
            vehicle,
            trailers,
            values,
            v: get_vec3(inp)?,
            network: get_ref_checked(inp, IID_NETWORK_OCCUPANT)?,
            u: inp.get_u32()?,
        })
    }
}

/// A whole automaton record, by class.
#[derive(Clone, Debug, PartialEq)]
pub enum AutomatonRecord {
    Pedestrian(PedestrianRecord),
    Vehicle(VehicleRecord),
    Emergency(EmergencyRecord),
    PatrolCar(PatrolCarRecord),
    Trailer(TrailerRecord),
}

impl AutomatonRecord {
    /// The record a loader builds for `class` (§6.4). Class `0x896E714A` is a
    /// plain vehicle (§12 quirk 3); aircraft and watercraft automata are not
    /// known (Q4) and give `None`.
    pub fn for_class(class: u32) -> Option<AutomatonRecord> {
        Some(match class {
            CLSID_PEDESTRIAN => AutomatonRecord::Pedestrian(PedestrianRecord::new()),
            CLSID_VEHICLE => AutomatonRecord::Vehicle(VehicleRecord::new()),
            CLSID_PATROL_CAR => AutomatonRecord::PatrolCar(PatrolCarRecord {
                version: PATROL_CAR_VERSION,
                emergency: EmergencyRecord::new(),
                target: RefValue::Null,
            }),
            CLSID_TRAILER => AutomatonRecord::Trailer(TrailerRecord {
                version: TRAILER_VERSION,
                vehicle: VehicleRecord::new(),
                trailers: Vec::new(),
                values: Vec::new(),
                v: [0.0; 3],
                network: RefValue::Null,
                u: 0,
            }),
            _ => return None,
        })
    }

    /// `GetGZCLSID`: an emergency vehicle answers the vehicle's class (§12
    /// quirk 3).
    pub fn class_id(&self) -> u32 {
        match self {
            AutomatonRecord::Pedestrian(_) => CLSID_PEDESTRIAN,
            AutomatonRecord::Vehicle(_) | AutomatonRecord::Emergency(_) => CLSID_VEHICLE,
            AutomatonRecord::PatrolCar(_) => CLSID_PATROL_CAR,
            AutomatonRecord::Trailer(_) => CLSID_TRAILER,
        }
    }

    pub fn traffic(&self) -> &TrafficRecord {
        match self {
            AutomatonRecord::Pedestrian(p) => &p.traffic,
            AutomatonRecord::Vehicle(v) => &v.traffic,
            AutomatonRecord::Emergency(e) => &e.vehicle.traffic,
            AutomatonRecord::PatrolCar(p) => &p.emergency.vehicle.traffic,
            AutomatonRecord::Trailer(t) => &t.vehicle.traffic,
        }
    }

    pub fn traffic_mut(&mut self) -> &mut TrafficRecord {
        match self {
            AutomatonRecord::Pedestrian(p) => &mut p.traffic,
            AutomatonRecord::Vehicle(v) => &mut v.traffic,
            AutomatonRecord::Emergency(e) => &mut e.vehicle.traffic,
            AutomatonRecord::PatrolCar(p) => &mut p.emergency.vehicle.traffic,
            AutomatonRecord::Trailer(t) => &mut t.vehicle.traffic,
        }
    }

    pub fn base(&self) -> &AutomatonBase {
        &self.traffic().base
    }

    pub fn base_mut(&mut self) -> &mut AutomatonBase {
        &mut self.traffic_mut().base
    }

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        match self {
            AutomatonRecord::Pedestrian(r) => r.write(out),
            AutomatonRecord::Vehicle(r) => r.write(out),
            AutomatonRecord::Emergency(r) => r.write(out),
            AutomatonRecord::PatrolCar(r) => r.write(out),
            AutomatonRecord::Trailer(r) => r.write(out),
        }
    }

    /// Reads a record of the same kind as `self`.
    pub fn read_as(&self, inp: &mut dyn InStream) -> Option<AutomatonRecord> {
        Some(match self {
            AutomatonRecord::Pedestrian(_) => AutomatonRecord::Pedestrian(PedestrianRecord::read(inp)?),
            AutomatonRecord::Vehicle(_) => AutomatonRecord::Vehicle(VehicleRecord::read(inp)?),
            AutomatonRecord::Emergency(_) => AutomatonRecord::Emergency(EmergencyRecord::read(inp)?),
            AutomatonRecord::PatrolCar(_) => AutomatonRecord::PatrolCar(PatrolCarRecord::read(inp)?),
            AutomatonRecord::Trailer(_) => AutomatonRecord::Trailer(TrailerRecord::read(inp)?),
        })
    }
}

/// The arguments of automaton slot 51 `SetState(type, …6 more)` after the
/// type (§7.4, §6.2; Q5 answered, re-integrate1). They are
/// `PushState`'s own: `SetState` = pop everything then `PushState`.
///
/// ```text
/// PushState(u32 type, cISC4AutomataController* controller, float timeout_ms,
///           bool final, ulong animId, ulong sfxId, tBehaviorTemplate const* template)
/// ```
///
/// The controller and the template travel as host tokens (0 = null).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetStateArgs {
    /// The controller (`+0x08`); 0 = null.
    pub controller: u32,
    /// The countdown in milliseconds (`+0x10`); ≤ 0 never expires.
    pub timeout_ms: f32,
    /// `final` (`+0x14`): `SetState` refuses to run while the top state has it.
    pub final_flag: u32,
    /// The animation id (`+0x1C`, `PushState` param 5).
    pub anim_id: u32,
    /// The SFX id (`+0x18`, `PushState` param 6).
    pub sfx_id: u32,
    /// The behaviour template (`+0x24`, not saved); 0 = null.
    pub template: u32,
}

impl Default for SetStateArgs {
    fn default() -> SetStateArgs {
        SetStateArgs {
            controller: 0,
            timeout_ms: -1.0,
            final_flag: 0,
            anim_id: 0,
            sfx_id: 0,
            template: 0,
        }
    }
}

impl SetStateArgs {
    /// `final` as the state base's bool.
    pub fn is_final(&self) -> bool {
        self.final_flag != 0
    }

    /// The state these arguments create of `state_type` (§6.2: `PushState`
    /// writes every one of the base's fields). `controller` is the reference
    /// the `controller` argument denotes — the C ABI carries that argument as
    /// a truncated `u32` host token with no class, so a caller that has the
    /// reference passes it here. Every documented call (§8.1) passes 0, and
    /// the state's controller is then null.
    pub fn to_state(&self, state_type: u32, controller: RefValue) -> State {
        let mut s = State::new(state_type);
        s.base.controller = controller;
        s.base.countdown_ms = self.timeout_ms;
        s.base.is_final = self.is_final();
        s.base.sfx_id = self.sfx_id;
        s.base.anim_id = self.anim_id;
        s
    }
}

/// An automaton object (`cISC4Automaton`). Its saved state is an
/// [`AutomatonRecord`]; the slots other specs use (§7.4) act on it. Movement,
/// animation and sound are stubbed.
pub struct Automaton {
    rec: RefCell<AutomatonRecord>,
    removed: Cell<bool>,
    returning: Cell<bool>,
    last_set_state: RefCell<Option<(u32, SetStateArgs)>>,
    group: Cell<u32>,
    token: Cell<u64>,
}

impl Automaton {
    /// A new automaton of `class` (pedestrian, vehicle, patrol car, trailer);
    /// `None` for any other class.
    pub fn for_class(class: u32) -> Option<Rc<Automaton>> {
        AutomatonRecord::for_class(class).map(Automaton::from_record)
    }

    /// A new emergency vehicle (it answers the vehicle's class).
    pub fn new_emergency() -> Rc<Automaton> {
        Automaton::from_record(AutomatonRecord::Emergency(EmergencyRecord::new()))
    }

    pub fn from_record(rec: AutomatonRecord) -> Rc<Automaton> {
        Rc::new(Automaton {
            rec: RefCell::new(rec),
            removed: Cell::new(false),
            returning: Cell::new(false),
            last_set_state: RefCell::new(None),
            group: Cell::new(0),
            token: Cell::new(0),
        })
    }

    pub fn record(&self) -> AutomatonRecord {
        self.rec.borrow().clone()
    }

    pub fn set_record(&self, rec: AutomatonRecord) {
        *self.rec.borrow_mut() = rec;
    }

    pub fn with_record<R>(&self, f: impl FnOnce(&mut AutomatonRecord) -> R) -> R {
        f(&mut self.rec.borrow_mut())
    }

    /// An emergency vehicle (never saved by the managers, §12 quirk 3).
    pub fn is_emergency(&self) -> bool {
        matches!(*self.rec.borrow(), AutomatonRecord::Emergency(_))
    }

    /// The automata group it was created for (not saved; 0 = unknown).
    pub fn group(&self) -> u32 {
        self.group.get()
    }

    pub fn set_group(&self, group: u32) {
        self.group.set(group);
    }

    /// Slot 7 `GetPosition`: the first vector of the base record *[choice]*
    /// (the spec does not name the fields of §6.1 field 2).
    pub fn position(&self) -> Vec3 {
        self.rec.borrow().base().vectors[0]
    }

    /// Slot 8 `SetPosition` (see [`Automaton::position`]).
    pub fn set_position(&self, p: Vec3) {
        self.rec.borrow_mut().base_mut().vectors[0] = p;
    }

    /// Slot 9 `SetVisible`: the occupant's visible flag (bit 2).
    pub fn set_visible(&self, visible: bool) {
        let mut r = self.rec.borrow_mut();
        let f = &mut r.base_mut().occupant.flags;
        if visible {
            *f |= OCCUPANT_FLAG_VISIBLE;
        } else {
            *f &= !OCCUPANT_FLAG_VISIBLE;
        }
    }

    pub fn is_visible(&self) -> bool {
        self.rec.borrow().base().occupant.flags & OCCUPANT_FLAG_VISIBLE != 0
    }

    /// Slot 13 `Remove` (dispatch despawn): marked; the manager drops it.
    pub fn remove(&self) {
        self.removed.set(true);
    }

    pub fn is_removed(&self) -> bool {
        self.removed.get()
    }

    /// Slot 29 `ReturnToSourceOccupant` (dispatch "go home"): marked.
    pub fn return_to_source_occupant(&self) {
        self.returning.set(true);
    }

    pub fn is_returning(&self) -> bool {
        self.returning.get()
    }

    /// Slot 51 `SetState(type, …)`: pop everything, then push one state of
    /// `state_type` (§6.2). **Returns false and changes nothing while the top
    /// state has `final`** (§12 quirk 11, `0x00509860`) — that is why the riot
    /// end's `SetState(1, …)` is a no-op against its leader (§8.1).
    pub fn set_state(&self, state_type: u32, args: SetStateArgs) -> bool {
        let mut r = self.rec.borrow_mut();
        if !r.base_mut().set_state(state_type, args) {
            return false;
        }
        *self.last_set_state.borrow_mut() = Some((state_type, args));
        true
    }

    /// Slot 52 `PushState` (§6.2). `PushState(0, 0, …)` delegates to
    /// [`Automaton::set_state`] (Mac `0x000bf7d0`).
    pub fn push_state(&self, state_type: u32, args: SetStateArgs) -> bool {
        if state_type == state_type::DEFAULT && args.controller == 0 {
            return self.set_state(state_type, args);
        }
        self.rec.borrow_mut().base_mut().push_state(state_type, args)
    }

    /// Slot 53 `PopState`: false when the stack is empty. **A `final` state
    /// that is not already type 1 does not reveal the state below — it pushes
    /// DISAPPEAR (type 1) instead** (§12 quirk 11, `0x0050B660`).
    pub fn pop_state(&self) -> bool {
        self.rec.borrow_mut().base_mut().pop_state()
    }

    /// Slot 56 `GetState`: the type of the top state.
    pub fn current_state(&self) -> Option<u32> {
        self.rec.borrow().base().states.last().map(|s| s.state_type)
    }

    pub fn last_set_state(&self) -> Option<(u32, SetStateArgs)> {
        *self.last_set_state.borrow()
    }

    /// Slots 48 / 49 `GetLeader` / `SetLeader`.
    pub fn leader(&self) -> RefValue {
        self.rec.borrow().base().leader.clone()
    }

    pub fn set_leader(&self, leader: RefValue) {
        self.rec.borrow_mut().base_mut().leader = leader;
    }

    /// The controller (§6.1 field 5).
    pub fn controller(&self) -> RefValue {
        self.rec.borrow().base().controller.clone()
    }

    pub fn set_controller(&self, c: RefValue) {
        self.rec.borrow_mut().base_mut().controller = c;
    }

    pub fn set_host_token(&self, token: u64) {
        self.token.set(token);
    }

    /// `QueryInterface`: the automaton, the occupant (slot 5 `AsOccupant`),
    /// `cIGZSerializable` and `cIGZUnknown`.
    pub fn answers_iid(&self, iid: u32) -> bool {
        matches!(iid, IID_UNKNOWN | IID_AUTOMATON | IID_OCCUPANT | IID_SERIALIZABLE)
    }
}

impl SaveObject for Automaton {
    fn class_id(&self) -> u32 {
        self.rec.borrow().class_id()
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        self.rec.borrow().write(out)
    }

    /// Reads a record of this automaton's kind; on failure nothing changes
    /// *[choice]*. A read occupant is "not yet initialised, loaded".
    fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.error() != 0 {
            return false;
        }
        let parsed = self.rec.borrow().read_as(inp);
        match parsed {
            Some(r) => {
                *self.rec.borrow_mut() = r;
                inp.error() == 0
            }
            None => false,
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn std::any::Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
