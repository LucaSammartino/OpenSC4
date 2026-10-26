// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4VertexHtConstraintSatisfier` (§9.1): one height per vertex under
//! range, fixed, equality, slope and smoothness constraints, with optional
//! preferred values and ranges.
//!
//! Every arithmetic result is a float (`r24`, foundations §18).

use std::collections::{BTreeSet, HashMap};

use crate::x87::r24;

/// The inversion tolerance of a range (`0x00AA8A74`, §9.1).
pub const RANGE_TOLERANCE: f32 = 0.01;

/// One constraint (§9.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Constraint {
    /// `h(a) = h(b)`.
    Equality(i32, i32),
    /// `|h(a) − h(b)| ≤ s`.
    Slope(i32, i32, f32),
    /// `|h(a) − 2·h(b) + h(c)| ≤ s`, `b` the middle vertex.
    Smoothness(i32, i32, i32, f32),
}

impl Constraint {
    fn vertices(&self) -> Vec<i32> {
        match *self {
            Constraint::Equality(a, b) | Constraint::Slope(a, b, _) => vec![a, b],
            Constraint::Smoothness(a, b, c, _) => vec![a, b, c],
        }
    }
}

/// One vertex's record (§9.1 "per vertex").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VertexRecord {
    pub id: i32,
    pub min: f32,
    pub max: f32,
    pub preferred: Option<f32>,
    pub preferred_range: Option<(f32, f32)>,
    /// The height given to `AddVertexToSolve`.
    pub initial: f32,
    pub solved: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct VertexHtConstraintSatisfier {
    initialized: bool,
    lo: f32,
    hi: f32,
    /// Stable record indices; pick iteration is the SGI bucket order below.
    recs: Vec<VertexRecord>,
    buckets: Vec<Vec<usize>>,
    index: HashMap<i32, usize>,
    cons: Vec<Constraint>,
    /// Per record index, the constraints naming it.
    adj: Vec<Vec<usize>>,
    /// Slope constraints by unordered pair (a second one keeps the smaller s).
    slopes: HashMap<(i32, i32), usize>,
    solution: Vec<(i32, f32)>,
}

impl Default for VertexHtConstraintSatisfier {
    fn default() -> Self {
        Self {
            initialized: false,
            lo: 0.0,
            hi: 0.0,
            recs: Vec::new(),
            buckets: vec![Vec::new(); 193],
            index: HashMap::new(),
            cons: Vec::new(),
            adj: Vec::new(),
            slopes: HashMap::new(),
            solution: Vec::new(),
        }
    }
}

/// `max(a, b)`, `min(a, b)` on floats with the first operand kept on ties
/// and NaN.
fn fmax(a: f32, b: f32) -> f32 {
    if b > a {
        b
    } else {
        a
    }
}
fn fmin(a: f32, b: f32) -> f32 {
    if b < a {
        b
    } else {
        a
    }
}

fn add(a: f32, b: f32) -> f32 {
    r24(f64::from(a) + f64::from(b)) as f32
}
fn sub(a: f32, b: f32) -> f32 {
    r24(f64::from(a) - f64::from(b)) as f32
}

/// Intersects `[min, max]` with `[a, b]`. An inversion by at most 0.01
/// collapses to the lower bound (§9.1, the range constraint) [inferred: the
/// result's minimum; the same rule is used inside propagation]. Returns the
/// new range; `min > max` means empty.
pub fn intersect(min: f32, max: f32, a: f32, b: f32) -> (f32, f32) {
    let lo = fmax(min, a);
    let mut hi = fmin(max, b);
    if lo > hi && sub(lo, hi) <= RANGE_TOLERANCE {
        hi = lo;
    }
    (lo, hi)
}

impl VertexHtConstraintSatisfier {
    pub fn new() -> VertexHtConstraintSatisfier {
        VertexHtConstraintSatisfier::default()
    }

