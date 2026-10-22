// SPDX-License-Identifier: GPL-3.0-or-later
use super::parse::*;
use super::*;

mod file_browser;
mod grid;
mod tree;

fn n(v: u32) -> Value {
    Value::U32(v)
}
fn i(v: i32) -> Value {
    Value::I32(v)
}
fn text(v: impl Into<String>) -> Value {
    Value::Text(v.into())
}
fn call(f: &mut impl Foreign, h: Handle, s: u32, args: Vec<Value>) -> Value {
    f.widget(h, s, args)
}
fn flag(p: Properties<'_>, key: u32) -> Option<bool> {
    p.number(key).map(|n| n != 0)
}
fn image_key(h: Handle, g: u32, i: u32, index: u32, f: &mut impl Foreign) {
    param(h, 0xdaa3b860u32.wrapping_add(index), Value::Pair(g, i), f);
}
fn font_word(
    h: Handle,
    interface: Handle,
    p: Properties<'_>,
    key: u32,
    index: u32,
    slot: u32,
    f: &mut impl Foreign,
) -> bool {
    if let Some(id) = p.number(key) {
        let font = f.font(id);
        call(f, interface, slot, vec![Value::Handle(font)]);
        param(h, 0xfaa3be85u32.wrapping_add(index), n(id), f);
        true
    } else if let Some(s) = p.text(key) {
        param(h, 0xfaa3be85u32.wrapping_add(index), text(s), f);
        true
    } else {
        false
    }
}

pub fn interpret_common(h: Handle, p: Properties<'_>, first: bool, f: &mut impl Foreign) -> Result<()> {
    p.validate()?;
    if first {
        for (bit, on) in [
            (1, true),
            (0x10000, true),
            (0x800, false),
            (0x20000, true),
            (0x40000, true),
            (4, false),
            (0x8000, true),
        ] {
            f.write(h, Write::Flag(bit, on));
        }
        for (key, bit) in [
            (0xf01a, 1),
            (0xf01c, 0x100),
            (0xf01d, 0x200),
            (0xf01e, 0x800),
            (0xf025, 0x8000),
            (0xf026, 4),
            (0xf023, 0x80000),
            (0xf024, 0x200000),
        ] {
            if let Some(v) = flag(p, key) {
                f.write(h, Write::Flag(bit, v));
            }
        }
        if let Some(v) = p.number(0x100) {
            f.write(h, Write::Id(v));
        }
        if let Some(s) = p.text(0x101) {
            // §8 expressly permits zero for indeterminate unparsed area fields.
            let rect = zero_scan::<4>(&s, "( %d, %d, %d, %d)")?.map(|v| v as i32);
            f.write(h, Write::Area(rect));
        } else {
            if let Some([x, y]) = tuple::<2>(p, 0x102)? {
                f.write(h, Write::Move(x, y));
            }
            if let Some([x, y]) = tuple::<2>(p, 0x103)? {
                f.write(h, Write::Size(x, y));
            }
        }
        if let Some(c) = colour(p, 0x104)? {
            f.write(h, Write::Fill(c));
        }
        if let Some(s) = p.text(0x105) {
            f.write(h, Write::Caption(s));
        }
        let (mut group, mut instance) = (0, 0);
        if let Some(s) = p.text(0x106) {
            [group, instance] = zero_scan::<2>(&s, "{%x,%x}")?;
            if let Some(s) = f.load_string(group, instance) {
                f.write(h, Write::Caption(s));
            }
            param(
                h,
                0x0b31771a,
                Value::Key(ResourceKey {
                    kind: 0x2026960b,
                    group,
                    instance,
                }),
                f,
            );
        }
        param(h, 0x3aca6906, Value::Pair(group, instance), f);
        if p.has(0x107) {
            f.write(h, Write::Flag(0x20000, true));
        }
        let comments = std::array::from_fn(|k| p.text(0x108 + k as u32).unwrap_or_default());
        param(h, 0xbaca6879, Value::Comments(comments), f);
    } else {
        for (key, bit) in [
            (0xf01b, 2),
            (0xf01f, 0x10000),
            (0xf020, 0x20000),
            (0xf021, 0x40000),
            (0xf022, 0x100000),
        ] {
            if let Some(on) = flag(p, key) {
                f.write(h, Write::Flag(bit, on));
            }
        }
    }
    Ok(())
}

/// QI/Release surround *both* passes, even if a known body does nothing. The
/// Custom shared bare-return stub is the sole exception (§9 table).
pub fn interpret(kind: Kind, h: Handle, p: Properties<'_>, first: bool, f: &mut impl Foreign) -> Result<()> {
    p.validate()?;
    if kind == Kind::Common {
        return interpret_common(h, p, first, f);
    }
    if kind == Kind::Custom {
        return Ok(());
    }
    let iid = kind.iid();
    let Some(interface) = f.query(h, iid) else {
        return Ok(());
    };
    let result = match kind {
        Kind::Slider => slider(h, interface, p, first, f),
        Kind::FlatRect => flat_rect(interface, p, first, f),
        Kind::Outline => outline(interface, p, first, f),
        Kind::Folders => folders(h, interface, p, first, f),
        Kind::TreeView => tree::run(h, interface, p, first, f),
        Kind::FileBrowser => file_browser::run(h, interface, p, first, f),
        Kind::Grid => grid::run(h, interface, p, first, f),
        _ => Err(Unsupported::Interpreter(iid)),
    };
    f.release(interface);
    result
}

