// SPDX-License-Identifier: GPL-3.0-or-later
//! Stateless, language-neutral vectors for the prescribed overlay-view scenarios.
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{check_expect, jf, ji, R};
use fc4_core::overlay_views::*;
use std::cell::Cell as Counter;

pub(crate) fn exec(_host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("ov_")?;
    Some(run(rest, op))
}

fn number(v: &Json) -> R<f32> {
    v.as_f64()
        .map(|n| n as f32)
        .filter(|n| n.is_finite())
        .ok_or_else(|| "expected finite number".into())
}
fn integer(v: &Json) -> R<i64> {
    v.as_i64().ok_or_else(|| "expected integer".into())
}
fn array(v: &Json) -> R<&[Json]> {
    v.as_array().ok_or_else(|| "expected array".into())
}
fn boolean(v: &Json) -> R<bool> {
    v.as_bool().ok_or_else(|| "expected boolean".into())
}
fn points(v: &Json) -> R<Vec<Vec3>> {
    array(v)?
        .iter()
        .map(|v| {
            let a = array(v)?;
            if a.len() != 3 {
                return Err("point needs three components".into());
            }
            Ok(Vec3::new(number(&a[0])?, number(&a[1])?, number(&a[2])?))
        })
        .collect()
}
fn core<T>(result: fc4_core::overlay_views::Result<T>) -> R<T> {
    result.map_err(|e| format!("{e:?}"))
}
fn jarray(values: impl IntoIterator<Item = Json>) -> Json {
    Json::Array(values.into_iter().collect())
}
fn jpoint(p: Vec3) -> Json {
    jarray([jf(p.x), jf(p.y), jf(p.z)])
}

fn run(name: &str, op: &Json) -> R<()> {
    let got = match name {
        "order" => {
            let mut view = PathView::default();
            for path in array(field(op, "paths")?)? {
                let path = array(path)?;
                if path.len() != 2 {
                    return Err("path needs id and priority".into());
                }
                let id = u32::try_from(integer(&path[0])?).map_err(|_| "id out of range")?;
                let priority = number(&path[1])?;
                view.new_path(id, priority, 0);
            }
            jarray(view.paths().iter().map(|p| ji(p.colour)))
        }
        "strip" => {
            let mut view = PathView::default();
            view.set_thickness(number(field(op, "thickness")?)?);
            view.new_path(u32::MAX, 0.0, 0);
            core(view.set_path_texture(arrow_key(0), number(field(op, "repeat")?)?, &mut Foreign))?;
            for p in points(field(op, "points")?)? {
                core(view.add_path_vertex(p))?;
            }
            let mut draws = draw_path(&view.paths()[0], view.thickness);
            let draw = draws.pop().ok_or("empty strip")?;
            jarray(
                draw.vertices
                    .into_iter()
                    .map(|v| jarray([jpoint(v.position), jarray(v.uv.unwrap().map(jf))])),
            )
        }
        "duplicate" => {
            let mut view = PathView::default();
            view.new_path(1, 0.0, 0);
            let mut accepted = Vec::new();
            for p in points(field(op, "points")?)? {
                let before = view.paths()[0].points.len();
                core(view.add_path_vertex(p))?;
                accepted.push(Json::Bool(view.paths()[0].points.len() != before));
            }
            jarray(accepted)
        }
        "cells" => {
            let count = usize::try_from(integer(field(op, "count")?)?).map_err(|_| "invalid count")?;
            if count > 4096 {
                return Err("fixture count exceeds terrain".into());
            }
            let view = MarkedCellView {
                cells: (0..count).map(|n| Cell((n % 64) as i32, (n / 64) as i32)).collect(),
                ..MarkedCellView::default()
            };
            let mut trace = Trace::default();
            core(view.draw_cells(None, &mut FlatTerrain, &mut Foreign, &mut trace))?;
            jarray(trace.0.iter().map(|d| ji(d.vertices.len() as u32)))
        }
        "vb_disabled" => {
            let mut driver = DisabledDriver(Counter::new(0));
            if core(VertexBufferExtension::default().get(false, 64, 64, &mut driver))? {
                return Err("disabled extension was made".into());
            }
            ji(driver.0.get())
        }
        "path_info" => {
            let key = u32::try_from(integer(field(op, "key")?)?).map_err(|_| "invalid key")?;
            let count = usize::try_from(integer(field(op, "count")?)?).map_err(|_| "invalid count")?;
            if count > 1000 {
                return Err("fixture count too large".into());
            }
            let path = NetworkPath {
                key,
                flags: 0,
                has_heights: true,
                points: (0..count).map(|n| Vec3::new(n as f32, 7.0, 3.0)).collect(),
            };
            let mut trace = Trace::default();
            core(draw_network_path(
                &path,
                0,
                0,
                &mut [None; 25],
                &mut Foreign,
                &mut trace,
            ))?;
            let strip = trace.0.first().ok_or("missing strip")?;
            let colour = strip.vertices.first().ok_or("missing first vertex")?.colour.unwrap();
            jarray([
                ji(strip.vertices.len() as u32),
                jarray([colour.r, colour.g, colour.b, colour.a.unwrap()].map(ji)),
            ])
        }
        "train_arrow" => {
            let e = array(field(op, "entry")?)?;
            if e.len() != 2 {
                return Err("entry needs two components".into());
            }
            ji(core(classify_path_bitmap(
                &points(field(op, "points")?)?,
                Vec3::new(number(&e[0])?, 0.0, number(&e[1])?),
                boolean(field(op, "selected")?)?,
            ))?)
        }
        "train_alpha" => jf(route_transparency(boolean(field(op, "near")?)?, true)),
        _ => return Err(format!("unknown overlay op {name}")),
    };
    check_expect(op, got)
}

