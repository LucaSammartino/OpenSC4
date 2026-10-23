// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable TextDecal owner, `text-decal-occupant.md` §§2–7.
//!
//! No native layout or binding is implied. `prop` and `base` are the existing
//! production prop/spatial state, not parallel TextDecal copies. Foreign calls
//! below are individual primitives. Callbacks may inspect state but mutation,
//! reentry, foreign exceptions and allocation failure are outside this domain.
//!
//! TDO1 is OPEN: callers must explicitly select Nearest24Normal. Each operation
//! rounds separately; q remains live across the size store. Dimensions are
//! restricted to 1..=2^24 (exact unsigned loads); inputs/intermediates/stores
//! must be normal finite or zero. No ambient x87 or arbitrary-bit claim.

use crate::automata::OccupantBase;
use crate::prop_occupant::PropOccupant;
use crate::prop_occupant::{read_prop_record, write_prop_record, PropRecord, ReadOutcome};
use crate::stream::{InStream, OutStream};
use std::ops::{Deref, DerefMut};

pub const ABSENT: u32 = u32::MAX;
pub const MESSAGES: [u32; 3] = [0xA6B7_9602, 0x06CD_B65B, 0x26D3_1EC1];
pub const SOURCE_CLASS: u32 = 0xCC1F_D62C;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arithmetic {
    Nearest24Normal,
    /// Other caller precision/control environments have not been implemented.
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    ArithmeticPolicy,
    NumericDomain,
    TableAccess,
    MissingManager,
    MissingBorrowedFilter,
    TruncatedRecord,
}
pub type Result<T = ()> = std::result::Result<T, Unsupported>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    AddRef(Handle),
    Release(Handle),
    Remove(Handle, u32),
    Color(Handle, u32, [f32; 3]),
    Alpha(Handle, u32, f32),
    Position(Handle, u32, [f32; 2]),
    NotifyMove,
    SetPropType(u32),
    Subscribe(u32),
    Unsubscribe(u32),
    DestroyText,
    DestroyBase,
}

/// Raw foreign primitives only. Base calls operate on the existing prop/base
/// state. In particular base_shutdown must clear its initialized state.
pub trait Host {
    fn effect(&mut self, state: &State, effect: Effect);
    /// Factory output transfers one source reference; no owner AddRef.
    fn create_source(&mut self, state: &State, class: u32) -> Option<Handle>;
    fn dimensions(&mut self, state: &State, source: Handle) -> (u32, u32);
    fn altitude(&mut self, state: &State, center: [f32; 2]) -> f64;
    fn level(&mut self, state: &State) -> u32;
    /// Includes obtaining a fresh terrain view for each lookup (§4).
    fn manager(&mut self, state: &State, index: u32) -> Option<Handle>;
    fn add(&mut self, state: &State, manager: Handle, source: Handle, center: [f32; 2], size: f32, turns: f32) -> u32;
    fn terrain_position(&mut self, state: &State, center: [f32; 2]);
    fn initialized(&mut self, state: &State) -> bool;
    fn base_init(&mut self, base: &mut OccupantBase, prop: &PropOccupant);
    fn base_shutdown(&mut self, base: &mut OccupantBase, prop: &PropOccupant);
    fn base_flags(&mut self, base: &mut OccupantBase, prop: &PropOccupant, flags: u32);
    fn base_position(&mut self, prop: &PropOccupant, position: [f32; 3]);
    fn base_message(&mut self, state: &State, message: u32) -> u32;
    fn temporary_filter(&mut self, state: &State) -> Option<Handle>;
    fn filter_accepts(&mut self, state: &State, filter: Handle) -> bool;
    fn city_config(&mut self, state: &State) -> bool;
}

/// One owned state. Construct it through Owner so Drop retains foreign releases.
pub struct State {
    // Construction stays under Owner so ordinary Drop cannot lose releases.
    _owned: (),
    pub prop: PropOccupant,
    pub base: OccupantBase,
    pub text: Vec<u8>,
    pub height: f32,
    pub ids: [u32; 2],
    pub dynamic: u8,
    pub size: f32,
    pub center: [f32; 2],
    pub rgb: [f32; 3],
    pub alpha: f32,
    pub orientation: u32,
    pub source: Option<Handle>,
    pub managers: [Option<Handle>; 2],
    pub arithmetic: Arithmetic,
}

