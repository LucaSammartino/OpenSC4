// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The bridge type specialisations (§3): a fixed piece list for one exact
//! span length of one bridge type, which replaces the parametric model for
//! that length.

use std::collections::BTreeMap;

/// The support kind of a specialisation piece (§3.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SupportKind {
    /// Not supported.
    #[default]
    None = 0,
    /// `pylon`.
    Pylon = 1,
    /// `end`.
    End = 2,
}

/// One piece of a specialisation body (§3.2): 20 bytes in the original.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpecialPiece {
    /// `Y`: `YOFFSET + ` the last step height when it was read (§3.2).
    pub y: f32,
    /// The exemplar instance.
    pub instance: u32,
    /// The rotation, 0–3.
    pub rotation: u8,
    /// The model depth (−1 when the item gave none).
    pub depth: f32,
    /// The support kind.
    pub support: SupportKind,
}

/// What one body builds (§3.2): the pieces and the step heights, which start
/// as `[0]`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Specialization {
    /// The pieces (`entry +0x00`).
    pub pieces: Vec<SpecialPiece>,
    /// The step heights (`entry +0x0C`).
    pub heights: Vec<f32>,
    /// The body's `height H` line (§3.1), applied at the end to every piece
    /// height and every step height of this body.
    pub height_add: f32,
}

impl Specialization {
    /// The end height of the list (§3.2).
    pub fn end_height(&self) -> f32 {
        self.heights.last().copied().unwrap_or(0.0)
    }
}

/// The tokenizer (§3.1, `NextToken` `0x0063DDF0`): white space separates
/// tokens, and `<`, `>`, `,`, `:`, `*`, `(` and `)` are tokens of their own.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let chars = text.chars().peekable();
    for c in chars {
        if c.is_whitespace() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            continue;
        }
        if matches!(c, '<' | '>' | ',' | ':' | '*' | '(' | ')') {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            out.push(c.to_string());
            continue;
        }
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// A cursor over the token list.
struct Lexer {
    toks: Vec<String>,
    i: usize,
}

impl Lexer {
    fn peek(&self) -> Option<&str> {
        self.toks.get(self.i).map(String::as_str)
    }
    fn next(&mut self) -> Option<String> {
        let t = self.toks.get(self.i).cloned();
        if t.is_some() {
            self.i += 1;
        }
        t
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.peek() == Some(s) {
            self.i += 1;
            true
        } else {
            false
        }
    }
}

fn as_u32(s: &str) -> Option<u32> {
    let t = s.trim();
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u32::from_str_radix(h, 16).ok()
    } else {
        t.parse().ok()
    }
}

fn as_f32(s: &str) -> Option<f32> {
    s.trim().parse().ok()
}

/// The parse state (§3.1).
#[derive(Default)]
struct Parser {
    /// The global sections, by name.
    global: BTreeMap<String, Specialization>,
    /// The sections of the bridge being parsed.
    local: BTreeMap<String, Specialization>,
    /// The length map of the bridge being parsed.
    lengths: BTreeMap<u32, Specialization>,
}

/// `ParseBridgeInstance` (`0x00641310`): `<INSTANCE, ROTATION[, HTDELTA[,
/// YOFFSET]]>` optionally followed by `:OPT{,OPT}`. Returns the piece, the
/// HTDELTA and the YOFFSET.
fn parse_instance(lx: &mut Lexer) -> Option<(SpecialPiece, f32, f32)> {
    if !lx.eat("<") {
        return None;
    }
    let inst = as_u32(&lx.next()?)?;
    if !lx.eat(",") {
        return None;
    }
    let rot = as_u32(&lx.next()?)? as u8;
    let mut ht_delta = 0.0f32;
    let mut y_off = 0.0f32;
    if lx.eat(",") {
        if let Some(t) = lx.peek() {
            if t != ">" {
                ht_delta = as_f32(&lx.next()?)?;
                if lx.eat(",") {
                    if let Some(t) = lx.peek() {
                        if t != ">" {
                            y_off = as_f32(&lx.next()?)?;
                        }
                    }
                }
            }
        }
    }
    if !lx.eat(">") {
        return None;
    }
    let mut p = SpecialPiece {
        y: y_off,
        instance: inst,
        rotation: rot & 3,
        depth: -1.0,
        support: SupportKind::None,
    };
    // `:OPT{,OPT}` where OPT is a number (the model depth), `pylon` or `end`.
    if lx.eat(":") {
        while let Some(t) = lx.next() {
            match t.as_str() {
                "pylon" => p.support = SupportKind::Pylon,
                "end" => p.support = SupportKind::End,
                other => {
                    if let Some(d) = as_f32(other) {
                        p.depth = d;
                    }
                }
            }
            if !lx.eat(",") {
                break;
            }
        }
    }
    Some((p, ht_delta, y_off))
}

