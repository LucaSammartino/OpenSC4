// SPDX-License-Identifier: GPL-3.0-or-later
//! Local status state and numeric presentation rules (view3d-window §6j).

use super::View3DWin;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CostMode {
    Hidden,
    Following,
    Fading,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CostDisplay {
    pub amount: i32,
    pub cost: i32,
    pub mode: CostMode,
    pub alpha: u8,
    pub colour: [u8; 3],
    pub position: (i32, i32),
    pub fade_left_ms: f32,
}

impl Default for CostDisplay {
    fn default() -> Self {
        Self {
            amount: 0,
            cost: 0,
            mode: CostMode::Hidden,
            alpha: 0,
            colour: [255, 255, 40],
            position: (0, 0),
            fade_left_ms: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorText {
    pub id: u32,
    pub priority: i32,
    pub text1: String,
    pub text2: String,
    pub flag: bool,
}

#[derive(Debug, Default)]
pub struct DisplayState {
    pub cost: CostDisplay,
    pub cursor: (i32, i32),
    pub texts: Vec<CursorText>,
    pub rating_column: i32,
    pub mission_panel: Option<u32>,
    pub fade_origin: (i32, i32),
}

/// Window drawing, localization, widget rectangles and simulation queries remain
/// foreign. The owner supplies frame/colour/position decisions, not pixels.
pub trait StatusHost {
    fn funds(&self) -> i32;
    fn draw_cost(&mut self, display: CostDisplay);
    fn cursor_callout(&mut self, entry: Option<&CursorText>, position: (i32, i32));
    fn project(&mut self, point: [f32; 3]) -> (i32, i32);
    fn clock_time(&self) -> (u32, u32);
    fn clock_widgets(&mut self, hour: u32, minute: u32, frame: u32);
    /// Button AA75CA06 state 0x2000, read through the window interface.
    fn clock_hover_active(&self) -> bool;
    /// Window AA771FD7, caption AA771FC7; formatting remains foreign.
    fn clock_hover_widgets(&mut self, visible: bool, hour: u32, minute: u32);
    fn rating_widgets(&mut self, column: i32, delta_pixels: i32);
    fn alert_picture(&mut self, picture: Option<u32>);
    fn speed_button(&mut self, id: u32, toggled: bool, lower_half: bool);
    /// Create a script panel with its right edge eight pixels inside the view.
    fn create_mission_panel(&mut self, script: u32, right_margin: i32) -> Option<u32>;
    fn delete_mission_panel(&mut self, panel: u32);
    fn mission_text(&mut self, panel: u32, time: &str, red: bool, progress: &str);
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatusSnapshot {
    pub date: [i32; 3],
    pub start_year: i32,
    pub population: u32,
    pub city_name: String,
    pub budget_visible: bool,
    pub visual_time: (u32, u32),
    pub rating: i8,
    pub rating_width: i32,
    pub speed: u32,
    pub paused: bool,
    pub disaster: bool,
    pub situation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WidgetCall {
    Date([i32; 3]),
    Funds(i32, [u8; 3]),
    Population(u32),
    CityName { window: u32, name: String },
    RefreshBudget,
    RefreshRenderer,
    Enable(u32, bool),
}

/// Data, localized formatting and lighting are foreign. Branches and numeric
/// conversions belong to the view, including choosing the visual clock.
pub trait WidgetHost: StatusHost {
    fn status_snapshot(&self) -> StatusSnapshot;
    fn widget(&mut self, call: WidgetCall);
    fn ambient_light(&mut self, visual_hour: f32) -> bool;
}

impl View3DWin {
    pub fn update_zoom_buttons(&self, host: &mut (impl WidgetHost + super::MotionHost)) {
        host.widget(WidgetCall::Enable(0x4992_046A, self.can_zoom_in(host)));
        host.widget(WidgetCall::Enable(0xE992_0494, self.can_zoom_out(host)));
    }

    pub fn update_clock_hover(&self, host: &mut impl StatusHost, time: Option<(u32, u32)>) {
        let (hour, minute) = time.unwrap_or_else(|| host.clock_time());
        host.clock_hover_widgets(host.clock_hover_active(), hour, minute);
    }
    pub fn update_date(&self, host: &mut impl WidgetHost) {
        let data = host.status_snapshot();
        let mut date = data.date;
        // §6k: zero and negative year differences pass through unchanged.
        let year = i64::from(date[0]) - i64::from(data.start_year);
        date[0] = if year > 10000 {
            ((year - 1) % 10000 + 1) as i32
        } else {
            year as i32
        };
        host.widget(WidgetCall::Date(date));
    }

    pub fn update_funds(&mut self, host: &mut impl WidgetHost) {
        let funds = host.funds();
        host.widget(WidgetCall::Funds(
            funds,
            if funds < 0 { [0x77, 0, 0] } else { [0x44, 0x55, 0x66] },
        ));
        if self.owner.display.cost.colour == [255, 40, 40] {
            self.update_budget_cost(host);
        }
        if host.status_snapshot().budget_visible {
            host.widget(WidgetCall::RefreshBudget);
        }
    }

    pub fn update_population(&self, host: &mut impl WidgetHost) {
        host.widget(WidgetCall::Population(host.status_snapshot().population));
    }

    pub fn update_city_name(&self, host: &mut impl WidgetHost) {
        host.widget(WidgetCall::CityName {
            window: 2,
            name: host.status_snapshot().city_name,
        });
    }

    pub fn update_ambient_light(&self, host: &mut impl WidgetHost) {
        let (hour, minute) = host.status_snapshot().visual_time;
        if host.ambient_light(hour as f32 + minute as f32 / 60.0) {
            host.widget(WidgetCall::RefreshRenderer);
        }
    }
    pub fn set_cost_indicator(&mut self, host: &mut impl StatusHost, amount: i32, cost: i32) {
        let old = self.owner.display.cost;
        let red = cost > host.funds();
        let cursor = self.owner.display.cursor;
        self.owner.display.cost = CostDisplay {
            amount,
            cost,
            mode: CostMode::Following,
            alpha: if red { 255 } else { 150 },
            colour: if red { [255, 40, 40] } else { [255, 255, 40] },
            position: (cursor.0.saturating_sub(15), cursor.1.saturating_sub(15)),
            fade_left_ms: 0.0,
        };
        if old != self.owner.display.cost {
            host.draw_cost(self.owner.display.cost);
        }
    }

    pub fn set_cost_amount(&mut self, host: &mut impl StatusHost, amount: i32) {
        self.set_cost_indicator(host, amount, amount);
    }

    pub fn clear_cost_indicator(&mut self, host: &mut impl StatusHost, fade: bool) {
        let cost = &mut self.owner.display.cost;
        if fade {
            cost.mode = CostMode::Fading;
            cost.colour = [255, 255, 255];
            cost.fade_left_ms = 1500.0;
            self.owner.display.fade_origin = self.owner.display.cursor;
            cost.position = self.owner.display.cursor;
        } else {
            cost.mode = CostMode::Hidden;
            cost.alpha = 0;
            cost.fade_left_ms = 0.0;
        }
        host.draw_cost(*cost);
    }

    pub fn update_cost_indicator(&mut self, host: &mut impl StatusHost, dt: f32) {
        let cost = &mut self.owner.display.cost;
        if cost.mode == CostMode::Fading {
            let remaining = f64::from(cost.fade_left_ms) - f64::from(dt) * 1000.0;
            cost.fade_left_ms = remaining as f32;
            if remaining <= 0.0 {
                cost.mode = CostMode::Hidden;
                cost.alpha = 0;
            } else {
                cost.alpha = (remaining * f64::from(0.17_f32)) as u8;
                let origin = self.owner.display.fade_origin;
                cost.position = (
                    origin.0,
                    (f64::from(origin.1) - (1500.0 - remaining) * f64::from(0.133_333_34_f32)) as i32,
                );
            }
            host.draw_cost(*cost);
        }
    }

    pub fn update_budget_cost(&mut self, host: &mut impl StatusHost) {
        let cost = self.owner.display.cost;
        if cost.mode == CostMode::Following {
            self.set_cost_indicator(host, cost.amount, cost.cost);
        }
    }

    pub fn set_cursor_text(&mut self, host: &mut impl StatusHost, entry: CursorText) {
        let texts = &mut self.owner.display.texts;
        if let Some(old) = texts.iter_mut().find(|old| old.id == entry.id) {
            let priority = old.priority;
            *old = entry;
            old.priority = priority;
        } else {
            let index = texts
                .iter()
                .position(|old| (old.priority as u32) <= (entry.priority as u32))
                .unwrap_or(texts.len());
            texts.insert(index, entry);
        }
        self.refresh_cursor_text(host);
    }

    pub fn clear_cursor_text(&mut self, host: &mut impl StatusHost, id: u32) {
        self.owner.display.texts.retain(|entry| entry.id != id);
        self.refresh_cursor_text(host);
    }

    pub fn refresh_cursor_text(&self, host: &mut impl StatusHost) {
        host.cursor_callout(self.owner.display.texts.first(), self.owner.display.cursor);
    }

    pub fn update_cursor_position(&mut self, host: &mut impl StatusHost, x: i32, y: i32) {
        self.owner.display.cursor = (x, y);
        self.refresh_cursor_text(host);
        let cost = &mut self.owner.display.cost;
        if cost.mode == CostMode::Following {
            let position = (x.saturating_sub(15), y.saturating_sub(15));
            if position != cost.position {
                cost.position = position;
                host.draw_cost(*cost);
            }
        }
    }

    pub fn set_cursor_position_3d(&mut self, host: &mut impl StatusHost, point: [f32; 3]) {
        let (x, y) = host.project(point);
        self.update_cursor_position(host, x, y);
    }

    pub fn update_24hour_clock(&self, host: &mut impl StatusHost, hour: u32, minute: u32) {
        let (hour, minute) = if hour > 23 { host.clock_time() } else { (hour, minute) };
        host.clock_widgets(hour, minute, 2 * hour + u32::from(minute > 29));
        self.update_clock_hover(host, Some((hour, minute)));
    }

    pub fn update_mayor_rating(&mut self, host: &mut impl StatusHost, rating: i8, width: i32) {
        // A missing/zero-width picture has no column; avoid an invalid range.
        if width <= 0 {
            return;
        }
        let column = ((i64::from((i32::from(rating) + 100).clamp(0, 200)) * i64::from(width)) / 200)
            .clamp(0, i64::from(width - 1)) as i32;
        let delta = (column - self.owner.display.rating_column).clamp(-3, 3);
        self.owner.display.rating_column = column;
        host.rating_widgets(column, 7 * delta);
    }

    pub fn update_alert_border(&self, host: &mut impl StatusHost, disaster: bool, situation: bool, paused: bool) {
        host.alert_picture(if disaster {
            Some(0x1431_5E60)
        } else if situation {
            Some(0x1431_5E62)
        } else if paused {
            Some(0x1431_5E61)
        } else {
            None
        });
    }

    pub fn update_speed_buttons(&self, host: &mut impl StatusHost, speed: u32, paused: bool) {
        for (index, id) in [0x6A4F_BB31, 0x4A4F_BB60, 0x8998_BBDF].into_iter().enumerate() {
            host.speed_button(id, index as u32 == speed, paused);
        }
    }

    pub fn show_mission_status_panel(&mut self, host: &mut impl StatusHost, script: u32) {
        if self.owner.display.mission_panel.is_none() {
            self.owner.display.mission_panel = host.create_mission_panel(script, 8);
        }
    }

    pub fn hide_mission_status_panel(&mut self, host: &mut impl StatusHost) {
        if let Some(panel) = self.owner.display.mission_panel.take() {
            host.delete_mission_panel(panel);
        }
    }

    pub fn update_mission_status_panel(&self, host: &mut impl StatusHost, seconds: u32, progress: &str) {
        if let Some(panel) = self.owner.display.mission_panel {
            let time = format!("{:02}:{:02}", seconds / 60, seconds % 60);
            let red = seconds < 60 && (seconds > 20 || seconds % 2 == 1);
            host.mission_text(panel, &time, red, progress);
        }
    }
}
