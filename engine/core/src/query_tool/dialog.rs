// SPDX-License-Identifier: GPL-3.0-or-later
use super::model::loc;
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Readout {
    pub text: String,
    pub extra: String,
    pub extra2: String,
    pub open: Vec<Open>,
}
impl Default for Readout {
    fn default() -> Self {
        Self {
            text: "Query".into(),
            extra: "Query extra: none".into(),
            extra2: "Query extra2: none".into(),
            open: Vec::new(),
        }
    }
}

/// Null table entries stay null even where §8.12.4 describes a zone worker.
/// §8.16.1 confirms landfill 1026..1028 are dispatched only by the zone handler.
pub fn filler_index(id: u32) -> Option<u32> {
    if id & 0x1000 == 0 {
        return None;
    }
    let i = id & 0xFFF;
    matches!(i, 0..=4 | 6..=37 | 41..=71 | 73..=74 | 81..=92 | 94..=99 | 107..=109 | 115..=126).then_some(i)
}

fn subject_lot(world: &impl World, s: Subject) -> Option<Token> {
    match s {
        Subject::Occupant(o) => world.lot_for_occupant(o),
        Subject::Cell(c) => world.lot_at(c),
    }
}
pub fn occupant_capacity(world: &impl World, o: Token) -> u32 {
    if let Some(l) = world.lot_for_occupant(o) {
        let l = world.lot(l);
        if l.capacity == 0 {
            u32::from(l.fallback_capacity)
        } else {
            l.capacity
        }
    } else {
        0
    }
}
pub fn current_occupancy(world: &impl World, o: Token) -> u32 {
    if let Some(l) = world.lot_for_occupant(o) {
        let l = world.lot(l);
        if l.building_type != 0 {
            (l.occupancy * f64::from(occupant_capacity(world, o))) as u32
        } else {
            0
        }
    } else {
        0
    }
}
pub fn mayor_percent(world: &impl World, o: Token) -> f64 {
    let Some(l) = world.lot_for_occupant(o) else {
        return 0.0;
    };
    let r = world.lot(l).rect;
    let mut sum = 0.0;
    let mut count = 0_u64;
    // CT19: the spec does not state rect endpoint convention. Literal cell
    // rectangle includes both endpoints; no engine-native layout is claimed.
    for z in r.min.z..=r.max.z {
        for x in r.min.x..=r.max.x {
            sum += world.read(Read::Aura, Subject::Cell(Cell { x, z })) + 128.0;
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        100.0 * sum / count as f64 / 255.0
    }
}
pub fn subjective_level(world: &impl World, s: Subject, i: u8) -> Option<u32> {
    let lot = world.lot(subject_lot(world, s)?);
    developer_rating(world, &lot, i).map(SubjectiveRating::label)
}
pub fn goofy_stat(curve: &ResponseCurve, mayor: f64) -> Option<i32> {
    curve.evaluate(mayor as f32).map(|v| v as i32)
}

#[derive(Clone, Copy)]
enum Funding {
    Fire,
    Police,
    SchoolStaff,
    SchoolBus,
    HospitalStaff,
    HospitalBus,
    Power,
}
fn funding(i: u32) -> Option<Funding> {
    match i {
        2 => Some(Funding::Fire),
        9 => Some(Funding::Police),
        24 => Some(Funding::SchoolStaff),
        25 => Some(Funding::SchoolBus),
        33 => Some(Funding::HospitalStaff),
        34 => Some(Funding::HospitalBus),
        99 => Some(Funding::Power),
        _ => None,
    }
}
fn funding_sources(f: Funding) -> (Read, Read, Write, Vec<u32>) {
    match f {
        Funding::Fire => (Read::FireFunding, Read::FireCost, Write::FireFunding, vec![]),
        Funding::Police => (Read::PoliceFunding, Read::PoliceCost, Write::PoliceFunding, vec![]),
        Funding::SchoolStaff => (
            Read::SchoolStaffFunding,
            Read::SchoolStaffCost,
            Write::SchoolFunding,
            vec![0x101A, 0x101B, 0x101D, 0x101F],
        ),
        Funding::SchoolBus => (
            Read::SchoolBusFunding,
            Read::SchoolBusCost,
            Write::SchoolFunding,
            vec![],
        ),
        Funding::HospitalStaff => (
            Read::HospitalStaffFunding,
            Read::HospitalStaffCost,
            Write::HospitalFunding,
            vec![0x1023, 0x1024, 4, 0x3F80_0000],
        ),
        Funding::HospitalBus => (
            Read::HospitalBusFunding,
            Read::HospitalBusCost,
            Write::HospitalFunding,
            vec![],
        ),
        Funding::Power => (Read::PowerFunding, Read::PowerCost, Write::PowerFunding, vec![]),
    }
}
fn write_funding(host: &mut impl Host, w: Token, s: Subject, f: Funding) {
    let Value::Number(v) = host.control_value(w) else {
        return;
    };
    let value = v * 0.01;
    let (_, _, write, _) = funding_sources(f);
    let value = match f {
        Funding::SchoolStaff => Value::Funding(value, host.read(Read::SchoolBusFunding, s)),
        Funding::SchoolBus => Value::Funding(host.read(Read::SchoolStaffFunding, s), value),
        Funding::HospitalStaff => Value::Funding(value, host.read(Read::HospitalBusFunding, s)),
        Funding::HospitalBus => Value::Funding(host.read(Read::HospitalStaffFunding, s), value),
        _ => Value::Number(value),
    };
    host.effect(Effect::Write(s, write, value));
}
fn percentage(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        0.0
    } else {
        100.0 * a / b
    }
}
fn per_unit(a: f64, b: f64) -> f64 {
    if b == 0.0 {
        0.0
    } else {
        a / b
    }
}

