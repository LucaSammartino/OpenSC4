// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cRZMessage2Standard` — the message payload every sound message carries
//! (`docs/specs/audio.md` §3.2). Object size **`0x2C` (44 bytes)**, vtable
//! `0x00ACEC00`. Section references (`§n`) point at that spec.
//!
//! The setters each OR one bit into the presence-flag byte at `+0x08`. That
//! byte is the compatibility surface a replacement must reproduce *[compat]*.

/// Object size of `cRZMessage2Standard`, in bytes (§3.2).
pub const MESSAGE_SIZE: usize = 0x2C;

/// Presence bit of the `type` field (`+0x0C`), setter slot 5 (§3.2).
pub const PRESENCE_TYPE: u8 = 0x01;
/// Presence bit of `data1` (`+0x10`), setter slot 11 (§3.2).
pub const PRESENCE_DATA1: u8 = 0x02;
/// Presence bit of `data2` (`+0x14`), setter slot 16 (§3.2).
pub const PRESENCE_DATA2: u8 = 0x04;
/// Presence bit of `data3` (`+0x18`), setter slot 21 (§3.2).
pub const PRESENCE_DATA3: u8 = 0x08;
/// Presence bit of `data4` (`+0x1C`), setter slot 26 (§3.2, *[inferred]* bit).
pub const PRESENCE_DATA4: u8 = 0x10;
/// Presence bit of `void1` / the target (`+0x24`), setter slot 33 (§3.2).
pub const PRESENCE_VOID1: u8 = 0x80;

/// A `cRZMessage2Standard` object: the 0x2C-byte message payload of §3.2.
///
/// The offsets below are the original's; they are kept as named fields here
/// and are re-exported through [`MessageView`] so a vector can assert on the
/// raw layout without a C object.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StandardMessage {
    /// `+0x04` — the `cRZRefCount` reference count.
    pub refcount: u32,
    /// `+0x08` — the presence-flag byte.
    pub flags: u8,
    /// `+0x0C` — the message type.
    pub msg_type: u32,
    /// `+0x10` — data1.
    pub data1: u32,
    /// `+0x14` — data2.
    pub data2: u32,
    /// `+0x18` — data3.
    pub data3: u32,
    /// `+0x1C` — data4.
    pub data4: u32,
    /// `+0x24` — void1, the target pointer (held as a host token).
    pub void1: u64,
}

impl StandardMessage {
    /// An object as the default constructor leaves it: zeroed, one live
    /// reference, no presence bits (§3.2).
    pub const fn new() -> Self {
        Self {
            refcount: 1,
            flags: 0,
            msg_type: 0,
            data1: 0,
            data2: 0,
            data3: 0,
            data4: 0,
            void1: 0,
        }
    }

    /// `SetType` (setter slot 5, `0x009132DB`): store the type and set bit 0 —
    /// **except** that type 0 is treated as "no type" and *clears* bit 0
    /// *[quirk]* (§3.2, §14.4).
    pub fn set_type(&mut self, t: u32) {
        self.msg_type = t;
        if t == 0 {
            self.flags &= !PRESENCE_TYPE;
        } else {
            self.flags |= PRESENCE_TYPE;
        }
    }

    /// `SetData1` (setter slot 11, `0x00913322`): store and set bit 1 (§3.2).
    pub fn set_data1(&mut self, v: u32) {
        self.data1 = v;
        self.flags |= PRESENCE_DATA1;
    }

    /// `SetData2` (setter slot 16, `0x00913330`): store and set bit 2 (§3.2).
    pub fn set_data2(&mut self, v: u32) {
        self.data2 = v;
        self.flags |= PRESENCE_DATA2;
    }

    /// `SetData3` (setter slot 21, `0x0091333E`): store and set bit 3 (§3.2).
    pub fn set_data3(&mut self, v: u32) {
        self.data3 = v;
        self.flags |= PRESENCE_DATA3;
    }

    /// `SetData4` (setter slot 26, `0x0091334C`): store and set bit 4
    /// (§3.2, the bit is *[inferred]*).
    pub fn set_data4(&mut self, v: u32) {
        self.data4 = v;
        self.flags |= PRESENCE_DATA4;
    }

