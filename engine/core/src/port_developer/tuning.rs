// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The port tuning (`docs/specs/port-developers.md` §6): the tuning
//! exemplar's list of port type exemplars and each type's growth rules.

use crate::property::PropertyHolder;
use crate::variant::{mem, Elem, Variant};

/// The type and group of the tuning and port type exemplars (§6).
pub const EXEMPLAR_TYPE: u32 = 0x6534_284A;
pub const EXEMPLAR_GROUP: u32 = 0x67CD_5FA1;

/// Tuning exemplar: the port type exemplar instances (u32 list).
pub const PROP_TYPE_LIST: u32 = 0x8A27_0FC3;
/// Port type properties (§6).
pub const PROP_STAGE_COUNT: u32 = 0xCA27_1780;
pub const PROP_LOT_CONFIGS: u32 = 0xCA27_1781;
pub const PROP_MONTHLY_COST: u32 = 0xCA27_1783;
pub const PROP_CAPACITY: u32 = 0xCA27_1784;
pub const PROP_THRESHOLD: u32 = 0xCA27_1785;
pub const PROP_UPGRADE_COST: u32 = 0xCA27_1786;
pub const PROP_SIZE_CLASS: u32 = 0xCA27_1787;
pub const PROP_GRID_ROWS: u32 = 0xCA27_1788;
pub const PROP_GRID_COLUMNS: u32 = 0xCA27_1789;
pub const PROP_GRID: u32 = 0xCA27_1800;

/// The default size class (§2).
pub const DEFAULT_SIZE_CLASS: u8 = 2;

/// One stage's growth rule (§2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GrowthRule {
    /// The stage's lot configuration ID.
    pub lot_config: u32,
    pub monthly_cost: i64,
    /// Trips.
    pub capacity: u32,
    /// The upgrade cost into the stage.
    pub upgrade_cost: i64,
}

/// A terrain grid (§4.8): `rows × columns` altitudes, row-major.
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainGrid {
    pub rows: u32,
    pub columns: u32,
    pub values: Vec<f32>,
}

/// A port type (`cPortTypeConfig`, §2).
#[derive(Clone, Debug, PartialEq)]
pub struct PortType {
    /// The exemplar instance it was loaded from.
    pub instance: u32,
    pub stage_count: u32,
    /// The efficiency threshold (property ×0.01).
    pub threshold: f32,
    pub size_class: u8,
    /// One rule per stage (`stage_count` entries).
    pub rules: Vec<GrowthRule>,
    /// Kept only when it holds exactly `rows × columns` values (§4.8).
    pub grid: Option<TerrainGrid>,
}

fn ints(h: &PropertyHolder, id: u32) -> Option<Vec<i64>> {
    h.get(id).map(|v| v.items().iter().map(Elem::as_i64).collect())
}

fn first_int(h: &PropertyHolder, id: u32) -> Option<i64> {
    h.get(id).and_then(|v| v.items().first().map(Elem::as_i64))
}

impl PortType {
    /// Reads a port type exemplar (§6). `None` without the stage count
    /// (required) [choice].
    pub fn from_exemplar(instance: u32, h: &PropertyHolder) -> Option<PortType> {
        let stage_count = first_int(h, PROP_STAGE_COUNT)? as u32;
        // The four per-stage lists are read only if all four exist; missing
        // entries read as 0 [choice].
        let lists = (
            ints(h, PROP_LOT_CONFIGS),
            ints(h, PROP_MONTHLY_COST),
            ints(h, PROP_CAPACITY),
            ints(h, PROP_UPGRADE_COST),
        );
        let at = |v: &[i64], i: usize| v.get(i).copied().unwrap_or(0);
        let rules = match lists {
            (Some(cfg), Some(cost), Some(cap), Some(up)) => (0..stage_count as usize)
                .map(|i| GrowthRule {
                    lot_config: at(&cfg, i) as u32,
                    monthly_cost: at(&cost, i),
                    capacity: at(&cap, i) as u32,
                    upgrade_cost: at(&up, i),
                })
                .collect(),
            _ => vec![GrowthRule::default(); stage_count as usize],
        };
        let threshold = h
            .get(PROP_THRESHOLD)
            .and_then(|v| v.items().first().map(|e| e.as_f64() as f32))
            .map_or(0.0, |t| t * 0.01f32);
        let size_class = first_int(h, PROP_SIZE_CLASS).map_or(DEFAULT_SIZE_CLASS, |v| v as u8);
        let rows = first_int(h, PROP_GRID_ROWS).unwrap_or(0) as u32;
        let columns = first_int(h, PROP_GRID_COLUMNS).unwrap_or(0) as u32;
        let grid = h.get(PROP_GRID).and_then(|v| {
            let values: Vec<f32> = v.items().iter().map(|e| e.as_f64() as f32).collect();
            (rows > 0 && columns > 0 && values.len() == (rows * columns) as usize).then_some(TerrainGrid {
                rows,
                columns,
                values,
            })
        });
        Some(PortType {
            instance,
            stage_count,
            threshold,
            size_class,
            rules,
            grid,
        })
    }
}

