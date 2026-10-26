// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::sync::Arc;

/// Pixel storage belongs to the foreign buffer. The six-word exposed record
/// uses an opaque token rather than dereferencing a game pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BufferRecord {
    pub pixels: usize,
    pub stride: i32,
    pub width: u32,
    pub height: u32,
    pub colour_type: u32,
    pub bytes_per_pixel: u32,
}
impl BufferRecord {
    pub fn canvas(self) -> Rect {
        Rect(0, 0, self.width as i32, self.height as i32)
    }
}
pub trait Buffer: Send + Sync {
    fn lock(&self, flags: u32) -> bool;
    fn unlock(&self, flags: u32);
    fn record(&self) -> BufferRecord;
    fn native_rgb(&self, colour: u32) -> u32;
    /// Foreign storage access only; the supplied record identifies the cached
    /// pixel allocation, not a refreshed per-draw buffer record.
    fn pixels(&self, record: BufferRecord, operation: &mut dyn FnMut(&mut [u8], usize) -> Result<()>) -> Result<()>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanFormat {
    Eight,
    Rgb1555,
    Rgb565,
    Rgba4444,
    Rgba8888,
}
impl SpanFormat {
    pub fn select(colour_type: u32) -> Option<Self> {
        match colour_type {
            2 => Some(Self::Eight),
            4 | 6 => Some(Self::Rgb1555),
            5 => Some(Self::Rgb565),
            7 => Some(Self::Rgba4444),
            8 | 9 => Some(Self::Rgba8888),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct DrawPlan {
    pub result: u32,
    /// AA groups or hardware input batches. Edge/device generation is GZG2.
    pub polygons: Vec<Vec<Point>>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct EngineState {
    pub clip: Rect,
    pub colours: [u32; 4],
    pub alpha: bool,
    pub alpha_above_one: bool,
    pub modulation: u8,
    pub aa: u8,
    pub thickness: f32,
    pub gradient: Option<Vec<u32>>,
    pub gradient_origin: [f32; 2],
    pub tiling: Option<[u32; 4]>,
    pub dirty: bool,
    pub transform_dirty: bool,
}
impl Default for EngineState {
    fn default() -> Self {
        // Explicit safe placeholders for hardware defaults left open in GZG2;
        // attachment supplies the context's actual drawing state before drawing.
        Self {
            clip: Rect::default(),
            colours: [0; 4],
            alpha: false,
            alpha_above_one: false,
            modulation: 0,
            aa: 0,
            thickness: 1.,
            gradient: None,
            gradient_origin: [0.; 2],
            tiling: None,
            dirty: false,
            transform_dirty: false,
        }
    }
}
impl EngineState {
    fn apply(&mut self, c: StateChange, hardware: bool) {
        match c {
            StateChange::Clip(r) => {
                self.clip = r;
                self.transform_dirty = hardware;
            }
            StateChange::Colours(v) => {
                self.colours = v;
                self.dirty |= hardware;
            }
            StateChange::Alpha(v) => {
                self.alpha = v > 0;
                self.alpha_above_one = hardware && v > 1;
                self.dirty |= hardware;
            }
            StateChange::Modulation(v) => {
                self.modulation = v;
                self.dirty |= hardware;
            }
            StateChange::Antialias(v) => self.aa = v,
            StateChange::Thickness(v) => self.thickness = v,
            StateChange::Gradient(v, x, y) => {
                self.gradient = v;
                self.gradient_origin = [x, y];
                self.dirty |= hardware;
            }
            StateChange::Tiling(v) => {
                self.tiling = v;
                self.transform_dirty |= hardware;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GradientCoefficients {
    pub constant: [i32; 4],
    pub x: [i32; 4],
    pub y: [i32; 4],
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IntegerRounding {
    #[default]
    NearestEven,
    Down,
    Up,
    TowardZero,
}
impl GradientCoefficients {
    pub fn convert(words: &[u32], anchor: Point) -> Result<Self> {
        Self::convert_with_rounding(words, anchor, IntegerRounding::NearestEven)
    }
    pub fn convert_with_rounding(words: &[u32], anchor: Point, rounding: IntegerRounding) -> Result<Self> {
        if words.len() != 12 || anchor.iter().any(|v| !v.is_finite()) {
            return Err(Error::InvalidInput);
        }
        // Rust 1.75: spell out default x87 nearest-even, without round_ties_even.
        fn integer(word: u32, rounding: IntegerRounding) -> Result<i32> {
            let v = (f32::from_bits(word) as f64 * 16_711_680.) as f32;
            let v = v as f64;
            if !v.is_finite() || v < i32::MIN as f64 || v > i32::MAX as f64 {
                return Err(Error::InvalidInput);
            }
            let floor = v.floor();
            let fraction = v - floor;
            match rounding {
                IntegerRounding::Down => return Ok(floor as i32),
                IntegerRounding::Up => return Ok(v.ceil() as i32),
                IntegerRounding::TowardZero => return Ok(v as i32),
                IntegerRounding::NearestEven => (),
            }
            Ok(if fraction < 0.5 || (fraction == 0.5 && (floor as i64) & 1 == 0) {
                floor as i32
            } else {
                (floor + 1.) as i32
            })
        }
        let mut out = Self::default();
        for i in 0..4 {
            let c = integer(words[i], rounding)?;
            out.x[i] = integer(words[4 + i], rounding)?;
            out.y[i] = integer(words[8 + i], rounding)?;
            out.constant[i] = (c.wrapping_add(0x8000) as f64
                - (out.x[i] as f64 * anchor[0] as f64 + out.y[i] as f64 * anchor[1] as f64))
                as i32;
        }
        Ok(out)
    }
}

pub struct BufferEngine {
    buffer: Option<Arc<dyn Buffer>>,
    record: Option<BufferRecord>,
    pub renderer: Option<SpanFormat>,
    pub state: EngineState,
    pub native_colours: [u32; 4],
    pub source: Option<Arc<dyn Target>>,
    pub gradient: GradientCoefficients,
    /// Adapter-supplied x87 integer-conversion mode; default FP environment.
    pub gradient_rounding: IntegerRounding,
    locked: bool,
    /// Fault injection models the specified native success-with-null outcome.
    pub renderer_allocation_available: bool,
}
impl Default for BufferEngine {
    fn default() -> Self {
        Self {
            buffer: None,
            record: None,
            renderer: None,
            state: EngineState::default(),
            native_colours: [0; 4],
            source: None,
            gradient: GradientCoefficients::default(),
            gradient_rounding: IntegerRounding::NearestEven,
            locked: false,
            renderer_allocation_available: true,
        }
    }
}
impl BufferEngine {
    pub fn plan_draw(&self, kind: u32, points: &[Point]) -> DrawPlan {
        if self.state.aa == 0 {
            return DrawPlan {
                result: 2,
                polygons: Vec::new(),
            };
        }
        let polygons = match kind {
            6 => points.chunks_exact(3).map(<[Point]>::to_vec).collect(),
            7 => points.chunks_exact(4).map(<[Point]>::to_vec).collect(),
            8 => vec![points.to_vec()],
            _ => {
                return DrawPlan {
                    result: 1,
                    polygons: Vec::new(),
                }
            }
        };
        DrawPlan { result: 0, polygons }
    }
    pub fn span_route(&self) -> Result<SpanRoute> {
        if self.renderer.is_none() {
            return Err(Error::Foreign);
        }
        Ok(if self.source.is_some() {
            SpanRoute::Blit
        } else {
            match (self.state.gradient.is_some(), self.state.alpha) {
                (true, true) => SpanRoute::AlphaGradient,
                (true, false) => SpanRoute::Gradient,
                (false, true) => SpanRoute::Alpha,
                (false, false) => SpanRoute::Solid,
            }
        })
    }
    pub fn set_colours(&mut self, first: usize, colours: &[u32]) -> Result<()> {
        let end = first
            .checked_add(colours.len())
            .filter(|&n| n <= 4)
            .ok_or(Error::InvalidInput)?;
        if colours.is_empty() {
            return Ok(());
        }
        let buffer = self.buffer.as_ref().ok_or(Error::Detached)?;
        for (i, &colour) in (first..end).zip(colours) {
            self.state.colours[i] = colour;
            self.native_colours[i] = buffer.native_rgb(colour);
        }
        Ok(())
    }
    /// Already clipped packed spans. No duplicated pixel kernels: the selected
    /// existing software module performs all writes. Source binding remains GZG2.
    pub fn draw_preclipped_spans(&self, spans: &[(u32, u32)]) -> Result<()> {
        if spans.is_empty() {
            return Ok(());
        }
        let route = self.span_route()?;
        if route == SpanRoute::Blit {
            return Err(Error::Unsupported("GZG2"));
        }
        let record = self.record.ok_or(Error::Detached)?;
        let buffer = self.buffer.as_ref().ok_or(Error::Detached)?;
        let format = self.renderer.ok_or(Error::Foreign)?;
        let native = self.native_colours[0];
        let argb = self.state.colours[0];
        let g = self.gradient;
        buffer.pixels(record, &mut |bytes, origin| {
            macro_rules! runs {
                ($module:ident) => {
                    spans
                        .iter()
                        .map(|&(start, end)| crate::$module::Span { start, end })
                        .collect::<Vec<_>>()
                };
            }
            match format {
                SpanFormat::Eight => {
                    use crate::span_render8 as k;
                    let mut dest = k::Target {
                        data: bytes,
                        origin,
                        stride: record.stride,
                    };
                    let spans = runs!(span_render8);
                    if route == SpanRoute::Gradient || route == SpanRoute::AlphaGradient {
                        let g = k::Gradient {
                            start: g.constant.map(|v| v as u32),
                            dx: g.x.map(|v| v as u32),
                            dy: g.y.map(|v| v as u32),
                        };
                        if route == SpanRoute::Gradient {
                            k::render_gradient(&spans, &mut dest, &g)
                        } else {
                            k::render_alpha_gradient(&spans, &mut dest, &g)
                        }
                    } else if route == SpanRoute::Alpha {
                        k::render_alpha(&spans, &mut dest, native, argb)
                    } else {
                        k::render_solid(&spans, &mut dest, native)
                    }
                    .map_err(|_| Error::InvalidInput)
                }
                SpanFormat::Rgb565 => {
                    use crate::span_render565 as k;
                    let dest = k::Dest {
                        base: origin as i64,
                        stride: record.stride,
                    };
                    let spans = runs!(span_render565);
                    if route == SpanRoute::Gradient || route == SpanRoute::AlphaGradient {
                        let g = k::Gradient {
                            c: g.constant.map(|v| v as u32),
                            dx: g.x.map(|v| v as u32),
                            dy: g.y.map(|v| v as u32),
                        };
                        if route == SpanRoute::Gradient {
                            k::render_gradient(bytes, &spans, &dest, &g)
                        } else {
                            k::render_alpha_gradient(bytes, &spans, &dest, &g)
                        }
                    } else if route == SpanRoute::Alpha {
                        k::render_alpha(bytes, &spans, &dest, native, argb)
                    } else {
                        k::render_solid(bytes, &spans, &dest, native)
                    }
                    .map_err(|_| Error::InvalidInput)
                }
                SpanFormat::Rgb1555 => {
                    use crate::span_render1555 as k;
                    let mut dest = k::Surface {
                        buf: bytes,
                        base: origin as i64,
                        stride: record.stride,
                    };
                    let spans = runs!(span_render1555);
                    if route == SpanRoute::Gradient || route == SpanRoute::AlphaGradient {
                        let g = k::GradientRecord {
                            c: g.constant,
                            dx: g.x,
                            dy: g.y,
                        };
                        if route == SpanRoute::Gradient {
                            k::render_gradient(&spans, &mut dest, &g)
                        } else {
                            k::render_alpha_gradient(&spans, &mut dest, &g)
                        }
                    } else if route == SpanRoute::Alpha {
                        k::render_alpha(&spans, &mut dest, native, argb)
                    } else {
                        k::render_solid(&spans, &mut dest, native)
                    }
                    .map_err(|_| Error::InvalidInput)
                }
                SpanFormat::Rgba4444 => {
                    use crate::span_render4444 as k;
                    let mut dest = k::Target {
                        data: bytes,
                        origin,
                        stride: record.stride,
                    };
                    let spans = runs!(span_render4444);
                    if route == SpanRoute::Gradient || route == SpanRoute::AlphaGradient {
                        let g = k::Gradient {
                            start: g.constant,
                            dx: g.x,
                            dy: g.y,
                        };
                        if route == SpanRoute::Gradient {
                            k::render_gradient(&spans, &mut dest, &g)
                        } else {
                            k::render_alpha_gradient(&spans, &mut dest, &g)
                        }
                    } else if route == SpanRoute::Alpha {
                        k::render_alpha(&spans, &mut dest, native, argb)
                    } else {
                        k::render_solid(&spans, &mut dest, native)
                    }
                    .map_err(|_| Error::InvalidInput)
                }
                SpanFormat::Rgba8888 => {
                    use crate::span_render8888 as k;
                    let dest = k::Surface {
                        base: origin,
                        stride: record.stride,
                    };
                    let spans = runs!(span_render8888);
                    if route == SpanRoute::Gradient || route == SpanRoute::AlphaGradient {
                        let g = k::Gradient {
                            start: g.constant.map(|v| v as u32),
                            dx: g.x.map(|v| v as u32),
                            dy: g.y.map(|v| v as u32),
                        };
                        if route == SpanRoute::Gradient {
                            k::render_gradient(&spans, bytes, dest, &g)
                        } else {
                            k::render_alpha_gradient(&spans, bytes, dest, &g)
                        }
                    } else if route == SpanRoute::Alpha {
                        k::render_alpha(&spans, bytes, dest, native, argb)
                    } else {
                        k::render_solid(&spans, bytes, dest, native)
                    }
                    .map_err(|_| Error::InvalidInput)
                }
            }
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanRoute {
    Solid,
    Gradient,
    Alpha,
    AlphaGradient,
    Blit,
}
impl RenderEngine for BufferEngine {
    fn init(&mut self, target: &Arc<dyn Target>) -> bool {
        self.shutdown();
        let Some(buffer) = target.buffer() else {
            return false;
        };
        if !buffer.lock(0x8040) {
            return false;
        }
        let record = buffer.record();
        let Some(format) = SpanFormat::select(record.colour_type) else {
            // Native quirk: successful lock followed by unsupported format is
            // released without Unlock (§4.2 / SR8 §10.3).
            return false;
        };
        self.renderer = self.renderer_allocation_available.then_some(format);
        self.record = Some(record);
        self.buffer = Some(buffer);
        self.locked = true;
        self.state.tiling = None;
        true
    }
    fn shutdown(&mut self) {
        self.renderer = None;
        if self.locked {
            if let Some(b) = &self.buffer {
                b.unlock(0x8040);
            }
        }
        self.locked = false;
        self.buffer = None;
        self.record = None;
    }
    fn canvas(&self) -> Rect {
        self.record.map(BufferRecord::canvas).unwrap_or_default()
    }
    fn state(&mut self, c: StateChange) -> Result<()> {
        if let StateChange::Colours(colours) = c {
            return self.set_colours(0, &colours);
        }
        if let StateChange::Gradient(Some(words), x, y) = &c {
            self.gradient = GradientCoefficients::convert_with_rounding(words, [*x, *y], self.gradient_rounding)?;
        }
        self.state.apply(c, false);
        Ok(())
    }
    fn bind(&mut self, b: BlitBinding) -> Result<Option<Rect>> {
        if b.source.is_some() {
            return Err(Error::Unsupported("GZG2"));
        }
        self.source = None;
        Ok(None)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
    fn suspend(&mut self) -> Result<()> {
        if self.locked {
            if let Some(b) = &self.buffer {
                b.unlock(0x8040);
            }
            self.locked = false;
        }
        Ok(())
    }
    fn resume(&mut self) -> Result<()> {
        if !self.locked {
            if let Some(b) = &self.buffer {
                let _ = b.lock(0x8040);
                self.locked = true;
            }
        }
        Ok(())
    }
    fn lock(&self) -> Result<BufferRecord> {
        self.record.ok_or(Error::Detached)
    }
    fn unlock(&mut self) -> Result<()> {
        Ok(())
    }
    fn draw(&mut self, kind: u32, points: &[Point]) -> Result<u32> {
        let plan = self.plan_draw(kind, points);
        if !plan.polygons.is_empty() {
            return Err(Error::Unsupported("GZG2"));
        }
        Ok(plan.result)
    }
    fn generic(&mut self, route: GeometryRoute, kind: u32, points: &[Point]) -> Result<()> {
        let state = self.state.clone();
        super::geometry::execute(self, route, kind, points, &state)
    }
    fn spans(&mut self, spans: &[PixelSpan]) -> Result<()> {
        let packed: Vec<_> = spans
            .iter()
            .map(|s| {
                let start = ((s.y as u32) << 16) | u32::from(s.left as u16);
                (start, start.wrapping_add(s.right.wrapping_sub(s.left) as u32))
            })
            .collect();
        self.draw_preclipped_spans(&packed)
    }
}
// Deliberately no Drop->Shutdown: final native Release does not unlock (§4.2).
// The context explicitly shuts down; directly owned engines require Shutdown.

pub trait HardwareDevice: Send + Sync {
    fn init(&mut self, buffer: &Arc<dyn Buffer>) -> bool;
    fn apply_state(&mut self, state: &EngineState) -> Result<()>;
    fn update_transform(&mut self, state: &EngineState) -> Result<()>;
    /// Device submission only; unresolved vertex/subdivision rules are not
    /// supplied as guessed vertices. Implementations can reject with GZG2.
    fn submit(&mut self, kind: u32, points: &[Point], state: &EngineState) -> Result<()>;
    fn suspend(&mut self) -> Result<()>;
    fn resume(&mut self) -> Result<()>;
    fn shutdown(&mut self) {}
    fn viewport(&mut self, _bounds: Rect) -> Result<()> {
        Err(Error::Unsupported("GZG2"))
    }
    fn matrix_mode(&mut self, _mode: u32) -> Result<()> {
        Err(Error::Unsupported("GZG2"))
    }
    fn projection(&mut self, _matrix: &[f32; 16]) -> Result<()> {
        Err(Error::Unsupported("GZG2"))
    }
    fn vertices(&mut self, _primitive: u32, _records: &[[u32; 6]]) -> Result<()> {
        Err(Error::Unsupported("GZG2"))
    }
}
pub struct HardwareEngine {
    pub state: EngineState,
    buffer: Option<Arc<dyn Buffer>>,
    record: Option<BufferRecord>,
    pub source: Option<Arc<dyn Target>>,
    device: Box<dyn HardwareDevice>,
}
impl HardwareEngine {
    pub fn new(device: Box<dyn HardwareDevice>) -> Self {
        Self {
            state: EngineState::default(),
            buffer: None,
            record: None,
            source: None,
            device,
        }
    }
    pub fn set_colours(&mut self, first: usize, colours: &[u32]) -> Result<()> {
        let end = first
            .checked_add(colours.len())
            .filter(|&n| n <= 4)
            .ok_or(Error::InvalidInput)?;
        self.state.colours[first..end].copy_from_slice(colours);
        self.state.dirty = true;
        Ok(())
    }
}
impl RenderEngine for HardwareEngine {
    fn init(&mut self, target: &Arc<dyn Target>) -> bool {
        self.buffer = None;
        self.record = None;
        let Some(buffer) = target.buffer() else {
            return false;
        };
        if !self.device.init(&buffer) {
            return false;
        }
        self.record = Some(buffer.record());
        self.buffer = Some(buffer);
        self.state.dirty = true;
        true
    }
    fn shutdown(&mut self) {
        // §4.4: the base only tail-dispatches its hardware hook.
        self.device.shutdown();
    }
    fn canvas(&self) -> Rect {
        self.record.map(BufferRecord::canvas).unwrap_or_default()
    }
    fn state(&mut self, c: StateChange) -> Result<()> {
        if matches!(&c, StateChange::Gradient(Some(_), _, _)) {
            return Err(Error::Unsupported("GZG2"));
        }
        self.state.apply(c, true);
        Ok(())
    }
    fn bind(&mut self, b: BlitBinding) -> Result<Option<Rect>> {
        if b.source.is_some() {
            return Err(Error::Unsupported("GZG2"));
        }
        self.source = None;
        Ok(None)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
    fn suspend(&mut self) -> Result<()> {
        self.device.suspend()
    }
    fn resume(&mut self) -> Result<()> {
        self.device.resume()
    }
    fn lock(&self) -> Result<BufferRecord> {
        Err(Error::Unsupported("GZG2"))
    }
    fn unlock(&mut self) -> Result<()> {
        Ok(())
    }
    fn draw(&mut self, kind: u32, points: &[Point]) -> Result<u32> {
        if kind != 3 && kind != 6 {
            return Ok(1);
        }
        if self.state.dirty {
            self.device.apply_state(&self.state)?;
            self.state.dirty = false;
        }
        if self.state.transform_dirty {
            self.device.update_transform(&self.state)?;
            self.state.transform_dirty = false;
        }
        self.device.submit(kind, points, &self.state)?;
        Ok(0)
    }
    fn generic(&mut self, route: GeometryRoute, kind: u32, points: &[Point]) -> Result<()> {
        let state = self.state.clone();
        super::geometry::execute(self, route, kind, points, &state)
    }
}

/// The service reference is retained independently from the target/device.
pub trait DrawService: Send + Sync {
    fn dispatch_slot9(&self);
}
pub struct SimGlEngine {
    pub base: HardwareEngine,
    pub service: Option<Arc<dyn DrawService>>,
    pub half_pixel: f32,
    pub controls: [u8; 3],
    pub specialized: bool,
    pub device_resources: [Option<usize>; 3],
    pub image_binding: ImageBinding,
}
impl SimGlEngine {
    pub fn new(device: Box<dyn HardwareDevice>, service: Option<Arc<dyn DrawService>>) -> Self {
        Self {
            base: HardwareEngine::new(device),
            service,
            half_pixel: 0.5,
            controls: [0, 1, 0],
            specialized: false,
            device_resources: [None; 3],
            image_binding: ImageBinding::default(),
        }
    }
    pub fn factory_accepts(target: &dyn Target) -> bool {
        target.supports_simgl()
    }
    pub fn specialized_pair_count(&self, kind: u32, points: &[Point]) -> Option<usize> {
        (kind == 3 && self.specialized && self.base.source.is_some()).then_some(points.len() / 2)
    }
    pub fn hw_viewport(&mut self, bounds: Rect) -> Result<()> {
        self.base.device.viewport(Rect(
            bounds.0,
            self.base.canvas().3 - bounds.3,
            bounds.width(),
            bounds.height(),
        ))
    }
    pub fn hw_projection(&mut self, matrix: &[f32; 16]) -> Result<()> {
        self.base.device.matrix_mode(1)?;
        self.base.device.projection(matrix)
    }
    pub fn hw_vertices(&mut self, triangles: bool, records: &[[u32; 6]]) -> Result<()> {
        self.base.device.vertices(if triangles { 6 } else { 0 }, records)
    }
}
impl RenderEngine for SimGlEngine {
    fn init(&mut self, t: &Arc<dyn Target>) -> bool {
        self.base.init(t)
    }
    fn shutdown(&mut self) {
        self.base.shutdown();
    }
    fn canvas(&self) -> Rect {
        self.base.canvas()
    }
    fn state(&mut self, c: StateChange) -> Result<()> {
        self.base.state(c)
    }
    fn bind(&mut self, b: BlitBinding) -> Result<Option<Rect>> {
        self.base.bind(b)
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
    fn suspend(&mut self) -> Result<()> {
        self.base.suspend()
    }
    fn resume(&mut self) -> Result<()> {
        self.base.resume()
    }
    fn lock(&self) -> Result<BufferRecord> {
        self.base.lock()
    }
    fn unlock(&mut self) -> Result<()> {
        Ok(())
    }
    fn draw(&mut self, kind: u32, points: &[Point]) -> Result<u32> {
        match self.specialized_pair_count(kind, points) {
            Some(0) => Ok(0),
            Some(_) => {
                // SPEC-GAP: §4.4 requires native specialized vertices and result-3 captures.
                Err(Error::Unsupported("GZG2"))
            }
            None => self.base.draw(kind, points),
        }
    }
    fn generic(&mut self, route: GeometryRoute, kind: u32, points: &[Point]) -> Result<()> {
        let state = self.base.state.clone();
        super::geometry::execute(self, route, kind, points, &state)
    }
}