/// Owns the callback context until destruction. Explicit Shutdown is required
/// before dropping an initialized subscription owner. Drop performs destruction
/// (B, A, source, text, base), never an invented implicit Shutdown.
pub struct Owner<H: Host> {
    state: State,
    pub host: H,
}
impl<H: Host> Owner<H> {
    pub fn new(host: H, prop: PropOccupant, base: OccupantBase, arithmetic: Arithmetic) -> Self {
        prop.set_bounding_box(prop.bounding_box().0, [f32::from_bits(0x3DCC_CCCD); 3]);
        Self {
            host,
            state: State {
                prop,
                base,
                arithmetic,
                text: Vec::new(),
                height: 1.0,
                ids: [ABSENT; 2],
                dynamic: 0,
                size: 100.0,
                center: [0.0; 2],
                rgb: [255.0; 3],
                alpha: 1.0,
                orientation: 1,
                source: None,
                managers: [None; 2],
                _owned: (),
            },
        }
    }
    /// Split borrows allow production methods and inspecting primitive callbacks.
    pub fn parts(&mut self) -> (&mut State, &mut H) {
        (&mut self.state, &mut self.host)
    }
}
impl<H: Host> Deref for Owner<H> {
    type Target = State;
    fn deref(&self) -> &State {
        &self.state
    }
}
impl<H: Host> DerefMut for Owner<H> {
    fn deref_mut(&mut self) -> &mut State {
        &mut self.state
    }
}
impl<H: Host> Drop for Owner<H> {
    fn drop(&mut self) {
        for handle in [self.state.managers[1], self.state.managers[0], self.state.source]
            .into_iter()
            .flatten()
        {
            self.host.effect(&self.state, Effect::Release(handle));
        }
        self.host.effect(&self.state, Effect::DestroyText);
        drop(std::mem::take(&mut self.state.text));
        self.host.effect(&self.state, Effect::DestroyBase);
    }
}

fn normal(x: f64) -> Result<f64> {
    if x == 0.0 || (x.is_finite() && (f64::from(f32::MIN_POSITIVE)..=f64::from(f32::MAX)).contains(&x.abs())) {
        Ok(x)
    } else {
        Err(Unsupported::NumericDomain)
    }
}
fn rounded(x: f64) -> Result<f64> {
    normal(x)?;
    normal(crate::x87::r24(x))
}

impl State {
    /// §6: shared base runs before all owner consequences.
    pub fn set_flags(&mut self, host: &mut impl Host, mut flags: u32) -> Result {
        let old = u32::from(self.base.flags);
        if (old ^ flags) & 2 != 0 && flags & 2 != 0 && !host.city_config(self) {
            flags &= !2;
        }
        let changed = (old ^ flags) & 2 != 0;
        host.base_flags(&mut self.base, &self.prop, flags);
        if !changed {
            return Ok(());
        }
        if flags & 2 == 0 {
            return self.remove(host);
        }
        if self.managers.iter().all(Option::is_some) {
            self.rebuild(host)?;
        } else {
            self.route(host)?;
        }
        if self.source.is_none() && !self.text.is_empty() {
            self.replace_source(host)?;
        }
        Ok(())
    }

