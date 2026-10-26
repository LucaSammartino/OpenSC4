// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled owner bodies in video-preferences §9; controls remain foreign.
use super::*;

/// Explicit snapshot of the two serialized global dialog bytes (§7.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DialogGlobals {
    pub initialized: u8,
    pub guard: u8,
}

pub const BINDINGS: [u32; 35] = [
    0xea57da6f, 0xea57da59, 0x6a57da48, 0x6a57da49, 0x6a57da4a, 0x6a57da4b, 0x6a57da4c, 0x6a57da4d, 0x6a57da4e,
    0x6a57da4f, 0x6a57da50, 0x6a57da51, 0x2ab6c373, 0x2ab6c374, 0x2ab6c375, 0x2ab6c376, 0x2ab6c377, 0x6a57da54,
    0x6a57da55, 0x6a57da56, 0x6a57da57, 0x6a57da58, 0x6a57da5a, 0x6a57da5b, 0x6a57da5c, 0x6a57da5d, 0xea5e99d9,
    0x6a57da60, 0x0a8b7e00, 0xab81ded5, 0xab81ded6, 0xab81ded7, 0x0b85cf92, 0x2b85cf9f, 0x2b85cfaa,
];

impl Owner {
    /// §9.1 constructor. Globals are not reset by constructing another owner.
    pub fn new(owner: Object, globals: DialogGlobals) -> Result<Self, Unsupported> {
        let mut result = Self::from_raw(
            RawState {
                owner,
                host: None,
                preferences: None,
                baselines: Baselines {
                    b10: 0,
                    b12: 0,
                    b14: 0,
                    b16: 0,
                    b18: 0,
                    b1c: 0,
                },
                marker: 0,
                first_redraw: 1,
                guard: globals.guard,
                initialized: globals.initialized,
            },
            Domain {
                valid_objects: true,
                unaliased_storage: true,
                no_foreign_reentry: true,
            },
        )?;
        result.references = Some(0);
        Ok(result)
    }

    pub fn globals(&self) -> DialogGlobals {
        DialogGlobals {
            initialized: self.state.initialized,
            guard: self.state.guard,
        }
    }

    pub fn add_ref(&mut self) -> Result<u32, Unsupported> {
        self.live()?;
        let count = self.references.as_mut().ok_or(Unsupported::RawReferenceCount)?;
        // SPEC-GAP: the shared AddRef arithmetic is unqualified; use wrapping u32 increment.
        *count = count.wrapping_add(1);
        self.effects.push(Effect::Count(*count));
        Ok(*count)
    }

    pub fn release(&mut self, host: &mut impl Host) -> Result<u32, Unsupported> {
        let mut count = self.references.ok_or(Unsupported::RawReferenceCount)?;
        let result = self.release_from_raw_count(host, &mut count)?;
        self.references = Some(count);
        Ok(result)
    }

    pub(super) fn application_preferences(
        &mut self,
        host: &mut impl Host,
        app: Object,
        slot: u16,
    ) -> Result<(), Unsupported> {
        self.live()?;
        let preferences = self.state.preferences.as_mut().ok_or(Unsupported::NullPreferences)?;
        let call = Call::Slot {
            object: app,
            slot,
            args: vec![Arg::Payload(preferences.payload())],
        };
        self.effects.push(Effect::Call(call.clone()));
        host.preferences(&call, preferences)?;
        Ok(())
    }

    pub(super) fn selection_event(&mut self, host: &mut impl Host, id: u32) -> Result<(), Unsupported> {
        let groups: &[&[u32]] = &[
            &[0x6a57da49, 0x6a57da4a, 0x6a57da4b],
            &[0x6a57da4c, 0x6a57da4d, 0x6a57da4e],
            &[0x6a57da4f, 0x6a57da50, 0x6a57da51],
            &[0x2ab6c373, 0x2ab6c374, 0x2ab6c375],
            &[0x2ab6c376, 0x2ab6c377],
            &[0x6a57da57, 0x6a57da58],
            &[0x6a57da5a, 0x6a57da5b, 0x6a57da5c, 0x6a57da5d],
            &[0x0b85cf92, 0x2b85cf9f, 0x2b85cfaa],
        ];
        if let Some(ids) = groups.iter().find(|ids| ids.contains(&id)) {
            let mut local = None;
            self.request_group(
                host,
                ids,
                ids.iter().position(|&v| v == id).expect("group member"),
                &mut local,
            )?;
            self.release_local(host, &mut local)?;
            Ok(())
        } else if matches!(
            id,
            0x6a57da54..=0x6a57da56 | 0x6a57da60 | 0x0a8b7e00 | 0x6a57da52 | 0x6a57da53
        ) {
            Ok(())
        } else {
            Err(Unsupported::UnqualifiedEvent(id))
        }
    }

