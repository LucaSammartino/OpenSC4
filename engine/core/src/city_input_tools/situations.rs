// SPDX-License-Identifier: GPL-3.0-or-later
use std::num::NonZeroU32;

use super::{Pixel, Resource, Unsupported};

pub const CITY_SITUATIONS_ID: u32 = 0xABEC_6CFE;
pub const CITY_SITUATIONS_CURSOR: u32 = 0x4340_1C71;
pub const SITUATION_NOTIFICATION: u32 = 0xA6B7_9602;

/// Borrowed indicator identity; None represents the published zero sentinel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Indicator(pub NonZeroU32);

pub trait SituationsHost {
    fn subscribe(&mut self, message: u32);
    fn view_level(&mut self) -> u32;
    fn both_managers_present(&mut self) -> bool;
    fn find_indicator(&mut self, mouse: Pixel) -> Option<Indicator>;
    fn lock_indicator(&mut self, handle: Indicator, on: bool);
    fn set_active_indicator(&mut self, handle: Option<Indicator>, state: u32);
    /// Borrowed, no invented AddRef/Release of the occupant or vehicle view.
    fn active_occupant(&mut self) -> Option<Resource>;
    fn initiate_situation(&mut self, occupant: Resource, advice: Option<Resource>) -> bool;
}

#[derive(Debug)]
pub struct CitySituations {
    locked: Option<Indicator>,
    pressed: Option<Indicator>,
}

impl CitySituations {
    /// CT14: caller supplies constructor fields not published in section 4.
    pub fn new(locked: Option<Indicator>, pressed: Option<Indicator>) -> Self {
        Self { locked, pressed }
    }

    /// Partial Init: only the published notification; no guessed base/COM or
    /// acquisition/shutdown contract. The host supplies primitive managers.
    pub fn init(&self, host: &mut impl SituationsHost) {
        host.subscribe(SITUATION_NOTIFICATION);
    }

    pub fn locked(&self) -> Option<Indicator> {
        self.locked
    }

    pub fn pressed(&self) -> Option<Indicator> {
        self.pressed
    }

    pub fn mouse_move(&mut self, host: &mut impl SituationsHost, mouse: Pixel) -> Result<bool, Unsupported> {
        let hit = host.find_indicator(mouse);
        match self.locked {
            None => {
                // CT16: do not invent LockIndicator(0,true) or silently no-op.
                let hit = hit.ok_or(Unsupported::EmptyHover)?;
                host.lock_indicator(hit, true);
                self.locked = Some(hit);
                host.set_active_indicator(Some(hit), 3);
            }
            Some(old) if Some(old) != hit => {
                host.lock_indicator(old, false);
                self.locked = None;
            }
            Some(_) => {}
        }
        Ok(false)
    }

    pub fn press(&mut self, host: &mut impl SituationsHost, mouse: Pixel) -> bool {
        if host.view_level() == 1 {
            if let Some(hit) = host.find_indicator(mouse) {
                host.set_active_indicator(Some(hit), 2);
                self.pressed = Some(hit);
                return true;
            }
        }
        false
    }

    pub fn release(&mut self, host: &mut impl SituationsHost, mouse: Pixel) -> bool {
        if host.view_level() == 1 && host.both_managers_present() {
            if let Some(hit) = host.find_indicator(mouse) {
                host.set_active_indicator(Some(hit), 1);
                if self.pressed == Some(hit) {
                    if let Some(occupant) = host.active_occupant() {
                        host.initiate_situation(occupant, None);
                    }
                }
                // CT16: most literal reading: every indicator hit is handled;
                // only gate/miss failure forgets pressed. No inferred native
                // return from the situation manager's independent result.
                return true;
            }
        }
        self.pressed = None;
        false
    }

    pub fn message(&mut self, host: &mut impl SituationsHost, message: u32) -> Result<bool, Unsupported> {
        if !matches!(message, 0x06CD_B65B | 0xA6B7_9602 | 0xA6B7_9612) {
            return Err(Unsupported::UnknownMessage);
        }
        // CT16: unlock a held handle only; do not synthesize an unlock(0).
        if let Some(locked) = self.locked {
            host.lock_indicator(locked, false);
        }
        host.set_active_indicator(None, 1);
        self.locked = None;
        self.pressed = None;
        Ok(true)
    }
}
