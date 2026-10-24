// SPDX-License-Identifier: GPL-3.0-or-later
//! Paint passes from win-grid §5.12. Effective format merging remains WG5.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaintRecord {
    pub rect: Rect,
    pub clip: Rect,
    pub coordinate: Coordinate,
    pub cell: Option<Cell>,
    pub format: Format,
    pub selected: bool,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PaintOp {
    Primitive(Draw),
    Clip(Rect),
    Capability(u32, bool),
    LineWidth(f32),
    Line { from: [i32; 2], to: [i32; 2], color: u32 },
}

pub trait PaintHost: GridHost {
    fn paint_op(&mut self, operation: PaintOp);
    fn owner_paint(&mut self, target: Handle, id: u32, record: &PaintRecord) -> bool;
    /// Actual font line breaks/metrics, not an owner layout algorithm.
    fn wrapped_lines(&mut self, font: Handle, text: &[u8], width: i32) -> Vec<(Vec<u8>, i32)>;
}

fn offset(mut r: Rect, origin: [i32; 2]) -> Rect {
    for i in 0..4 {
        r[i] = r[i].wrapping_add(origin[i % 2]);
    }
    r
}
fn intersect(a: Rect, b: Rect) -> Rect {
    [a[0].max(b[0]), a[1].max(b[1]), a[2].min(b[2]), a[3].min(b[3])]
}
fn inset(mut rect: Rect, gutters: [i32; 4]) -> Rect {
    for i in 0..4 {
        rect[i] = rect[i].wrapping_add(if i < 2 { gutters[i] } else { -gutters[i] });
    }
    rect
}
fn color(format: &Format, byte_offset: usize) -> u32 {
    format.words[byte_offset / 4 - 1]
}
fn alignment(format: &Format) -> [u8; 2] {
    let bytes = format.words[6].to_le_bytes(); // format offsets 0x1c..0x1f
    [bytes[1], bytes[2]]
}

impl Grid {
    fn fill_paint(&self, rect: Rect, color: u32, f: &mut impl PaintHost) {
        if color != 0x4000_0000 && rect[2] > rect[0] && rect[3] > rect[1] {
            f.paint_op(PaintOp::Capability(0x8001, true));
            f.paint_op(PaintOp::Primitive(Draw::Fill { rect, color }));
            f.paint_op(PaintOp::Capability(0x8001, false));
        }
    }

    fn paint_background(&self, record: &PaintRecord, f: &mut impl PaintHost) {
        let mut rect = record.rect;
        if self.flags & 0x20 != 0 {
            rect[0] = rect[0].wrapping_add(self.line_widths[0] as i32);
        }
        if self.flags & 0x40 != 0 {
            rect[1] = rect[1].wrapping_add(self.line_widths[1] as i32);
        }
        let selected = record.selected && self.flags & 0x80000 == 0;
        self.fill_paint(
            intersect(rect, record.clip),
            color(&record.format, if selected { 0x18 } else { 8 }),
            f,
        );
    }

    fn paint_content(&self, record: &PaintRecord, f: &mut impl PaintHost) -> Result<()> {
        if self.flags & 0x20000 == 0 {
            self.paint_background(record, f);
        }
        let mut rect = inset(record.rect, self.cell_gutters);
        if self.flags & 0x20000 != 0 {
            rect[2] = record.clip[2];
        }
        let align = alignment(&record.format);
        f.paint_op(PaintOp::Clip(record.clip));
        match record.cell.as_ref().map(|c| &c.value) {
            Some(Value::Text(text)) => self.paint_text(record, rect, text, f)?,
            Some(Value::Bitmap(image)) => {
                let size = f.image_area(*image);
                let [w, h] = [size[2] - size[0], size[3] - size[1]];
                let x = match align[0] {
                    3 => (rect[0] + rect[2] - w) / 2,
                    4 => rect[2] - w,
                    _ => rect[0],
                };
                let y = match align[1] {
                    6 => (rect[1] + rect[3] - h) / 2,
                    7 => rect[3] - h,
                    _ => rect[1],
                };
                f.paint_op(PaintOp::Capability(0x8010, true));
                f.paint_op(PaintOp::Primitive(Draw::Blit {
                    source: *image,
                    rect: [x, y, x + w, y + h],
                }));
                f.paint_op(PaintOp::Capability(0x8010, false));
            }
            _ => {}
        }
        Ok(())
    }

