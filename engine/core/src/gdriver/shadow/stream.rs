// SPDX-License-Identifier: GPL-3.0-or-later
//! Strict same-call device transcript replay. Native results are foreign inputs;
//! expected calls and their arguments are produced independently by Driver.
use super::{replay, Snapshot};
use crate::gdriver::backend::{Call, Device, FAIL};
use crate::gdrivers_director::{Allocation, Fault, Host, Op};

pub const LIMIT: usize = 128;

/// Private static-link wire format, mirrored in FC4GDriverTrace.h. Slots are
/// public IDirect3DDevice7 vtable slots, including IUnknown.
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct Record {
    pub target: u32,
    pub slot: u32,
    pub args_len: u32,
    pub data_len: u32,
    pub out_len: u32,
    pub result: u32,
    pub args: [u32; 6],
    pub data: [u32; 16],
    pub out: [u32; 6],
}

fn signature(slot: u32) -> Option<(usize, usize, usize)> {
    Some(match slot {
        5 | 6 => (0, 0, 0),
        10 => (4, 0, 0),
        11 => (1, 64, 0),
        13 => (6, 0, 0),
        15 => (0, 0, 6),
        20 | 35 => (2, 0, 0),
        21 => (1, 0, 1),
        37 => (3, 0, 0),
        _ => return None,
    })
}

struct Replay<'a> {
    records: &'a [Record],
    cursor: usize,
    differences: u32,
    unsupported: bool,
}
impl Host for Replay<'_> {
    fn alloc(&mut self, _: u32, _: Allocation) -> u32 {
        self.unsupported = true;
        0
    }
    fn free(&mut self, _: u32, _: Allocation) {
        self.unsupported = true;
    }
    fn read(&mut self, _: u32, _: &mut [u8]) -> crate::gdrivers_director::Result<()> {
        self.unsupported = true;
        Err(Fault::MissingSlot)
    }
    fn write(&mut self, _: u32, _: &[u8]) -> crate::gdrivers_director::Result<()> {
        self.unsupported = true;
        Err(Fault::MissingSlot)
    }
    fn call(&mut self, _: u32, _: Op, _: &[u32], _: &[u8], _: &mut [u32]) -> u32 {
        self.unsupported = true;
        FAIL
    }
    fn enumerate_child(&mut self, _: u32, _: u32, _: &mut dyn FnMut(u32, u32, u32) -> bool) {
        self.unsupported = true;
    }
}
impl Device for Replay<'_> {
    fn dx(&mut self, target: u32, call: Call, args: &[u32], data: &[u8], out: &mut [u32]) -> u32 {
        let slot = match call {
            Call::BeginScene => 5,
            Call::EndScene => 6,
            Call::Clear => 10,
            Call::Transform => 11,
            Call::Viewport => 13,
            Call::GetViewport => 15,
            Call::RenderState => 20,
            Call::GetRenderState => 21,
            Call::Texture => 35,
            Call::StageState => 37,
            _ => {
                self.unsupported = true;
                return FAIL;
            }
        };
        let record = self.records.get(self.cursor);
        self.cursor += 1;
        let Some(record) = record else {
            self.differences += 1;
            return FAIL;
        };
        let bytes: Vec<u8> = record.data.iter().flat_map(|v| v.to_le_bytes()).collect();
        if record.target != target
            || record.slot != slot
            || args != &record.args[..record.args_len as usize]
            || data != &bytes[..record.data_len as usize]
            || out.len() != record.out_len as usize
        {
            self.differences += 1;
            return FAIL;
        }
        out.copy_from_slice(&record.out[..out.len()]);
        record.result
    }
}