/// Owner's field selection/formatting; hosts supply only primitive simulator
/// values and locale formatting. A false return means recurse into children.
pub fn fill_control(host: &mut impl Host, w: Token, id: u32, get: bool, subject: Subject) -> bool {
    let zone = matches!(subject, Subject::Cell(_));
    let i = id & 0xFFF;
    if id & 0x1000 == 0 {
        return false;
    }
    if zone {
        if !matches!(i, 17..=21 | 38..=40 | 120 | 123..=126) {
            return false;
        }
    } else if filler_index(id).is_none() {
        return false;
    }
    let s = if matches!(i, 17..=21 | 120 | 123..=126) {
        match subject {
            Subject::Occupant(o) => Subject::Cell(host.occupant(o).cell),
            _ => subject,
        }
    } else {
        subject
    };
    if !get {
        if let Some(f) = funding(i) {
            write_funding(host, w, subject, f);
        } else if i == 0 {
            if let (Subject::Occupant(o), Value::Text(text)) = (subject, host.control_value(w)) {
                if text != host.occupant(o).name {
                    host.effect(Effect::Write(subject, Write::Name, Value::Text(text)));
                }
            }
        } else if i == 122 {
            if let Value::Bool(v) = host.control_value(w) {
                host.effect(Effect::Write(subject, Write::Historical, Value::Bool(v)));
            }
        }
        return true;
    }
    if i == 18 {
        // CT19: airport noise arithmetic remains unverified.
        host.effect(Effect::Unsettled(Open::BuildingValuesCt19));
        return true;
    }
    if i == 20 {
        let Subject::Cell(cell) = s else {
            return false;
        };
        if let Some(value) = airport_lost_luggage(host, cell) {
            host.effect(Effect::Text(w, value.to_string()));
            return true;
        }
        return false;
    }
    if let Some(f) = funding(i) {
        let (r, c, _, related) = funding_sources(f);
        let v = (100.0 * host.read(r, s)).round() as i32;
        let cost = host.money_value(c, s);
        host.effect(Effect::Slider(w, v + 1));
        host.effect(Effect::Slider(w, v));
        host.effect(Effect::SliderProperties(w, id, cost, related));
        let text = host.money((f64::from(v) * 0.01 * cost as f64) as i64);
        host.effect(Effect::MoneyChild(w, id + 0x100, text));
        if i != 99 {
            // CT21: §8.16.1 calls this a separate ID+2 update, but its cited
            // §8.12.4 specifies ID+0x100. Keep the explicit helper contract;
            // do not guess another target, amount or update phase.
            host.effect(Effect::Unsettled(Open::FundingCostCt21));
        }
        return true;
    }
    let r = |read| host.read(read, s);
    let cap = r(Read::JailCapacity);
    let (value, format) = match i {
        1 => (r(Read::FireEfficiency), Format::Meter),
        3 => (r(Read::FireDispatch), Format::Number),
        4 => (r(Read::FireCost), Format::Money),
        6 => (r(Read::PoliceEfficiency), Format::Meter),
        7 => (r(Read::Arrests), Format::Number),
        8 => (r(Read::Crimes), Format::Number),
        10 => (r(Read::PoliceDispatch), Format::Number),
        11 => (r(Read::PoliceCost), Format::Money),
        12 => (
            f64::from(load_ratio((r(Read::JailedFraction) * cap) as u32, cap as u32)),
            Format::Meter,
        ),
        13 => ((0.8 * cap).trunc(), Format::Number),
        14 => ((r(Read::JailedFraction) * cap).trunc(), Format::Number),
        15 => (r(Read::JailCost), Format::Money),
        17 => (
            f64::from(load_ratio(
                r(Read::AirportUsage) as u32,
                r(Read::AirportCapacity) as u32,
            )),
            Format::Meter,
        ),
        19 => (r(Read::AirportCost), Format::Money),
        21 => (
            f64::from(load_ratio(
                r(Read::SeaportUsage) as u32,
                r(Read::SeaportCapacity) as u32,
            )),
            Format::Meter,
        ),
        22 => (r(Read::PollutionWater), Format::Number),
        23 => (r(Read::SchoolGrade), Format::Meter),
        26 | 35 => (r(Read::EstimatedStaff), Format::Number),
        27 | 29 | 31 | 36 => (r(Read::EstimatedCapacity), Format::Number),
        28 | 30 => (r(Read::SchoolServed), Format::Number),
        32 => (r(Read::HospitalGrade), Format::Meter),
        37 => (r(Read::HospitalServed), Format::Number),
        38 => (
            f64::from(load_ratio(
                r(Read::LandfillUsage) as u32,
                r(Read::LandfillCapacity) as u32,
            )),
            Format::Meter,
        ),
        39 => (
            (r(Read::GarbageLandfilled) * r(Read::GarbageScaling)).trunc(),
            Format::Number,
        ),
        40 => (r(Read::LandfillBudget), Format::Money),
        41 => ((100.0 * r(Read::GarbageEfficiency)).trunc(), Format::Meter),
        42 => (
            percentage(r(Read::IncineratorUsed), r(Read::IncineratorCurrent)),
            Format::Percent,
        ),
        43 => (
            (r(Read::IncineratorUsed) * r(Read::GarbageScaling)).trunc(),
            Format::Number,
        ),
        44 => (r(Read::PollutionAir), Format::Number),
        45 | 52 => (r(Read::PowerEfficiency), Format::Meter),
        46 | 53 => (percentage(r(Read::PowerUsed), r(Read::PowerProduced)), Format::Percent),
        47 | 54 => (r(Read::PowerMaximum), Format::Number),
        48 | 55 => (r(Read::PowerProduced), Format::Number),
        49 | 56 => (r(Read::PowerUsed), Format::Number),
        50 | 57 => (per_unit(r(Read::PowerCost), r(Read::PowerProduced)), Format::Decimal),
        51 => (
            (r(Read::WasteToEnergyUsed) * r(Read::GarbageScaling)).trunc(),
            Format::Number,
        ),
        58 | 109 => (r(Read::WaterEfficiency), Format::Meter),
        59 => (percentage(r(Read::WaterUsed), r(Read::WaterCurrent)), Format::Percent),
        60 => (r(Read::WaterMaximum), Format::Number),
        61 => (r(Read::WaterCurrent), Format::Number),
        62 => (r(Read::WaterUsed), Format::Number),
        63 => (per_unit(r(Read::WaterCost), r(Read::WaterCurrent)), Format::Decimal),
        65 => (
            if r(Read::StationPresent) == 0.0 || r(Read::StationAvailable) == 0.0 {
                0.0
            } else if r(Read::StationUsage) == 0.0 {
                1.0
            } else {
                (r(Read::StationAvailable) / r(Read::StationUsage)).min(1.0)
            },
            Format::Meter,
        ),
        66 => (
            if r(Read::StationPresent) == 0.0 {
                0.0
            } else {
                percentage(r(Read::StationUsage), r(Read::StationMaximum))
            },
            Format::Percent,
        ),
        67 => (r(Read::StationCost), Format::Money),
        68 => (r(Read::StationUsage), Format::Number),
        69 => (r(Read::StationMaximum), Format::Number),
        70 => (100.0 * r(Read::TransitFunding), Format::Percent),
        71 => (r(Read::TransitLocalCost), Format::Money),
        73 => (
            r(Read::TransitLocalCost) * r(Read::TransitDepartmentFunding),
            Format::Money,
        ),
        94 => (
            if r(Read::RecyclingCurrent) <= 0.0 {
                0.0
            } else {
                (r(Read::RecyclingMaximum) / r(Read::RecyclingCurrent)).min(1.0)
            },
            Format::Meter,
        ),
        95 => (
            if r(Read::RecyclingCurrent) <= 0.0 {
                0.0
            } else {
                100.0
                    * (per_unit(r(Read::GarbageRecycled), r(Read::GarbageProduced)) / r(Read::RecyclingCurrent))
                        .min(1.0)
            },
            Format::Percent,
        ),
        96 => (
            if r(Read::RecyclingCenters) == 0.0 {
                0.0
            } else {
                (r(Read::GarbageRecycled) * r(Read::GarbageScaling)).trunc() / r(Read::RecyclingCenters)
            },
            Format::Number,
        ),
        97 | 98 => (100.0 * r(Read::PowerCondition), Format::Percent),
        115 => (100.0 * r(Read::WaterCondition), Format::Percent),
        120 => (r(Read::SeaportCost), Format::Money),
        123 => (r(Read::AirportUsage), Format::Number),
        124 => (r(Read::AirportCapacity), Format::Number),
        125 => (r(Read::SeaportUsage), Format::Number),
        126 => (r(Read::SeaportCapacity), Format::Number),
        _ => return fill_special(host, w, i, s),
    };
    let effect = match format {
        Format::Meter => Effect::Meter(w, value as f32),
        Format::Number => Effect::Text(w, format!("{}", value as i32)),
        Format::Percent => Effect::Text(w, format!("{}%", value as i32)),
        Format::Money => {
            let source = match i {
                4 => Read::FireCost,
                11 => Read::PoliceCost,
                15 => Read::JailCost,
                19 => Read::AirportCost,
                40 => Read::LandfillBudget,
                67 => Read::StationCost,
                71 | 73 => Read::TransitLocalCost,
                120 => Read::SeaportCost,
                _ => unreachable!(),
            };
            let cost = host.money_value(source, s);
            let cost = if i == 73 {
                (cost as f64 * host.read(Read::TransitDepartmentFunding, s)) as i64
            } else {
                cost
            };
            Effect::Text(w, host.money(cost))
        }
        Format::Decimal => Effect::Text(w, host.decimal(value)),
    };
    host.effect(effect);
    true
}
enum Format {
    Meter,
    Number,
    Percent,
    Money,
    Decimal,
}

