// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The bridge type choice (`FindBestBridgeType`, §6.1) and the padding
//! (`PadStepsToMakeMinBridgeSpan`, §6.2).

use crate::network_geometry::drag::StepKind;
use crate::network_geometry::Run;

use super::span::SpanWorld;
use super::types::BridgeType;
use super::{regular3, BridgeTool};

/// A candidate of §6.1, with its score.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeCandidate {
    /// The index into the network type's bridge type vector.
    pub index: u32,
    /// `d := n − minSpan`, and `d := d − 100000` when `d < 0`.
    pub score: i64,
}

/// `PadStepsToMakeMinBridgeSpan` (`0x0062ECD0`, §6.2): alternate one step
/// back, then one step forward, until the span is at least `min_len`.
///
/// Padding does not test for occupants. `v` is the tool's `+0x78`; `straight`
/// its `+0x74`.
pub fn pad_steps_to_make_min_bridge_span(w: &SpanWorld<'_>, min_len: u32, start: &mut u32, end: &mut u32) -> bool {
    let run: &Run = w.run;
    if end.saturating_sub(*start) + 1 >= min_len {
        return true;
    }
    let mut back_open = true;
    let mut forward_open = true;
    while end.saturating_sub(*start) + 1 < min_len {
        let mut moved = false;
        // Each direction runs only while steps are still needed (6.2): a
        // round that satisfies the span on its back step does not also take a
        // forward one.
        if end.saturating_sub(*start) + 1 >= min_len {
            break;
        }
        // Back: allowed while `start > 1`, steps `start` and `start − 1` are
        // kind 0 and step `start` passes the edge test.
        if back_open {
            let s = *start - 1;
            if s > 1
                && s < run.steps.len() as u32
                && regular3(run, s)
                && s > 0
                && run.steps.get(s as usize).is_some_and(|st| st.kind == StepKind::Regular)
                && w.edge_ok(s)
                && run
                    .steps
                    .get(s as usize - 1)
                    .is_some_and(|st| st.kind == StepKind::Regular)
            {
                *start = s;
                moved = true;
            } else {
                back_open = false;
            }
        }
        // Forward: allowed while `end < v`, step `end` is kind 0 and passes
        // the edge test.
        if forward_open && end.saturating_sub(*start) + 1 < min_len {
            let e = *end + 1;
            if e < w.valid
                && e < run.steps.len() as u32
                && run.steps.get(e as usize).is_some_and(|st| st.kind == StepKind::Regular)
                && w.edge_ok(e)
            {
                *end = e;
                moved = true;
            } else {
                forward_open = false;
            }
        }
        if !moved {
            return false;
        }
    }
    // At the end: true when `end < +0x74` and the span is at least `minLen`.
    *end < w.straight && end.saturating_sub(*start) + 1 >= min_len
}

/// `FindBestBridgeType` (`0x0062F760`, §6.1): the candidate list is the
/// tool's water list when `water` is true, else its land list. Returns the
/// winner's index into the bridge type vector, or `None` when no candidate
/// fits.
///
/// `h` is the clearance (§5.2). The winner's padding is applied to `start`
/// and `end` when its score is negative.
pub fn find_best_bridge_type(
    tool: &BridgeTool,
    w: &SpanWorld<'_>,
    start: &mut u32,
    end: &mut u32,
    water: bool,
    h: f32,
) -> Option<u32> {
    let list = if water { &tool.water } else { &tool.land };
    let n = end.saturating_sub(*start) + 1;
    let mut candidates: Vec<BridgeCandidate> = Vec::new();
    for &idx in list {
        let Some(t) = tool.types.get(idx as usize) else {
            continue;
        };
        // A preferred bridge is set (`+0x200 ≥ 0`) and this is not it.
        if tool.preferred >= 0 && idx as i32 != tool.preferred {
            continue;
        }
        // The clearance range, both ends inclusive.
        if h < t.min_clearance || h > t.max_clearance {
            continue;
        }
        // Too long.
        if t.max_span < n {
            continue;
        }
        // Too short, and padding fails on **copies** of start and end.
        if t.min_span > n {
            let (mut s, mut e) = (*start, *end);
            if !pad_steps_to_make_min_bridge_span(w, t.min_span, &mut s, &mut e) {
                continue;
            }
        }
        // `d := n − minSpan`, and `d := d − 100000` when `d < 0`.
        let d = i64::from(n) - i64::from(t.min_span);
        let score = if d < 0 { d - 100_000 } else { d };
        candidates.push(BridgeCandidate { index: idx, score });
    }
    if candidates.is_empty() {
        return None;
    }
    // The smallest `|d|` wins; the **first** candidate in list order wins a
    // tie.
    let mut best = 0usize;
    for (k, c) in candidates.iter().enumerate().skip(1) {
        if c.score.abs() < candidates[best].score.abs() {
            best = k;
        }
    }
    let winner = candidates[best];
    // If the winner's `d < 0`, pad the **real** start and end (the result is
    // ignored).
    if winner.score < 0 {
        if let Some(t) = tool.types.get(winner.index as usize) {
            let _ = pad_steps_to_make_min_bridge_span(w, t.min_span, start, end);
        }
    }
    Some(winner.index)
}

/// §6.1 with the candidate list written back into the tool (`+0x1F4`), as
/// the real call does. Returns the winner's index.
pub fn find_best_bridge_type_recording(
    tool: &mut BridgeTool,
    w: &SpanWorld<'_>,
    start: &mut u32,
    end: &mut u32,
    water: bool,
    h: f32,
) -> Option<u32> {
    let list = if water { tool.water.clone() } else { tool.land.clone() };
    let n = end.saturating_sub(*start) + 1;
    let mut candidates: Vec<u32> = Vec::new();
    for &idx in &list {
        let Some(t) = tool.types.get(idx as usize) else {
            continue;
        };
        if tool.preferred >= 0 && idx as i32 != tool.preferred {
            continue;
        }
        if h < t.min_clearance || h > t.max_clearance {
            continue;
        }
        if t.max_span < n {
            continue;
        }
        if t.min_span > n {
            let (mut s, mut e) = (*start, *end);
            if !pad_steps_to_make_min_bridge_span(w, t.min_span, &mut s, &mut e) {
                continue;
            }
        }
        candidates.push(idx);
    }
    // `+0x1F4` is replaced by the candidates of **this** call.
    tool.candidates = candidates.clone();
    if candidates.is_empty() {
        return None;
    }
    let mut best: Option<(u32, i64)> = None;
    for &idx in &candidates {
        let t = &tool.types[idx as usize];
        let d = i64::from(n) - i64::from(t.min_span);
        let score = if d < 0 { d - 100_000 } else { d };
        let better = match best {
            None => true,
            Some((_, bs)) => score.abs() < bs.abs(),
        };
        if better {
            best = Some((idx, score));
        }
    }
    let (idx, score) = best?;
    if score < 0 {
        let min = tool.types[idx as usize].min_span;
        let _ = pad_steps_to_make_min_bridge_span(w, min, start, end);
    }
    Some(idx)
}

/// §6.1's score, exposed for the tests.
pub fn candidate_score(n: u32, min_span: u32) -> i64 {
    let d = i64::from(n) - i64::from(min_span);
    if d < 0 {
        d - 100_000
    } else {
        d
    }
}

/// The type a candidate index names.
pub fn type_at(tool: &BridgeTool, index: u32) -> Option<&BridgeType> {
    tool.types.get(index as usize)
}