/// Appends `src` to `dst` (§3.2): the pieces and the heights rebased on the
/// current last height.
///
/// [choice] `rev` "appends them in reverse order, XORs each rotation with 2
/// and rebases against the referenced list's end height". This reads that as
/// the reverse run starting at the current last height and going back down
/// the referenced heights: the shift is `last − end_height`, the pieces come
/// in reverse with `rotation ^= 2`, and the step heights are appended as
/// `last − h` over the referenced heights in reverse. See question 4.
fn append_ref(dst: &mut Specialization, src: &Specialization, rev: bool, width: usize) {
    let last = dst.end_height();
    if !rev {
        for p in &src.pieces {
            let mut q = *p;
            q.y += last;
            dst.pieces.push(q);
        }
        for &h in src.heights.iter().skip(1) {
            dst.heights.push(last + h);
        }
    } else {
        let end = src.end_height();
        let shift = last - end;
        for p in src.pieces.iter().rev() {
            let mut q = *p;
            q.y += shift;
            q.rotation ^= 2;
            dst.pieces.push(q);
        }
        for &h in src.heights.iter().skip(1).rev() {
            dst.heights.push(last - h + end - end);
            let _ = (h, width);
        }
    }
}

/// `ParseBridgeSpecialization` (`0x00643EA50`): one body's lines.
///
/// `width` is the bridge type's W (§3.2: the step height advances after every
/// W pieces). Returns `false` on a malformed body.
fn parse_body(lx: &mut Lexer, p: &Parser, width: usize, inside_bridge: bool) -> Option<Specialization> {
    let mut body = Specialization {
        heights: vec![0.0],
        ..Specialization::default()
    };
    let mut count_in_step = 0usize;
    while let Some(head) = lx.peek().map(str::to_string) {
        match head.as_str() {
            "end" | "stop" | "section" | "bridge" | "length" => break,
            "height" => {
                lx.next();
                let v = as_f32(&lx.next()?)?;
                body.height_add += v;
            }
            "piece" => {
                lx.next();
                // To the end of the line: the tokenizer has no newlines, so a
                // body's line is a run of items until the next keyword.
                while let Some(t) = lx.peek().map(str::to_string) {
                    if matches!(
                        t.as_str(),
                        "end" | "stop" | "section" | "bridge" | "length" | "height" | "piece"
                    ) {
                        break;
                    }
                    // `COUNT *`
                    let mut repeat = 1usize;
                    let save = lx.i;
                    let first = lx.next()?;
                    if first == "*" {
                        // `COUNT` was read as `first`.
                        repeat = as_u32(&first).unwrap_or(1).max(1) as usize;
                        lx.next()?;
                    } else if let Some(n) = as_u32(&first) {
                        if lx.eat("*") {
                            repeat = n.max(1) as usize;
                        } else {
                            lx.i = save;
                        }
                    } else {
                        lx.i = save;
                    }
                    // The item.
                    let item = lx.peek().map(str::to_string)?;
                    let before = body.pieces.len();
                    let ht_delta;
                    if item == "<" {
                        let (mut piece, hd, _yo) = parse_instance(lx)?;
                        ht_delta = hd;
                        for _ in 0..repeat {
                            let mut q = piece;
                            q.y += body.end_height();
                            body.pieces.push(q);
                            count_in_step += 1;
                            if width > 0 && count_in_step == width {
                                body.heights.push(body.end_height() + ht_delta);
                                count_in_step = 0;
                            }
                        }
                        let _ = &mut piece;
                    } else {
                        // A section name, `rev(NAME)`, `length(L)` or
                        // `rev(length(L))`.
                        let (name, rev) = parse_ref(lx)?;
                        let src = lookup(p, &name, inside_bridge)?;
                        let n = src.pieces.len();
                        append_ref(&mut body, src, rev, width);
                        count_in_step = (count_in_step + n) % width.max(1);
                        if width > 0 && n > 0 && count_in_step == 0 {
                            // The reference's own heights carried the steps.
                        }
                    }
                    let _ = before;
                }
            }
            _ => return None,
        }
    }
    // §3.1 `height H`: added at the end to every piece height and every step
    // height of this body.
    if body.height_add != 0.0 {
        let a = body.height_add;
        for p in body.pieces.iter_mut() {
            p.y += a;
        }
        for h in body.heights.iter_mut() {
            *h += a;
        }
    }
    Some(body)
}