/// The shipped tuning exemplar instances (§2).
pub const AIRPORT_TUNING: u32 = 0xCA27_0E9D;
pub const SEAPORT_TUNING: u32 = 0xEA29_AD92;

fn u32s(v: &[u32]) -> Variant {
    Variant::array(mem::UINT32, v.iter().map(|&x| Elem::U32(x)).collect())
}

fn i64s(v: &[i64]) -> Variant {
    Variant::array(mem::SINT64, v.iter().map(|&x| Elem::I64(x)).collect())
}

/// One shipped type row: instance, configurations, costs, capacities,
/// upgrade costs, size class, grid rows and columns, grid value count.
type Row = (u32, [u32; 3], [i64; 3], [u32; 3], [i64; 3], u8, u8, u8, usize);

fn type_exemplar(r: &Row) -> (u32, PropertyHolder) {
    let mut h = PropertyHolder::new();
    h.add(PROP_STAGE_COUNT, Variant::Scalar(Elem::U32(3)));
    h.add(PROP_LOT_CONFIGS, u32s(&r.1));
    h.add(PROP_MONTHLY_COST, i64s(&r.2));
    h.add(PROP_CAPACITY, u32s(&r.3));
    h.add(PROP_UPGRADE_COST, i64s(&r.4));
    h.add(PROP_THRESHOLD, Variant::Scalar(Elem::U32(25)));
    h.add(PROP_SIZE_CLASS, Variant::Scalar(Elem::U8(r.5)));
    h.add(PROP_GRID_ROWS, Variant::Scalar(Elem::U8(r.6)));
    h.add(PROP_GRID_COLUMNS, Variant::Scalar(Elem::U8(r.7)));
    if r.8 > 0 {
        h.add(
            PROP_GRID,
            Variant::array(mem::FLOAT32, (0..r.8).map(|_| Elem::F32(0.0)).collect()),
        );
    }
    (r.0, h)
}

/// The shipped exemplars of §6 (tuning first, then the types), as property
/// maps keyed by instance. The seaport's grid values are zeros (only their
/// count, 72, matters: the grid is rejected).
pub fn shipped_exemplars(airport: bool) -> Vec<(u32, PropertyHolder)> {
    let rows: Vec<Row> = if airport {
        vec![
            (
                0x0A27_1653,
                [0xCA24_36C0, 0xCA24_36C1, 0xCA24_36C2],
                [700, 800, 1000],
                [10_000, 17_500, 30_600],
                [0, 5700, 9700],
                1,
                5,
                15,
                0,
            ),
            (
                0x0A27_1647,
                [0x6A51_26D0, 0x6A51_26D1, 0x6A51_26D2],
                [1200, 1400, 1700],
                [53_600, 93_800, 164_200],
                [0, 16_500, 28_100],
                2,
                6,
                18,
                0,
            ),
            (
                0x2A27_163F,
                [0x0A51_2810, 0x0A51_2811, 0x0A51_2812],
                [2000, 2400, 2900],
                [287_400, 503_000, 880_300],
                [0, 47_800, 81_300],
                3,
                11,
                22,
                0,
            ),
        ]
    } else {
        vec![(
            0xCA2D_8CE6,
            [0x4A52_917D, 0x4A63_F3C1, 0x8A63_F610],
            [500, 600, 700],
            [650, 900, 1200],
            [0, 6000, 6000],
            1,
            11,
            5,
            72,
        )]
    };
    let mut tuning = PropertyHolder::new();
    tuning.add(PROP_TYPE_LIST, u32s(&rows.iter().map(|r| r.0).collect::<Vec<_>>()));
    if airport {
        // Not read by any code (§6).
        tuning.add(0xAA60_E745, u32s(&[0, 100, 100, 0]));
    }
    let mut out = vec![(if airport { AIRPORT_TUNING } else { SEAPORT_TUNING }, tuning)];
    out.extend(rows.iter().map(type_exemplar));
    out
}