    /// `SetVoid1` (setter slot 33, `0x0091346B`): set bit 7 when the pointer is
    /// non-null and **clear** it otherwise, then write `+0x24` (§3.2). A null
    /// target therefore leaves the field written but unmarked.
    pub fn set_void1(&mut self, p: u64) {
        if p != 0 {
            self.flags |= PRESENCE_VOID1;
        } else {
            self.flags &= !PRESENCE_VOID1;
        }
        self.void1 = p;
    }

    /// `AddRef` (slot 1).
    pub fn add_ref(&mut self) {
        self.refcount = self.refcount.wrapping_add(1);
    }

    /// `Release` (slot 2): decrement; returns `true` when the count reached 0
    /// and the caller must free the object.
    pub fn release(&mut self) -> bool {
        self.refcount = self.refcount.wrapping_sub(1);
        self.refcount == 0
    }
}

/// The raw layout of a [`StandardMessage`], for a vector that asserts on the
/// offsets of §3.2 rather than on the typed fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MessageView {
    /// `+0x08`.
    pub flags: u8,
    /// `+0x0C`.
    pub msg_type: u32,
    /// `+0x10`.
    pub data1: u32,
    /// `+0x14`.
    pub data2: u32,
    /// `+0x18`.
    pub data3: u32,
    /// `+0x1C`.
    pub data4: u32,
    /// `+0x24`.
    pub void1: u64,
}

impl From<&StandardMessage> for MessageView {
    fn from(m: &StandardMessage) -> Self {
        MessageView {
            flags: m.flags,
            msg_type: m.msg_type,
            data1: m.data1,
            data2: m.data2,
            data3: m.data3,
            data4: m.data4,
            void1: m.void1,
        }
    }
}

/// `std::size_of` the original compares against in `cGZSndEventStd::operator
/// new` is `0x38`; the message base alone is `0x2C` (§3.1, §3.2).
#[cfg(test)]
mod tests {
    use super::*;

    /// The four setters of a play message must leave `+0x08` at `0x87`
    /// (conformance test 4).
    #[test]
    fn play_shape_flags_are_0x87() {
        let mut m = StandardMessage::new();
        m.set_type(0x8A38_5DD8);
        m.set_data1(0x1234_5678);
        m.set_data2(0x0000_002A);
        m.set_void1(0x00B0_0000);
        assert_eq!(
            m.flags,
            PRESENCE_TYPE | PRESENCE_DATA1 | PRESENCE_DATA2 | PRESENCE_VOID1
        );
        assert_eq!(m.flags, 0x87);
    }

    /// Type 0 stores 0 and clears bit 0 (§14.4).
    #[test]
    fn set_type_zero_clears_the_bit() {
        let mut m = StandardMessage::new();
        m.set_type(0x8A38_5DD8);
        assert_eq!(m.flags & PRESENCE_TYPE, PRESENCE_TYPE);
        m.set_type(0);
        assert_eq!(m.msg_type, 0);
        assert_eq!(m.flags & PRESENCE_TYPE, 0);
    }

    /// A null void1 clears bit 7 but still writes the field (§3.2).
    #[test]
    fn set_void1_null_clears_the_bit() {
        let mut m = StandardMessage::new();
        m.set_void1(0x1122_3344);
        assert_eq!(m.flags & PRESENCE_VOID1, PRESENCE_VOID1);
        assert_eq!(m.void1, 0x1122_3344);
        m.set_void1(0);
        assert_eq!(m.flags & PRESENCE_VOID1, 0);
        assert_eq!(m.void1, 0);
    }

    /// The typed fields must line up with the raw offsets of §3.2.
    #[test]
    fn view_matches_the_offsets() {
        let mut m = StandardMessage::new();
        m.set_type(1);
        m.set_data1(2);
        m.set_data2(3);
        m.set_data3(4);
        m.set_data4(5);
        m.set_void1(6);
        let v = MessageView::from(&m);
        assert_eq!(v.msg_type, 1);
        assert_eq!(v.data1, 2);
        assert_eq!(v.data2, 3);
        assert_eq!(v.data3, 4);
        assert_eq!(v.data4, 5);
        assert_eq!(v.void1, 6);
    }
}
