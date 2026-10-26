// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled lifecycle orchestration, win-filebrowser §§8.5–8.7.
use super::*;

pub trait BrowserHost: PopulationIo {
    fn set_width(&mut self, window: Handle, width: u32);
    fn set_height(&mut self, window: Handle, height: u32);
    fn base_destroy(&mut self, window: Handle);
}

/// Individual child operations; construction order and configuration live here.
pub trait ControlIo: IconIo {
    fn component(&mut self, class: u32, iid: u32) -> Option<Handle>;
    fn builtin(&mut self, class: u32, iid: u32) -> Option<Handle>;
    fn builtin_spec(&mut self, class: u32, iid: u32, _layout: [u32; 3]) -> Option<Handle> {
        self.builtin(class, iid)
    }
    fn component_result(&mut self, class: u32, iid: u32) -> (bool, Option<Handle>) {
        let value = self.component(class, iid);
        (value.is_some(), value)
    }
    fn control(&mut self, control: Handle, operation: ControlOp);
    fn control_name(&mut self, index: usize) -> Vec<u8>;
    fn confirmation_caption(&mut self) -> Vec<u8>;
}
pub fn obtain_control(index: usize, f: &mut impl ControlIo) -> Result<Option<Handle>> {
    let (class, iid) = match index {
        0 | 1 | 5 => (0xdaa6_b9bf, 0xdaa6_b9be),
        2 | 3 | 4 | 8 | 9 => (0x22ec_fc47, 0x0000_8810),
        6 => (0x231a_1c58, 0x231a_1c57),
        7 => (0x59b, 0x412c_e496),
        10..=12 => (0x592, 0x212c_dc1f),
        _ => return Err(Unsupported::Precondition("FileBrowser child index")),
    };
    let (success, control) = f.component_result(class, iid);
    if success {
        return control
            .map(Some)
            .ok_or(Unsupported::Precondition("factory true with null child"));
    }
    let layout = match index {
        0 | 1 | 5 => [744, 0x19bff, 0],
        6 => [656, 0x11ff, 0xd8],
        7 => [316, 1, 0xd8],
        10..=12 => [276, 0, 0xd8],
        _ => [436, 0, 0],
    };
    let control = f.builtin_spec(class, iid, layout);
    let control = control.ok_or(Unsupported::Precondition("null built-in child"))?;
    f.add_ref(control);
    Ok(Some(control))
}

/// Virtual dispatch remains overridable; defaults execute the settled owner.
pub trait BrowserOverrides<H: BrowserHost> {
    fn property(&mut self, b: &mut FileBrowser, slot: u32, args: &[u32], f: &mut H) -> Result<()> {
        b.init_property(slot, args, f)
    }
    fn add_controls(&mut self, b: &mut FileBrowser, f: &mut H) -> Result<bool> {
        b.add_controls(f)
    }
    fn refresh(&mut self, b: &mut FileBrowser, kind: Refresh, f: &mut H) -> Result<()> {
        match kind {
            Refresh::DirectoryList => {
                b.populate_directory_list(f)?;
            }
            Refresh::FileList => {
                b.populate_file_list(f)?;
            }
        }
        Ok(())
    }
}
pub struct Unsettled;
impl<H: BrowserHost> BrowserOverrides<H> for Unsettled {}

impl FileBrowser {
    pub fn update_path_live<H: BrowserHost>(
        &mut self,
        f: &mut H,
        overrides: &mut impl BrowserOverrides<H>,
    ) -> Result<bool> {
        if !f.directory_exists(&self.current) {
            self.current.clone_from(&self.fallback);
        }
        let id = f.window_id(self.window());
        f.notify_path(0x7abe_7205, id, &self.current, 0);
        overrides.refresh(self, Refresh::DirectoryList, f)?;
        overrides.refresh(self, Refresh::FileList, f)?;
        Ok(true)
    }