struct FlatTerrain;
impl CellTerrain for FlatTerrain {
    fn dimensions(&self) -> (u32, u32) {
        (64, 64)
    }
    fn terrain_height(&mut self, _x: i32, _z: i32) -> Option<f32> {
        Some(0.0)
    }
    fn split(&mut self, _: Cell) -> bool {
        false
    }
    fn water_height(&mut self, _: i32, _: i32) -> f32 {
        0.0
    }
    fn in_frustum(&mut self, _: Vec3) -> bool {
        true
    }
    fn lookup_lot(&mut self, _: Cell, _: Handle) -> Option<Lot> {
        None
    }
    fn lot_altitude(&mut self, _: Handle, _: f32, _: f32) -> f32 {
        0.0
    }
}
struct DisabledDriver(Counter<u32>);
impl VertexDriver for DisabledDriver {
    fn acquire(&mut self, _: u32) -> Option<DriverBuffer> {
        self.0.set(self.0.get() + 1);
        None
    }
    fn release_interface(&mut self, _: Handle) {
        unreachable!()
    }
    fn lock(&mut self, _: Handle, _: Handle, _: usize) -> Option<Handle> {
        unreachable!()
    }
    fn copy_vertices(&mut self, _: Handle, _: usize, _: &[TerrainVertex]) {
        unreachable!()
    }
    fn unlock(&mut self, _: Handle, _: Handle) {
        unreachable!()
    }
    fn draw_indexed(&mut self, _: Handle, _: Handle, _: usize, _: &[u16]) {
        unreachable!()
    }
}

#[derive(Default)]
struct Trace(Vec<Draw>);
impl Renderer for Trace {
    fn depth_offset(&self) -> i32 {
        0
    }
    fn emit(&mut self, command: Command) {
        if let Command::Draw(draw) = command {
            self.0.push(draw);
        }
    }
}
struct Foreign;
impl Resources for Foreign {
    fn render_service_available(&mut self) -> bool {
        false
    }
    fn resource_service_available(&mut self) -> bool {
        false
    }
    fn binding(&mut self, _: ResourceKey) -> Option<Handle> {
        None
    }
    fn model(&mut self, _: ResourceKey) -> Option<Handle> {
        unreachable!()
    }
    fn lot_filter(&mut self, _: u32) -> Option<Handle> {
        unreachable!()
    }
    fn retain(&mut self, _: Handle) {
        unreachable!()
    }
    fn release(&mut self, _: Handle) {
        unreachable!()
    }
    fn base_cleanup(&mut self, _: u32) {
        unreachable!()
    }
    fn free_object(&mut self, _: u32) {
        unreachable!()
    }
}
impl NetworkHost for Foreign {
    fn manager_available(&mut self) -> bool {
        unreachable!()
    }
    fn visible_cells(&mut self) -> (Cell, Cell) {
        unreachable!()
    }
    fn occupants(&mut self, _: Cell) -> Vec<Handle> {
        unreachable!()
    }
    fn paths(&mut self, _: Handle) -> Vec<NetworkPath> {
        unreachable!()
    }
    fn altitude(&mut self, _: Handle, _: f32, _: f32) -> f32 {
        0.0
    }
    fn routes(&mut self, _: Handle, _: u8) -> RouteSet {
        unreachable!()
    }
    fn path(&mut self, _: Handle, _: u32) -> Option<NetworkPath> {
        unreachable!()
    }
    fn route_origin(&mut self, _: Handle, _: u32) -> Option<Vec3> {
        unreachable!()
    }
    fn terrain_altitude(&mut self, _: f32, _: f32) -> f32 {
        unreachable!()
    }
}
