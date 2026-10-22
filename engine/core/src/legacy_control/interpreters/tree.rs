// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

struct Node {
    id: String,
    label: String,
    parent: String,
    position: i32,
    icons: [u32; 4],
    colours: [u32; 4],
    font: Handle,
    expanded: bool,
    selected: bool,
}

pub(super) fn run(h: Handle, t: Handle, p: Properties<'_>, first: bool, f: &mut impl Foreign) -> Result<()> {
    if first {
        for (key, bit) in [(0x1401, 2), (0x1402, 4), (0x1403, 8), (0x1404, 16)] {
            if let Some(v) = flag(p, key) {
                call(f, t, 0x0c, vec![n(bit), n(v as u32)]);
            }
        }
        for (key, slot) in [(0x1405, 0x1c), (0x1336, 0x2c), (0x1406, 0x34)] {
            if let Some(v) = p.number(key) {
                call(f, t, slot, vec![n(v)]);
            }
        }
        for (key, slot) in [(0x133f, 0x3c), (0x408, 0x44)] {
            if let Some(c) = colour(p, key)? {
                let c = f.colour(c);
                call(f, t, slot, vec![n(c)]);
            }
        }
        return Ok(());
    }
    let mut name_icon = BTreeMap::new();
    let mut icon_name = BTreeMap::new();
    let mut name_key = BTreeMap::new();
    for (_, s) in p.texts(0xf016) {
        let v = fields(&s, ':', 3);
        let parsed = scan(v[1], "{%x,%x}")?;
        let g = parsed.first().copied().unwrap_or(0);
        let inst = parsed.get(1).copied().unwrap_or(0);
        // LC10: duplicate map keys are unpublished, not silently guessed.
        if name_key.contains_key(v[0]) {
            return Err(Unsupported::Malformed("duplicate TreeView image name"));
        }
        name_key.insert(v[0].to_string(), (g, inst));
        let mut rect = zero_scan::<4>(v[2], "(%d,%d,%d,%d)")?.map(|v| v as i32);
        if parsed.len() == 2 {
            if let Some(b) = f.load_image(g, inst) {
                if rect == [0; 4] {
                    rect = f.image_rect(b);
                }
                let result = word(call(f, t, 0x50, vec![Value::Handle(b), Value::Rect(rect)]));
                f.release(b);
                let index = result?;
                if icon_name.contains_key(&index) {
                    return Err(Unsupported::Malformed("duplicate TreeView icon index"));
                }
                name_icon.insert(v[0].to_string(), index);
                icon_name.insert(index, v[0].to_string());
            }
        }
    }
    param(h, 0x46c207a5, Value::StringNumbers(name_icon.clone()), f);
    param(h, 0xc6c207c8, Value::NumberStrings(icon_name), f);
    param(h, 0x66c20802, Value::StringKeys(name_key), f);
    if let Some(s) = p.text(0x1407) {
        call(f, t, 0x80, vec![n(0), text(s)]);
    }
    for state in 0..4 {
        if let Some(s) = p.text(0x1408 + state) {
            let icon = name_icon.get(&s).copied().unwrap_or(state);
            call(f, t, 0x90, vec![n(0), n(state), n(icon)]);
        }
    }
    for which in 0..4 {
        if let Some(c) = colour(p, 0x140c + which)? {
            let c = f.colour(c);
            call(f, t, 0xac, vec![n(0), n(which), n(c)]);
        }
    }
    if let Some(id) = p.number(0x1410) {
        let font = f.font(id);
        call(f, t, 0x9c, vec![n(0), Value::Handle(font)]);
        param(h, 0xfaa3be85, n(id), f);
    } else if let Some(s) = p.text(0x1410) {
        param(h, 0xfaa3be85, text(s), f);
    }
    if let Some(v) = p.number(0x1411) {
        call(f, t, 0xb4, vec![n(0), n((v == 1) as u32)]);
    }
    if p.number(0x1412) == Some(1) {
        call(f, t, 0xe0, vec![n(0), n(0)]);
    }
    let black = f.colour_four([0, 0, 0], 1);
    call(f, t, 0x24, vec![n(black)]);
    let mut pending = vec![];
    let mut node_fonts = BTreeMap::new();
    // Q5 expressly allows -3 for the first unrecognised position. Subsequent
    // records retain the previous record's parsed position, before tree ordering.
    let mut position = -3;
    for (_, s) in p.texts(0x1102) {
        let v = fields(&s, ':', 15);
        position = match v[3] {
            "first" => -2,
            "last" => -3,
            "sorted" => -4,
            _ => position,
        };
        let icons = std::array::from_fn(|k| name_icon.get(v[4 + k]).copied().unwrap_or(k as u32));
        let mut colours = [0; 4];
        for (k, entry) in colours.iter_mut().enumerate() {
            *entry = f.colour(zero_scan::<3>(v[8 + k], "(%d,%d,%d)")?.map(|v| v as u8));
        }
        let font_id = f.font_id(v[12]);
        let font = f.font(font_id);
        if node_fonts.contains_key(v[0]) {
            return Err(Unsupported::Malformed("duplicate TreeView node id"));
        }
        node_fonts.insert(v[0].to_string(), font_id);
        pending.push(Node {
            id: v[0].into(),
            label: v[1].into(),
            parent: v[2].into(),
            position,
            icons,
            colours,
            font,
            expanded: v[13] == "yes",
            selected: v[14] == "yes",
        });
    }
    param(h, 0x86c21da0, Value::StringNumbers(node_fonts), f);
    let mut ids = BTreeMap::new();
    let mut handles = BTreeMap::new();
    let mut stack = vec![("Root".to_string(), 0)];
    while let Some((id, parent)) = stack.last() {
        let Some(k) = pending.iter().position(|v| v.parent == *id) else {
            stack.pop();
            continue;
        };
        let node = pending.remove(k);
        let handle = word(call(
            f,
            t,
            0x60,
            vec![n(*parent), i(node.position), text(node.label), n(0)],
        ))?;
        for s in 0..4 {
            call(f, t, 0x90, vec![n(handle), n(s), n(node.icons[s as usize])]);
        }
        call(f, t, 0x9c, vec![n(handle), Value::Handle(node.font)]);
        for w in 0..4 {
            call(f, t, 0xac, vec![n(handle), n(w), n(node.colours[w as usize])]);
        }
        call(f, t, 0xb4, vec![n(handle), n(node.expanded as u32)]);
        if node.selected {
            call(f, t, 0xe0, vec![n(handle), n(0)]);
        }
        // LC10: duplicate reverse-map keys have no published insertion contract.
        if handles.contains_key(&handle) {
            return Err(Unsupported::Malformed("duplicate TreeView node handle"));
        }
        ids.insert(node.id.clone(), handle);
        handles.insert(handle, node.id.clone());
        stack.push((node.id, handle));
    }
    param(h, 0xe6c21d83, Value::StringNumbers(ids), f);
    param(h, 0x26c21e35, Value::NumberStrings(handles), f);
    Ok(())
}