/// None means unmodeled/truncated capture, never an exact sample. In particular
/// queued VB draws require a primitive-queue prestate and cannot be treated as
/// synthetic expectations copied from the observed device stream.
pub fn compare(slot: u32, args: &[u32; 20], before: &Snapshot, after: &Snapshot, records: &[Record]) -> Option<u32> {
    if records.len() > LIMIT
        || records
            .iter()
            .any(|r| signature(r.slot) != Some((r.args_len as usize, r.data_len as usize, r.out_len as usize)))
    {
        return None;
    }
    let mut host = Replay {
        records,
        cursor: 0,
        differences: 0,
        unsupported: false,
    };
    let state = replay(slot, args, before, after, &mut host, true)?;
    if host.unsupported {
        return None;
    }
    Some(state + host.differences + records.len().saturating_sub(host.cursor) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pre() -> Snapshot {
        let mut pre = [0; super::super::WORDS];
        pre[6] = 1;
        pre[48] = 123;
        pre
    }
    fn state(id: u32, value: u32, result: u32) -> Record {
        Record {
            target: 123,
            slot: 20,
            args_len: 2,
            args: [id, value, 0, 0, 0, 0],
            result,
            ..Record::default()
        }
    }
    #[test]
    fn detects_wrong_argument_target_order_and_missing_or_extra_call() {
        let pre = pre();
        let mut args = [0; 20];
        args[..2].copy_from_slice(&[4, 5]);
        let exact = [state(19, 5, 0), state(20, 6, 0)];
        assert_eq!(compare(21, &args, &pre, &pre, &exact), Some(0));
        let mut wrong = exact.clone();
        wrong[0].args[1] ^= 1;
        assert!(compare(21, &args, &pre, &pre, &wrong).unwrap() > 0);
        wrong = exact.clone();
        wrong[0].target ^= 1;
        assert!(compare(21, &args, &pre, &pre, &wrong).unwrap() > 0);
        wrong = exact.clone();
        wrong.reverse();
        assert!(compare(21, &args, &pre, &pre, &wrong).unwrap() > 0);
        assert!(compare(21, &args, &pre, &pre, &exact[..1]).unwrap() > 0);
        assert!(compare(40, &args, &pre, &pre, &exact).unwrap() > 0);
    }
    #[test]
    fn foreign_failure_drives_error_and_queries_drive_return_values() {
        let mut pre = pre();
        pre[43] = 6;
        let args = [0; 20];
        let mut post = pre;
        post[43] = 1;
        assert_eq!(compare(45, &args, &pre, &post, &[state(15, 0, FAIL)]), Some(0));
        assert!(compare(45, &args, &pre, &pre, &[state(15, 0, FAIL)]).unwrap() > 0);
        let query = Record {
            target: 123,
            slot: 21,
            args_len: 1,
            out_len: 1,
            args: [15, 0, 0, 0, 0, 0],
            out: [1, 0, 0, 0, 0, 0],
            ..Record::default()
        };
        post = pre;
        post[49] = 1;
        assert_eq!(compare(46, &args, &pre, &post, &[query]), Some(0));
    }
    #[test]
    fn never_credits_unmodeled_draws_or_invalid_wire_lengths() {
        let pre = pre();
        let args = [0; 20];
        for slot in [28, 29, 49, u32::MAX] {
            let unknown = Record {
                slot,
                ..Record::default()
            };
            assert_eq!(compare(40, &args, &pre, &pre, &[unknown]), None);
        }
        let malformed = Record {
            slot: 20,
            args_len: u32::MAX,
            ..Record::default()
        };
        assert_eq!(compare(40, &args, &pre, &pre, &[malformed]), None);
        assert_eq!(compare(40, &args, &pre, &pre, &vec![state(15, 1, 0); LIMIT + 1]), None);
    }
    #[test]
    fn detects_dirty_cache_and_error_outputs_previously_omitted() {
        let pre = pre();
        let args = [0; 20];
        let mut post = pre;
        post[44] = 1;
        post[45] = 1;
        assert_eq!(compare(28, &args, &pre, &post, &[]), Some(0));
        for index in [43, 44, 45, 46] {
            let mut wrong = post;
            wrong[index] ^= 1;
            assert!(compare(28, &args, &pre, &wrong, &[]).unwrap() > 0);
        }
    }
    #[test]
    fn compares_transform_bits_and_supplies_native_viewport_to_projection() {
        let mut pre = pre();
        pre[4] = 1;
        let mut args = [0; 20];
        args[..16].copy_from_slice(&crate::gdriver::backend::IDENTITY.map(f32::to_bits));
        let viewport = Record {
            target: 123,
            slot: 15,
            out_len: 6,
            out: [0, 0, 800, 600, 0, 0x3f80_0000],
            ..Record::default()
        };
        let projection = crate::gdriver::extended::projection(crate::gdriver::backend::IDENTITY, 800, 600);
        let transform = Record {
            target: 123,
            slot: 11,
            args_len: 1,
            data_len: 64,
            args: [3, 0, 0, 0, 0, 0],
            data: projection.map(f32::to_bits),
            ..Record::default()
        };
        let mut post = pre;
        post[9..25].copy_from_slice(&args[..16]);
        let mut records = [viewport, transform];
        assert_eq!(compare(41, &args, &pre, &post, &records), Some(0));
        records[1].data[12] ^= 1;
        assert!(compare(41, &args, &pre, &post, &records).unwrap() > 0);
        records[1].data[12] ^= 1;
        records[0].out[2] = 1024;
        assert!(compare(41, &args, &pre, &post, &records).unwrap() > 0);
    }
}