fn fill_special(host: &mut impl Host, w: Token, i: u32, s: Subject) -> bool {
    let o = match s {
        Subject::Occupant(id) => Some((id, host.occupant(id))),
        _ => None,
    };
    let lot = subject_lot(host, s).map(|id| host.lot(id));
    let text = match i {
        0 => o.as_ref().map(|(_, o)| o.name.clone()).unwrap_or_default(),
        16 | 108 => {
            let purposes = o.as_ref().and_then(|(_, o)| o.properties.get(&0xEA54_D285));
            let cost: i64 = match purposes {
                Some(Property::Words(v)) => v.iter().map(|p| host.money_value(Read::BudgetCost(*p), s)).sum(),
                _ => 0,
            };
            host.money(cost)
        }
        107 => {
            let curve = o.as_ref().and_then(|(_, o)| o.properties.get(&0xAA60_E745));
            if let Some(Property::Curve(points)) = curve {
                let Some(v) = goofy_stat(points, host.read(Read::MayorRating, s)) else {
                    host.effect(Effect::Unsettled(Open::BuildingValuesCt19));
                    return true;
                };
                v.to_string()
            } else if curve.is_none() {
                "256".into()
            } else {
                host.effect(Effect::Unsettled(Open::BuildingValuesCt19));
                return true;
            }
        }
        64 => loc(
            host,
            if host.read(Read::WaterPolluted, s) != 0.0 {
                0x0A5D_4F43
            } else {
                0x2A5D_4F3B
            },
        ),
        74 => match o.as_ref().and_then(|(_, o)| o.properties.get(&0xAA5C_23FA)) {
            Some(Property::Key(g, i)) => host.localize(*g, *i),
            _ => String::new(),
        },
        81 | 82 | 92 => {
            let Some((id, _)) = o else {
                return false;
            };
            let cap = occupant_capacity(host, id);
            let used = current_occupancy(host, id);
            if i == 81 {
                format!("{used}/{cap}")
            } else if i == 82 {
                if cap == 0 {
                    return true;
                }
                cap.to_string()
            } else {
                (percentage(f64::from(used), f64::from(cap)) as i32).to_string()
            }
        }
        83 => {
            let wealth = lot.as_ref().and_then(|l| {
                if l.building_type == 0 {
                    Some(l.wealth)
                } else {
                    host.building_development(l.building_type).map(|r| r.wealth)
                }
            });
            if lot.as_ref().is_some_and(|l| l.building_type != 0) && wealth.is_none() {
                // CT19: missing development records are outside the settled domain.
                host.effect(Effect::Unsettled(Open::BuildingValuesCt19));
                return true;
            }
            let wealth = wealth.unwrap_or(0);
            let value = if wealth == 0 {
                1.0
            } else {
                (f64::from(lot.as_ref().unwrap().wealth) / f64::from(wealth)).min(1.0)
            };
            host.effect(Effect::Meter(w, value as f32));
            if wealth != 0 {
                if let Some(window) = host.meter_window(w) {
                    host.effect(Effect::MeterGeometry(w, wealth_geometry(window, wealth)));
                }
            }
            return true;
        }
        84 | 117 | 89 | 90 | 85 | 116 => {
            let rating = match i {
                84 => 0,
                117 => 1,
                89 => 2,
                90 => 3,
                85 => 4,
                _ => 5,
            };
            if let Some(value) = lot.as_ref().and_then(|l| developer_rating(host, l, rating)) {
                loc(host, value.label())
            } else {
                // CT19: missing records/curves do not acquire invented ratings.
                host.effect(Effect::Unsettled(Open::BuildingValuesCt19));
                return true;
            }
        }
        86 => loc(
            host,
            match host.read(Read::LandValue, s) as u32 {
                2 => 0x0A5F_E260,
                3 => 0xAA5F_E249,
                _ => 0x4A5F_E266,
            },
        ),
        87 => {
            if lot.as_ref().is_some_and(|l| l.state == 4 && l.abandon_reason == 4) {
                loc(host, 0x0A71_FAD3)
            } else {
                if let Some(value) = lot.as_ref().and_then(|l| developer_rating(host, l, 6)) {
                    // §8.12.4's commute-specific labels, in rating-index order.
                    loc(host, [0xAA71_FAE3, 0x6A71_FADC, 0x0A71_FAD3][usize::from(value.index)])
                } else {
                    host.effect(Effect::Unsettled(Open::BuildingValuesCt19));
                    return true;
                }
            }
        }
        88 => o
            .as_ref()
            .map(|(id, _)| loc(host, rating_level(mayor_percent(host, *id))))
            .unwrap_or_default(),
        91 => {
            host.effect(Effect::Unsettled(Open::MySimCt14));
            return true;
        }
        118 | 119 => loc(
            host,
            if host.read(if i == 118 { Read::Powered } else { Read::Watered }, s) != 0.0 {
                0xCA5D_4F33
            } else {
                0x6A5D_4F37
            },
        ),
        121 => {
            let id = lot
                .as_ref()
                .filter(|l| l.state == 4)
                .map(|l| match l.abandon_reason {
                    1 => 0xCC06_A5DE,
                    2 => 0xCC06_A5DF,
                    3 => 0xCC06_A5E0,
                    4 => 0xCC06_A5E1,
                    6 => 0xCC06_A5E2,
                    _ => 0,
                })
                .unwrap_or(0);
            if id == 0 {
                String::new()
            } else {
                loc(host, id)
            }
        }
        122 => {
            host.effect(Effect::Toggle(w, lot.as_ref().is_some_and(|l| l.historical)));
            return true;
        }
        _ => return false,
    };
    host.effect(Effect::Text(w, text));
    true
}

