// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded, isolated native transition replay. The pre-call snapshot is an input;
//! the strict stream backend additionally checks sampled device outputs.
use super::{backend::*, stage::Stage, vertex, Driver};
use crate::gdrivers_director::{Allocation, Host, Op};

pub mod stream;

pub const WORDS: usize = 72;
pub type Snapshot = [u32; WORDS];

struct Sink {
    viewport: [u32; 6],
    render: [u32; 64],
}
impl Default for Sink {
    fn default() -> Self {
        Self {
            viewport: [0; 6],
            render: [0; 64],
        }
    }
}
impl Host for Sink {
    fn alloc(&mut self, _: u32, _: Allocation) -> u32 {
        0
    }
    fn free(&mut self, _: u32, _: Allocation) {}
    fn read(&mut self, _: u32, _: &mut [u8]) -> crate::gdrivers_director::Result<()> {
        Ok(())
    }
    fn write(&mut self, _: u32, _: &[u8]) -> crate::gdrivers_director::Result<()> {
        Ok(())
    }
    fn call(&mut self, _: u32, _: Op, _: &[u32], _: &[u8], _: &mut [u32]) -> u32 {
        0
    }
    fn enumerate_child(&mut self, _: u32, _: u32, _: &mut dyn FnMut(u32, u32, u32) -> bool) {}
}
impl Device for Sink {
    fn dx(&mut self, _: u32, call: Call, args: &[u32], _: &[u8], out: &mut [u32]) -> u32 {
        match call {
            Call::Viewport => self.viewport.copy_from_slice(args),
            Call::GetViewport => out.copy_from_slice(&self.viewport),
            Call::RenderState if (args[0] as usize) < self.render.len() => {
                self.render[args[0] as usize] = args[1];
            }
            Call::GetRenderState if (args[0] as usize) < self.render.len() => {
                out[0] = self.render[args[0] as usize];
            }
            _ => (),
        }
        0
    }
}

/// Returns the number of differing measured words, or None for unsupported input.
/// Words 0..8 are owner scalars, 9..24 projection, 25..42 cached stage fields,
/// 49 the observed query output, 50..55 full viewport, 56..59 queried rectangle.
/// Dirty/applied caches and unsupplied foreign effects are excluded.
pub fn compare(slot: u32, args: &[u32; 20], before: &Snapshot, after: &Snapshot) -> Option<u32> {
    replay(slot, args, before, after, &mut Sink::default(), false)
}

