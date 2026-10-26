// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// The published palette is retained without inventing the missing kind mapping.
pub const COLOURS: [&str; 6] = ["#8e9499", "#3f4967", "#095a22", "#b00000", "#f70a0b", "#0f0f0f"];
pub const TITLE_FACE: &str = "StocletITC TT";
pub const NOTE_FACE: &str = "Arta";
pub const ICON_RANGE: std::ops::RangeInclusive<u32> = 0x1441_6250..=0x1441_625f;

impl MySimWindow {
    pub fn advice_items(&self) -> &[Advice] {
        &self.advice
    }
    pub fn set_aside_items(&self) -> &BTreeSet<Advice> {
        &self.set_aside
    }
    pub fn visible_advice(&self) -> Result<&[Advice]> {
        // SPEC-GAP: open MW1; raw set operations do not establish visibility.
        Err(Unsupported::MessageVisibility)
    }
    /// §1.5, raw operations only. No inferred filtering by either flag.
    pub fn get_message_list(&mut self, host: &mut impl Host) -> Result<()> {
        let sim = self.selected_sim().ok_or(Unsupported::Precondition("selected Sim"))?;
        for item in &self.set_aside {
            let flags = host.advice_flags(*item);
            if flags & 2 == 0 {
                host.set_advice_flags(*item, flags & !0x20);
            }
        }
        self.set_aside.clear();
        self.advice = host.advice_items(sim);
        Ok(())
    }
    pub fn set_small_window(&mut self, windows: &mut Windows, host: &mut impl Host) -> Result<()> {
        self.set_text_window(BodySize::Small, windows, host)
    }
    pub fn set_large_window(&mut self, windows: &mut Windows, host: &mut impl Host) -> Result<()> {
        self.set_text_window(BodySize::Large, windows, host)
    }
    fn set_text_window(&mut self, size: BodySize, windows: &mut Windows, host: &mut impl Host) -> Result<()> {
        self.get_message_list(host)?;
        self.body_size = Some(size);
        let panel = match size {
            BodySize::Small => 0xea1f_1e4d,
            BodySize::Large => 0xaa1f_1ec5,
        };
        self.panels(windows, host, Some(panel));
        self.update_body_text_ctrl(host)
    }
    pub fn update_body_text_ctrl(&self, host: &mut impl Host) -> Result<()> {
        if let (Some(bindings), Some(size)) = (&self.bindings, self.body_size) {
            host.invalidate_text(bindings.body[usize::from(size == BodySize::Large)]);
        }
        // SPEC-GAP: MW3; a redraw request is supported, HTML construction is not.
        let _ = (COLOURS, TITLE_FACE, NOTE_FACE, ICON_RANGE);
        Err(Unsupported::MessageHtmlLayout)
    }
    pub fn do_message(
        &mut self,
        id: u32,
        item: Option<Advice>,
        windows: &mut Windows,
        host: &mut impl Host,
    ) -> Result<bool> {
        match id {
            0xaa25_827c => {
                if self.shown {
                    self.get_current_sims(host)?;
                    if self.selected_sim().is_none() {
                        self.selected = None;
                        self.panels(windows, host, Some(PANELS[0]));
                    }
                }
            }
            0xca09_fa6c | 0x8a09_fa64 => {
                if let Some(item) = item {
                    if self.selected_sim().is_some() && host.advice_owner(item) == self.selected_sim() {
                        if id == 0xca09_fa6c {
                            self.set_aside.remove(&item);
                        } else {
                            self.set_aside.insert(item);
                            let flags = host.advice_flags(item);
                            host.set_advice_flags(item, flags & !0x20);
                        }
                        self.update_body_text_ctrl(host)?;
                    }
                }
            }
            0x8a09_fa65 => {
                if self.selected_sim().is_some() {
                    self.update_body_text_ctrl(host)?;
                }
            }
            0xac08_746e | 0xac08_746f => {
                // SPEC-GAP: MW2; the U-Drive-It consequence is inferred only.
                return Err(Unsupported::TransportChoice);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