    fn paint_text(&self, record: &PaintRecord, mut rect: Rect, text: &[u8], f: &mut impl PaintHost) -> Result<()> {
        let align = alignment(&record.format);
        if matches!(align[0], 3..=5) {
            return Err(Unsupported::Open("WG6"));
        }
        let Some(font) = record.format.font else {
            return Ok(());
        };
        let line = f.font_height(font);
        rect[1] = match align[1] {
            6 => (rect[1] + rect[3]) / 2 - line / 2,
            7 => rect[3] - line,
            _ => rect[1],
        };
        let text_color = color(
            &record.format,
            if record.selected {
                0x18
            } else if record.enabled {
                0x10
            } else {
                8
            },
        );
        if self.flags & 0x80000 != 0 && text_color != 0x4000_0000 {
            let mut y = rect[1];
            for (_, width) in f.wrapped_lines(font, text, rect[2] - rect[0]) {
                self.fill_paint(
                    intersect([rect[0], y, rect[0] + width, y + line], record.clip),
                    text_color,
                    f,
                );
                y += line;
            }
        }
        if text_color != 0x4000_0000 {
            f.paint_op(PaintOp::Primitive(Draw::Text {
                font,
                text: text.to_vec(),
                rect,
                color: text_color,
            }));
        }
        Ok(())
    }

