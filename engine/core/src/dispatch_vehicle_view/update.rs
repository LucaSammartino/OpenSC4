// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::collections::BTreeMap;

const RADIANS: f32 = 0.017_453_292; // binary32 0x3C8EFA35; const-compatible with MSRV 1.75.
const CATEGORIES: [u8; 7] = [1, 1, 1, 1, 2, 2, 3];

#[derive(Clone, Debug, PartialEq)]
pub enum DrawOp {
    ModelTransform(Option<Transform>),
    ClearTexTransform(u32),
    Begin2D,
    End2D,
    Transparency(f32),
    ResetTransparency,
    State(Token),
    Texture(Option<Token>, u32),
    /// DrawPrims(6, 2, 4, vertices).
    Quad(Quad),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressNode {
    pub has_resource: bool,
    pub duration: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    /// Unsigned r.+60; None from progress() means r or t was null.
    pub numerator: u32,
    /// t followed by its parents, in that order.
    pub chain: Vec<ProgressNode>,
}

/// One host instance represents the SAME borrowed draw context for the entire
/// Update, including model drawing. No context is retained after the call.
pub trait FrameServices: Services + WindowServices {
    fn stop_and_sample_timer(&mut self) -> f32;
    fn start_timer(&mut self);
    fn context_blocked(&mut self) -> bool;
    fn render_allowed(&mut self) -> bool;
    fn simulator_state(&mut self) -> u8 {
        0
    }
    fn position(&mut self, occupant: Token) -> Vec3;
    /// None = no terrain. Some = slot 13 on-map, or slot 29 off-map.
    fn terrain_height(&mut self, position: Vec3) -> Option<f32>;
    fn project(&mut self, point: Vec3) -> Vec3;
    fn draw_op(&mut self, op: DrawOp);
    fn progress(&mut self, automaton: Token) -> Option<Progress>;
    fn model_transform(&mut self, model: Token) -> Transform;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloatRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}
impl FloatRect {
    fn from_rect(r: Rect) -> Self {
        Self {
            left: r.left as f32,
            top: r.top as f32,
            right: r.right as f32,
            bottom: r.bottom as f32,
        }
    }
    fn truncate(self) -> Rect {
        Rect::new(
            crate::x87::ftol32(f64::from(self.left)),
            crate::x87::ftol32(f64::from(self.top)),
            crate::x87::ftol32(f64::from(self.right)),
            crate::x87::ftol32(f64::from(self.bottom)),
        )
    }
}

/// Closed edges conflict; global obstacles match every candidate category.
pub fn conflict(candidate: Obstacle, obstacles: &[Obstacle], bounds: Rect) -> Option<Obstacle> {
    let r = candidate.rectangle;
    if r.left < bounds.left || r.top < bounds.top || r.right > bounds.right || r.bottom > bounds.bottom {
        return Some(Obstacle {
            rectangle: bounds,
            category: 0xff,
        });
    }
    obstacles.iter().copied().find(|o| {
        (o.category == 0xff || o.category == candidate.category)
            && r.left <= o.rectangle.right
            && r.right >= o.rectangle.left
            && r.top <= o.rectangle.bottom
            && r.bottom >= o.rectangle.top
    })
}

fn centre(bounds: Rect) -> [f32; 2] {
    [
        (f64::from(bounds.right.wrapping_sub(bounds.left)) * 0.5 + f64::from(bounds.left)) as f32,
        (f64::from(bounds.bottom.wrapping_sub(bounds.top)) * 0.5 + f64::from(bounds.top)) as f32,
    ]
}

/// Bounded native search, including its overlapping original fallback.
pub fn relocate(original: Obstacle, obstacles: &[Obstacle], bounds: Rect) -> Obstacle {
    let mut moving = [
        (f64::from(original.rectangle.left) + 21.0) as f32,
        (f64::from(original.rectangle.top) + 21.0) as f32,
    ];
    let target = centre(bounds);
    let d = [
        f64::from(target[0]) - f64::from(moving[0]),
        f64::from(target[1]) - f64::from(moving[1]),
    ];
    let length = (d[0] * d[0] + d[1] * d[1]).sqrt();
    let limit = (1.75 * length) as f32;
    let step = [(d[0] / length) as f32, (d[1] / length) as f32];
    let mut progress = 0.0_f32;
    let mut live_progress = 0.0_f64;
    let mut candidate = original;
    'search: while live_progress < f64::from(limit) {
        candidate.rectangle = Rect::new(
            crate::x87::ftol32(f64::from(moving[0]) - 21.0),
            crate::x87::ftol32(f64::from(moving[1]) - 21.0),
            crate::x87::ftol32(f64::from(moving[0]) + 21.0),
            crate::x87::ftol32(f64::from(moving[1]) + 21.0),
        );
        let Some(first) = conflict(candidate, obstacles, bounds) else {
            break;
        };
        if first.category < 0xff {
            for (dx, dy) in [(0, 43), (0, -43), (43, 0), (-43, 0)] {
                let r = first.rectangle;
                candidate.rectangle = Rect::new(
                    r.left.wrapping_add(dx),
                    r.top.wrapping_add(dy),
                    r.right.wrapping_add(dx),
                    r.bottom.wrapping_add(dy),
                );
                if conflict(candidate, obstacles, bounds).is_none() {
                    break 'search;
                }
            }
        }
        moving[0] = (f64::from(moving[0]) + 2.0 * f64::from(step[0])) as f32;
        moving[1] = (f64::from(moving[1]) + 2.0 * f64::from(step[1])) as f32;
        live_progress = f64::from(progress) + 2.0;
        progress = live_progress as f32;
    }
    if conflict(candidate, obstacles, bounds).is_some() {
        original
    } else {
        candidate
    }
}