    /// Full §7.1/§9.4 body. Successful initialization deliberately returns false.
    pub fn show_dialog(
        &mut self,
        host: &mut impl Host,
        selector: u32,
        global_preferences: Option<Preferences>,
    ) -> Result<bool, Unsupported> {
        if let ShowPrefix::ReturnedAl(al) = self.show_prefix(host, selector, global_preferences)? {
            return Ok(al);
        }
        self.redraw(host)?;
        for id in BINDINGS {
            let root = self.root(host)?;
            let control = Self::required(self.slot(host, root, 0x8c, vec![Arg::U32(id)])?)?;
            let target = self.root(host)?;
            self.slot(host, control, 0x158, vec![Arg::Object(Some(target))])?;
        }
        self.show_tail(host)
    }

    fn preference_byte(&self, field: Byte) -> Result<u8, Unsupported> {
        Ok(self
            .state
            .preferences
            .as_ref()
            .ok_or(Unsupported::NullPreferences)?
            .byte(field))
    }

    fn redraw_radio(
        &mut self,
        host: &mut impl Host,
        field: Byte,
        ids: &[u32],
        local: &mut Option<Object>,
    ) -> Result<Option<u16>, Unsupported> {
        let value = self.preference_byte(field)?;
        if (1..=ids.len() as u8).contains(&value) {
            self.request_group(host, ids, usize::from(value - 1), local)?;
            Ok(Some(u16::from(value - 1)))
        } else {
            Ok(None)
        }
    }

    fn set_toggle(&mut self, host: &mut impl Host, field: Byte, id: u32) -> Result<(), Unsupported> {
        let on = self.preference_byte(field)? == 1;
        let root = self.root(host)?;
        self.call(host, Call::SetToggle { root, id, on })?;
        Ok(())
    }

    pub(super) fn redraw_complete(&mut self, host: &mut impl Host) -> Result<(), Unsupported> {
        let FirstGroupExit::At0079020d { retained } = self.redraw_first_group(host)? else {
            return Ok(());
        };
        let mut local = retained;
        self.redraw_radio(host, Byte::F0e, &[0x6a57da4c, 0x6a57da4d, 0x6a57da4e], &mut local)?;
        if let Some(index) = self.redraw_radio(host, Byte::F16, &[0x2ab6c373, 0x2ab6c374, 0x2ab6c375], &mut local)? {
            if self.state.first_redraw != 0 {
                self.state.baselines.b16 = index;
            }
        }
        self.redraw_radio(host, Byte::F0f, &[0x6a57da4f, 0x6a57da50, 0x6a57da51], &mut local)?;
        self.set_toggle(host, Byte::F11, 0x6a57da54)?;
        self.set_toggle(host, Byte::F12, 0x6a57da55)?;
        for (field, id) in [(Byte::F15, 0x6a57da56), (Byte::F14, 0x6a57da60)] {
            let on = self.preference_byte(field)? == 1;
            if let Some(control) = self.lookup(host, &mut local, id)? {
                self.slot(host, control, if on { 0x18 } else { 0x1c }, vec![])?;
                if self.state.first_redraw != 0 {
                    if field == Byte::F15 {
                        self.state.baselines.b1c = u16::from(!on);
                    } else {
                        self.state.baselines.b10 = u16::from(on);
                    }
                }
            }
        }
        self.set_toggle(host, Byte::F18, 0x0a8b7e00)?;
        let renderer = self.preference_byte(Byte::F13)?;
        if renderer <= 1 {
            self.request_group(host, &[0x6a57da57, 0x6a57da58], usize::from(renderer == 0), &mut local)?;
            if self.state.first_redraw != 0 {
                self.state.baselines.b12 = u16::from(renderer);
            }
        }
        let speed = self.preference_byte(Byte::F28)?;
        // An out-of-range index selects nothing, while still clearing all rows.
        self.request_group(
            host,
            &[0xab81ded5, 0xab81ded6, 0xab81ded7],
            usize::from(speed.wrapping_sub(1)),
            &mut local,
        )?;
        for (id, value) in [(0x0b85cf92, 1), (0x2b85cf9f, 2), (0x2b85cfaa, 3)] {
            let on = self.preference_byte(Byte::F10)? == value;
            if let Some(control) = self.lookup(host, &mut local, id)? {
                self.slot(host, control, if on { 0x18 } else { 0x1c }, vec![])?;
            }
        }
        let p = self.state.preferences.as_ref().ok_or(Unsupported::NullPreferences)?;
        let pair = (p.dword(Dword::Width), p.dword(Dword::Height));
        let index = [(800, 600), (1024, 768), (1280, 1024), (1600, 1200)]
            .iter()
            .position(|&v| v == pair);
        self.request_group(
            host,
            &[0x6a57da5a, 0x6a57da5b, 0x6a57da5c, 0x6a57da5d],
            index.unwrap_or(4),
            &mut local,
        )?;
        if self.state.first_redraw != 0 {
            self.state.baselines.b18 = index.unwrap_or(0) as u16;
        }
        self.redraw_depth_tail(host, local)
    }
}
