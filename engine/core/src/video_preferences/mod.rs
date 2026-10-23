// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded portable owner from `docs/specs/video-preferences.md` §§2–7.
//!
//! This is not a native constructor, COM layout, widget implementation, or a
//! complete panel. Raw initial fields and pointer identities are explicit inputs.
//! Calls require valid, nonaliasing foreign storage and no foreign reentry. Rust
//! cleanup does not perform logical Release/free. Unknown slices fail explicitly.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Object(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Payload {
    pub storage: Object,
    pub offset: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Byte {
    F0c = 0x0c,
    F0d = 0x0d,
    F0e = 0x0e,
    F0f = 0x0f,
    F10 = 0x10,
    F11 = 0x11,
    F12 = 0x12,
    F13 = 0x13,
    F14 = 0x14,
    F15 = 0x15,
    F16 = 0x16,
    F18 = 0x18,
    F28 = 0x28,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dword {
    Width = 0x1c,
    Height = 0x20,
    Depth = 0x24,
}

/// Bytes relative to P+f00. Padding is retained, not normalized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preferences {
    pub identity: Object,
    raw: [u8; 0x29],
}

impl Preferences {
    pub fn from_raw(identity: Object, raw: [u8; 0x29]) -> Self {
        Self { identity, raw }
    }

    pub fn raw(&self) -> &[u8; 0x29] {
        &self.raw
    }

    pub fn byte(&self, field: Byte) -> u8 {
        self.raw[field as usize]
    }

    pub fn dword(&self, field: Dword) -> u32 {
        let i = field as usize;
        u32::from_le_bytes([self.raw[i], self.raw[i + 1], self.raw[i + 2], self.raw[i + 3]])
    }