    /// Resolved formats are explicit inputs because GetCellFormat's precedence
    /// is still open WG5. No inheritance/default resource is invented here.
    #[allow(clippy::too_many_arguments)]
    pub fn paint(
        &mut self,
        formats: &BTreeMap<Coordinate, Format>,
        header: &Format,
        origin: [i32; 2],
        window_color: u32,
        context: &mut LayoutContext,
        f: &mut impl PaintHost,
    ) -> Result<()> {
        if self.dirty {
            self.recalculate(context, f);
        }
        self.pause = 0;
        let size = f.window_size(self.window());
        let window = offset([0, 0, size[0], size[1]], origin);
        let area = offset(self.cell_area(size, f), origin);
        let used = self.used_range();
        let mut end = [0; 2];
        for axis in 0..2 {
            end[axis] = self.position[axis].wrapping_add(self.visible[axis] as i32);
            if self.flags & (0x8000 << axis) == 0 && used[axis + 2] <= end[axis] {
                end[axis] = used[axis + 2].wrapping_add(1);
            }
        }
        if self.flags & 0x100 != 0 {
            self.paint_window_background(window, window_color, f);
        }
        let enabled = f.enabled(self.window());
        // Grid-line extents use the whole row/column extent, clipped to viewport.
        let extent: [i32; 2] = std::array::from_fn(|axis| {
            let bound = if self.flags & (0x8000 << axis) != 0 {
                self.limits[axis]
            } else {
                used[axis + 2].wrapping_add(1)
            };
            if bound < 0 {
                area[axis + 2] - 1
            } else {
                (area[axis] + self.size_sum(axis, self.position[axis], bound.wrapping_sub(self.position[axis]), f))
                    .min(area[axis + 2] - 1)
            }
        });
        for axis in 0..2 {
            if self.flags & (0x20 << axis) == 0 || self.line_colors[axis] == 0x4000_0000 {
                continue;
            }
            for i in self.position[axis]..end[axis] {
                // Row lines exist only on a first column pass.
                if axis == 1 && end[0] <= self.position[0] {
                    break;
                }
                let coordinate = if axis == 0 { (i, 0) } else { (0, i) };
                let r = offset(self.cell_rect(coordinate, f), origin);
                let mut from = [area[0], area[1]];
                let mut to = [extent[0], extent[1]];
                from[axis] = r[axis + 2];
                to[axis] = from[axis];
                let record = PaintRecord {
                    rect: [from[0], from[1], to[0], to[1]],
                    clip: area,
                    coordinate,
                    cell: None,
                    format: Format::default(),
                    selected: false,
                    enabled,
                };
                if self
                    .owner_draw
                    .map_or(true, |target| f.owner_paint(target, 0xfa78_5cb0 + axis as u32, &record))
                {
                    f.paint_op(PaintOp::LineWidth(self.line_widths[axis] as f32));
                    f.paint_op(PaintOp::Line {
                        from,
                        to,
                        color: self.line_colors[axis],
                    });
                    f.paint_op(PaintOp::LineWidth(1.0));
                }
            }
        }
        for axis in 0..2 {
            if self.flags & (8 << axis) == 0 {
                continue;
            }
            for i in self.position[axis]..self.position[axis].wrapping_add(self.visible[axis].ceil() as i32) {
                let coordinate = if axis == 0 { (i, 0) } else { (0, i) };
                let mut rect = offset(self.cell_rect(coordinate, f), origin);
                // WG9: literal header band from the cell-area boundary; the
                // slot 72/73 bodies are not settled in the incorporated spec.
                if axis == 0 {
                    rect[1] = window[1] + self.gutters[1];
                    rect[3] = area[1];
                } else {
                    rect[0] = window[0] + self.gutters[0];
                    rect[2] = area[0];
                }
                let mut text = self.heading(axis, i).to_vec();
                if text.is_empty() && self.flags & 0x100000 != 0 {
                    text = format!("{} #{}", if axis == 0 { "Column" } else { "Row" }, i).into_bytes();
                }
                let record = PaintRecord {
                    rect,
                    clip: intersect(rect, window),
                    coordinate,
                    cell: None,
                    format: header.clone(),
                    selected: false,
                    enabled,
                };
                self.paint_background(&record, f);
                self.paint_text(&record, rect, &text, f)?;
            }
        }
        let mut records = Vec::new();
        for c in self.position[0]..end[0] {
            for r in self.position[1]..end[1] {
                let coordinate = (c, r);
                let rect = offset(self.cell_rect(coordinate, f), origin);
                let format = formats.get(&coordinate).ok_or(Unsupported::Open("WG5"))?.clone();
                records.push(PaintRecord {
                    rect,
                    clip: intersect(rect, area),
                    coordinate,
                    cell: self.cells.get(&coordinate).cloned(),
                    format,
                    selected: self.selected(coordinate),
                    enabled,
                });
            }
        }
        if self.flags & 0x20000 != 0 {
            for record in &records {
                self.paint_background(record, f);
            }
        }
        for record in records {
            if self
                .owner_draw
                .map_or(true, |target| f.owner_paint(target, 0xfa78_5cb2, &record))
            {
                self.paint_content(&record, f)?;
            }
        }
        f.paint_op(PaintOp::Clip(window));
        if self.bars.iter().all(Option::is_some) && self.flags & 0x100 != 0 {
            let corner = [area[2], area[3], window[2], window[3]];
            self.paint_window_background(
                if self.flags & 0x200 != 0 {
                    inset(corner, [1; 4])
                } else {
                    corner
                },
                window_color,
                f,
            );
        }
        for (flag, rect) in [(0x400, area), (0x200, window)] {
            if self.flags & flag != 0 {
                f.paint_op(PaintOp::Primitive(Draw::Outline {
                    rect,
                    colors: [self.outline; 2],
                    three_d: false,
                }));
            }
        }
        Ok(())
    }

    fn paint_window_background(&self, rect: Rect, color: u32, f: &mut impl PaintHost) {
        if let Some(source) = self.background {
            f.paint_op(PaintOp::Primitive(Draw::Blit { source, rect }));
        } else {
            self.fill_paint(rect, color, f);
        }
    }
}