/// A reference item: a section name, `rev(NAME)`, `length(L)` or
/// `rev(length(L))`.
fn parse_ref(lx: &mut Lexer) -> Option<(String, bool)> {
    let mut rev = false;
    let mut name = lx.next()?;
    if name == "rev" {
        rev = true;
        if !lx.eat("(") {
            return None;
        }
        name = lx.next()?;
        // `length(L)` inside the parentheses.
        if name == "length" {
            if !lx.eat("(") {
                return None;
            }
            let l = lx.next()?;
            if !lx.eat(")") {
                return None;
            }
            name = format!("length:{l}");
        }
        if !lx.eat(")") {
            return None;
        }
        return Some((name, rev));
    }
    if name == "length" {
        if !lx.eat("(") {
            return None;
        }
        let l = lx.next()?;
        if !lx.eat(")") {
            return None;
        }
        return Some((format!("length:{l}"), false));
    }
    Some((name, rev))
}

fn lookup<'a>(p: &'a Parser, name: &str, inside_bridge: bool) -> Option<&'a Specialization> {
    if let Some(l) = name.strip_prefix("length:") {
        let n: u32 = l.parse().ok()?;
        return p.lengths.get(&n);
    }
    if inside_bridge {
        if let Some(s) = p.local.get(name) {
            return Some(s);
        }
    }
    p.global.get(name)
}

/// `ReadBridgeTypeSpecializations` (`0x006436D0`) and its `Parse`
/// (`0x00643580`): the specialisation file of one network type.
///
/// `widths` is each bridge type's W by INI key; `min_span`/`max_span` bound
/// each `length L` (§3.1). Returns the length maps by **bridge type index**,
/// in the order of `keys`, and whether the parse succeeded. A failure leaves
/// whatever was built before it and ignores the rest of the file (§3.1).
pub fn parse_specializations(
    text: &str,
    keys: &[u32],
    widths: &[u32],
    min_span: &[u32],
    max_span: &[u32],
) -> (Vec<BTreeMap<u32, Specialization>>, bool) {
    let mut out: Vec<BTreeMap<u32, Specialization>> = vec![BTreeMap::new(); keys.len()];
    let mut p = Parser::default();
    let mut lx = Lexer {
        toks: tokenize(text),
        i: 0,
    };
    while let Some(t) = lx.next() {
        match t.as_str() {
            "stop" => return (out, true),
            "section" => {
                let Some(name) = lx.next() else {
                    return (out, false);
                };
                let Some(body) = parse_body(&mut lx, &p, 1, false) else {
                    return (out, false);
                };
                lx.eat("end");
                p.global.insert(name, body);
            }
            "bridge" => {
                let Some(key) = lx.next() else {
                    return (out, false);
                };
                let Some(k) = as_u32(&key) else {
                    return (out, false);
                };
                // If there is none, the parse fails and the rest is ignored.
                let Some(idx) = keys.iter().position(|&x| x == k) else {
                    return (out, false);
                };
                p.local.clear();
                p.lengths = out[idx].clone();
                let w = widths.get(idx).copied().unwrap_or(1).max(1) as usize;
                loop {
                    let Some(t) = lx.peek().map(str::to_string) else {
                        return (out, false);
                    };
                    match t.as_str() {
                        "end" => {
                            lx.next();
                            break;
                        }
                        "section" => {
                            lx.next();
                            let Some(name) = lx.next() else {
                                return (out, false);
                            };
                            let Some(body) = parse_body(&mut lx, &p, w, true) else {
                                return (out, false);
                            };
                            lx.eat("end");
                            p.local.insert(name, body);
                        }
                        "length" => {
                            lx.next();
                            let Some(lt) = lx.next() else {
                                return (out, false);
                            };
                            let Some(l) = as_u32(&lt) else {
                                return (out, false);
                            };
                            // L must be in [min span, max span] and not
                            // already in the length map, or the whole parse
                            // fails.
                            let (lo, hi) = (
                                min_span.get(idx).copied().unwrap_or(0),
                                max_span.get(idx).copied().unwrap_or(u32::MAX),
                            );
                            if l < lo || l > hi || p.lengths.contains_key(&l) {
                                return (out, false);
                            }
                            let Some(body) = parse_body(&mut lx, &p, w, true) else {
                                return (out, false);
                            };
                            lx.eat("end");
                            p.lengths.insert(l, body);
                        }
                        _ => return (out, false),
                    }
                }
                out[idx] = p.lengths.clone();
            }
            _ => return (out, false),
        }
    }
    (out, true)
}
