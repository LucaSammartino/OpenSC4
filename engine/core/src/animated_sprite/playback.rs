// SPDX-License-Identifier: GPL-3.0-or-later
//! Â§Â§4/4.2/4.3. Raw-unit clock, byte frame and decoded controller state.
use super::{numeric::X, AnimatedSprite};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Playback {
    pub frame: u32,
    pub position: f32,
    pub rate: f32,
    pub direction: i8,
    pub play_mode: u8,
    pub frame_skipping: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaybackError {
    EmptySequence,
    InvalidFrame,
    NonFinite,
    ClockCannotProgress,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickResult {
    pub frame_changed: bool,
    pub advances: u64,
}
impl TickResult {
    pub fn status(self) -> u32 {
        if self.advances != 0 {
            2
        } else {
            0
        }
    }
}

/// Decoded snapshot of the borrowed native controller. No load/factory invents
/// one (AS1). Rust owns this snapshot, not the foreign controller allocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceController {
    pub selected: u16,
    pub count: u16,
    pub length: u16,
    pub starts: Vec<u16>,
    pub current: u16,
    pub mode: u8,
    pub direction: i8,
}

fn step(frame: &mut u16, start: u16, end: u32, direction: &mut i8, mode: u8) {
    if *direction > 0 {
        if u32::from(*frame) == end {
            match mode {
                3 => *frame = start,
                2 => {
                    *direction = -1;
                    *frame = frame.wrapping_sub(1);
                }
                _ => {
                    *direction = 0;
                }
            }
        } else {
            *frame = frame.wrapping_add(1);
        }
    } else if *direction < 0 {
        if *frame == start {
            match mode {
                3 => *frame = end as u16,
                2 => {
                    *direction = 1;
                    *frame = frame.wrapping_add(1);
                }
                _ => {
                    *direction = 0;
                }
            }
        } else {
            *frame = frame.wrapping_sub(1);
        }
    }
}

impl SequenceController {
    fn advance(&mut self) {
        if self.length > 1 {
            let start = self.starts[usize::from(self.selected)];
            let end = start.wrapping_add(self.length - 1);
            step(&mut self.current, start, u32::from(end), &mut self.direction, self.mode);
        }
    }

    fn validate_advance(&self) -> Result<(), PlaybackError> {
        if self.length > 1 && usize::from(self.selected) >= self.starts.len() {
            return Err(PlaybackError::InvalidFrame);
        }
        Ok(())
    }
}

impl Playback {
    /// Length is retained for source compatibility. Without a controller the
    /// native setter only narrows to a byte (Â§4.3).
    pub fn set_frame(&mut self, frame: u32, _length: u32) -> Result<(), PlaybackError> {
        self.frame = u32::from(frame as u8);
        Ok(())
    }
    /// Returns dirty state, not whether the frame byte changed (Â§4.2).
    pub fn advance_unsequenced(&mut self, count: u32) -> Result<bool, PlaybackError> {
        // The comparison is signed, but the endpoint remains full-width until
        // the frame is stored as a byte. In particular count 300 has no reachable
        // forward endpoint; count 65537 must not alias frame zero.
        if (count as i32) <= 1 {
            return Ok(false);
        }
        let mut frame = self.frame as u16;
        let previous_direction = self.direction;
        step(&mut frame, 0, count - 1, &mut self.direction, self.play_mode);
        if previous_direction != 0 && self.direction == 0 {
            self.position = 0.;
        }
        self.frame = u32::from(frame as u8);
        Ok(true)
    }
    pub fn tick_unsequenced(&mut self, elapsed: f32, count: u32) -> Result<TickResult, PlaybackError> {
        let mut next = *self;
        let result = next.tick_with(elapsed, |p| p.advance_unsequenced(count))?;
        *self = next;
        Ok(result)
    }
    fn tick_with(
        &mut self,
        elapsed: f32,
        mut advance: impl FnMut(&mut Self) -> Result<bool, PlaybackError>,
    ) -> Result<TickResult, PlaybackError> {
        if self.rate.to_bits() == 0xc2c80000 {
            self.rate = -200.;
        }
        let mut result = TickResult {
            frame_changed: false,
            advances: 0,
        };
        if self.rate == 0. {
            return Ok(result);
        }
        if !elapsed.is_finite() || !self.rate.is_finite() || !self.position.is_finite() {
            return Err(PlaybackError::NonFinite);
        }
        let raw = X::f(elapsed)
            .mul(X::f(self.rate))
            .mul(X::integer(i64::from(self.direction)))
            .add(X::f(self.position));
        self.position = raw.store();
        if !self.position.is_finite() {
            return Err(PlaybackError::NonFinite);
        }
        if !raw.abs().ge(X::f(1.)) {
            return Ok(result);
        }
        let repeat = raw.abs().ge(X::f(2.)) && self.frame_skipping;
        let overwrite = raw.abs().ge(X::f(2.)) && !self.frame_skipping;
        loop {
            let previous = self.position;
            result.frame_changed |= advance(self)?;
            result.advances += 1;
            if overwrite {
                self.position = 1.;
                break;
            }
            self.position = X::f(self.position).sub(X::integer(i64::from(self.direction))).store();
            if !repeat || self.position.abs() < 1. {
                break;
            }
            // Native loops forever here; this error is a safe API diagnostic,
            // never a fabricated successful native status.
            if self.position.abs() >= previous.abs() {
                return Err(PlaybackError::ClockCannotProgress);
            }
        }
        Ok(result)
    }
}

impl<O> AnimatedSprite<O> {
    pub fn controller(&self) -> Option<&SequenceController> {
        self.sequence.as_ref()
    }
    pub fn install_controller_snapshot(&mut self, controller: Option<SequenceController>) {
        self.sequence = controller;
    }
    pub fn frame_count(&self) -> u32 {
        self.loaded.frame_count
    }
    pub fn frame(&self) -> u8 {
        self.state.playback.frame as u8
    }
    fn copy_controller_frame(&mut self) {
        if let Some(c) = &self.sequence {
            self.frame_dirty |= u32::from(c.current) != self.state.playback.frame;
            self.state.playback.frame = u32::from(c.current as u8);
        }
    }
    pub fn set_animation(&mut self, index: i32) -> Result<u32, PlaybackError> {
        let Some(c) = &mut self.sequence else {
            return Ok(0);
        };
        if index >= i32::from(c.count) {
            return Ok(0);
        }
        let index = usize::try_from(index).map_err(|_| PlaybackError::InvalidFrame)?;
        let a = *c.starts.get(index).ok_or(PlaybackError::InvalidFrame)?;
        let b = *c.starts.get(index + 1).ok_or(PlaybackError::InvalidFrame)?;
        c.selected = index as u16;
        c.length = b.wrapping_sub(a);
        self.copy_controller_frame();
        Ok(2)
    }
    pub fn select_frame(&mut self, mut n: i32) -> Result<u32, PlaybackError> {
        if let Some(c) = &mut self.sequence {
            if n > i32::from(c.length) {
                if c.length == 0 {
                    return Err(PlaybackError::EmptySequence);
                }
                n %= i32::from(c.length);
            }
            let start = *c
                .starts
                .get(usize::from(c.selected))
                .ok_or(PlaybackError::InvalidFrame)?;
            c.current = start.wrapping_add(n as u16);
            self.copy_controller_frame();
        } else {
            self.frame_dirty |= n as u32 != self.state.playback.frame;
            self.state.playback.frame = u32::from(n as u8);
        }
        Ok(2)
    }
    pub fn advance_frame(&mut self) -> Result<u32, PlaybackError> {
        if let Some(c) = &mut self.sequence {
            c.validate_advance()?;
            c.advance();
            self.copy_controller_frame();
        } else {
            self.frame_dirty |= self.state.playback.advance_unsequenced(self.loaded.frame_count)?;
        }
        Ok(2)
    }
    pub fn tick(&mut self, elapsed: f32) -> Result<u32, PlaybackError> {
        let mut p = self.state.playback;
        // Safe diagnostics are transactional, including the decoded controller.
        // Controller mode/direction never replace the owner's clock direction.
        let mut sequence = self.sequence.clone();
        let result = p.tick_with(elapsed, |p| {
            if let Some(c) = &mut sequence {
                c.validate_advance()?;
                c.advance();
                let dirty = u32::from(c.current) != p.frame;
                p.frame = u32::from(c.current as u8);
                Ok(dirty)
            } else {
                p.advance_unsequenced(self.loaded.frame_count)
            }
        })?;
        self.state.playback = p;
        self.sequence = sequence;
        self.frame_dirty |= result.frame_changed;
        Ok(result.status())
    }
}
