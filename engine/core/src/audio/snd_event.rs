// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cGZSndEventStd` — the sound-event object (`docs/specs/audio.md` §3.1, §5).
//! Section references (`§n`) point at that spec.
//!
//! A sound event is a **message object**: it derives from
//! [`StandardMessage`](crate::audio::message::StandardMessage) and is
//! allocated from the dedicated pool of §4 when — and only when — the
//! requested size is `0x38` *[compat]*.
//!
//! The `+0x2C` interface is a **secondary base whose vptr lives inside the
//! object**, not at `+0x00` *[compat]*: `QueryInterface` (`0x0096BA2B`)
//! answers IID `0x5B523BA4` by returning `this+0x2C` and calling `AddRef`.

use crate::audio::message::StandardMessage;

/// Object size of `cGZSndEventStd`, in bytes (§3.1, measured from
/// `cGZSndEventStd::operator.new` `0x0096BAD1`).
pub const SND_EVENT_SIZE: usize = 0x38;

/// The outer `+0x30` payload word and the `+0x34` "was set" byte live past the
/// `cRZMessage2Standard` base and the secondary vptr (§3.1).
pub const PAYLOAD_OFFSET: usize = 0x30;
/// The `+0x34` flag / count byte.
pub const FLAG_OFFSET: usize = 0x34;
/// The secondary interface's vptr is at `+0x2C` (§3.1).
pub const SECONDARY_VPTR_OFFSET: usize = 0x2C;

/// The message type a `cGZSndEventStd` carries (`kSndEventMsgTypeStd`,
/// §1.3, §5).
pub const MSG_SND_EVENT_STD: u32 = 0xDB53_383B;

/// The IID the `+0x2C` payload interface answers (§1.3, §3.1).
pub const IID_SND_EVENT_PAYLOAD: u32 = 0x5B52_3BA4;

/// A `cGZSndEventStd` (§3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SndEventStd {
    /// The `cRZMessage2Standard` base (`+0x04`…`+0x2B`).
    pub base: StandardMessage,
    /// `+0x30` — the event's payload word (ctor argument 2).
    pub payload: u32,
    /// `+0x34` — the flag / count byte: 1 from the five-argument constructor,
    /// 0 from the default one (§3.1).
    pub flag: u8,
}

impl SndEventStd {
    /// The **default constructor** (`0x0096BB0E`): stamps the message type and
    /// leaves `+0x30` and `+0x34` at 0 (§3.1, conformance test 2).
    pub fn new() -> Self {
        let mut base = StandardMessage::new();
        base.set_type(MSG_SND_EVENT_STD);
        SndEventStd {
            base,
            payload: 0,
            flag: 0,
        }
    }

    /// The **five-argument constructor** (`0x0096BA6D`): stamps the message
    /// type, stores argument 2 at `+0x30` and writes **1** to `+0x34` (§3.1,
    /// conformance test 2). The remaining arguments fill data1..data4 through
    /// the message setters.
    pub fn with_payload(payload: u32, data1: u32, data2: u32, data3: u32, data4: u32) -> Self {
        let mut base = StandardMessage::new();
        base.set_type(MSG_SND_EVENT_STD);
        base.set_data1(data1);
        base.set_data2(data2);
        base.set_data3(data3);
        base.set_data4(data4);
        SndEventStd { base, payload, flag: 1 }
    }

    /// `QueryInterface` (`0x0096BA2B`): IID `0x5B523BA4` answers the secondary
    /// interface at `this+0x2C` and bumps the refcount; every other IID falls
    /// through to `cRZMessage2Standard::QueryInterface` (§3.1, conformance
    /// test 3). Returns `Some(offset)` of the interface that answered.
    pub fn query_interface(&mut self, iid: u32) -> Option<usize> {
        if iid == IID_SND_EVENT_PAYLOAD {
            self.base.add_ref();
            Some(SECONDARY_VPTR_OFFSET)
        } else {
            // Falls through to the base: `cRZMessage2Standard::QueryInterface`
            // answers only its own IIDs. This module models that as "no
            // interface here"; the caller decides what the base would do.
            None
        }
    }

