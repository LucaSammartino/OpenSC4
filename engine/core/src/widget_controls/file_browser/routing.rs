// SPDX-License-Identifier: GPL-3.0-or-later
//! Primary-table DoMessage and synchronous navigation, §§2, 7, 8.2.2.
use super::*;

/// Decoded foreign inputs. `message` is retained verbatim for base/forward calls.
pub struct Received {
    pub message: Message,
    pub subtype: u32,
    pub sender: Handle,
    pub sender_id: u32,
    pub payload: [u32; 3],
    pub key: u32,
}

pub trait MessageIo: BrowserHost {
    fn combo_current(&mut self, combo: Handle) -> u32;
    fn current_cell(&mut self, grid: Handle) -> (i32, i32);
    fn forward_message(&mut self, window: Handle, message: Message);
    fn base_do_message(&mut self, window: Handle, message: Message) -> bool;
}

impl FileBrowser {
    pub fn set_current_live<H: BrowserHost>(
        &mut self,
        path: &[u8],
        f: &mut H,
        dispatch: &mut impl BrowserOverrides<H>,
    ) -> Result<bool> {
        if !f.directory_exists(path) {
            return Ok(false);
        }
        self.current = path.to_vec();
        self.update_path_live(f, dispatch)
    }

    pub fn parent_live<H: BrowserHost>(&mut self, f: &mut H, dispatch: &mut impl BrowserOverrides<H>) -> Result<bool> {
        let (count, second) = f.split_directory(&self.current, 2, 260);
        if count <= 1 {
            return Ok(false);
        }
        self.set_current_live(&second, f, dispatch)?;
        Ok(true)
    }

    pub fn file_double_click_live<H: BrowserHost>(
        &mut self,
        row: u32,
        f: &mut H,
        dispatch: &mut impl BrowserOverrides<H>,
    ) -> Result<bool> {
        let Some(record) = self.entries.get(row as usize) else {
            return Ok(false);
        };
        if let Some(record) = record {
            match record.tag {
                PARENT => {
                    self.parent_live(f, dispatch)?;
                }
                DIRECTORY => {
                    let path = [&self.current[..], &record.name[..], b"\\"].concat();
                    self.set_current_live(&path, f, dispatch)?;
                }
                _ => {}
            }
        }
        Ok(true)
    }

    pub fn do_message<H: MessageIo>(
        &mut self,
        input: Received,
        f: &mut H,
        dispatch: &mut impl BrowserOverrides<H>,
    ) -> Result<bool> {
        let Received {
            message,
            subtype,
            sender,
            sender_id,
            payload,
            key,
        } = input;
        match message.kind {
            3 if subtype == 0x2872_59f6 => match sender_id {
                0xbaa5_5d52 => {
                    self.open_popup(f)?;
                }
                0xbaa5_5d53 => {
                    self.parent_live(f, dispatch)?;
                }
                0xbaa5_5d54 => {
                    self.create_directory_live(f, dispatch)?;
                }
                0xbaa5_5d58 | 0xbaa5_5d59 => f.forward_message(self.window(), message),
                _ => {}
            },
            3 if subtype == 0xe871_1255 && sender_id == FIRST_CHILD_ID + 7 => {
                let index = f.combo_current(sender);
                if index != self.filter_index {
                    self.select_filter(index, f);
                }
            }
            0x9a15_52d2 if sender_id == FIRST_CHILD_ID + 5 => {
                if payload[1] as i32 != self.selected_row {
                    self.selection_change(payload[1] as usize, f)?;
                }
            }
            0x9a15_52d3 if sender_id == FIRST_CHILD_ID + 1 => {
                if payload[0] == 5 {
                    self.select_directory(payload[2] as i32, f, dispatch)?;
                }
                self.close_popup(f);
            }
            0x9a15_52d3 if sender_id == FIRST_CHILD_ID => {
                if let Some(grid) = self.children[0] {
                    f.clear_grid(grid);
                }
                self.open_popup(f)?;
            }
            0x9a15_52d4 if sender_id == FIRST_CHILD_ID + 5 && payload[0] == 5 => {
                self.selection_double_click(payload[2] as usize, f)?;
                self.file_double_click_live(payload[2], f, dispatch)?;
            }
            0x9a15_52d5 if sender_id == FIRST_CHILD_ID + 1 => {
                if key == 13 {
                    let (_, row) = f.current_cell(sender);
                    self.select_directory(row, f, dispatch)?;
                }
                if matches!(key, 13 | 27) {
                    self.close_popup(f);
                }
            }
            0x9a15_52d7 if sender_id == FIRST_CHILD_ID + 1 => {
                self.close_popup(f);
            }
            0x9b15_52d8 if self.children[6] == Some(sender) => self.notify_selection(0x7abe_7203, f),
            _ => {}
        }
        Ok(f.base_do_message(self.window(), message))
    }
}