/// A locked rectangle bypasses this function in Update.
pub fn clamp_rectangle(mut r: FloatRect, bounds: Rect) -> (FloatRect, bool) {
    let w = (f64::from(r.right) - f64::from(r.left)) as f32;
    let h = (f64::from(r.bottom) - f64::from(r.top)) as f32;
    let mut moved = false;
    if r.left < bounds.left as f32 {
        r.left = bounds.left as f32;
        r.right = (f64::from(r.left) + f64::from(w)) as f32;
        moved = true;
    } else if r.right > bounds.right as f32 {
        r.right = bounds.right as f32;
        r.left = (f64::from(r.right) - f64::from(w)) as f32;
        moved = true;
    }
    if r.top < bounds.top as f32 {
        r.top = bounds.top as f32;
        r.bottom = (f64::from(r.top) + f64::from(h)) as f32;
        moved = true;
    } else if r.bottom > bounds.bottom as f32 {
        r.bottom = bounds.bottom as f32;
        r.top = (f64::from(r.bottom) - f64::from(h)) as f32;
        moved = true;
    }
    (r, moved)
}

impl DispatchVehicleView {
    pub fn update(&mut self, host: &mut impl FrameServices) -> Result<bool, Unsupported> {
        let elapsed = host.stop_and_sample_timer();
        let delta = (f64::from(elapsed) * f64::from(0.001_f32) * 2.0) as f32;
        if host.context_blocked() || !host.render_allowed() {
            return Ok(true);
        }
        let simulator = host.simulator_state();
        self.rebuild_windows(host);
        let main = self.windows.first().ok_or(Unsupported::MissingMainWindow)?.rectangle;
        let bounds = Rect::new(
            main.left.wrapping_add(32),
            main.top.wrapping_add(32),
            main.right.wrapping_sub(32),
            main.bottom.wrapping_sub(32),
        );
        let mut obstacles: Vec<_> = self.windows.iter().skip(1).copied().collect();
        let mut per_kind = [0_u32; 7];
        let mut per_occupant = BTreeMap::<Token, u8>::new();
        let mut index = 0;
        while index < self.tracks.len() {
            let info = &mut self.tracks[index];
            if info.has(flags::FADES) {
                let value = if info.has(flags::REMOVING) {
                    let value = f64::from(info.fade) - f64::from(delta);
                    if value < 0.0 {
                        0.0
                    } else {
                        value
                    }
                } else {
                    let value = f64::from(info.fade) + f64::from(delta);
                    if value > 1.0 {
                        1.0
                    } else {
                        value
                    }
                };
                info.fade = value as f32;
                if info.has(flags::REMOVING) && value < f64::from(0.01_f32) {
                    self.complete_removal(index, host);
                    continue;
                }
            }
            if info.has(flags::FLASHING) {
                info.flash_accumulator = (f64::from(info.flash_accumulator) + f64::from(elapsed) * 0.001) as f32;
                if info.flash_accumulator > info.flash_period {
                    info.flash_accumulator = 0.0;
                }
                info.fade = (f64::from(info.flash_accumulator) / f64::from(info.flash_period)
                    * f64::from(std::f32::consts::FRAC_PI_2))
                .cos() as f32;
            }
            let kind = info.kind as usize;
            if !info.has(flags::VISIBLE)
                || !info.has(flags::TOP)
                || self.hidden.contains(&info.kind)
                || (info.kind == 4 && simulator != 0)
                || per_kind[kind] >= self.limits[kind]
            {
                index += 1;
                continue;
            }
            per_kind[kind] += 1;
            if !info.has(flags::REMOVING) {
                if let Some(automaton) = host.automaton(info.occupant) {
                    if host.automaton_flag(automaton, 1) {
                        info.flags |= flags::REMOVING;
                    }
                    host.resource_op(ResourceOp::Release(Reference::Automaton, automaton));
                }
            }
            if info.has(flags::MODEL) {
                update_model(info, elapsed, host)?;
                index += 1;
                continue;
            }
            let mut p = host.position(info.occupant);
            if let Some(h) = host.terrain_height(p) {
                if p[1].partial_cmp(&h) != Some(std::cmp::Ordering::Greater) {
                    p[1] = h;
                }
            }
            let transform = Transform {
                translation: p,
                ..Transform::default()
            };
            let rect = if info.has(flags::LOCKED) {
                FloatRect::from_rect(info.rectangle)
            } else if info.has(flags::BUILDING) {
                let n = per_occupant.entry(info.occupant).or_default();
                *n = n.wrapping_add(1);
                p = host.project(p);
                let theta = f64::from(72 * i32::from(*n) - 90) * f64::from(RADIANS);
                let left = (theta.cos() * 50.0 + f64::from(p[0]) - 21.0) as f32;
                let top = (theta.sin() * 50.0 + f64::from(p[1]) - 21.0) as f32;
                FloatRect {
                    left,
                    top,
                    right: (f64::from(left) + 42.0) as f32,
                    bottom: (f64::from(top) + 42.0) as f32,
                }
            } else {
                let translated =
                    |v: Vertex| std::array::from_fn(|i| (f64::from(v.position[i]) + f64::from(p[i])) as f32);
                let a = host.project(translated(info.icon_quad[0]));
                let b = host.project(translated(info.icon_quad[2]));
                FloatRect {
                    left: a[0],
                    top: a[1],
                    right: b[0],
                    bottom: b[1],
                }
            };
            let (rect, moved) = if info.has(flags::LOCKED) {
                (rect, false)
            } else {
                clamp_rectangle(rect, bounds)
            };
            if !moved && !info.has(flags::BUILDING) && info.kind != 4 && !info.has(flags::DISPLAY) {
                draw_world(info, transform, host, self.states.ok_or(Unsupported::OpenRenderStates)?);
                info.rectangle = rect.truncate();
                info.anchor = p;
            } else {
                let candidate = if info.has(flags::LOCKED) {
                    info.rectangle
                } else {
                    FloatRect {
                        left: rect.left,
                        top: rect.top,
                        right: (f64::from(rect.left) + 42.0) as f32,
                        bottom: (f64::from(rect.top) + 42.0) as f32,
                    }
                    .truncate()
                };
                let candidate = Obstacle {
                    rectangle: candidate,
                    category: CATEGORIES[kind],
                };
                let selected = if info.has(flags::LOCKED) {
                    candidate
                } else {
                    relocate(candidate, &obstacles, bounds)
                };
                obstacles.push(selected);
                info.rectangle = selected.rectangle;
                if !info.has(flags::BUILDING) {
                    p = host.project(p);
                }
                if !info.has(flags::LOCKED) {
                    let c = centre(info.rectangle);
                    info.anchor = [c[0], c[1], 0.0];
                }
                let icon_first = info.icon.map_or(0, |i| i.identity) != self.icons[2].map_or(0, |i| i.identity);
                draw_2d(
                    info,
                    p,
                    icon_first,
                    host,
                    self.states.ok_or(Unsupported::OpenRenderStates)?,
                );
            }
            index += 1;
        }
        host.start_timer();
        Ok(true)
    }
}

