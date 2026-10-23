// SPDX-License-Identifier: GPL-3.0-or-later
use super::model::loc;
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Tip {
    pub title: String,
    pub body: String,
    pub icon: u32,
    pub secondary: u32,
    pub fraction: f32,
}
impl Default for Tip {
    fn default() -> Self {
        Self {
            title: String::new(),
            body: String::new(),
            icon: 0,
            secondary: 0,
            fraction: 0.5,
        }
    }
}

pub fn load_ratio(used: u32, capacity: u32) -> f32 {
    if capacity == 0 {
        return 0.0;
    }
    if used as f32 <= 0.8 * capacity as f32 {
        1.0
    } else {
        capacity.saturating_sub(used) as f32 / (0.199_999_99 * capacity as f32)
    }
}
pub fn rating_level(percent: f64) -> u32 {
    if percent < 33.3 {
        0x4A5F_E266
    } else if percent < 66.6 {
        0x0A5F_E260
    } else {
        0xAA5F_E249
    }
}
pub(crate) fn append_line(body: &mut String, text: &str) {
    if !body.is_empty() {
        body.push('\n');
    }
    body.push_str(text);
}
fn headlines(world: &impl World, lot: Option<Token>, body: &mut String) {
    if let Some(l) = lot {
        for line in world.lot(l).headlines.iter().take(3) {
            append_line(body, line);
        }
    }
}
pub fn display_name(world: &impl World, o: &Occupant) -> String {
    if let Some(Property::Text(s)) = o.properties.get(&0x899A_FBAD) {
        return s.clone();
    }
    if let Some(Property::Key(g, i)) = o.properties.get(&0x8A41_6A99) {
        return world.localize(*g, *i);
    }
    if let Some(Property::Text(s)) = o.properties.get(&0x20) {
        return s.clone();
    }
    String::new()
}

