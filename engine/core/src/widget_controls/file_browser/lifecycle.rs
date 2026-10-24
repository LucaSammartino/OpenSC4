// SPDX-License-Identifier: GPL-3.0-or-later
//! Settled lifecycle orchestration, win-filebrowser §§8.5–8.7.
use super::*;

pub trait BrowserHost: IconIo + BrowserIo {
    fn set_width(&mut self, window: Handle, width: u32);
    fn set_height(&mut self, window: Handle, height: u32);
    fn base_destroy(&mut self, window: Handle);
}

/// Primitive factory operations for the specified AddControls prefix. The
/// creation order/configuration remains FBQ2, so this is not full AddControls.
pub trait ControlIo: BrowserForeign {
    fn component(&mut self, class: u32, iid: u32) -> Option<Handle>;
    fn builtin(&mut self, class: u32, iid: u32) -> Option<Handle>;
}
pub fn obtain_control(index: usize, f: &mut impl ControlIo) -> Result<Option<Handle>> {
    let (class, iid) = match index {
        0 | 5 => (0xdaa6_b9bf, 0xdaa6_b9be),
        2 | 3 | 4 | 9 => (0x22ec_fc47, 0x0000_8810),
        6 => (0x231a_1c58, 0x231a_1c57),
        // The table does not establish the remaining factory classes.
        _ => return Err(Unsupported::Open("FBQ2")),
    };
    if let Some(control) = f.component(class, iid) {
        return Ok(Some(control));
    }
    let control = f.builtin(class, iid);
    if let Some(control) = control {
        f.add_ref(control);
    }
    Ok(control)
}

/// Explicit virtual-override boundary for the *unsettled* primary bodies.
/// The default never pretends to implement their missing owner algorithms.
/// Tests may supply overrides to reach later, independently settled Init paths.
pub trait BrowserOverrides<H: BrowserHost> {
    fn property(&mut self, _: &mut FileBrowser, _: u32, _: &[u32], _: &mut H) -> Result<()> {
        Err(Unsupported::Open("FBQ4"))
    }
    fn add_controls(&mut self, _: &mut FileBrowser, _: &mut H) -> Result<bool> {
        Err(Unsupported::Open("FBQ2"))
    }
    fn refresh(&mut self, _: &mut FileBrowser, _: Refresh, _: &mut H) -> Result<()> {
        Err(Unsupported::Open("FBQ5"))
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
        // Slot 93/94 bodies remain FBQ4; preserve their explicit virtual calls.
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
                Self::create_default_button_buffer(&mut buffer, f)?;
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

    /// Placement is supplied explicitly until FBQ2 settles background geometry.
    /// No background operation means the native optional-background path is off.
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
            f.invalidate(self.window());
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
