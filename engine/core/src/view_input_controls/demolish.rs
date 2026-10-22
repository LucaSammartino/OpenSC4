// SPDX-License-Identifier: GPL-3.0-or-later
use super::{sound, Pixel, PropFilter, PropId, PropWorld, DEMOLISH};

#[derive(Debug, Default)]
pub struct SignLabelDemolish {
    selected: Option<PropId>,
}
impl SignLabelDemolish {
    pub const ID: u32 = 0xCBCC_987E;
    pub const CURSOR: u32 = 0x8BD4_B428;
    pub fn init(&self, host: &mut impl PropWorld) {
        host.set_cursor(Self::CURSOR);
        host.show_signs_and_labels(true);
    }
    pub fn selected(&self) -> Option<PropId> {
        self.selected
    }
    pub fn pick(&self, host: &mut impl PropWorld, at: Pixel) -> Option<PropId> {
        host.pick_prop(at, PropFilter::SignOrLabel).or_else(|| {
            host.pick_terrain(at)
                .and_then(|p| host.first_prop_at(p, PropFilter::SignOrLabel))
        })
    }
    fn highlight(host: &mut impl PropWorld, prop: PropId, on: bool) {
        let info = host.prop_info(prop);
        if info.prop_type == super::LABEL_TYPE {
            host.set_alpha(prop, if on { 0.7 } else { 0.5 });
        } else if info.prop_type == super::SIGN_TYPE {
            host.set_dynamic(prop, on);
        }
    }
    pub fn select(&mut self, host: &mut impl PropWorld, prop: Option<PropId>) {
        if self.selected == prop {
            return;
        }
        if let Some(old) = self.selected {
            Self::highlight(host, old, false);
        }
        self.selected = prop;
        if let Some(new) = prop {
            Self::highlight(host, new, true);
        }
    }
    pub fn mouse_move(&mut self, host: &mut impl PropWorld, at: Pixel) -> bool {
        let prop = self.pick(host, at);
        self.select(host, prop);
        true
    }
    pub fn mouse_up(&mut self, host: &mut impl PropWorld) -> bool {
        if let Some(prop) = self.selected.take() {
            host.remove_prop(prop);
            sound(host, DEMOLISH);
        }
        true
    }
}