impl QueryTool {
    pub fn terrain_tip(&self, world: &impl World, tip: &mut Tip) {
        let c = world.position_to_cell(self.point);
        if world.cell_has_water(c) {
            return;
        }
        tip.title = loc(world, 0x6A5B_BCA6);
        const ZONES: [u32; 16] = [
            0xAA5B_BDE4,
            0x0A5B_BDEF,
            0xAA5B_BE08,
            0x8A5B_BE12,
            0x6A5B_BE1F,
            0x6A5B_BE29,
            0xAA5B_BE33,
            0x0A5B_BE3C,
            0xCA5B_BE4A,
            0x0A5B_BE51,
            0x6A5B_BE61,
            0xAA5B_BE69,
            0x2A5B_BE6F,
            0xCA5B_BE78,
            0x2A5B_BE80,
            0x2A5B_BE85,
        ];
        let z = world.zone(c) as usize;
        if let Some(id) = ZONES.get(z) {
            if z == 0 || z == 15 {
                tip.title = loc(world, *id);
            } else {
                tip.body = loc(world, *id);
            }
        }
        headlines(world, world.lot_at(c), &mut tip.body);
    }
    pub fn network_tip(&self, world: &impl World, id: Token, tip: &mut Tip) {
        let o = world.occupant(id);
        if !o.network_interface {
            return;
        }
        const TYPES: [(u8, u32); 12] = [
            (2, 0xAA55_89FB),
            (8, 0x8B8C_5CBD),
            (12, 0x2BE0_9C9E),
            (0, 0x6A55_89D4),
            (1, 0x4A55_89F3),
            (10, 0x4BD9_FDD8),
            (11, 0xCBE0_9C9B),
            (9, 0x0BE0_9CA2),
            (3, 0x8A55_8A04),
            (4, 0x0A55_8A0A),
            (7, 0x8A55_8A10),
            (6, 0x8B8C_5CD0),
        ];
        if let Some((t, s)) = TYPES.iter().find(|(t, _)| o.network_types.contains(t)) {
            tip.title = loc(world, *s);
            if *t == 4 && world.debug_query_enabled() {
                tip.body = format!(
                    "Distressed Pipes: {} / {}\n",
                    world.read(Read::PipeDistressed, Subject::Occupant(id)) as i32,
                    world.read(Read::PipeCount, Subject::Occupant(id)) as i32
                );
            }
        }
    }
    pub fn building_tip(&self, host: &mut impl Host, id: Token, tip: &mut Tip) {
        let o = host.occupant(id);
        let lot = host.lot_for_occupant(id);
        let s = Subject::Occupant(id);
        if host.debug_query_enabled() {
            tip.icon = 0;
            tip.title = match o.properties.get(&0x20) {
                Some(Property::Text(v)) => v.clone(),
                _ => String::new(),
            };
            host.effect(Effect::Unsettled(Open::DebugWritersCt14));
            return;
        }
        tip.title = o.name.clone();
        // Literal typed-array requirement: mistyped/missing groups skips icons.
        if let Some(Property::Words(groups)) = o.properties.get(&0xAA1D_D396) {
            const ROWS: [(u32, u32, u32); 16] = [
                (0x1000, 0x1441_6208, 0),
                (0x1001, 0x1441_6202, 0),
                (0x1002, 0x1441_6206, 0),
                (0x1003, 0x1441_6209, 0),
                (0x1004, 0x1441_620A, 0),
                (0x1501, 0x1441_6207, 0x1441_6212),
                (0x1500, 0x1441_6207, 0x1441_6212),
                (0x1502, 0x1441_6203, 0x1441_6211),
                (0x1503, 0x1441_6201, 0x1441_6210),
                (0x1504, 0x1441_6201, 0x1441_6210),
                (0x1505, 0x1441_6201, 0x1441_6210),
                (0x1506, 0x1441_6201, 0x1441_6210),
                (0x1507, 0x1441_6201, 0x1441_6216),
                (0x150A, 0x1441_620C, 0),
                (0x150B, 0x1441_620C, 0),
                (0x1006, 0x1441_6204, 0),
            ];
            let row = ROWS
                .iter()
                .find(|(g, _, _)| groups.contains(g))
                .copied()
                .unwrap_or((0, 0x1441_6200, 0));
            tip.icon = row.1;
            tip.secondary = row.2;
            match row.0 {
                0x1000..=0x1002 if lot.is_some() => {
                    let dev = host.read(Read::DeveloperType, s) as u32;
                    let entries: &[(u32, u32)] = match row.0 {
                        0x1000 => &[(0x1010, 0xCA95_F27E), (0x1020, 0x6A95_F2B0), (0x1030, 0xCA95_F2B8)],
                        0x1001 => &[
                            (0x3110, 0x8A95_F2BF),
                            (0x3120, 0xAA95_F2DC),
                            (0x3130, 0x8A95_F2E4),
                            (0x3320, 0x0A95_F39A),
                            (0x3330, 0x6A95_F3B3),
                        ],
                        _ => &[
                            (0x4100, 0x4A95_F2EB),
                            (0x4200, 0x2A95_F324),
                            (0x4300, 0x6A95_F313),
                            (0x4400, 0xCA95_F31C),
                        ],
                    };
                    if let Some((_, str_id)) = entries.iter().find(|(t, _)| *t == dev) {
                        tip.body = loc(host, *str_id);
                    }
                }
                0x1501 => {
                    let n = host.read(Read::JailCapacity, s) as u32;
                    tip.fraction = load_ratio((host.read(Read::JailedFraction, s) * f64::from(n)) as u32, n);
                }
                0x1500 => tip.fraction = host.read(Read::PoliceEfficiency, s) as f32,
                0x1502 => tip.fraction = host.read(Read::FireEfficiency, s) as f32,
                0x1503..=0x1506 => {
                    if host.read(Read::SchoolPresent, s) != 0.0 {
                        tip.fraction = host.read(Read::SchoolGrade, s) as f32;
                    }
                }
                0x1507 => {
                    if host.read(Read::HospitalPresent, s) != 0.0 {
                        tip.fraction = host.read(Read::HospitalGrade, s) as f32;
                    }
                }
                0x1003 => {
                    if groups.contains(&0x1508) {
                        tip.secondary = 0x1441_6214;
                        if lot.is_some() {
                            tip.fraction = load_ratio(
                                host.read(Read::AirportUsage, s) as u32,
                                host.read(Read::AirportCapacity, s) as u32,
                            );
                        }
                    } else if groups.contains(&0x1509) {
                        tip.secondary = 0x1441_6215;
                        if lot.is_some() {
                            tip.fraction = load_ratio(
                                host.read(Read::SeaportUsage, s) as u32,
                                host.read(Read::SeaportCapacity, s) as u32,
                            );
                        }
                    } else {
                        let cs = Subject::Cell(host.position_to_cell(o.position));
                        let available = host.read(Read::StationAvailable, cs);
                        let usage = host.read(Read::StationUsage, cs);
                        tip.fraction = if host.read(Read::StationPresent, cs) == 0.0 || available == 0.0 {
                            0.0
                        } else if usage == 0.0 {
                            1.0
                        } else {
                            (available / usage).min(1.0) as f32
                        };
                        let sub = [
                            (0x1307, 0x1441_621C),
                            (0x1300, 0x1441_621B),
                            (0x1301, 0x1441_621A),
                            (0x1302, 0x1441_621C),
                            (0x1303, 0x1441_621C),
                            (0x1304, 0x1441_6215),
                            (0x130B, 0x1441_620F),
                        ];
                        if let Some((_, i)) = sub.iter().find(|(g, _)| groups.contains(g)) {
                            tip.secondary = *i;
                        }
                    }
                }
                0x1004 => {
                    if groups.contains(&0x1400) {
                        tip.secondary = 0x1441_6218;
                        tip.fraction = host.read(Read::PowerEfficiency, s) as f32;
                    } else if groups.contains(&0x1401) {
                        tip.secondary = 0x1441_6219;
                        tip.fraction = host.read(Read::WaterEfficiency, s) as f32;
                        if host.read(Read::WaterPolluted, s) != 0.0 {
                            tip.body = loc(host, 0x4A5F_B66A);
                        }
                    } else if groups.contains(&0x1402) {
                        tip.secondary = 0x1441_6217;
                        let a = host.read(Read::RecyclingMaximum, s);
                        let b = host.read(Read::RecyclingCurrent, s);
                        tip.fraction = if b > 0.0 { (b / a).min(1.0) as f32 } else { 0.0 };
                    }
                }
                _ => {}
            }
        }
        headlines(host, lot, &mut tip.body);
    }
    pub fn build_tip(&self, host: &mut impl Host) -> Tip {
        let mut t = Tip::default();
        if let Some(id) = self.highlighted {
            let o = host.occupant(id);
            if self.mode == Mode::Route {
                if matches!(o.kind, Kind::Building | Kind::Network) {
                    host.effect(Effect::Unsettled(Open::TrafficInfoCt20));
                }
            } else {
                match o.kind {
                    Kind::Flora | Kind::Prop => t.title = display_name(host, &o),
                    Kind::Building => self.building_tip(host, id, &mut t),
                    Kind::Network => self.network_tip(host, id, &mut t),
                    Kind::PowerPole => t.title = loc(host, 0x0A5B_B9FD),
                    Kind::Landfill => t.title = loc(host, 0x4A5B_BA21),
                    _ => {}
                }
            }
        } else if self.terrain_query {
            self.terrain_tip(host, &mut t);
        }
        if self.terrain_query {
            if t.title.is_empty() {
                t.title = "Terrain".into();
            }
            let c = host.position_to_cell(self.point);
            append_line(
                &mut t.body,
                &format!(
                    "x={:.1} y={:.1} z={:.1}\ncell x={} cell z={}",
                    self.point.x, self.point.y, self.point.z, c.x, c.z
                ),
            );
        }
        t
    }
    pub fn set_tip(&self, host: &mut impl Host, title: &str, body: &str) {
        if title.is_empty() && body.is_empty() {
            host.effect(Effect::ClearCursorText(CURSOR_TEXT));
            if let Some(w) = self.callout {
                host.effect(Effect::Hide(w));
            }
            return;
        }
        if let Some(w) = self.callout {
            host.effect(Effect::CalloutText(w, title.into(), body.into()));
            if !host.visible(w) {
                let p = host.cursor_position(true);
                host.effect(Effect::MoveCallout(w, p));
                host.effect(Effect::Show(w));
            }
        }
    }
    pub fn display_tip(&self, host: &mut impl Host, t: &Tip) {
        if let Some(w) = self.callout.filter(|_| t.icon != 0) {
            // CT13: semantic image intent is published; native callout IID/slots
            // and integer argument meaning remain unsettled. No adapter guessed.
            host.effect(Effect::CalloutImages(w, t.icon, t.secondary, t.fraction));
            self.set_tip(host, &t.title, &t.body);
            host.effect(Effect::ClearCursorText(CURSOR_TEXT));
        } else {
            if let Some(w) = self.callout {
                host.effect(Effect::Hide(w));
            }
            if host.has_view() {
                host.effect(Effect::CursorText(CURSOR_TEXT, t.title.clone(), t.body.clone()));
            }
        }
    }
}
