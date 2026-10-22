// SPDX-License-Identifier: GPL-3.0-or-later
//! Literal raw-unit clock (§4). Sequence-table stepping is unpublished (AS5),
//! so these methods explicitly implement only the no-animation-table path.

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
    InvalidDirection,
    RateSentinel,
    NonFinite,
    ReverseBoundaryUnspecified,
    ClockCannotProgress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickResult {
    /// §4's value 2 means a frame was advanced. Other native return codes
    /// are unpublished (AS5); do not invent a native success code for them.
    pub frame_changed: bool,
    pub advances: u64,
}

impl Playback {
    pub fn set_frame(&mut self, frame: u32, length: u32) -> Result<(), PlaybackError> {
        if length == 0 {
            return Err(PlaybackError::EmptySequence);
        }
        self.frame = frame % length;
        Ok(())
    }

    fn validate(&self, frame_count: u32) -> Result<(), PlaybackError> {
        if frame_count == 0 {
            return Err(PlaybackError::EmptySequence);
        }
        if self.frame >= frame_count {
            return Err(PlaybackError::InvalidFrame);
        }
        if !(-1..=1).contains(&self.direction) {
            return Err(PlaybackError::InvalidDirection);
        }
        Ok(())
    }

    /// Transactional on an unpublished reverse-end rule (AS5). No fabricated
    /// lower-end loop/ping-pong body is used when direction is negative at 0.
    pub fn advance_unsequenced(&mut self, frame_count: u32) -> Result<bool, PlaybackError> {
        self.validate(frame_count)?;
        if frame_count <= 1 || self.direction == 0 {
            return Ok(false);
        }
        if self.direction < 0 {
            if self.frame == 0 {
                return Err(PlaybackError::ReverseBoundaryUnspecified);
            }
            self.frame -= 1;
        } else if self.frame + 1 < frame_count {
            self.frame += 1;
        } else {
            match self.play_mode {
                3 => self.frame = 0,
                2 => {
                    self.direction = -1;
                    self.frame -= 1;
                }
                _ => {
                    self.direction = 0;
                    self.position = 0.0;
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    /// No milliseconds/seconds conversion. Rate and elapsed are multiplied
    /// exactly in the raw units supplied by §4, including negative rates.
    pub fn tick_unsequenced(&mut self, elapsed: f32, frame_count: u32) -> Result<TickResult, PlaybackError> {
        let mut next = *self;
        let result = next.tick_inner(elapsed, frame_count)?;
        *self = next;
        Ok(result)
    }

    fn tick_inner(&mut self, elapsed: f32, frame_count: u32) -> Result<TickResult, PlaybackError> {
        self.validate(frame_count)?;
        if self.rate == -100.0 || self.rate == -200.0 {
            return Err(PlaybackError::RateSentinel);
        }
        if !self.rate.is_finite() || !self.position.is_finite() || !elapsed.is_finite() {
            return Err(PlaybackError::NonFinite);
        }
        let mut result = TickResult {
            frame_changed: false,
            advances: 0,
        };
        if self.rate == 0.0 {
            return Ok(result);
        }
        self.position += elapsed * self.rate * f32::from(self.direction);
        if !self.position.is_finite() {
            return Err(PlaybackError::NonFinite);
        }
        if self.position.abs() < 1.0 {
            return Ok(result);
        }
        if self.position.abs() >= 2.0 && !self.frame_skipping {
            result.frame_changed = self.advance_unsequenced(frame_count)?;
            result.advances = u64::from(result.frame_changed);
            self.position = 1.0;
            return Ok(result);
        }
        // Below 2, exactly one advance, even if a reversal changes direction.
        // At/above 2 with skipping, repeat until below 1. AS5 asks whether
        // direction subtraction is before/after reversal; literal order here.
        let repeat = self.position.abs() >= 2.0 && self.frame_skipping;
        loop {
            let previous = self.position;
            let changed = self.advance_unsequenced(frame_count)?;
            result.frame_changed |= changed;
            result.advances += u64::from(changed);
            self.position -= f32::from(self.direction);
            if !repeat || self.position.abs() < 1.0 {
                break;
            }
            // Raw negative rates, stopped/single-frame clips or f32 saturation
            // can make the stated loop diverge. AS5: fail explicitly, do not
            // hang, cap the elapsed time or silently convert its units.
            if self.position.abs() >= previous.abs() {
                return Err(PlaybackError::ClockCannotProgress);
            }
        }
        Ok(result)
    }
}