    /// `Init(lo, hi)`: the default range; a second `Init` before `Shutdown`
    /// does nothing.
    pub fn init(&mut self, lo: f32, hi: f32) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        self.lo = lo;
        self.hi = hi;
    }

    /// `Shutdown`: forgets everything.
    pub fn shutdown(&mut self) {
        // NG1: clear nodes, retaining the grown bucket count for the tool.
        let mut buckets = std::mem::take(&mut self.buckets);
        for bucket in &mut buckets {
            bucket.clear();
        }
        *self = Self {
            buckets,
            ..Self::default()
        };
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn default_range(&self) -> (f32, f32) {
        (self.lo, self.hi)
    }

    pub fn vertex_count(&self) -> usize {
        self.recs.len()
    }

    pub fn constraint_count(&self) -> usize {
        self.cons.len()
    }

    pub fn constraints(&self) -> &[Constraint] {
        &self.cons
    }

    pub fn record(&self, v: i32) -> Option<&VertexRecord> {
        self.index.get(&v).map(|&i| &self.recs[i])
    }

    pub fn records(&self) -> &[VertexRecord] {
        &self.recs
    }

    pub fn contains(&self, v: i32) -> bool {
        self.index.contains_key(&v)
    }

    /// The current allowed range of `v`.
    pub fn range(&self, v: i32) -> Option<(f32, f32)> {
        self.record(v).map(|r| (r.min, r.max))
    }

    /// `GetHeight(v)`: the solved height.
    pub fn height(&self, v: i32) -> Option<f32> {
        self.record(v).and_then(|r| r.solved)
    }

    /// The solved vertices in solution order (§9.7).
    pub fn solution(&self) -> &[(i32, f32)] {
        &self.solution
    }

    /// `AddVertexToSolve(v, h)`: range [lo, hi], initial height `h`. Adding
    /// a vertex twice keeps the first record. Returns whether it was new.
    pub fn add_vertex(&mut self, v: i32, initial: f32) -> bool {
        // SGI insert resizes before checking for a duplicate (NG1).
        if self.recs.len() + 1 > self.buckets.len() {
            let count = [
                389, 769, 1543, 3079, 6151, 12289, 24593, 49157, 98317, 196613, 393241, 786433, 1572869, 3145739,
            ]
            .into_iter()
            .find(|&n| n > self.recs.len())
            .expect("network vertex inventory exceeds specified SGI bucket table");
            let mut buckets = vec![Vec::new(); count];
            for bucket in &self.buckets {
                for &i in bucket {
                    let b = self.recs[i].id as u32 as usize % count;
                    buckets[b].insert(0, i);
                }
            }
            self.buckets = buckets;
        }
        if self.index.contains_key(&v) {
            return false;
        }
        self.index.insert(v, self.recs.len());
        let bucket = v as u32 as usize % self.buckets.len();
        self.buckets[bucket].insert(0, self.recs.len());
        self.recs.push(VertexRecord {
            id: v,
            min: self.lo,
            max: self.hi,
            preferred: None,
            preferred_range: None,
            initial,
            solved: None,
        });
        self.adj.push(Vec::new());
        true
    }

    /// [choice] A range, fixed or preference call on a vertex that was not
    /// added adds it with initial height 0 (the callers of §9.3 and §9.6
    /// always add first).
    fn rec_mut(&mut self, v: i32) -> &mut VertexRecord {
        self.add_vertex(v, 0.0);
        let i = self.index[&v];
        &mut self.recs[i]
    }

    /// `InsertRangeConstraint(v, a, b)`: intersect v's range with [a, b];
    /// an inversion by at most 0.01 collapses to the lower bound. A larger
    /// inversion stays, and `Solve` fails on it.
    pub fn insert_range(&mut self, v: i32, a: f32, b: f32) {
        let r = self.rec_mut(v);
        (r.min, r.max) = intersect(r.min, r.max, a, b);
    }

    /// `InsertFixedVertex(v, h)`: add v with initial height h, and range
    /// [h, h]. [inferred] On a vertex already added the range is replaced.
    pub fn insert_fixed(&mut self, v: i32, h: f32) {
        self.add_vertex(v, h);
        let r = self.rec_mut(v);
        r.min = h;
        r.max = h;
    }

    /// `h(a) = h(b)`.
    pub fn insert_equality(&mut self, a: i32, b: i32) {
        self.push(Constraint::Equality(a, b));
    }

    /// `|h(a) − h(b)| ≤ s`. A second slope constraint on the same pair
    /// (either order, [inferred]) keeps the smaller s.
    pub fn insert_slope(&mut self, a: i32, b: i32, s: f32) {
        let key = (a.min(b), a.max(b));
        if let Some(&i) = self.slopes.get(&key) {
            if let Constraint::Slope(_, _, old) = &mut self.cons[i] {
                if s < *old {
                    *old = s;
                }
            }
            return;
        }
        let i = self.push(Constraint::Slope(a, b, s));
        self.slopes.insert(key, i);
    }

    /// `|h(a) − 2·h(b) + h(c)| ≤ s`.
    pub fn insert_smoothness(&mut self, a: i32, b: i32, c: i32, s: f32) {
        self.push(Constraint::Smoothness(a, b, c, s));
    }

    /// The preferred value of `v`.
    pub fn set_preferred_value(&mut self, v: i32, h: f32) {
        self.rec_mut(v).preferred = Some(h);
    }

    /// The preferred range of `v`.
    pub fn set_preferred_range(&mut self, v: i32, a: f32, b: f32) {
        self.rec_mut(v).preferred_range = Some((a, b));
    }

    fn push(&mut self, c: Constraint) -> usize {
        let i = self.cons.len();
        self.cons.push(c);
        // The adjacency is rebuilt at solve time: a constraint may name a
        // vertex that is added later.
        i
    }

    /// Narrows record `i` to [a, b]. A solved vertex keeps its height and
    /// only checks it. Returns false when the range becomes empty.
    fn narrow(&mut self, i: usize, a: f32, b: f32, affected: &mut BTreeSet<(i32, usize)>) -> bool {
        let r = &mut self.recs[i];
        let (lo, hi) = intersect(r.min, r.max, a, b);
        if lo > hi || lo.is_nan() || hi.is_nan() {
            return false;
        }
        if r.solved.is_some() {
            return true;
        }
        if r.min != lo || r.max != hi {
            r.min = lo;
            r.max = hi;
            affected.insert((r.id, i));
        }
        true
    }

    /// Applies one constraint once. [choice] A constraint that names a
    /// vertex outside the to-solve set constrains nothing (Q NG10).
    fn apply(&mut self, c: Constraint, pinned: &mut BTreeSet<(i32, usize)>) -> bool {
        let ix = |v: i32| self.index.get(&v).copied();
        match c {
            // Equalities propagate from the source to its entire group below.
            Constraint::Equality(_, _) => true,
            Constraint::Slope(a, b, s) => {
                let (Some(ia), Some(ib)) = (ix(a), ix(b)) else {
                    return true;
                };
                let (amin, amax) = (self.recs[ia].min, self.recs[ia].max);
                if !self.narrow(ib, sub(amin, s), add(amax, s), pinned) {
                    return false;
                }
                let (bmin, bmax) = (self.recs[ib].min, self.recs[ib].max);
                self.narrow(ia, sub(bmin, s), add(bmax, s), pinned)
            }
            Constraint::Smoothness(a, b, c, s) => {
                let (Some(ia), Some(ib), Some(ic)) = (ix(a), ix(b), ix(c)) else {
                    return true;
                };
                // [inferred] The operation order: b from (a + c ∓ s) / 2,
                // then a and c from 2b − the other ∓ s.
                let r = |i: usize, recs: &[VertexRecord]| (recs[i].min, recs[i].max);
                let (amin, amax) = r(ia, &self.recs);
                let (cmin, cmax) = r(ic, &self.recs);
                let blo = r24(f64::from(sub(add(amin, cmin), s)) * 0.5) as f32;
                let bhi = r24(f64::from(add(add(amax, cmax), s)) * 0.5) as f32;
                if !self.narrow(ib, blo, bhi, pinned) {
                    return false;
                }
                let (bmin, bmax) = r(ib, &self.recs);
                let two = |v: f32| r24(f64::from(v) * 2.0) as f32;
                let (cmin, cmax) = r(ic, &self.recs);
                if !self.narrow(ia, sub(sub(two(bmin), cmax), s), add(sub(two(bmax), cmin), s), pinned) {
                    return false;
                }
                let (amin, amax) = r(ia, &self.recs);
                self.narrow(ic, sub(sub(two(bmin), amax), s), add(sub(two(bmax), amin), s), pinned)
            }
        }
    }

    /// §9.1: alternate signed-id ordered work sets until no bounds change.
    fn propagate(
        &mut self,
        mut current: BTreeSet<(i32, usize)>,
        groups: &[BTreeSet<(i32, usize)>],
        queued: &mut [bool],
        pending: &mut Vec<(usize, f32)>,
    ) -> bool {
        while !current.is_empty() {
            let mut next = BTreeSet::new();
            for (_, i) in current {
                for k in 0..self.adj[i].len() {
                    if !self.apply(self.cons[self.adj[i][k]], &mut next) {
                        return false;
                    }
                }
                let (lo, hi) = (self.recs[i].min, self.recs[i].max);
                for &(_, other) in &groups[i] {
                    if other != i && !self.narrow(other, lo, hi, &mut next) {
                        return false;
                    }
                }
                if lo == hi && !queued[i] {
                    queued[i] = true;
                    pending.push((i, lo));
                }
            }
            current = next;
        }
        true
    }

    /// `Solve` (§9.1): equal widths use SGI bucket/chain order (NG1).
    pub fn solve(&mut self) -> bool {
        self.solution.clear();
        for r in &mut self.recs {
            r.solved = None;
        }
        // The adjacency (constraints may name vertices added after them).
        for a in &mut self.adj {
            a.clear();
        }
        for (ci, c) in self.cons.iter().enumerate() {
            for v in c.vertices() {
                if let Some(&i) = self.index.get(&v) {
                    if !self.adj[i].contains(&ci) {
                        self.adj[i].push(ci);
                    }
                }
            }
        }
        // Equality insertion merges equivalence groups; only allowed bounds
        // propagate, preserving each member's initial height and preferences.
        let mut groups: Vec<BTreeSet<_>> = self
            .recs
            .iter()
            .enumerate()
            .map(|(i, r)| BTreeSet::from([(r.id, i)]))
            .collect();
        for c in &self.cons {
            if let Constraint::Equality(a, b) = c {
                if let (Some(&ia), Some(&ib)) = (self.index.get(a), self.index.get(b)) {
                    let merged: BTreeSet<_> = groups[ia].union(&groups[ib]).copied().collect();
                    for &(_, i) in &merged {
                        groups[i] = merged.clone();
                    }
                }
            }
        }
        // An inverted range from the inserts fails at once.
        if self
            .recs
            .iter()
            .any(|r| r.min > r.max || r.min.is_nan() || r.max.is_nan())
        {
            return false;
        }
        let mut pending = Vec::new();
        let mut queued = vec![false; self.recs.len()];
        let all = self.recs.iter().enumerate().map(|(i, r)| (r.id, i)).collect();
        // NT36: complete the initial propagation before comparing widths.
        if !self.propagate(all, &groups, &mut queued, &mut pending) {
            return false;
        }
        // Erasing solved nodes cannot change the relative order of survivors.
        let order: Vec<_> = self.buckets.iter().flatten().copied().collect();
        // 2. Singleton visits append pending pairs; drain them LIFO before
        // scanning the hash buckets for the narrowest remaining range.
        loop {
            let stacked = pending.pop();
            let mut pick = stacked.map(|(i, _)| i);
            if pick.is_none() {
                let mut best_w = 0.0f32;
                for &i in &order {
                    let r = &self.recs[i];
                    if r.solved.is_some() {
                        continue;
                    }
                    let w = sub(r.max, r.min);
                    if w < 0.0 {
                        return false;
                    }
                    if !w.is_nan() && (pick.is_none() || w < best_w) {
                        pick = Some(i);
                        best_w = w;
                    }
                    if w == 0.0 {
                        break;
                    }
                }
            }
            let Some(i) = pick else {
                break;
            };
            let h = stacked.map_or_else(|| self.choose(i), |(_, h)| h);
            if stacked.is_none() {
                self.recs[i].min = h;
                self.recs[i].max = h;
                queued[i] = true;
                let current = BTreeSet::from([(self.recs[i].id, i)]);
                if !self.propagate(current, &groups, &mut queued, &mut pending) {
                    return false;
                }
            }
            let r = &mut self.recs[i];
            r.solved = Some(h);
            self.solution.push((r.id, h));
        }
        // 3. Every vertex got a height.
        self.recs.iter().all(|r| r.solved.is_some())
    }

    /// §9.1 step 2.2: the preferred value if set and inside the range; else
    /// the overlap with the preferred range (within 0.01), then the initial
    /// height clamped into the range.
    fn choose(&mut self, i: usize) -> f32 {
        let r = &mut self.recs[i];
        if let Some(p) = r.preferred {
            if r.min <= p && p <= r.max {
                return p;
            }
        }
        if let Some((a, b)) = r.preferred_range {
            let (lo, hi) = intersect(r.min, r.max, a, b);
            if lo <= hi {
                r.min = lo;
                r.max = hi;
            }
        }
        let h = r.initial;
        if h < r.min {
            r.min
        } else if h > r.max {
            r.max
        } else {
            h
        }
    }
}