    fn temporary_filter(&mut self, host: &mut impl Host) -> Result {
        if let Some(filter) = host.temporary_filter(self) {
            if host.filter_accepts(self, filter) && host.city_config(self) {
                self.set_flags(host, u32::from(self.base.flags) | 2)?;
            }
            host.effect(self, Effect::Release(filter));
        }
        Ok(())
    }
    pub fn init(&mut self, host: &mut impl Host) -> Result<bool> {
        if host.initialized(self) {
            return Ok(true);
        }
        host.effect(self, Effect::SetPropType(0xCB79_919B));
        self.prop.set_prop_type(0xCB79_919B);
        host.base_init(&mut self.base, &self.prop);
        if self.base.status & 2 == 0 {
            self.route(host)?;
            self.temporary_filter(host)?;
        }
        for message in MESSAGES {
            host.effect(self, Effect::Subscribe(message));
        }
        Ok(true)
    }
    pub fn message(&mut self, host: &mut impl Host, message: u32, borrowed_filter: Option<Handle>) -> Result<u32> {
        match message {
            0xA6B7_9602 => self.rebuild(host)?,
            0x26D3_1EC1 => {
                self.route(host)?;
                self.temporary_filter(host)?;
            }
            0x06CD_B65B => {
                let filter = borrowed_filter.ok_or(Unsupported::MissingBorrowedFilter)?;
                let enabled = host.filter_accepts(self, filter) && host.city_config(self);
                if enabled != (self.base.flags & 2 != 0) {
                    let flags = if enabled {
                        self.base.flags | 2
                    } else {
                        self.base.flags & !2
                    };
                    self.set_flags(host, u32::from(flags))?;
                }
            }
            _ => {}
        }
        Ok(host.base_message(self, message))
    }
    pub fn shutdown(&mut self, host: &mut impl Host) -> Result<bool> {
        if !host.initialized(self) {
            return Ok(true);
        }
        for message in MESSAGES {
            host.effect(self, Effect::Unsubscribe(message));
        }
        for i in 0..2 {
            if let Some(manager) = self.managers[i] {
                if self.ids[i] != ABSENT {
                    host.effect(self, Effect::Remove(manager, self.ids[i]));
                    self.ids[i] = ABSENT;
                }
                self.managers[i] = None;
                host.effect(self, Effect::Release(manager));
            }
        }
        if let Some(source) = self.source.take() {
            host.effect(self, Effect::Release(source));
        }
        host.base_shutdown(&mut self.base, &self.prop);
        Ok(true)
    }
    pub fn remove(&mut self, host: &mut impl Host) -> Result {
        for i in 0..2 {
            if self.ids[i] != ABSENT {
                let manager = self.managers[i].ok_or(Unsupported::MissingManager)?;
                host.effect(self, Effect::Remove(manager, self.ids[i]));
                self.ids[i] = ABSENT;
            }
        }
        Ok(())
    }

