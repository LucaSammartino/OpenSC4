// SPDX-License-Identifier: GPL-3.0-or-later
//! legacy-control §9.20; foreign procedures and receiving widgets stay foreign.
use super::controls::*;
use super::feedback::unchecked;
use super::*;

pub(super) fn run(
    kind: Kind,
    h: Handle,
    t: Handle,
    p: Properties<'_>,
    first: bool,
    f: &mut impl Foreign,
) -> Result<()> {
    if kind == Kind::Gen {
        generator(h, t, p, first, f)
    } else {
        options(h, t, p, first, f)
    }
}
fn generator(h: Handle, t: Handle, p: Properties<'_>, first: bool, f: &mut impl Foreign) -> Result<()> {
    if first {
        for (key, mask, invert) in [
            (0x10004, 1, false),
            (0x10005, 2, false),
            (0x10006, 4, false),
            (0x10007, 0x10, false),
            (0x10008, 0x20, false),
            (0x10009, 0x40, false),
            (0x1000a, 0x100, false),
            (0x1000b, 0x200, false),
            (0x1000c, 0x400, false),
            (0x1000d, 0x1000, true),
            (0xf00d, 0x2000, true),
            (0xf019, 0x4000, true),
            (0x1000e, 0x8000, true),
            (0x1000f, 0x10000, false),
        ] {
            if let Some(v) = p.number(key) {
                call(f, t, 0x18, vec![n(mask), n(u32::from((v != 0) ^ invert))]);
            }
        }
        numbers(t, p, &[(0x10000, 0x4c)], f);
        if let Some((g, inst)) = key(p, 0xf016)? {
            let mode = match p.number(0x10001).unwrap_or(0) {
                0 => 0,
                2 => 2,
                _ => 1,
            };
            if let Some(b) = f.load_image(g, inst) {
                call(f, t, 0x34, vec![n(mode), Value::Handle(b)]);
                f.release(b);
            }
            resource(h, 0xab428c5d, 0x856ddbac, g, inst, f);
            image_key(h, g, inst, 0, f);
        }
        pair(t, p, 0xf004, 0x68, f)?;
        let proc = f.new_default_procedure();
        call(f, t, 0x1c, vec![proc.map_or(Value::None, Value::Handle)]);
    } else {
        for prop in p.0 {
            let Some(s) = Properties(std::slice::from_ref(prop)).text(prop.key) else {
                continue;
            };
            match prop.key {
                0x10002 => {
                    let v = scan(&s, "(%u,%d)")?;
                    if v.len() == 2 {
                        call(f, t, 0x78, vec![n(v[0]), i(v[1] as i32)]);
                    }
                }
                0x10003 => {
                    let v = scan(&s, "(%u,%d,%d,%d,%d)")?;
                    if v.len() == 5 {
                        call(
                            f,
                            t,
                            0x74,
                            vec![
                                n(v[0]),
                                n(0),
                                n(0),
                                Value::Rect([v[1] as i32, v[2] as i32, v[3] as i32, v[4] as i32]),
                            ],
                        );
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}
fn options(h: Handle, t: Handle, p: Properties<'_>, first: bool, f: &mut impl Foreign) -> Result<()> {
    if first {
        for (key, mask) in [(0xf00, 1), (0xf01, 2), (0xf02, 4), (0xf019, 8)] {
            if let Some(v) = p.number(key) {
                call(f, t, 0x14, vec![n(mask), n(u32::from(v == 1))]);
            }
        }
        pair(t, p, 0xf004, 0x18, f)?;
        font(h, t, p, 0xf03, 0, 0x20, true, None, f)?;
        font(h, t, p, 0xf04, 1, 0x24, true, None, f)?;
        colours(
            t,
            p,
            0x28,
            &[(0xf05, 0), (0xf06, 1), (0xf07, 2), (0xf08, 3), (0xf09, 4)],
            f,
        )?;
        if let Some(s) = p.text(0xf0a) {
            let [g, inst] = unchecked::<2>(&s, "{%x,%x}")?;
            if let Some(s) = f.load_string(g, inst) {
                if let Value::Handle(label) = call(f, t, 0x0c, vec![]) {
                    f.write(label, Write::Caption(s));
                } else {
                    return Err(Unsupported::ForeignResult("OptGrp label"));
                }
            }
            param(h, 0x4a4af4ac, Value::Pair(g, inst), f);
        }
        if let Some((g, inst)) = key(p, 0xf0b)? {
            if let Some(b) = f.load_image(g, inst) {
                call(f, t, 0x50, vec![Value::Handle(b)]);
                f.release(b);
            }
            image_key(h, g, inst, 0, f);
        }
        if let Some(s) = p.text(0x40b) {
            let v = scan(&s, "(%d,%d,%d)(%d,%d,%d)(%d,%d,%d)(%d,%d,%d)")?;
            if v.len() == 12 {
                let args = v
                    .chunks_exact(3)
                    .map(|c| n(f.colour([c[0] as u8, c[1] as u8, c[2] as u8])))
                    .collect();
                call(f, t, 0x54, args);
            }
        }
        if let Some(v) = p.number(0xf0c) {
            if v == 0xf0e {
                if let Some(size) = p.number(0xf10) {
                    call(f, t, 0x70, vec![n(0), n(size)]);
                }
            } else {
                call(f, t, 0x70, vec![n(1), n(0)]);
            }
        }
    } else {
        for (_, s) in p.texts(0xf0d) {
            let parts = fields(&s, ':', 5);
            let [id] = unchecked::<1>(parts[0], "%d")?;
            let [style] = unchecked::<1>(parts[2], "%d")?;
            // LC11: partial image scratch has no defined portable value.
            let [g, inst] = unchecked::<2>(parts[3], "{%x,%x}")?;
            let rect = scan(parts[4], "(%d,%d,%d,%d)")?;
            let rect = if rect.len() == 4 {
                std::array::from_fn(|k| rect[k] as i32)
            } else {
                [0, 0, 1, 1]
            };
            let b = f.load_image(g, inst);
            call(
                f,
                t,
                0x3c,
                vec![
                    i(id as i32),
                    text(parts[1]),
                    i(style as i32),
                    b.map_or(Value::None, Value::Handle),
                    Value::Rect(rect),
                ],
            );
            let result = call(f, t, 0x4c, vec![i(id as i32)]);
            let result = if let Value::Handle(item) = result {
                let win = call(f, item, 0x0c, vec![n(inst), n(0)]);
                let result = if let Value::Handle(win) = win {
                    image_key(win, g, inst, 0, f);
                    Ok(())
                } else {
                    Err(Unsupported::ForeignResult("OptGrp item window"))
                };
                f.release(item);
                result
            } else {
                Ok(())
            };
            if let Some(b) = b {
                f.release(b);
            }
            result?;
        }
        for prop in p.0.iter().filter(|prop| matches!(prop.key, 0xf11 | 0xf12)) {
            let Some(s) = Properties(std::slice::from_ref(prop)).text(prop.key) else {
                continue;
            };
            let parts = fields(&s, ':', 2);
            let [id] = unchecked::<1>(parts[0], "%d")?;
            let v = scan(parts[1], "(%d,%d)")?;
            if v.len() == 2 {
                call(
                    f,
                    t,
                    if prop.key == 0xf11 { 0x44 } else { 0x48 },
                    vec![i(id as i32), i(v[0] as i32), i(v[1] as i32)],
                );
            }
        }
        numbers(t, p, &[(0x501, 0x38)], f);
        fit(h, t, p, &[0x60], f);
    }
    Ok(())
}