pub fn walk(host: &mut impl Host, root: Token, s: Subject, get: bool) {
    for w in host.children(root) {
        if !fill_control(host, w, host.control_id(w), get, s) {
            walk(host, w, s, get);
        }
    }
}
fn all_controls(host: &mut impl Host, root: Token, prepare: bool) {
    for w in host.children(root) {
        host.effect(if prepare {
            Effect::PrepareControl(w)
        } else {
            Effect::ClearFilter(w)
        });
        all_controls(host, w, prepare);
    }
}
pub fn run_dialog(host: &mut impl Host, script: Script, s: Subject, coverage: bool) -> bool {
    let Some(w) = host.create_dialog(script) else {
        return false;
    };
    let saved = host.categories();
    let categories = match s {
        Subject::Occupant(o) if coverage => host.occupant_categories(o),
        _ => 0,
    };
    if categories != 0 {
        host.effect(Effect::Categories(categories));
        host.effect(Effect::RecalcCoverage);
        host.effect(Effect::CoverageTimer(true, 330));
    }
    let proc = host.supports_proc(w);
    if proc {
        if let Subject::Occupant(o) = s {
            host.effect(Effect::Acquire(o));
        }
        host.effect(Effect::AttachProc(w, s));
    }
    all_controls(host, w, true);
    if let Subject::Occupant(o) = s {
        host.effect(Effect::Acquire(o));
    }
    host.effect(Effect::Acquire(w));
    host.effect(Effect::PublishDialog(Some((w, s))));
    walk(host, w, s, true);
    let result = host.modal(w, s);
    host.effect(Effect::PublishDialog(None));
    host.effect(Effect::Release(w));
    if let Subject::Occupant(o) = s {
        host.effect(Effect::Release(o));
    }
    if result == 0 {
        walk(host, w, s, false);
    }
    all_controls(host, w, false);
    if proc {
        host.effect(Effect::DetachProc(w));
        if let Subject::Occupant(o) = s {
            host.effect(Effect::Release(o));
        }
    }
    host.effect(Effect::DestroyWindow(w));
    host.effect(Effect::Release(w));
    host.effect(Effect::FocusView);
    if categories != 0 {
        host.effect(Effect::Categories(saved));
        host.effect(Effect::RecalcCoverage);
        host.effect(Effect::CoverageTimer(false, 330));
    }
    result == 0
}

