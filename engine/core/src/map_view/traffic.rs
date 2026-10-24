// SPDX-License-Identifier: GPL-3.0-or-later
//! QV2: raw traffic records -> fresh owned grids, marked cells and animation.
use super::*;
use std::sync::atomic::{AtomicU8, Ordering};

static PHASE: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Debug)]
pub struct FerryPath {
    pub normal: u16,
    pub alternate: u16,
    /// Raw compressed-reader points, in GetNext order, including terminal points.
    pub points: Vec<([i16; 2], u8)>,
}
#[derive(Clone, Debug)]
pub struct FerryRoute {
    pub paths: [FerryPath; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrafficGrids {
    pub size: [usize; 2],
    pub volume: Vec<u16>,
    pub x: Vec<i16>,
    pub z: Vec<i16>,
}

pub fn traffic_network(travel: u8) -> u8 {
    match travel {
        0 => 0,
        3 | 5 => 2,
        6 => 3,
        7 => 4,
        8 => 5,
        _ => 1,
    }
}

pub fn reconstruct_traffic(
    size: [usize; 2],
    view: u32,
    types: &[u8],
    scale: u32,
    alternate: bool,
    simulators: &mut impl Simulators,
) -> Result<TrafficGrids, OwnerError> {
    let n = size[0]
        .checked_mul(size[1])
        .filter(|n| *n > 0)
        .ok_or(Unsupported::GridGeometry)?;
    let mut grids = TrafficGrids {
        size,
        volume: vec![0; n],
        x: vec![0; n],
        z: vec![0; n],
    };
    if view == 0x54 {
        for route in simulators.ferry_routes()? {
            for path in route.paths {
                let count = if alternate { path.alternate } else { path.normal };
                for (cell, direction) in path.points {
                    if cell[0] < 0 || cell[1] < 0 || cell[0] as usize >= size[0] || cell[1] as usize >= size[1] {
                        return Err(Unsupported::GridGeometry.into());
                    }
                    let i = cell[0] as usize * size[1] + cell[1] as usize;
                    grids.volume[i] = grids.volume[i].wrapping_add(count);
                    let (target, sign) = match direction {
                        0 => (&mut grids.x[i], -1),
                        1 => (&mut grids.z[i], -1),
                        2 => (&mut grids.x[i], 1),
                        3 => (&mut grids.z[i], 1),
                        _ => continue,
                    };
                    *target = (i32::from(*target) + sign * i32::from(count)).clamp(-32768, 32767) as i16;
                }
            }
        }
    } else {
        let bias = (scale.wrapping_add(383) >> 8) as u16;
        let cap = (scale.wrapping_mul(255) >> 8) as u16;
        for x in 0..size[0] {
            for z in 0..size[1] {
                let (mut volume, mut bx, mut bz) = (0u32, 0i32, 0i32);
                for &travel in types {
                    if simulators.traffic_capacity(traffic_network(travel), [x, z])? <= 0 {
                        continue;
                    }
                    let h = simulators.traffic_density(travel, [x, z], alternate)?.map(i32::from);
                    if volume == 0 {
                        volume = u32::from(bias);
                    }
                    volume += h[16] as u32;
                    bx += 2 * (h[2] - h[8]) - h[4] - h[9] - h[12] - h[11] + h[3] + h[1] + h[6] + h[14];
                    bz += 2 * (h[7] - h[13]) - h[9] - h[12] - h[1] - h[14] + h[4] + h[3] + h[11] + h[6];
                }
                let i = x * size[1] + z;
                grids.volume[i] = volume.min(u32::from(cap)) as u16;
                grids.x[i] = bx.clamp(-32768, 32767) as i16;
                grids.z[i] = bz.clamp(-32768, 32767) as i16;
            }
        }
    }
    Ok(grids)
}

impl TrafficGrids {
    pub fn animate(&mut self, phase: u8, scale: u32) {
        for i in 0..self.volume.len() {
            let volume = self.volume[i];
            if volume <= 10 {
                continue;
            }
            let x = f64::from(self.x[i]) / f64::from(volume);
            let z = f64::from(self.z[i]) / f64::from(volume);
            // Integer magnitude comparison is exactly the unspilled ratio comparison.
            let (ratio, coordinate) = if i32::from(self.x[i]).abs() > i32::from(self.z[i]).abs() {
                (x, i / self.size[1])
            } else {
                (z, i % self.size[1])
            };
            let coordinate = if ratio > 0.9 {
                coordinate
            } else if ratio < -0.9 {
                !coordinate
            } else {
                continue;
            };
            if coordinate & 15 == usize::from(phase & 15) {
                self.volume[i] = scale as u16;
            }
        }
    }
}

impl MapView {
    pub(super) fn draw_traffic(
        &mut self,
        renderer: &mut impl Renderer,
        simulators: &mut impl Simulators,
    ) -> Result<OwnerUpdate, OwnerError> {
        let size = simulators.city_cells()?;
        let scale = self.config.as_ref().map_or(256, |c| c.scale);
        // Literal §4.1: use the committed current identity, including during selection's redraw.
        let view = self.current_view.unwrap_or(0);
        let mut grids = reconstruct_traffic(
            size,
            view,
            &self.travel_types,
            scale,
            self.two_way_toggle == Some(true),
            simulators,
        )?;
        let cells = [
            simulators.city_world_width()? / size[0] as f32,
            simulators.city_world_height()? / size[1] as f32,
        ];
        let mut background = vec![0; grids.volume.len()];
        let mut marks = MarkedCells {
            cell_size: cells,
            ..MarkedCells::default()
        };
        let mut icon = None;
        let mut attempted = false;
        let sprite_result = (|| {
            for x in 0..size[0] {
                for z in 0..size[1] {
                    let i = x * size[1] + z;
                    let damaged = simulators.damaged_road([x, z])?;
                    if damaged {
                        if !attempted {
                            icon = renderer.damaged_road_icon()?;
                            attempted = true;
                        }
                        if let Some(icon) = &icon {
                            blit_incident(
                                &mut background,
                                size,
                                icon,
                                incident_center([x as i32, z as i32], size[0], size[0]),
                                renderer.orientation().wrapping_neg() & 3,
                            )?;
                        }
                    }
                    if self.overlay_enabled && (damaged || grids.volume[i] != 0) {
                        let color = if damaged {
                            0
                        } else {
                            if scale == 0 {
                                return Err(Unsupported::FloatOutOfRange.into());
                            }
                            let index = ((u32::from(grids.volume[i]) * 255 / scale) as u16) as usize;
                            *self
                                .palettes
                                .as_ref()
                                .ok_or(Unsupported::RampGeneration)?
                                .straight
                                .0
                                .get(index)
                                .ok_or(Unsupported::FloatOutOfRange)?
                        };
                        marks.coordinates.push([x, z]);
                        marks.colors.push(color);
                    }
                }
            }
            Ok::<(), OwnerError>(())
        })();
        if let Some(icon) = icon {
            renderer.release_incident_icon(icon.handle)?;
        }
        sprite_result?;
        if self.overlay_enabled {
            renderer.overlay_replace(&marks)?;
        }
        let phase = PHASE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |p| Some(p.wrapping_add(1) & 15))
            .unwrap_or_else(|p| p)
            .wrapping_add(1)
            & 15;
        grids.animate(phase, scale);
        let palettes = self.palettes.as_mut().ok_or(Unsupported::RampGeneration)?;
        palettes.straight.0[255] = 0xcfff_ffff;
        palettes.premultiplied.0[255] = 0xffff_ffff;
        let grid = RawMapGrid {
            width: size[0],
            height: size[1],
            cell_size: cells,
            values: GridValues::U16(grids.volume),
        };
        let enabled = self.overlay_enabled;
        self.overlay_enabled = false;
        let result = self.draw_grid(
            grid,
            ValueType::U16,
            SourceSelection::TrafficVolumeQV2,
            false,
            Some(background),
            renderer,
            simulators,
        );
        self.overlay_enabled = enabled;
        result
    }
}