fn draw_world(info: &TrackInfo, transform: Transform, host: &mut impl FrameServices, states: [Token; 3]) {
    host.draw_op(DrawOp::ClearTexTransform(0));
    host.draw_op(DrawOp::ModelTransform(Some(transform)));
    host.draw_op(DrawOp::Transparency((f64::from(info.fade) * 0.5) as f32));
    host.draw_op(DrawOp::State(states[1]));
    host.draw_op(DrawOp::Texture(info.icon.and_then(|i| i.texture), 0));
    host.draw_op(DrawOp::Quad(info.icon_quad));
    host.draw_op(DrawOp::Transparency(info.fade));
    host.draw_op(DrawOp::State(states[0]));
    host.draw_op(DrawOp::Texture(info.icon.and_then(|i| i.texture), 0));
    host.draw_op(DrawOp::Quad(info.icon_quad));
    host.draw_op(DrawOp::ResetTransparency);
    host.draw_op(DrawOp::ModelTransform(None));
}

pub fn screen_quads(info: &TrackInfo, projected: Vec3) -> (Quad, Quad) {
    let dx = f64::from(projected[0]) - f64::from(info.anchor[0]);
    let dy = f64::from(projected[1]) - f64::from(info.anchor[1]);
    let k = 1.0 / (dx * dx + dy * dy).sqrt();
    let theta = (-f64::from((dx * k) as f32)).atan2(dy * k) as f32;
    let c = f64::from(f64::from(theta).cos() as f32);
    let s = f64::from(f64::from(theta).sin() as f32);
    let template = [
        ([-32.0, -32.0], [0.0, 0.0]),
        ([-32.0, 32.0], [0.0, 1.0]),
        ([32.0, 32.0], [1.0, 1.0]),
        ([32.0, -32.0], [1.0, 0.0]),
    ];
    let icon = template.map(|([x, y], uv)| Vertex {
        position: [
            (f64::from((y * -s + x * c) as f32) + f64::from(info.anchor[0])) as f32,
            (f64::from((y * c + x * s) as f32) + f64::from(info.anchor[1])) as f32,
            0.0,
        ],
        uv,
    });
    let hw = info.display_size[0] * 0.5;
    let hh = info.display_size[1] * 0.5;
    let number = std::array::from_fn(|i| {
        let x = if i < 2 { -hw } else { hw };
        let y = if i == 0 || i == 3 { -hh } else { hh };
        Vertex {
            position: [
                (f64::from(x) + f64::from(info.anchor[0])) as f32,
                (f64::from(y) + f64::from(info.anchor[1])) as f32,
                0.0,
            ],
            uv: info.display_quad[i].uv,
        }
    });
    (icon, number)
}

