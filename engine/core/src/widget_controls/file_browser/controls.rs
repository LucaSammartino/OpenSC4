// SPDX-License-Identifier: GPL-3.0-or-later
//! AddControls, win-filebrowser §8.2. Foreign calls are individual primitives.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlOp {
    Call(u32, Vec<u32>),
    Font(Option<Handle>),
    Id(u32),
    Name(Vec<u8>),
    Caption(Vec<u8>),
    AddChild(Handle),
    Dimensions(u32, u32, bool),
    WindowFlag(u32, bool),
    Height(u32),
    Fill(u32),
    Borders([u32; 4]),
    Image(u32, Option<Handle>, Option<Rect>),
    Buffer(Handle),
    Append(Vec<u8>, u32),
    Select(u32, bool),
    Style(u32, i32, i32),
    SelectionWords(u32, u32),
    Outline(u8, u32),
    Color(u32, u32),
}

impl FileBrowser {
    pub fn add_controls(&mut self, f: &mut impl ControlIo) -> Result<bool> {
        let black = f.convert_rgb(self.window(), [0, 0, 0]);
        let white = f.convert_rgb(self.window(), [255, 255, 255]);
        for index in [0, 2, 5, 3, 4, 6, 7, 8, 9, 10, 11, 12] {
            if self.enabled[index] == 0 {
                continue;
            }
            let control = obtain_control(index, f)?.ok_or(Unsupported::Precondition("required child"))?;
            // Rebuilding intentionally overwrites the retained reference without releasing it.
            self.children[index] = Some(control);
            if index == 7 {
                self.filter_combo = Some(control);
            }
            f.control(control, ControlOp::Id(FIRST_CHILD_ID + index as u32));
            let order = match index {
                0 => 0,
                2 => 1,
                5 => 2,
                3 => 3,
                4 => 4,
                _ => index - 1,
            };
            let name = f.control_name(order);
            f.control(control, ControlOp::Name(name));
            match index {
                0 | 5 => {
                    f.control(
                        control,
                        ControlOp::Dimensions(
                            if index == 0 { 2 } else { self.word(0x228) },
                            u32::from(index == 5),
                            index == 0,
                        ),
                    );
                    if self.common_font.is_some() {
                        f.control(control, ControlOp::Font(self.common_font));
                    }
                    let colors = if index == 0 {
                        [0x4000_0000; 2]
                    } else {
                        [self.word(0x304), self.word(0x300)]
                    };
                    f.control(control, ControlOp::SelectionWords(colors[0], colors[1]));
                    f.control(control, ControlOp::Fill(white));
                    if index == 0 {
                        f.control(control, ControlOp::Borders([1; 4]));
                    }
                    if index == 0 {
                        f.control(control, ControlOp::Outline(self.raw_bytes[&0x308], black));
                    }
                    for (color, slot) in [(black, 0), (white, 1)] {
                        f.control(control, ControlOp::Color(color, slot));
                    }
                    if index == 5 {
                        f.control(control, ControlOp::Borders([1; 4]));
                        f.control(control, ControlOp::Outline(self.raw_bytes[&0x308], black));
                    }
                    let flags: &[u32] = if index == 0 { &[0x800, 2, 4] } else { &[2] };
                    for &flag in flags {
                        f.control(control, ControlOp::WindowFlag(flag, false));
                    }
                    f.control(control, ControlOp::WindowFlag(0x20000, true));
                    if index == 0 {
                        f.control(control, ControlOp::Height(self.word(0x224).wrapping_add(2)));
                        f.control(control, ControlOp::Style(0x2a4, -1, -1));
                    }
                }
                2 => {
                    f.control(control, ControlOp::Call(0x14, vec![0x20, 0]));
                    let mut buffer = f.default_interface(0x6896_3ed8);
                    if let Some(buffer) = buffer {
                        f.add_ref(buffer);
                    } else {
                        self.create_combo_arrow(&mut buffer, f)?;
                    }
                    let buffer = buffer.ok_or(Unsupported::Precondition("required arrow buffer"))?;
                    let bounds = f.buffer_bounds(buffer);
                    f.control(control, ControlOp::Image(2, Some(buffer), Some(bounds)));
                    f.release(buffer);
                }
                3 | 4 => {
                    f.control(control, ControlOp::Call(0x14, vec![0x20, 0]));
                    if let Some(buffer) = self.button_buffers[index - 1] {
                        f.control(control, ControlOp::Buffer(buffer));
                        f.control(control, ControlOp::Image(2, Some(buffer), None));
                    }
                }
                6 => {
                    for flag in [0x200, 0x800] {
                        f.control(control, ControlOp::WindowFlag(flag, false));
                    }
                    f.control(control, ControlOp::Font(self.common_font));
                    for (slot, color) in [(0, black), (5, black), (1, white), (4, white)] {
                        f.control(control, ControlOp::Color(color, slot));
                    }
                }
                7 => {
                    f.control(control, ControlOp::Call(0x54, vec![0]));
                    f.control(control, ControlOp::Font(self.common_font));
                    f.control(control, ControlOp::Fill(white));
                    for (slot, color) in [(0, black), (2, white)] {
                        f.control(control, ControlOp::Color(color, slot));
                    }
                    f.control(control, ControlOp::Call(0x40, vec![black]));
                    for (i, label) in self.filter_labels.iter().enumerate() {
                        f.control(control, ControlOp::Append(label.clone(), i as u32));
                    }
                    if !self.filter_labels.is_empty() {
                        f.control(control, ControlOp::Select(self.filter_index, false));
                    }
                }
                8 | 9 => {
                    let caption = if index == 8 {
                        f.confirmation_caption()
                    } else {
                        b"Cancel".to_vec()
                    };
                    f.control(control, ControlOp::Caption(caption));
                    f.control(control, ControlOp::Image(2, None, None));
                    f.control(control, ControlOp::Call(0x14, vec![0x20, 0]));
                    if let Some(buffer) = self.button_buffers[index - 8] {
                        f.control(control, ControlOp::Buffer(buffer));
                    }
                }
                10..=12 => {
                    let caption: &[u8] = match index {
                        10 => b"Look in:",
                        11 => b"File name:",
                        _ => b"Files of type:",
                    };
                    f.control(control, ControlOp::Caption(caption.to_vec()));
                    f.control(control, ControlOp::Call(0x28, vec![black]));
                    f.control(control, ControlOp::Call(0x1c, vec![4, 0]));
                }
                _ => unreachable!(),
            }
            f.control(control, ControlOp::AddChild(self.window()));
        }
        Ok(true)
    }
}