    pub fn init<H: BrowserHost>(&mut self, f: &mut H, overrides: &mut impl BrowserOverrides<H>) -> Result<bool> {
        if self.initialized {
            return Ok(true);
        }
        f.base_init(self.window()); // The Boolean base result is deliberately ignored.
        self.initialized = true;
        f.set_width(self.window(), 396);
        f.set_height(self.window(), 236);
        for index in 0..7 {
            self.load_default_icon(index, f)?;
        }
        overrides.property(self, 93, &[], f)?;
        overrides.property(self, 94, &[], f)?;
        if self.current.is_empty() {
            self.current.clone_from(&self.fallback);
        }
        self.raw_words.insert(0x30c, 0xff7c_90ff);
        overrides.property(self, 40, &[0, 5, 1], f)?;
        if self.common_font.is_none() {
            if let Some(font) = f.default_interface(0x6896_3c4c) {
                f.add_ref(font);
                self.common_font = Some(font);
                let rect = f.font_rect(font, b"");
                let height = rect[3].wrapping_sub(rect[1]) as u32;
                let current = self.raw_words.entry(0x224).or_insert(20);
                if height > *current {
                    *current = height;
                }
            }
        }
        overrides.property(self, 73, &[5, 5, 5], f)?;
        for slot in [61, 63] {
            overrides.property(self, slot, &[self.raw_words[&0x2e8]], f)?;
        }
        overrides.property(self, 65, &[self.raw_words[&0x224]], f)?;
        for slot in [67, 69] {
            overrides.property(self, slot, &[0, 0], f)?;
        }
        self.raw_bytes.insert(0x29e, 6);
        self.raw_bytes.insert(0x2c2, 6);
        // RGB packing is resolved by the window conversion, not assumed device pixels.
        let dark = f.convert_rgb(self.window(), [8, 36, 107]);
        let white = f.convert_rgb(self.window(), [255, 255, 255]);
        overrides.property(self, 71, &[dark, white], f)?;
        overrides.property(self, 79, &[1], f)?;
        let modal = self.flags & 2 != 0;
        f.set_window_flag(self.window(), 0x20000, !modal);
        f.set_window_flag(self.window(), 0x40000, !modal);
        if modal {
            f.set_window_flag(self.window(), 0x10000, true);
        }
        for (selector, resource) in [(2, 0x683d_b4e5), (3, 0x683d_b4e6)] {
            let mut buffer = f.default_interface(resource);
            if let Some(buffer) = buffer {
                f.add_ref(buffer);
            } else {
                self.create_button_buffer(selector, &mut buffer, f)?;
            }
            self.set_button_buffer(selector, buffer, f);
            let buffer = buffer.ok_or(Unsupported::Precondition("FileBrowser Init temporary Release"))?;
            f.release(buffer);
        }
        if overrides.add_controls(self, f)? {
            self.select_filter(self.filter_index, f);
            // Slot 92 is the specified Boolean-true no-op.
            self.update_path_live(f, overrides)?;
        }
        self.dirty = false;
        Ok(true)
    }

    /// Explicit draw operation is retained for callers with a resolved context.
    pub fn paint<H: BrowserHost>(
        &mut self,
        background: Option<Draw>,
        f: &mut H,
        overrides: &mut impl BrowserOverrides<H>,
    ) -> Result<bool> {
        if let Some(operation) = background {
            f.draw(operation);
        }
        if self.dirty {
            self.update_path_live(f, overrides)?;
            self.dirty = false;
        }
        Ok(true)
    }

    /// The direct destructor deliberately omits browser Shutdown. Rust drops
    /// strings/storage; retained interface handles are not released here.
    pub fn destroy(self, f: &mut impl BrowserHost) {
        f.base_destroy(self.window());
    }
    pub fn copy_current<R>(&self, copy: impl FnOnce(&[u8]) -> R) -> R {
        copy(&self.current)
    }
    pub fn copy_parent_label<R>(&self, copy: impl FnOnce(&[u8]) -> R) -> R {
        copy(&self.parent_label)
    }
    pub fn set_new_directory_label(&mut self, label: &[u8]) -> bool {
        self.new_directory_label = label.to_vec();
        true
    }
    pub fn sort_words(&self, first: &mut u32, second: &mut u32) -> bool {
        *first = self.sort_words[0];
        *second = self.sort_words[1];
        true
    }
    pub fn wildcard_option(&self, output: &mut u32) -> bool {
        *output = self.wildcard_option;
        true
    }
}

pub struct BrowserOwner {
    pub browser: Option<Box<FileBrowser>>,
    pub references: u32,
}
impl BrowserOwner {
    pub fn new(browser: FileBrowser) -> Self {
        Self {
            browser: Some(Box::new(browser)),
            references: 1,
        }
    }
    pub fn add_ref(&mut self) -> u32 {
        assert!(self.browser.is_some(), "released browser owner");
        self.references = self.references.wrapping_add(1);
        self.references
    }
    pub fn release(&mut self, f: &mut impl BrowserHost) -> u32 {
        if self.references > 1 {
            self.references -= 1;
            return self.references;
        }
        let mut browser = self.browser.take().expect("released browser owner");
        browser.shutdown(f);
        (*browser).destroy(f);
        self.references = 0;
        0
    }
}