    pub fn replace_source(&mut self, host: &mut impl Host) -> Result {
        if let Some(old) = self.source.take() {
            host.effect(self, Effect::Release(old));
        }
        self.source = host.create_source(self, SOURCE_CLASS);
        if self.source.is_some() {
            self.rebuild(host)?;
        }
        Ok(())
    }
    pub fn set_text(&mut self, host: &mut impl Host, text: &[u8]) -> Result {
        self.text = text.to_vec();
        self.replace_source(host)
    }
    /// §3: copies the string only; does not bind or acquire the texture source.
    pub fn get_text(&self, output: &mut Vec<u8>) {
        output.clone_from(&self.text);
    }
    pub fn route(&mut self, host: &mut impl Host) -> Result {
        self.remove(host)?;
        let offset = if self.dynamic != 0 { 2 } else { 0 };
        for i in [1, 0] {
            let new = host.manager(self, offset + i as u32);
            let old = self.managers[i];
            if new != old {
                if let Some(p) = new {
                    host.effect(self, Effect::AddRef(p));
                }
                self.managers[i] = new;
                if let Some(p) = old {
                    host.effect(self, Effect::Release(p));
                }
            }
        }
        self.rebuild(host)
    }
    pub fn rebuild(&mut self, host: &mut impl Host) -> Result {
        self.remove(host)?;
        let Some(source) = self.source else {
            return Ok(());
        };
        if self.base.flags & 2 == 0 {
            return Ok(());
        }
        if self.arithmetic != Arithmetic::Nearest24Normal {
            return Err(Unsupported::ArithmeticPolicy);
        }
        let (w, h) = host.dimensions(self, source);
        if w == 0 || h == 0 || w > 1 << 24 || h > 1 << 24 {
            return Err(Unsupported::NumericDomain);
        }
        let height = normal(f64::from(self.height))?;
        // Exact integer operands, correctly rounded division, then two distinct
        // binary32-significand operations. q is retained, never reloaded from size.
        let q = rounded(rounded(rounded(f64::from(w) / f64::from(h))? * height)? * 16.0)?;
        self.size = q as f32;
        let mut hx = rounded(q * 0.5)? as f32;
        let mut hz = rounded(height * 8.0)? as f32;
        let altitude = rounded(host.altitude(self, self.center))?;
        if self.orientation == 0 {
            std::mem::swap(&mut hx, &mut hz);
        }
        let level = host.level(self);
        const TURNS: [[f32; 4]; 4] = [
            [0.25, 0.0, 0.125, 0.875],
            [0.75, 0.0, 0.625, 0.875],
            [0.75, 0.5, 0.625, 0.375],
            [0.25, 0.5, 0.125, 0.375],
        ];
        let turns = *TURNS
            .get(level as usize)
            .and_then(|r| r.get(self.orientation as usize))
            .ok_or(Unsupported::TableAccess)?;
        let x = normal(f64::from(self.center[0]))?;
        let z = normal(f64::from(self.center[1]))?;
        let m = f64::from(f32::from_bits(0x3DCC_CCCD));
        let min = [
            rounded(x - f64::from(hx))? as f32,
            rounded(altitude - m)? as f32,
            rounded(z - f64::from(hz))? as f32,
        ];
        let max = [
            rounded(f64::from(hx) + x)? as f32,
            rounded(altitude + m)? as f32,
            rounded(f64::from(hz) + z)? as f32,
        ];
        self.prop.set_bounding_box(min, max);
        for i in 0..2 {
            let manager = self.managers[i].ok_or(Unsupported::MissingManager)?;
            self.ids[i] = host.add(self, manager, source, self.center, self.size, turns);
            if self.ids[i] != ABSENT {
                host.effect(self, Effect::Color(manager, self.ids[i], self.rgb));
                host.effect(self, Effect::Alpha(manager, self.ids[i], self.alpha));
            }
        }
        host.effect(self, Effect::NotifyMove);
        Ok(())
    }
    fn live(&self, host: &mut impl Host, alpha: bool) -> Result {
        for i in 0..2 {
            if self.ids[i] != ABSENT {
                let m = self.managers[i].ok_or(Unsupported::MissingManager)?;
                host.effect(
                    self,
                    if alpha {
                        Effect::Alpha(m, self.ids[i], self.alpha)
                    } else {
                        Effect::Color(m, self.ids[i], self.rgb)
                    },
                );
            }
        }
        Ok(())
    }
    pub fn set_height(&mut self, host: &mut impl Host, value: f32) -> Result {
        if value > 0.0 && value < 8.0 {
            self.height = value;
            self.rebuild(host)?;
        }
        Ok(())
    }
    pub fn set_color(&mut self, host: &mut impl Host, value: [f32; 3]) -> Result {
        // Masked-exception comparison domain, nearest24 arithmetic. NaN and
        // infinity differences have the specified unordered/greater branches.
        if self.arithmetic != Arithmetic::Nearest24Normal {
            return Err(Unsupported::ArithmeticPolicy);
        }
        let epsilon = f32::from_bits(0x38D1_B717);
        if value.iter().zip(self.rgb).any(|(&a, b)| {
            let d = (a - b).abs();
            d.is_nan() || d >= epsilon
        }) {
            self.rgb = value;
            if self.dynamic != 0 {
                self.live(host, false)?;
            } else {
                self.rebuild(host)?;
            }
        }
        Ok(())
    }
    pub fn set_alpha(&mut self, host: &mut impl Host, value: f32) -> Result {
        if value != self.alpha {
            self.alpha = value;
            if self.dynamic != 0 {
                self.live(host, true)?;
            } else {
                self.rebuild(host)?;
            }
        }
        Ok(())
    }
    pub fn set_orientation(&mut self, host: &mut impl Host, value: u32) -> Result {
        if value != self.orientation {
            self.orientation = value;
            self.rebuild(host)?;
        }
        Ok(())
    }
    pub fn set_dynamic(&mut self, host: &mut impl Host, value: u8) -> Result {
        if value != self.dynamic {
            self.dynamic = value;
            self.route(host)?;
        }
        Ok(())
    }
    pub fn set_position(&mut self, host: &mut impl Host, value: [f32; 3]) -> Result {
        self.center = [value[0], value[2]];
        host.base_position(&self.prop, value);
        host.terrain_position(self, self.center);
        if self.dynamic == 0 {
            return self.rebuild(host);
        }
        self.move_decals(host)
    }
    /// §3 MoveDecal: static is quiet; dynamic moves A then B, no rebuild.
    pub fn move_decals(&self, host: &mut impl Host) -> Result {
        if self.dynamic != 0 {
            for i in 0..2 {
                if self.ids[i] != ABSENT {
                    let m = self.managers[i].ok_or(Unsupported::MissingManager)?;
                    host.effect(self, Effect::Position(m, self.ids[i], self.center));
                }
            }
        }
        Ok(())
    }
}