    // ---- the `+0x2C` payload interface (§3.1) -----------------------------
    //
    // The secondary interface has seven methods; slot 7 is a null word. The
    // four real ones read and write the **outer** object's fields, which is
    // why they are on `SndEventStd` and not on a separate type.

    /// Slot 3 (`0x00891D96`, `mov al, [ecx+8]`): **GetFlag** → the outer
    /// `+0x34` byte.
    pub fn get_flag(&self) -> u8 {
        self.flag
    }

    /// Slot 4 (`0x009D7E63`, `mov eax, [ecx+4]`): **GetValue** → the outer
    /// `+0x30` dword.
    pub fn get_value(&self) -> u32 {
        self.payload
    }

    /// Slot 5 (`0x0096BA02`, `[ecx+4] = arg; [ecx+8] = 1`): **SetValue(u32)**
    /// writes `v` to the outer `+0x30` and marks it set (§3.1).
    pub fn set_value(&mut self, v: u32) {
        self.payload = v;
        self.flag = 1;
    }

    /// Slot 6 (`0x0096BA27`, `lea eax, [ecx-0x2C]`): **GetOuter** → `this -
    /// 0x2C`. Modelled as the outer object's identity: the secondary
    /// interface address minus `0x2C` (§3.1, conformance test 3).
    pub fn get_outer(secondary_addr: u32) -> u32 {
        secondary_addr.wrapping_sub(SECONDARY_VPTR_OFFSET as u32)
    }
}

impl Default for SndEventStd {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Conformance test 2: both constructors stamp `0xDB53383B`, and differ in
    /// `+0x30` / `+0x34`.
    #[test]
    fn both_constructors_stamp_the_type() {
        let d = SndEventStd::new();
        assert_eq!(d.base.msg_type, MSG_SND_EVENT_STD);
        assert_eq!(d.base.flags & 0x01, 0x01, "presence bit 0 is set");
        assert_eq!(d.payload, 0, "default ctor leaves +0x30 at 0");
        assert_eq!(d.flag, 0, "default ctor leaves +0x34 at 0");

        let f = SndEventStd::with_payload(0x00C0_FFEE, 1, 2, 3, 4);
        assert_eq!(f.base.msg_type, MSG_SND_EVENT_STD);
        assert_eq!(f.payload, 0x00C0_FFEE, "five-arg ctor stores arg 2 at +0x30");
        assert_eq!(f.flag, 1, "five-arg ctor writes 1 to +0x34");
        assert_eq!(f.base.data1, 1);
        assert_eq!(f.base.data2, 2);
        assert_eq!(f.base.data3, 3);
        assert_eq!(f.base.data4, 4);
    }

    /// Conformance test 3: QI answers `0x5B523BA4` at `this+0x2C` and bumps
    /// the refcount; another IID falls through.
    #[test]
    fn query_interface_answers_the_secondary_base() {
        let mut e = SndEventStd::new();
        let before = e.base.refcount;
        assert_eq!(e.query_interface(IID_SND_EVENT_PAYLOAD), Some(SECONDARY_VPTR_OFFSET));
        assert_eq!(e.base.refcount, before + 1, "QI calls AddRef");
        assert_eq!(e.query_interface(0x1234_5678), None, "any other IID falls through");
        assert_eq!(e.base.refcount, before + 1, "a fall-through does not AddRef");
    }

    /// Conformance test 3: SetValue / GetValue / GetFlag / GetOuter.
    #[test]
    fn payload_interface_reads_and_writes_the_outer_fields() {
        let mut e = SndEventStd::new();
        assert_eq!(e.get_value(), 0);
        assert_eq!(e.get_flag(), 0);
        e.set_value(0x00AB_CDEF);
        assert_eq!(e.get_value(), 0x00AB_CDEF);
        assert_eq!(e.get_flag(), 1, "SetValue marks +0x34 set");
        assert_eq!(SndEventStd::get_outer(0x00B0_002C), 0x00B0_0000);
    }
}