    pub fn payload(&self) -> Payload {
        Payload {
            storage: self.identity,
            offset: 0xf0c,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Baselines {
    pub b10: u16,
    pub b12: u16,
    pub b14: u16,
    pub b16: u16,
    pub b18: u16,
    pub b1c: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawState {
    pub owner: Object,
    pub host: Option<Object>,
    pub preferences: Option<Preferences>,
    pub baselines: Baselines,
    pub marker: u8,
    pub first_redraw: u8,
    pub guard: u8,
    pub initialized: u8,
}

/// Required qualifications, not facts inferred from raw pointer integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Domain {
    pub valid_objects: bool,
    pub unaliased_storage: bool,
    pub no_foreign_reentry: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    InvalidObject,
    AliasedStorage,
    ForeignReentry,
    NullPreferences,
    NullHost,
    NullRequiredOutput,
    FailedLookupWithOutput,
    WrongChannel,
    MissingService,
    AllocationFailure,
    UnqualifiedEvent(u32),
    RemainingRedrawVp1,
    AutoRedrawVp1Vp4,
    ShowMiddleVp3,
    FreedOwner,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    U32(u32),
    Object(Option<Object>),
    Payload(Payload),
}

/// Individual foreign operations only. No host operation decides a branch,
/// computes owner output state, or implements a complete redraw/window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    Root(Object),
    Lookup { root: Object, id: u32, iid: u32 },
    Slot { object: Object, slot: u16, args: Vec<Arg> },
    Utility { root: Object, id: u32 },
    Global(Global),
    Framework,
    Service { framework: Object, id: u32, iid: u32 },
    WindowManager,
    Allocate(u32),
    ConstructMessage(Object),
    Pause([u32; 4]),
    Unpause(Object),
    FreeOwner(Object),
    CreateScript { key: [u32; 3], args: [u32; 3] },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Global {
    App,
    Notifications,
    ModalService,
    Preferences,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    Ignored,
    Al(u8),
    U32(u32),
    Object(Option<Object>),
    Lookup { al: u8, output: Option<Object> },
}

pub trait Host {
    /// Must not reenter the owner or alias its storage. Errors stop the portable
    /// slice; they are never native graceful-failure claims or rollback requests.
    fn call(&mut self, call: &Call) -> Result<Reply, Unsupported>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Call(Call),
    Byte(Byte, u8),
    Dword(Dword, u32),
    Marker(u8),
    BaselineDepth(u16),
    FirstRedraw(u8),
    Host(Option<Object>),
    Preferences(Option<Object>),
    Guard(u8),
    Initialized(u8),
    Count(u32),
    ResetPublishedVptr,
    Freed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndResult {
    GuardZeroAl,
    TailReleaseEax(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowPrefix {
    ReturnedAl(bool),
    /// Qualified stop before the unreviewed full redraw and middle bindings.
    AtRedraw,
}

/// Qualified first-group exit at 0079020d. The retained reference belongs to
/// the unimplemented continuation. Dropping this token emits no native Release.
#[derive(Debug, PartialEq, Eq)]
pub enum FirstGroupExit {
    NullPreferencesReturn,
    At0079020d { retained: Option<Object> },
}

pub struct Owner {
    state: RawState,
    effects: Vec<Effect>,
    freed: bool,
}

impl Owner {
    /// Portable boundary choice, not the native constructor (VP3).
    pub fn from_raw(state: RawState, domain: Domain) -> Result<Self, Unsupported> {
        if !domain.valid_objects {
            return Err(Unsupported::InvalidObject);
        }
        if !domain.unaliased_storage {
            return Err(Unsupported::AliasedStorage);
        }
        if !domain.no_foreign_reentry {
            return Err(Unsupported::ForeignReentry);
        }
        if state.owner.0 == 0
            || state.host.is_some_and(|h| h.0 == 0)
            || state.preferences.as_ref().is_some_and(|p| p.identity.0 == 0)
        {
            return Err(Unsupported::InvalidObject);
        }
        Ok(Self {
            state,
            effects: Vec::new(),
            freed: false,
        })
    }

    pub fn state(&self) -> &RawState {
        &self.state
    }
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }
    pub fn take_effects(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.effects)
    }

    fn live(&self) -> Result<(), Unsupported> {
        if self.freed {
            Err(Unsupported::FreedOwner)
        } else {
            Ok(())
        }
    }

    fn call(&mut self, host: &mut impl Host, call: Call) -> Result<Reply, Unsupported> {
        self.live()?;
        self.effects.push(Effect::Call(call.clone()));
        host.call(&call)
    }

    fn slot(&mut self, host: &mut impl Host, object: Object, slot: u16, args: Vec<Arg>) -> Result<Reply, Unsupported> {
        if object.0 == 0 {
            return Err(Unsupported::InvalidObject);
        }
        self.call(host, Call::Slot { object, slot, args })
    }

    fn object(reply: Reply) -> Result<Option<Object>, Unsupported> {
        match reply {
            Reply::Object(Some(Object(0))) => Err(Unsupported::InvalidObject),
            Reply::Object(o) => Ok(o),
            _ => Err(Unsupported::WrongChannel),
        }
    }

    fn required(reply: Reply) -> Result<Object, Unsupported> {
        Self::object(reply)?.ok_or(Unsupported::NullRequiredOutput)
    }

    fn al(reply: Reply) -> Result<bool, Unsupported> {
        match reply {
            Reply::Al(v) => Ok(v != 0),
            _ => Err(Unsupported::WrongChannel),
        }
    }

    fn word(reply: Reply) -> Result<u32, Unsupported> {
        match reply {
            Reply::U32(v) => Ok(v),
            _ => Err(Unsupported::WrongChannel),
        }
    }

    fn root(&mut self, host: &mut impl Host) -> Result<Object, Unsupported> {
        let h = self.state.host.ok_or(Unsupported::NullHost)?;
        Self::required(self.call(host, Call::Root(h))?)
    }

    fn release_local(&mut self, host: &mut impl Host, local: &mut Option<Object>) -> Result<(), Unsupported> {
        // Clear the local BEFORE invoking Release. This is logical ownership;
        // dropping the Rust Object value alone has no foreign effect.
        if let Some(object) = local.take() {
            self.slot(host, object, 8, vec![])?;
        }
        Ok(())
    }

    fn lookup(
        &mut self,
        host: &mut impl Host,
        local: &mut Option<Object>,
        id: u32,
    ) -> Result<Option<Object>, Unsupported> {
        let root = self.root(host)?; // §3: acquire next root BEFORE prior Release.
        self.release_local(host, local)?;
        match self.call(host, Call::Lookup { root, id, iid: 0x8810 })? {
            Reply::Lookup { al: 0, output: None } => Ok(None),
            Reply::Lookup { al: 0, output: Some(_) } => Err(Unsupported::FailedLookupWithOutput),
            Reply::Lookup { output: None, .. } => Err(Unsupported::NullRequiredOutput),
            Reply::Lookup {
                output: Some(Object(0)),
                ..
            } => Err(Unsupported::InvalidObject),
            Reply::Lookup { output: Some(o), .. } => {
                *local = Some(o);
                Ok(Some(o))
            }
            _ => Err(Unsupported::WrongChannel),
        }
    }

    fn checked(
        &mut self,
        host: &mut impl Host,
        local: &mut Option<Object>,
        id: u32,
    ) -> Result<Option<bool>, Unsupported> {
        match self.lookup(host, local, id)? {
            Some(o) => Ok(Some(Self::al(self.slot(host, o, 0x20, vec![])?)?)),
            None => Ok(None),
        }
    }

    fn byte(&mut self, field: Byte, value: u8) -> Result<(), Unsupported> {
        let p = self.state.preferences.as_mut().ok_or(Unsupported::NullPreferences)?;
        p.raw[field as usize] = value;
        self.effects.push(Effect::Byte(field, value));
        Ok(())
    }

    fn dword(&mut self, field: Dword, value: u32) -> Result<(), Unsupported> {
        let p = self.state.preferences.as_mut().ok_or(Unsupported::NullPreferences)?;
        let i = field as usize;
        p.raw[i..i + 4].copy_from_slice(&value.to_le_bytes());
        self.effects.push(Effect::Dword(field, value));
        Ok(())
    }

    fn changed(&mut self, differs: bool) {
        if differs {
            self.state.marker = 1;
            self.effects.push(Effect::Marker(1));
        }
    }

    /// Complete normal §3 body. No AL/EAX result is claimed.
    pub fn readback(&mut self, host: &mut impl Host) -> Result<(), Unsupported> {
        self.live()?;
        if self.state.preferences.is_none() {
            return Err(Unsupported::NullPreferences);
        }
        let mut local = None;
        for (base, field) in [
            (0x6a57da49, Byte::F0d),
            (0x6a57da4c, Byte::F0e),
            (0x6a57da4f, Byte::F0f),
        ] {
            for index in 0..3 {
                if self.checked(host, &mut local, base + index)? == Some(true) {
                    self.byte(field, index as u8 + 1)?;
                }
            }
        }
        for index in 0..3 {
            if self.checked(host, &mut local, 0x2ab6c373 + index)? == Some(true) {
                self.byte(Byte::F16, index as u8 + 1)?;
                self.changed(self.state.baselines.b16 != index as u16);
            }
        }
        for index in 0..3 {
            if self.checked(host, &mut local, 0xab81ded5 + index)? == Some(true) {
                self.byte(Byte::F28, index as u8 + 1)?;
                break;
            }
        }
        for (id, value) in [(0x0b85cf92, 1), (0x2b85cf9f, 2), (0x2b85cfaa, 3)] {
            if self.checked(host, &mut local, id)? == Some(true) {
                self.byte(Byte::F10, value)?;
            }
        }
        for (id, value) in [(0x6a57da57, 1), (0x6a57da58, 0)] {
            if self.checked(host, &mut local, id)? == Some(true) {
                self.byte(Byte::F13, value)?;
                if self.state.baselines.b12 != u16::from(value) {
                    self.changed(true);
                    self.byte(Byte::F0c, 1)?;
                }
            }
        }
        self.utility(host, 0x6a57da54, Byte::F11)?;
        if let Some(v) = self.checked(host, &mut local, 0x6a57da55)? {
            self.byte(Byte::F12, u8::from(v))?;
        }
        if let Some(v) = self.checked(host, &mut local, 0x6a57da56)? {
            self.byte(Byte::F15, u8::from(v))?;
            self.changed(self.state.baselines.b1c != u16::from(!v));
        }
        if let Some(v) = self.checked(host, &mut local, 0x6a57da60)? {
            self.byte(Byte::F14, u8::from(v))?;
            self.changed(self.state.baselines.b10 != u16::from(v));
        }
        self.utility(host, 0x0a8b7e00, Byte::F18)?;
        for (index, width, height) in [(0, 800, 600), (1, 1024, 768), (2, 1280, 1024), (3, 1600, 1200)] {
            if self.checked(host, &mut local, 0x6a57da5a + index)? == Some(true) {
                self.dword(Dword::Width, width)?;
                self.dword(Dword::Height, height)?;
                self.changed(self.state.baselines.b18 != index as u16);
            }
        }
        for (id, depth) in [(0x2ab6c376, 16), (0x2ab6c377, 32)] {
            if self.checked(host, &mut local, id)? == Some(true) {
                self.dword(Dword::Depth, depth)?;
                let stored = self
                    .state
                    .preferences
                    .as_ref()
                    .ok_or(Unsupported::NullPreferences)?
                    .dword(Dword::Depth);
                self.changed(u32::from(self.state.baselines.b14) != stored);
            }
        }
        self.release_local(host, &mut local)
    }

    fn utility(&mut self, host: &mut impl Host, id: u32, field: Byte) -> Result<(), Unsupported> {
        let root = self.root(host)?;
        let value = Self::al(self.call(host, Call::Utility { root, id })?)?;
        self.byte(field, u8::from(value))
    }

    fn request_group(
        &mut self,
        host: &mut impl Host,
        ids: &[u32],
        selected: usize,
        local: &mut Option<Object>,
    ) -> Result<(), Unsupported> {
        for (index, &id) in ids.iter().enumerate() {
            if let Some(o) = self.lookup(host, local, id)? {
                self.slot(host, o, if index == selected { 0x18 } else { 0x1c }, vec![])?;
            }
        }
        Ok(())
    }

    /// Entry through 0079020d only; not full SetButtonsBasedOnPreferences.
    pub fn redraw_first_group(&mut self, host: &mut impl Host) -> Result<FirstGroupExit, Unsupported> {
        self.live()?;
        let Some(p) = &self.state.preferences else {
            self.state.first_redraw = 0;
            self.effects.push(Effect::FirstRedraw(0));
            return Ok(FirstGroupExit::NullPreferencesReturn);
        };
        let raw = p.byte(Byte::F0d);
        let mut local = None;
        if (1..=3).contains(&raw) {
            self.request_group(
                host,
                &[0x6a57da49, 0x6a57da4a, 0x6a57da4b],
                usize::from(raw - 1),
                &mut local,
            )?;
        }
        Ok(FirstGroupExit::At0079020d { retained: local })
    }

    /// Full redraw has only one qualified complete path: null P (§6.1).
    pub fn redraw(&mut self, host: &mut impl Host) -> Result<(), Unsupported> {
        self.live()?;
        if self.state.preferences.is_some() {
            return Err(Unsupported::RemainingRedrawVp1);
        }
        self.redraw_first_group(host)?;
        Ok(())
    }

    /// Qualified entry at 00791201 with the live preceding local reference.
    /// Ownership of `incoming` is transferred; no AddRef is synthesized.
    pub fn redraw_depth_tail(&mut self, host: &mut impl Host, incoming: Option<Object>) -> Result<(), Unsupported> {
        self.live()?;
        let depth = self
            .state
            .preferences
            .as_ref()
            .ok_or(Unsupported::NullPreferences)?
            .dword(Dword::Depth);
        let mut local = incoming;
        if depth == 16 || depth == 32 {
            self.request_group(host, &[0x2ab6c376, 0x2ab6c377], usize::from(depth == 32), &mut local)?;
        }
        if self.state.first_redraw != 0 {
            let current = self
                .state
                .preferences
                .as_ref()
                .ok_or(Unsupported::NullPreferences)?
                .dword(Dword::Depth);
            self.state.baselines.b14 = current as u16;
            self.effects.push(Effect::BaselineDepth(current as u16));
        }
        self.release_local(host, &mut local)?;
        self.state.first_redraw = 0;
        self.effects.push(Effect::FirstRedraw(0));
        Ok(())
    }

    pub fn admitted(id: u32) -> bool {
        matches!(id, 0x0a8b7e00 | 0x0b85cf92 | 0x2b85cf9f | 0x2b85cfaa |
            0x2ab6c373..=0x2ab6c377 | 0x6a57da48..=0x6a57da51 |
            0x6a57da54..=0x6a57da58 | 0x6a57da5a..=0x6a57da5d |
            0xab81ded5..=0xab81ded7 | 0x6a57da60 | 0xea57da59 | 0xea57da6f | 0xea5e99d9)
    }

    pub fn message(
        &mut self,
        host: &mut impl Host,
        source: Object,
        message: Option<[u32; 3]>,
    ) -> Result<bool, Unsupported> {
        let root = self.root(host)?;
        if source != root {
            return Ok(false);
        }
        if !Self::al(self.slot(host, source, 0x11c, vec![])?)? {
            return Ok(false);
        }
        let [kind, subtype, id] = message.ok_or(Unsupported::NullRequiredOutput)?;
        if kind != 3 {
            return Ok(false);
        }
        if subtype == 0x287259f6 && Self::admitted(id) {
            self.action(host, id)?; // helper AL ignored; type-three AL is forced true.
        }
        Ok(true)
    }

    pub fn end_dialog(&mut self, host: &mut impl Host) -> Result<EndResult, Unsupported> {
        self.live()?;
        if self.state.guard == 0 {
            return Ok(EndResult::GuardZeroAl);
        }
        // These are the published virtual slot calls. AddRef arithmetic and
        // replacement vtables are unqualified. Direct shared-slot Release is
        // modeled separately by release_from_raw_count; no count is invented.
        self.slot(host, self.state.owner, 4, vec![])?;
        if self.state.host.is_some() {
            let root = self.root(host)?;
            if let Some(parent) = Self::object(self.slot(host, root, 0x18, vec![])?)? {
                if Self::al(self.slot(host, root, 0x10c, vec![Arg::U32(0x1000)])?)? {
                    self.slot(
                        host,
                        parent,
                        0x24,
                        vec![Arg::U32(15), Arg::Object(Some(root)), Arg::U32(1)],
                    )?;
                }
                self.slot(host, parent, 0x5c, vec![Arg::Object(Some(root))])?;
            }
        }
        if let Some(h) = self.state.host.take() {
            self.effects.push(Effect::Host(None));
            self.slot(host, h, 8, vec![])?;
        }
        self.state.guard = 0;
        self.effects.push(Effect::Guard(0));
        let eax = Self::word(self.slot(host, self.state.owner, 8, vec![])?)?;
        Ok(EndResult::TailReleaseEax(eax))
    }

    fn global(&mut self, host: &mut impl Host, global: Global) -> Result<Option<Object>, Unsupported> {
        Self::object(self.call(host, Call::Global(global))?)
    }

    fn raw_widget(&mut self, host: &mut impl Host, id: u32, second: u32) -> Result<(), Unsupported> {
        let root = self.root(host)?;
        let d = Self::required(self.slot(host, root, 0x2c, vec![])?)?;
        let w = Self::required(self.slot(host, d, 0x8c, vec![Arg::U32(id)])?)?;
        self.slot(host, w, 0x110, vec![Arg::U32(1), Arg::U32(second)])?;
        Ok(())
    }

    /// Explicit qualified ShowDialog tail entry; does not claim middle bindings.
    pub fn show_tail(&mut self, host: &mut impl Host) -> Result<bool, Unsupported> {
        self.raw_widget(host, 0x2a57cb83, 0)?;
        self.raw_widget(host, 0x2a57cb84, 1)?;
        Ok(false)
    }

    /// Qualified slice after the first redraw, ending before middle bindings.
    pub fn show_first_bindings(&mut self, host: &mut impl Host) -> Result<(), Unsupported> {
        for id in [0xea57da6f, 0xea57da59, 0x6a57da48] {
            let root = self.root(host)?;
            let widget = Self::required(self.slot(host, root, 0x8c, vec![Arg::U32(id)])?)?;
            let root = self.root(host)?;
            self.slot(host, widget, 0x158, vec![Arg::Object(Some(root))])?;
        }
        Ok(())
    }

    pub fn action(&mut self, host: &mut impl Host, id: u32) -> Result<bool, Unsupported> {
        self.live()?;
        match id {
            0x6a57da48 => {
                self.end_dialog(host)?;
            }
            0xea57da59 | 0xea57da6f => {
                self.readback(host)?;
                if id == 0xea57da59 && self.state.marker != 0 {
                    self.raw_widget(host, 0x2a57cb83, 1)?;
                    self.raw_widget(host, 0x2a57cb84, 0)?;
                } else {
                    self.save_apply_notify(host)?;
                }
            }
            0xea5e99d9 => {
                if let Some(app) = self.global(host, Global::App)? {
                    if self.state.preferences.is_some() {
                        // VP1/VP4: no literal Auto outputs or full redraw. Stop
                        // before the unqualified work; do not call a host oracle.
                        return Err(Unsupported::AutoRedrawVp1Vp4);
                    }
                    self.slot(host, app, 4, vec![])?;
                    self.slot(host, app, 8, vec![])?;
                }
            }
            0xab81ded5..=0xab81ded7 => {
                let mut local = None;
                self.request_group(
                    host,
                    &[0xab81ded5, 0xab81ded6, 0xab81ded7],
                    (id - 0xab81ded5) as usize,
                    &mut local,
                )?;
                self.release_local(host, &mut local)?;
            }
            _ => return Err(Unsupported::UnqualifiedEvent(id)),
        }
        Ok(true)
    }

    fn save_apply_notify(&mut self, host: &mut impl Host) -> Result<(), Unsupported> {
        let app = self.global(host, Global::App)?.ok_or(Unsupported::NullRequiredOutput)?;
        self.slot(host, app, 4, vec![])?;
        self.slot(host, app, 0x44, vec![])?;
        let payload = self
            .state
            .preferences
            .as_ref()
            .ok_or(Unsupported::NullPreferences)?
            .payload();
        self.slot(host, app, 0x4c, vec![Arg::Payload(payload)])?;
        self.slot(host, app, 8, vec![])?;
        let allocation = Self::object(self.call(host, Call::Allocate(44))?)?.ok_or(Unsupported::AllocationFailure)?;
        let message = Self::required(self.call(host, Call::ConstructMessage(allocation))?)?;
        self.slot(host, message, 4, vec![])?;
        self.slot(host, message, 0x14, vec![Arg::U32(0x231bbf91)])?;
        let notifications = self
            .global(host, Global::Notifications)?
            .ok_or(Unsupported::NullRequiredOutput)?;
        self.slot(host, notifications, 0x10, vec![Arg::Object(Some(message)), Arg::U32(0)])?;
        self.end_dialog(host)?;
        self.slot(host, message, 8, vec![])?;
        Ok(())
    }

    pub fn query_interface(
        &mut self,
        host: &mut impl Host,
        id: u32,
        output: Option<&mut Object>,
    ) -> Result<bool, Unsupported> {
        self.live()?;
        if id != 1 && id != 0x22e85d8e {
            return Ok(false);
        }
        *output.ok_or(Unsupported::NullRequiredOutput)? = self.state.owner;
        self.slot(host, self.state.owner, 4, vec![])?;
        Ok(true)
    }

    pub fn shutdown(&mut self) -> Result<bool, Unsupported> {
        self.live()?;
        self.state.preferences = None;
        self.effects.push(Effect::Preferences(None));
        Ok(true)
    }

    /// Shared Release slot, supplied raw W+0c snapshot (initial count and
    /// AddRef arithmetic unknown). Returns exact EAX; no Rust deallocation claim.
    pub fn release_from_raw_count(&mut self, host: &mut impl Host, count: &mut u32) -> Result<u32, Unsupported> {
        self.live()?;
        if *count > 1 {
            *count -= 1;
            self.effects.push(Effect::Count(*count));
            return Ok(*count);
        }
        self.shutdown()?;
        self.effects.push(Effect::ResetPublishedVptr);
        if let Some(h) = self.state.host {
            self.slot(host, h, 8, vec![])?;
        }
        self.call(host, Call::FreeOwner(self.state.owner))?;
        self.freed = true;
        self.effects.push(Effect::Freed);
        Ok(0)
    }

    /// Full ShowDialog only supports the qualified early returns. For selector
    /// zero use show_prefix explicitly, with its qualified stop at redraw.
    pub fn show(&mut self, selector: u32) -> Result<bool, Unsupported> {
        self.live()?;
        if self.state.initialized == 0 {
            self.state.initialized = 1;
            self.effects.push(Effect::Initialized(1));
            self.state.guard = 0;
            self.effects.push(Effect::Guard(0));
        } else if self.state.guard != 0 {
            return Ok(true);
        }
        if selector != 0 {
            return Ok(false);
        }
        Err(Unsupported::ShowMiddleVp3)
    }

    /// §7.1 prefix, ending before first full redraw. A borrowed global P must
    /// be supplied as explicit raw storage, never manufactured from city state.
    pub fn show_prefix(
        &mut self,
        host: &mut impl Host,
        selector: u32,
        global_preferences: Option<Preferences>,
    ) -> Result<ShowPrefix, Unsupported> {
        match self.show(selector) {
            Ok(al) => return Ok(ShowPrefix::ReturnedAl(al)),
            Err(Unsupported::ShowMiddleVp3) => {}
            Err(e) => return Err(e),
        }
        if let Some(h) = self.state.host.take() {
            self.effects.push(Effect::Host(None));
            self.slot(host, h, 8, vec![])?;
        }
        let reply = self.call(
            host,
            Call::CreateScript {
                key: [0, 0x96a006b0, 0x8a7e052f],
                args: [0, 0x2a57cb82, 0x5386d516],
            },
        )?;
        let al = match reply {
            Reply::Lookup { al, output } => {
                if output.is_some_and(|o| o.0 == 0) {
                    return Err(Unsupported::InvalidObject);
                }
                self.state.host = output;
                self.effects.push(Effect::Host(output));
                al
            }
            _ => return Err(Unsupported::WrongChannel),
        };
        if al == 0 {
            return Ok(ShowPrefix::ReturnedAl(false));
        }
        let root = self.root(host)?;
        let (first, second) = if let Some(d) = Self::object(self.slot(host, root, 0x2c, vec![])?)? {
            let first = Self::word(self.slot(host, d, 0xa4, vec![])?)?;
            let second = Self::word(self.slot(host, d, 0xa8, vec![])?)?;
            (first, second)
        } else {
            (640, 480)
        };
        let root = self.root(host)?;
        self.slot(host, root, 0xcc, vec![Arg::U32(first)])?;
        let root = self.root(host)?;
        self.slot(host, root, 0xd0, vec![Arg::U32(second)])?;
        if self.state.preferences.is_none() {
            self.state.preferences = global_preferences;
            self.effects
                .push(Effect::Preferences(self.state.preferences.as_ref().map(|p| p.identity)));
        }
        self.state.guard = 1;
        self.effects.push(Effect::Guard(1));
        self.root(host)?;
        let h = self.state.host.ok_or(Unsupported::NullHost)?;
        self.slot(host, h, 0x1c, vec![Arg::Object(Some(self.state.owner))])?;
        Ok(ShowPrefix::AtRedraw)
    }

    /// §7.2 normal service-success, nonreentrant bounded portable call model.
    /// Foreign modal result and framework service-query AL are ignored.
    pub fn do_modal(&mut self, host: &mut impl Host) -> Result<bool, Unsupported> {
        self.live()?;
        if self.state.host.is_none() {
            return Ok(true);
        }
        let pause = Self::required(self.call(host, Call::Pause([0, 0, 1, 1]))?)?;
        let framework = Self::object(self.call(host, Call::Framework)?)?;
        let service = if let Some(framework) = framework {
            match self.call(
                host,
                Call::Service {
                    framework,
                    id: 0xa417445e,
                    iid: 0x5a4,
                },
            )? {
                Reply::Lookup { output: Some(s), .. } if s.0 != 0 => s,
                _ => return Err(Unsupported::MissingService),
            }
        } else {
            return Err(Unsupported::MissingService);
        };
        let focus = Self::word(self.slot(host, service, 0x90, vec![])?)?;
        let h = self.state.host.ok_or(Unsupported::NullHost)?;
        let root = Self::object(self.call(host, Call::Root(h))?)?;
        if let Some(r) = root {
            self.slot(host, r, 4, vec![])?;
        }
        let modal = self
            .global(host, Global::ModalService)?
            .ok_or(Unsupported::MissingService)?;
        self.slot(host, modal, 0x28, vec![Arg::Object(root), Arg::U32(1)])?;
        self.slot(host, service, 0x94, vec![Arg::Object(root)])?;
        let manager = Self::required(self.call(host, Call::WindowManager)?)?;
        self.slot(host, manager, 0xa4, vec![Arg::Object(root)])?;
        self.slot(host, modal, 0x18, vec![Arg::Object(root)])?;
        if Self::al(self.slot(host, service, 0x60, vec![Arg::U32(focus)])?)? {
            self.slot(host, service, 0x94, vec![Arg::U32(focus)])?;
        }
        self.end_dialog(host)?;
        if let Some(r) = root {
            self.slot(host, r, 8, vec![])?;
        }
        self.slot(host, service, 8, vec![])?;
        self.call(host, Call::Unpause(pause))?;
        Ok(true)
    }
}

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;