/// One typed raw base-record operation is substitutable for testing the
/// published base-false suffix skip. The default adapter uses production codecs.
pub trait BaseCodec {
    fn write(&mut self, base: &OccupantBase, prop: &PropOccupant, out: &mut dyn OutStream) -> bool;
    fn read(&mut self, base: &mut OccupantBase, prop: &PropOccupant, inp: &mut dyn InStream) -> Result<bool>;
}
pub struct PropCodec;
impl BaseCodec for PropCodec {
    fn write(&mut self, base: &OccupantBase, prop: &PropOccupant, out: &mut dyn OutStream) -> bool {
        write_prop_record(&PropRecord::of(prop, base.clone()), out)
    }
    fn read(&mut self, base: &mut OccupantBase, prop: &PropOccupant, inp: &mut dyn InStream) -> Result<bool> {
        match read_prop_record(inp) {
            ReadOutcome::Ok(record) => {
                record.apply(prop);
                *base = record.base;
                Ok(true)
            }
            // Preserve the existing base codec's reported success, not a new
            // conjunction or repaired generic-stream/base result.
            ReadOutcome::BaseFailed => Ok(true),
            ReadOutcome::Truncated => Err(Unsupported::TruncatedRecord),
        }
    }
}
impl State {
    /// §7: complete valid records only. Individual primitive statuses do not
    /// short circuit Write. The final status examines AL, not all error bits.
    pub fn write(&self, codec: &mut impl BaseCodec, out: &mut dyn OutStream) -> bool {
        out.put_u16(3);
        if codec.write(&self.base, &self.prop, out) {
            out.put_str(&self.text);
            for v in self.center {
                out.put_f32(v);
            }
            out.put_f32(self.height);
            out.put_f32(self.size);
            for v in self.center {
                out.put_f32(v);
            }
            for v in self.rgb {
                out.put_f32(v);
            }
            out.put_f32(self.alpha);
            out.put_u8(self.orientation as u8);
        }
        out.error() as u8 == 0
    }
    /// No rollback or invented scratch values on truncation. Fields already
    /// read remain changed; the explicit Unsupported result is not false.
    pub fn read(&mut self, codec: &mut impl BaseCodec, inp: &mut dyn InStream) -> Result<bool> {
        let version = inp.get_u16().ok_or(Unsupported::TruncatedRecord)?;
        if codec.read(&mut self.base, &self.prop, inp)? {
            self.text = inp.get_str().ok_or(Unsupported::TruncatedRecord)?;
            if version < 3 {
                inp.get_u32().ok_or(Unsupported::TruncatedRecord)?;
            }
            for v in &mut self.center {
                *v = inp.get_f32().ok_or(Unsupported::TruncatedRecord)?;
            }
            self.height = if version == 1 {
                let value = inp.get_u32().ok_or(Unsupported::TruncatedRecord)?;
                if self.arithmetic != Arithmetic::Nearest24Normal {
                    return Err(Unsupported::ArithmeticPolicy);
                }
                crate::x87::u32_to_float(value as i32) as f32
            } else {
                inp.get_f32().ok_or(Unsupported::TruncatedRecord)?
            };
            self.size = inp.get_f32().ok_or(Unsupported::TruncatedRecord)?;
            for v in &mut self.center {
                *v = inp.get_f32().ok_or(Unsupported::TruncatedRecord)?;
            }
            for v in &mut self.rgb {
                *v = inp.get_f32().ok_or(Unsupported::TruncatedRecord)?;
            }
            self.alpha = inp.get_f32().ok_or(Unsupported::TruncatedRecord)?;
            self.orientation = u32::from(inp.get_u8().ok_or(Unsupported::TruncatedRecord)?);
        }
        Ok(inp.error() as u8 == 0)
    }
}

#[cfg(test)]
mod tests;
