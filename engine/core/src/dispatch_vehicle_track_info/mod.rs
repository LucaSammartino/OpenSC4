// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned portable TrackInfo record, dispatch-vehicle-view.md §§1.2, 2, 3.12.

pub type Token = u64;
pub type Vec3 = [f32; 3];
pub type Quad = [Vertex; 4];

pub mod flags {
    pub const BUILDING: u32 = 1;
    pub const DISPLAY: u32 = 1 << 1;
    pub const SELECTABLE: u32 = 1 << 2;
    pub const FADES: u32 = 1 << 3;
    pub const REMOVING: u32 = 1 << 4;
    pub const VISIBLE: u32 = 1 << 5;
    pub const MODEL: u32 = 1 << 6;
    pub const LOCKED: u32 = 1 << 7;
    pub const FLASHING: u32 = 1 << 8;
    pub const PROGRESS: u32 = 1 << 9;
    pub const ALTERNATE: u32 = 1 << 10;
    pub const TOP: u32 = 1 << 11;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    pub position: Vec3,
    pub uv: [f32; 2],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }
    pub fn has_extents(self) -> bool {
        self.left != self.right && self.top != self.bottom
    }
}

/// Identity is the native resource-handle identity, distinct from its texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Icon {
    pub identity: Token,
    pub texture: Option<Token>,
}

/// Not Clone: the processor is uniquely owned; foreign references require an
/// explicit host acquisition. Rust strings/quads supply the value-copy part.
#[derive(Debug)]
pub struct TrackInfo {
    pub handle: u32,
    pub kind: u32,
    pub occupant: Token,
    pub texture: Option<Token>,
    pub number: u32,
    pub image_instance: u32,
    pub label: String,
    pub icon: Option<Icon>,
    pub display_quad: Quad,
    pub icon_quad: Quad,
    pub display_size: [f32; 2],
    pub rectangle: Rect,
    pub anchor: Vec3,
    pub frame: u32,
    pub fade: f32,
    pub flags: u32,
    pub model: Option<Token>,
    pub flash_accumulator: f32,
    pub flash_period: f32,
    pub processor: Option<Token>,
}
impl TrackInfo {
    /// Fully initialized TrackOccupant scratch record, rather than exposing the
    /// native constructor's unwritten fields to safe Rust.
    pub fn for_tracking(occupant: Token, kind: u32, number: u32, label: Option<&str>, icon: Option<Icon>) -> Self {
        let (flags, fade, frame) = match kind {
            0..=2 => (0x2a, 0.0, 0),
            3 => (0x22, 1.0, 0),
            4 => (0x2e, 0.0, 1),
            6 => (0x68, 0.0, 0),
            _ => (0x28, 0.0, 0),
        };
        Self {
            handle: 0,
            kind,
            occupant,
            texture: None,
            number: if kind <= 2 { number } else { 0 },
            image_instance: if (kind == 3 || kind == 4) && label.is_none() {
                number
            } else {
                0
            },
            label: if kind == 3 || kind == 4 {
                label.unwrap_or_default().to_owned()
            } else {
                String::new()
            },
            icon,
            display_quad: [Vertex::default(); 4],
            icon_quad: [Vertex::default(); 4],
            display_size: [0.0; 2],
            rectangle: Rect::default(),
            anchor: [0.0; 3],
            frame,
            fade,
            flags,
            model: None,
            flash_accumulator: 0.0,
            flash_period: 1.0,
            processor: None,
        }
    }
    pub fn key(&self) -> u8 {
        self.kind as u8
    }
    pub fn has(&self, flag: u32) -> bool {
        self.flags & flag != 0
    }
    pub fn set(&mut self, flag: u32, on: bool) {
        if on {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worked_kinds_and_track_defaults() {
        for (kind, expected) in [0x2a, 0x2a, 0x2a, 0x22, 0x2e, 0x28, 0x68].into_iter().enumerate() {
            let t = TrackInfo::for_tracking(7, kind as u32, 42, None, None);
            assert_eq!(t.flags, expected);
            assert_eq!(t.frame, u32::from(kind == 4));
            assert_eq!(t.fade, if kind == 3 { 1.0 } else { 0.0 });
            assert_eq!((t.flash_accumulator, t.flash_period, t.anchor), (0.0, 1.0, [0.0; 3]));
            assert_eq!(t.image_instance, if kind == 3 || kind == 4 { 42 } else { 0 });
        }
        let t = TrackInfo::for_tracking(7, 3, 42, Some("image"), None);
        assert_eq!((t.image_instance, t.label.as_str()), (0, "image"));
    }
}
