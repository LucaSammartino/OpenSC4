// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct SignpostData {
    pub rating: f32,
    pub fields: Vec<(u32, Field)>,
    pub visible: Vec<(u32, bool)>,
    pub enabled: Vec<(u32, bool)>,
    /// Borrowed for the fill call; controls must retain it if they keep it.
    pub rating_image: Option<Surface>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Field {
    Text(Vec<u8>),
    Money(i64),
    Number(i64),
}

pub fn rating_fraction(rating: i8) -> f32 {
    f32::from(rating) * 0.005 + 0.5
}

pub fn signpost_data(
    record: Option<&CityRecord>,
    hide_mayor: bool,
    negative_budget_child: bool,
) -> Result<SignpostData> {
    let Some(record) = record.filter(|record| record.established) else {
        // SPEC-GAP: RGV2 asks which resource owns the unclaimed-land price.
        return Err(Unsupported::Open("RGV2"));
    };
    let negative = record.budget < 0.0 && negative_budget_child;
    let budget_child = if negative { 0x4a55_2006 } else { 0x4a55_2002 };
    let mut data = SignpostData {
        rating: rating_fraction(record.mayor_rating),
        rating_image: None,
        fields: vec![
            (0x4a55_2000, Field::Text(record.city_name.clone())),
            (
                0x4a55_2001,
                Field::Text(if record.is_tutorial() && hide_mayor {
                    Vec::new()
                } else {
                    record.mayor_name.clone()
                }),
            ),
            (budget_child, Field::Money(record.budget as i64)),
            (0x4a55_2003, Field::Number(i64::from(record.population))),
            (0x4a55_2004, Field::Number(i64::from(record.commercial_jobs))),
            (0x4a55_2005, Field::Number(i64::from(record.industrial_jobs))),
        ],
        visible: vec![(0x4a55_2002, !negative), (0x4a55_2006, negative)],
        enabled: Vec::new(),
    };
    for (i, child) in [0x6c06_f4a0, 0xac06_f4c4, 0xcc06_f4cf].into_iter().enumerate() {
        data.visible.push((child, usize::from(record.difficulty) == i));
    }
    Ok(data)
}

#[derive(Clone, Debug, Default)]
pub struct Animator {
    pub start: u32,
    pub child: Option<Window>,
}

pub trait AnimatorHost {
    fn clock(&mut self) -> u32;
    fn find_child(&mut self, window: Window, id: u32) -> Option<Window>;
    fn retain_child(&mut self, child: Window);
    fn release_child(&mut self, child: Window);
    fn parent_child_heights(&mut self, child: Window) -> (i32, i32);
    fn move_child(&mut self, child: Window, position: (i32, i32));
}

impl Animator {
    pub fn message(&mut self, host: &mut impl AnimatorHost, window: Window, message: u32) -> bool {
        match message {
            0xa2bf_8ad5 => {
                self.start = host.clock();
                self.shutdown(host);
                self.child = host.find_child(window, 0x4a63_0000);
                if let Some(child) = self.child {
                    host.retain_child(child);
                }
                true
            }
            0xa2bf_8ad6 => {
                self.shutdown(host);
                true
            }
            0xa2bf_8acd => {
                if let Some(child) = self.child {
                    let elapsed = host.clock().wrapping_sub(self.start) % 1500;
                    #[allow(clippy::approx_constant)] // Compatibility constant in §7.
                    let phase = 6.28318 * f64::from(elapsed) / 1500.0;
                    let (parent, height) = host.parent_child_heights(child);
                    host.move_child(
                        child,
                        (0, (f64::from(parent - height) * (1.0 - phase.sin().abs())) as i32),
                    );
                }
                false
            }
            _ => false,
        }
    }

    pub fn shutdown(&mut self, host: &mut impl AnimatorHost) {
        if let Some(child) = self.child.take() {
            host.release_child(child);
        }
    }
}

impl RegionView {
    pub fn update_signpost(&mut self, host: &mut impl Host, id: CityId) -> Result<()> {
        let index = self
            .cities
            .iter()
            .position(|c| c.id == id)
            .ok_or(Unsupported::Precondition("city entry"))?;
        if let Some(old) = self.cities[index].signpost.take() {
            host.remove_release_signpost(self.window, old);
        }
        let key = self.key()?;
        let cell = self.cities[index].cell;
        let arrow = self.tutorial_arrow
            && self.selected != Some(id)
            && cell.0 >= 0
            && cell.1 >= 0
            && host.tutorial_tile(key) == Some(cell);
        let record = host.city_record(key, cell);
        if !arrow
            && (self.signpost_type.0 == 0
                || (!self.signposts.0 && self.selected != Some(id))
                || (!record.as_ref().is_some_and(|r| r.established) && !self.signposts.1))
        {
            return Ok(());
        }
        let (instance, window_id) = if arrow {
            (0xca53_9343, 0x0a55_1c53)
        } else {
            self.signpost_type
        };
        let state = host.manager_state();
        let result = if let Some(signpost) = host.create_signpost(self.window, instance, window_id) {
            self.cities[index].signpost = Some(signpost);
            self.place_signpost(host, index);
            let hide = record
                .as_ref()
                .is_some_and(|r| r.is_tutorial() && host.hide_tutorial_mayor(r.tutorial_id));
            let negative_child = host.signpost_has_child(signpost, 0x4a55_2006);
            signpost_data(record.as_ref(), hide, negative_child).and_then(|mut data| {
                data.rating_image = host.load_image((0, 0x46a0_06b0, 0x1441_6327, 9, 0));
                host.fill_signpost(signpost, &data);
                if let Some(image) = data.rating_image {
                    host.release_surface(image);
                }
                let mut children = host.signpost_children(signpost);
                while let Some(child) = children.pop() {
                    host.clear_child_flag(child, 0x8000);
                    // SPEC-GAP: RGV6 leaves IID 8810's notification contract open.
                    if host.child_has_8810(child) {
                        return Err(Unsupported::Open("RGV6"));
                    }
                    children.extend(host.signpost_children(child));
                }
                host.install_animator(signpost);
                Ok(())
            })
        } else {
            Ok(())
        };
        if host.manager_state_active(state) {
            host.restore_manager_state(state);
        }
        result
    }

    fn place_signpost(&self, host: &mut impl Host, index: usize) {
        let city = &self.cities[index];
        if let Some(signpost) = city.signpost {
            let (x, y) = self.draw_position(host, city);
            let (w, h) = host.dimensions(city.image);
            let (sw, sh) = host.signpost_size(signpost);
            host.move_signpost(signpost, ((x + x + w) / 2 - sw / 2, (y + y + h) / 2 - sh));
        }
    }

    pub fn set_scroll_pos(&mut self, host: &mut impl Host, x: i32, y: i32) {
        if self.scroll == (x, y) {
            return;
        }
        self.scroll = (x, y);
        for index in 0..self.cities.len() {
            self.place_signpost(host, index);
        }
        if let Some(canvas) = &mut self.canvas {
            canvas.scroll_to(x, y);
        }
    }
}