fn slider(h: Handle, t: Handle, p: Properties<'_>, first: bool, f: &mut impl Foreign) -> Result<()> {
    if first {
        if let Some([a, b]) = tuple::<2>(p, 0xc00)? {
            call(f, t, 0x20, vec![i(a.max(b))]);
            call(f, t, 0x18, vec![i(a.min(b))]);
        }
        if let Some(v) = p.number(0xc01) {
            call(f, t, 0x28, vec![n(if v == 0xc03 { 2 } else { 1 })]);
        }
        if let Some((g, inst)) = key(p, 0xf016)? {
            if let Some(buffer) = f.load_image(g, inst) {
                let result = tuple::<4>(p, 0xf018);
                // Ensure the acquired buffer is released even on unsupported parsing.
                match result {
                    Ok(rect) => {
                        call(
                            f,
                            t,
                            0x30,
                            vec![Value::Handle(buffer), rect.map_or(Value::None, Value::Rect)],
                        );
                        param(
                            h,
                            0x6b4bcbd4,
                            Value::Key(ResourceKey {
                                kind: 0x856ddbac,
                                group: g,
                                instance: inst,
                            }),
                            f,
                        );
                        image_key(h, g, inst, 0, f);
                    }
                    Err(e) => {
                        f.release(buffer);
                        return Err(e);
                    }
                }
                f.release(buffer);
            }
        }
    } else if let Some(v) = p.number(0x40a) {
        call(f, t, 0x14, vec![i(v as i32)]);
    }
    Ok(())
}
fn flat_rect(t: Handle, p: Properties<'_>, first: bool, f: &mut impl Foreign) -> Result<()> {
    if !first {
        return Ok(());
    }
    let mut style = 0;
    for item in p.0.iter().filter(|p| p.key == 0xf005) {
        if let Some(Variant::Scalar(Elem::U32(v))) = &item.value {
            style |= match *v {
                0xf006 => 1,
                0x204 => 2,
                0xf007 => 4,
                0x205 => 8,
                0x206 => 15,
                0x207 => 13,
                0x208 => 14,
                0xf00c => 16,
                0xf00b => 32,
                0xf00e => 64,
                _ => 0,
            };
        }
    }
    call(f, t, 0x30, vec![n(style)]);
    for edge in 0..4 {
        if let Some(c) = colour(p, 0x200 + edge)? {
            call(
                f,
                t,
                0x28,
                vec![n(edge), n(c[0] as u32), n(c[1] as u32), n(c[2] as u32)],
            );
        }
    }
    Ok(())
}
fn outline(t: Handle, p: Properties<'_>, first: bool, f: &mut impl Foreign) -> Result<()> {
    if !first {
        return Ok(());
    }
    let mut style = 0;
    for item in p.0.iter().filter(|p| p.key == 0xf005) {
        if let Some(Variant::Scalar(Elem::U32(v))) = &item.value {
            style |= match *v {
                0xf009 => 1,
                0xf00b => 2,
                0xf00c => 4,
                0xf00d => 32,
                _ => 0,
            };
        }
    }
    if style != 0 {
        call(f, t, 0x0c, vec![n(style)]);
    }
    for (key, which) in [(0x1100, 1), (0x1101, 2)] {
        if let Some(c) = colour(p, key)? {
            call(
                f,
                t,
                0x14,
                vec![n(which), n(c[0] as u32), n(c[1] as u32), n(c[2] as u32)],
            );
        }
    }
    let mut points = vec![];
    for (_, s) in p.texts(0x1102).take(32) {
        // Q12 choice: zero each unparsed component, not stale native stack bytes.
        points.push(zero_scan::<2>(&s, "(%d,%d)")?.map(|v| v as i32));
    }
    if !points.is_empty() {
        call(
            f,
            t,
            0x34,
            vec![n(points.len() as u32), Value::Points(points), n(1), n(1)],
        );
    }
    Ok(())
}
fn folders(h: Handle, t: Handle, p: Properties<'_>, first: bool, f: &mut impl Foreign) -> Result<()> {
    if first {
        font_word(h, t, p, 0xf000, 0, 0x28, f);
        if let Some(s) = p.text(0xf016) {
            // Q11 choice explicitly allowed in §9.6: zero unparsed fields.
            let [g, inst] = zero_scan::<2>(&s, "{%x,%x}")?;
            let buffer = f.load_image(g, inst);
            if let Some(b) = buffer {
                call(f, t, 0x2c, vec![Value::Handle(b)]);
            }
            image_key(h, g, inst, 0, f);
            if let Some(b) = buffer {
                f.release(b);
            }
        }
    } else {
        call(f, t, 0x1c, vec![]);
        for (_, s) in p.texts(0x502) {
            call(f, t, 0x10, vec![text(s), n(0)]);
        }
    }
    Ok(())
}