fn draw_2d(info: &TrackInfo, projected: Vec3, icon_first: bool, host: &mut impl FrameServices, states: [Token; 3]) {
    let (icon, number) = screen_quads(info, projected);
    host.draw_op(DrawOp::ModelTransform(None));
    host.draw_op(DrawOp::ClearTexTransform(0));
    host.draw_op(DrawOp::Begin2D);
    host.draw_op(DrawOp::Transparency(info.fade));
    host.draw_op(DrawOp::State(states[0]));
    let icon_call = |host: &mut _| {
        FrameServices::draw_op(host, DrawOp::Texture(info.icon.and_then(|i| i.texture), 0));
        FrameServices::draw_op(host, DrawOp::Quad(icon));
    };
    if icon_first {
        icon_call(host);
    }
    if info.has(flags::DISPLAY) {
        host.draw_op(DrawOp::Texture(info.texture, 0));
        host.draw_op(DrawOp::Quad(number));
    }
    if !icon_first {
        icon_call(host);
    }
    host.draw_op(DrawOp::ResetTransparency);
    host.draw_op(DrawOp::End2D);
}

fn update_model(info: &mut TrackInfo, elapsed: f32, host: &mut impl FrameServices) -> Result<(), Unsupported> {
    let Some(model) = info.model else {
        return Ok(());
    };
    if info.occupant == 0 {
        return Ok(());
    }
    let automaton = host.automaton(info.occupant);
    // Release the QI even at a safe unsupported numeric boundary.
    let result = update_model_acquired(info, model, automaton, elapsed, host);
    if let Some(automaton) = automaton {
        host.resource_op(ResourceOp::Release(Reference::Automaton, automaton));
    }
    result
}