fn replay(
    slot: u32,
    args: &[u32; 20],
    before: &Snapshot,
    after: &Snapshot,
    host: &mut impl Device,
    recorded: bool,
) -> Option<u32> {
    let mut d = Driver::new(0, 0);
    d.clear_color = before[0];
    d.clear_depth = f32::from_bits(before[1]);
    d.clear_stencil = before[2];
    d.row_length = before[3];
    d.matrix_mode = before[4];
    d.current_stage = before[5] as usize;
    if before[6] > 8 || before[5] >= before[6] {
        return None;
    }
    d.stages = vec![Stage::default(); before[6] as usize];
    d.current.set_byte(0x29, before[7] as u8);
    d.lost = before[8] != 0;
    d.full_viewport.copy_from_slice(&before[50..56]);
    d.projection = std::array::from_fn(|i| f32::from_bits(before[9 + i]));
    let selected = if recorded { before[63] as usize } else { d.current_stage };
    let s = d.stages.get_mut(selected)?;
    s.enabled = before[25] != 0;
    s.environment = before[26];
    s.function.copy_from_slice(&before[27..29]);
    for c in 0..2 {
        s.sources[c].copy_from_slice(&before[29 + c * 3..32 + c * 3]);
        s.operands[c].copy_from_slice(&before[35 + c * 3..38 + c * 3]);
    }
    s.scale.copy_from_slice(&before[41..43]);
    if recorded {
        d.error = before[43];
        d.stages_dirty = before[44] != 0;
        s.dirty = before[45] != 0;
        s.applied = (before[46] != u32::MAX).then_some(before[46]);
        d.current.set_byte(0x21, before[47] as u8);
        d.device = before[48];
        s.alpha = before[60] != 0;
        s.name = before[61];
        s.parameters.copy_from_slice(&before[64..68]);
        s.coordinates = before[68];
    }
    let mut expected = *before;
    let range = match slot {
        6 | 8..=10 => {
            expected[49] = match slot {
                6 => vertex::pack(args[0]),
                8 => vertex::stride(args[0]),
                9 => vertex::offset(args[0], args[1] as usize, args[2]).ok()?,
                _ => vertex::elements(args[0], args[1] as usize).ok()?,
            };
            49..50
        }
        12 | 14 => {
            d.state(slot as u8, args, host).ok()?;
            expected[0] = d.clear_color;
            expected[2] = d.clear_stencil;
            0..3
        }
        13 => {
            d.clear_depth(f64::from_bits(u64::from(args[0]) | (u64::from(args[1]) << 32)), host);
            expected[1] = d.clear_depth.to_bits();
            0..3
        }
        26 => {
            d.pixel_store(args[0], args[1]);
            expected[3] = d.row_length;
            3..4
        }
        40 => {
            d.matrix_mode(args[0]);
            expected[4] = d.matrix_mode;
            4..5
        }
        41 => {
            d.load_matrix(std::array::from_fn(|i| f32::from_bits(args[i])), host);
            for i in 0..16 {
                expected[9 + i] = d.projection[i].to_bits();
            }
            9..25
        }
        44 | 45 if matches!(args[0], 0..=5 | 7) => {
            d.capability(args[0], slot == 44, host).ok()?;
            if args[0] == 5 {
                expected[25] = u32::from(d.stages[d.current_stage].enabled);
                25..43
            } else if !recorded {
                expected[49] = u32::from(d.is_enabled(args[0], host).ok()?);
                49..50
            } else {
                0..0
            }
        }
        46 if args[0] <= 7 => {
            expected[49] = u32::from(d.is_enabled(args[0], host).ok()?);
            49..50
        }
        48 => {
            d.get_integer(args[0], &mut expected[49]);
            49..50
        }
        51 => {
            d.tex_stage(args[0]);
            expected[5] = d.current_stage as u32;
            5..6
        }
        74 | 75 => {
            let rect = (slot == 74).then(|| [args[0], args[1], args[2], args[3]]);
            d.set_viewport(rect, host);
            if !recorded {
                expected[56..60].copy_from_slice(&d.get_viewport(host));
            }
            56..60
        }
        28 | 54..=57 | 62 => {
            match slot {
                28 => d.tex_env(args[2]).ok()?,
                62 => {
                    // The plugin snapshots the explicitly selected stage, rather
                    // than pretending SetCombiner uses the current stage.
                    let bytes = std::array::from_fn(|i| args[i] as u8);
                    d.set_combiner(&bytes, selected, host).ok()?;
                }
                _ => d.combine_field(slot as u8, args[0] as usize, args[1]).ok()?,
            }
            let s = &d.stages[selected];
            expected[26] = s.environment;
            expected[27..29].copy_from_slice(&s.function);
            for c in 0..2 {
                expected[29 + c * 3..32 + c * 3].copy_from_slice(&s.sources[c]);
                expected[35 + c * 3..38 + c * 3].copy_from_slice(&s.operands[c]);
            }
            expected[41..43].copy_from_slice(&s.scale);
            25..43
        }
        11 => {
            d.clear(args[0], host);
            0..0
        }
        15..=23 | 67 => {
            d.state(slot as u8, args, host).ok()?;
            0..0
        }
        27 => {
            d.tex_env_color(std::array::from_fn(|i| f32::from_bits(args[i])), host);
            0..0
        }
        42 => {
            d.load_identity(host);
            0..0
        }
        47 | 49 => 0..0,
        68..=73 => {
            d.blit_unsupported();
            0..0
        }
        50 => {
            expected[49] = d.get_error();
            0..0
        }
        52 => {
            d.tex_stage_coord(args[0], host);
            0..0
        }
        53 => {
            let matrix = (args[16] != 0).then(|| std::array::from_fn(|i| f32::from_bits(args[i])));
            d.tex_stage_matrix(matrix, args[17], args[19], host);
            0..0
        }
        76 => {
            expected[56..60].copy_from_slice(&d.get_viewport(host));
            0..0
        }
        78 => {
            expected[49] = d.class_id();
            0..0
        }
        81 => {
            expected[49] = u32::from(d.is_device_ready());
            0..0
        }
        82 => {
            expected[49] = 0;
            0..0
        }
        _ => return None,
    };
    if recorded {
        expected[0] = d.clear_color;
        expected[1] = d.clear_depth.to_bits();
        expected[2] = d.clear_stencil;
        expected[3] = d.row_length;
        expected[4] = d.matrix_mode;
        expected[5] = d.current_stage as u32;
        expected[9..25].copy_from_slice(&d.projection.map(f32::to_bits));
        let s = &d.stages[selected];
        expected[25] = u32::from(s.enabled);
        expected[43] = d.error;
        expected[44] = u32::from(d.stages_dirty);
        expected[45] = u32::from(s.dirty);
        expected[46] = s.applied.unwrap_or(u32::MAX);
        return Some(expected.iter().zip(after).filter(|(a, b)| a != b).count() as u32);
    }
    Some(range.filter(|&i| expected[i] != after[i]).count() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combines_use_capability_and_distinguish_alpha_selector_gap() {
        let mut pre = [0; WORDS];
        pre[6] = 1;
        let mut args = [0; 20];
        args[0] = 4;
        args[1] = 7;
        let mut post = pre;
        post[32] = 7;
        assert_eq!(compare(56, &args, &pre, &post), Some(0));
        assert_eq!(compare(55, &args, &pre, &post), Some(1));
        pre[7] = 1;
        post = pre;
        post[38] = 7;
        assert_eq!(compare(55, &args, &pre, &post), Some(0));
        args[0] = 3;
        assert_eq!(compare(55, &args, &pre, &pre), Some(0));
    }
    #[test]
    fn projection_capture_does_not_claim_device_transform_coverage() {
        let mut pre = [0; WORDS];
        pre[6] = 1;
        let mut args = [0; 20];
        args[..16].copy_from_slice(&IDENTITY.map(f32::to_bits));
        assert_eq!(compare(41, &args, &pre, &pre), Some(0));
        pre[4] = 1;
        let mut post = pre;
        post[9..25].copy_from_slice(&args[..16]);
        assert_eq!(compare(41, &args, &pre, &post), Some(0));
        post[9] ^= 1;
        assert_eq!(compare(41, &args, &pre, &post), Some(1));
    }
    #[test]
    fn viewport_clamps_unsigned_coordinates_and_queries_bottom_up_edges() {
        let mut pre = [0; WORDS];
        pre[6] = 1;
        pre[52] = 800;
        pre[53] = 600;
        let mut args = [0; 20];
        args[..4].copy_from_slice(&[100, 50, 200, 100]);
        let mut post = pre;
        post[56..60].copy_from_slice(&[100, 50, 300, 150]);
        assert_eq!(compare(74, &args, &pre, &post), Some(0));
        args[0] = u32::MAX;
        post[56..60].copy_from_slice(&[800, 50, 800, 150]);
        assert_eq!(compare(74, &args, &pre, &post), Some(0));
        pre[8] = 1;
        post[56..60].fill(0);
        assert_eq!(compare(74, &args, &pre, &post), Some(0));
    }
}