pub struct Notification {
    pub message_type: u32,
    pub from_dialog: bool,
    pub code: u32,
    pub value: u32,
    pub slider: Option<Token>,
}
pub fn dialog_notification(host: &mut impl Host, dialog: Token, s: Subject, n: Notification) -> bool {
    if n.message_type != 3 || !n.from_dialog {
        return false;
    }
    if n.code == 0x2872_59F6 && n.value < 2 {
        host.effect(Effect::EndDialog(dialog, n.value));
        return true;
    }
    if n.code != 0x8871_13A3 {
        return false;
    }
    let Some(w) = n.slider else {
        return false;
    };
    let (entry, cost, related) = host.slider_properties(w);
    let Value::Number(v) = host.control_value(w) else {
        return false;
    };
    let id = host.control_id(w);
    let text = host.money((v * 0.01 * cost as f64) as i64);
    host.effect(Effect::MoneyChild(w, id + 0x100, text));
    if matches!(s, Subject::Occupant(_)) {
        fill_control(host, w, entry, false, s);
        for id in related {
            if let Some(child) = host.child_by_id(dialog, id) {
                fill_control(host, child, id, true, s);
            }
        }
    }
    true
}

impl QueryTool {
    pub fn do_neighbor_dialog(&self, host: &mut impl Host, id: Token) -> Result<bool, Open> {
        let Some(plan) = neighbor_plan(host, &host.occupant(id)) else {
            return Ok(false);
        };
        // Resource selection is complete; CT14 still blocks row filling and
        // therefore opening a faithful dialog. Keep the plan, not a blank dialog.
        host.effect(Effect::NeighborPlan(plan));
        host.effect(Effect::Unsettled(Open::NeighborDialogCt14));
        Err(Open::NeighborDialogCt14)
    }
    pub fn do_query_dialog(&self, host: &mut impl Host, o: Token) -> bool {
        let Some(script) = host.occupant(o).u32_property(0x2A49_9F85) else {
            return false;
        };
        run_dialog(host, Script::query(script), Subject::Occupant(o), true)
    }
    pub fn do_zone_dialog(&self, host: &mut impl Host, zone: u32, c: Cell) -> bool {
        let script = match zone {
            11 => 0x6A56_1B3A,
            12 => 0x4A56_2DA5,
            14 => {
                host.effect(Effect::Sound(0x2A62_C927, None));
                0xAA56_1F93
            }
            _ => return false,
        };
        run_dialog(host, Script::query(script), Subject::Cell(c), false)
    }
    pub fn play_occupant_sound(&self, host: &mut impl Host, id: Token) {
        let Some(l) = host.lot_for_occupant(id) else {
            return;
        };
        let o = host.occupant(id);
        let property = match host.lot(l).condition {
            4 => Some(0x0A90_2434),
            2 | 3 => Some(0xAA90_5AB9),
            _ => None,
        };
        let sound = property
            .and_then(|p| o.u32_property(p))
            .or_else(|| o.u32_property(0xAA1D_D397));
        if let Some(sound) = sound {
            host.effect(Effect::Sound(sound, Some(id)));
        }
    }
    pub fn describe_pick(&self, host: &mut impl Host, pixel: Pixel, debug: bool) -> Readout {
        let mut out = Readout::default();
        if !host.has_view() {
            return out;
        }
        let mut picked = Self::pick(host, pixel, Filter::Describe);
        if picked.is_none() {
            if let Some(point) = host.pick_terrain(pixel) {
                let c = host.position_to_cell(point);
                picked = host.lot_at(c).and_then(|l| host.lot(l).building);
            }
        }
        if let Some(id) = picked {
            if let Some(l) = host.lot_for_occupant(id) {
                if let Some(b) = host.lot(l).building {
                    picked = Some(b);
                }
            }
        }
        let mut shown = false;
        let mut zone_shown = false;
        let mut cell = None;
        if let Some(id) = picked {
            let o = host.occupant(id);
            self.play_occupant_sound(host, id);
            if o.network_interface {
                if let Err(open) = self.do_neighbor_dialog(host, id) {
                    out.open.push(open);
                }
            } else if o.pole_interface {
                if let Err(open) = self.do_neighbor_dialog(host, id) {
                    out.open.push(open);
                }
                shown = true;
            }
            if !shown {
                if o.u32_property(0x2A49_9F85).is_some() {
                    self.do_query_dialog(host, id);
                    shown = true;
                } else {
                    let c = host.position_to_cell(o.position);
                    let zone = host.zone(c);
                    if matches!(zone, 11 | 12 | 14) {
                        self.do_zone_dialog(host, zone, c);
                        shown = true;
                        zone_shown = true;
                    } else {
                        host.effect(Effect::Sound(0xAA62_C9D5, None));
                    }
                }
            }
            if matches!(o.kind, Kind::Building | Kind::Network) {
                cell = Some(host.position_to_cell(o.position));
                if o.kind == Kind::Building {
                    host.effect(Effect::DebugPhase(DebugPhase::ResidentialQueryData(id)));
                }
                // Detailed network strings require unspecified geometry/readout
                // writer data (CT14/20); never synthesize a native description.
                if o.kind == Kind::Network {
                    let c = cell.unwrap();
                    if let Some(p) = o.network_piece {
                        out.text = format!("Query cell ({}, {}): Id {:08x}, flip: {}, rotation: {}, model Y Height {:.6}, floor heights {:.6} {:.6} {:.6} {:.6}\n",
                            c.x, c.z, p.id, if p.flip { "yes" } else { "no" }, p.rotation, p.model_y, p.floors[0],p.floors[1],p.floors[2],p.floors[3]);
                    } else if let Some(p) = o.network_tile {
                        let s = Subject::Cell(c);
                        let mut cap = host.read(Read::TrafficCapacity1, s);
                        if cap == 0.0 {
                            cap = host.read(Read::TrafficCapacity2, s);
                        }
                        let vol = 10 * (host.read(Read::TrafficVolumeByte, s) as u32 & 0xFF);
                        out.text = format!("Query cell ({}, {}): Piece Id: {:08x}, Rotation:{}, Flipped:{} \n, traffic cap({:.0}) vol({}) cong({:.6})",c.x,c.z,p.id,p.rotation,p.flipped,cap,vol,f64::from(vol)/cap);
                    }
                    out.open.push(Open::DebugWritersCt14);
                }
            }
            if zone_shown {
                return out;
            }
        }
        if cell.is_none() {
            cell = host.pick_terrain(pixel).map(|p| host.position_to_cell(p));
        }
        let Some(c) = cell else {
            return out;
        };
        if !shown {
            if matches!(host.zone(c), 11 | 12 | 14) {
                self.do_zone_dialog(host, host.zone(c), c);
            } else if picked.is_none() {
                // CT18: conformance test 10 explicitly requires this sound on
                // an empty non-zone cell, absent from DescribePick's step list.
                host.effect(Effect::Sound(0xAA62_C9D5, None));
            }
        }
        let lot = host.lot_at(c).map(|l| host.lot(l));
        let cell_building = host.building_at(c);
        if let Some(building) = cell_building {
            for phase in [
                DebugPhase::Building(building),
                DebugPhase::Population(c),
                DebugPhase::Demand(c),
            ] {
                host.effect(Effect::DebugPhase(phase));
            }
        } else {
            host.effect(Effect::DebugPhase(DebugPhase::Demand(c)));
        }
        host.effect(Effect::DebugPhase(DebugPhase::EqHq(c)));
        host.effect(Effect::DebugPhase(DebugPhase::Cell(c)));
        let name = lot
            .as_ref()
            .map(|l| format!("{}_{:08x}", l.config_name, l.config_id))
            .unwrap_or_else(|| "no lot template".into());
        let building_name = cell_building
            .map(|b| host.occupant(b).name)
            .unwrap_or_else(|| "empty".into());
        out.extra2 = format!("[Building {building_name}] [Lot: {name}]\n");
        out.open.push(Open::DebugWritersCt14);
        host.effect(Effect::Unsettled(Open::DebugWritersCt14));
        if debug {
            host.effect(Effect::Unsettled(Open::TrafficInfoCt20));
        }
        out
    }
}