fn update_model_acquired(
    info: &mut TrackInfo,
    model: Token,
    automaton: Option<Token>,
    elapsed: f32,
    host: &mut impl FrameServices,
) -> Result<(), Unsupported> {
    let mut q = 0.0;
    if let Some(automaton) = automaton {
        if let Some(progress) = host.progress(automaton) {
            if host.automaton_flag(automaton, 0x0800_0000) {
                q = 1.0;
            } else if let Some(node) = progress.chain.iter().find(|n| n.has_resource) {
                if node.duration != 0.0 {
                    q = f64::from(progress.numerator) / f64::from(node.duration);
                }
            }
        }
    }
    if info.has(flags::PROGRESS) {
        let other = 1.0 - q;
        let mut material = host.model_material(model);
        material.rgba = [
            (0.921875 * q + 0.12890625 * other) as f32,
            (f64::from((0.0546875 * q) as f32) + f64::from((0.89453125 * other) as f32)) as f32,
            (f64::from((0.05859375 * q) as f32) + f64::from((0.09765625 * other) as f32)) as f32,
            1.0,
        ];
        host.model_op(model, ModelOp::Material(material));
    }
    if let Some(processor) = info.processor {
        host.model_op(model, ModelOp::Light { processor, flags: 0 });
    }
    let mut p = host.position(info.occupant);
    let camera = host.camera();
    host.model_op(model, ModelOp::ZoomRotation(camera.zoom, camera.rotation));
    p[1] = ((f64::from(p[1]) + 10.0) + f64::from(info.display_size[1]) * 0.5) as f32;
    let mut transform = host.model_transform(model);
    transform.translation = p;
    transform.flags[1] |= 5;
    transform.flags[0] = transform.flags[0].wrapping_add(3);
    let a = 7.0 * f64::from(RADIANS);
    let c = f64::from(a.cos() as f32);
    let s = a.sin();
    for (i, row) in transform.rotation.iter_mut().enumerate() {
        let x = f64::from(row[0]);
        let z = f64::from(row[2]);
        let (xc, zs, xs, zc) = (x * c, z * s, x * s, z * c);
        row[0] = if i == 0 {
            (xc - zs) as f32
        } else {
            (f64::from(xc as f32) - f64::from(zs as f32)) as f32
        };
        row[2] = if i == 0 {
            (xs + zc) as f32
        } else if i == 1 {
            (f64::from(xs as f32) + f64::from(zc as f32)) as f32
        } else {
            // SPEC-GAP: §5.7 does not identify the one unrounded product in
            // row 2. Accept only an unambiguous stored result (DVV30-2).
            let first = (xs + f64::from(zc as f32)) as f32;
            let second = (f64::from(xs as f32) + zc) as f32;
            if first.to_bits() != second.to_bits() {
                return Err(Unsupported::OpenSpinRow2);
            }
            first
        };
    }
    host.model_op(model, ModelOp::Transform(transform));
    host.model_op(model, ModelOp::Fade(info.fade < 1.0, info.fade));
    host.model_op(model, ModelOp::Advance(elapsed));
    host.model_op(model, ModelOp::Frame(1));
    host.model_op(model, ModelOp::Draw(0));
    Ok(())
}
