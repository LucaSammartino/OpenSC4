// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::widget_controls::{Delivery, Draw, Message, Rect, Resource};

#[derive(Default)]
pub(super) struct RecordingHost {
    pub events: Vec<String>,
    pub cells: [usize; 2],
    pub finds: usize,
    pub network_cell: Option<[usize; 2]>,
    pub network_interface_present: bool,
    pub network_mask: u32,
    pub occupant_cell: [i32; 2],
    pub surfaces: std::collections::BTreeMap<Handle, SurfacePixels>,
    pub fail_lock: Option<Handle>,
    pub rotation: u32,
    pub targets: Vec<[f32; 3]>,
    pub factory: Option<Option<Handle>>,
    pub init_surface_ok: bool,
    pub terrain: Option<([usize; 2], f32)>,
    pub water: Option<Box<[f32; 256]>>,
    pub height_rects: Vec<base::Rect>,
    pub zone: i8,
    pub ramps: std::collections::BTreeMap<u32, Vec<(u32, u32)>>,
    pub geometry: Option<PaintGeometry>,
    pub buffer: Option<PaintPixels>,
    pub marker: Option<Option<Handle>>,
    pub active_marker: bool,
}

impl base::Host for RecordingHost {}
impl Host for RecordingHost {
    fn rotation(&mut self) -> u32 {
        self.rotation
    }
    fn tuning_ramp(&mut self, key: [u32; 3], property: u32) -> Result<Option<Vec<(u32, u32)>>> {
        assert_eq!(key, TUNING_KEY);
        Ok(self.ramps.get(&property).cloned())
    }
    fn city_cells(&mut self) -> [usize; 2] {
        self.cells
    }
    fn animation_target(&mut self, _: Window, register: bool) {
        self.events.push(format!("tick:{register}"));
    }
    fn subscribe(&mut self, _: Window, message: u32, subscribe: bool) {
        self.events.push(format!("sub:{message:08x}:{subscribe}"));
    }
}

impl Terrain for RecordingHost {
    fn terrain_info(&mut self) -> Option<([usize; 2], f32)> {
        self.terrain
    }
    fn terrain_heights(&mut self, rect: base::Rect) -> Result<[f32; 256]> {
        self.height_rects.push(rect);
        Ok([90.0; 256])
    }
    fn terrain_water_values(&mut self, _: base::Rect) -> Result<[f32; 256]> {
        self.water.as_deref().copied().ok_or(Unsupported::Open("MMW1"))
    }
    fn terrain_normal(&mut self, _: usize, _: usize) -> Result<[f32; 3]> {
        Ok([0.0, 1.0, 0.0])
    }
    fn zone_at(&mut self, _: usize, _: usize) -> Result<i8> {
        Ok(self.zone)
    }
}

impl Network for RecordingHost {
    fn find_occupant(&mut self, x: usize, z: usize, _: &SurfaceNetworkFilter) -> Option<Handle> {
        self.finds += 1;
        (self.network_cell == Some([x, z])).then_some(100)
    }
    fn network_interface(&mut self, _: Handle) -> Option<Handle> {
        self.network_interface_present.then_some(200)
    }
    fn network_type_mask(&mut self, _: Handle) -> u32 {
        self.network_mask
    }
    fn occupant_cell(&mut self, _: Handle) -> [i32; 2] {
        self.occupant_cell
    }
}

impl Surface for RecordingHost {
    fn create_surface(&mut self, service: u32, iid: u32) -> Result<Option<Handle>> {
        assert_eq!((service, iid), (0xc416025c, 0x0073283c));
        self.factory.ok_or(Unsupported::Open("MMW4"))
    }
    fn init_surface(&mut self, s: Handle, n: usize, format: [u32; 2]) -> Result<bool> {
        assert_eq!(format, [9, 32]);
        self.events.push(format!("init:{s}:{n}"));
        if self.init_surface_ok {
            self.surfaces.insert(
                s,
                SurfacePixels {
                    pixels: vec![0; n * n],
                    pitch: n,
                },
            );
        }
        Ok(self.init_surface_ok)
    }
    fn shutdown_surface(&mut self, s: Handle) {
        self.events.push(format!("shutdown:{s}"));
    }
    fn surface_lock(&mut self, s: Handle, code: u32) -> Result<Option<SurfacePixels>> {
        self.events.push(format!("lock:{s}:{code:04x}"));
        if self.fail_lock == Some(s) {
            return Ok(None);
        }
        self.surfaces
            .get(&s)
            .cloned()
            .map(Some)
            .ok_or(Unsupported::Open("MMW4"))
    }
    fn surface_unlock(&mut self, s: Handle, p: SurfacePixels) {
        self.events.push(format!("unlock:{s}"));
        self.surfaces.insert(s, p);
    }
    fn surface_clear(&mut self, s: Handle, color: [u8; 4], lock: &mut SurfacePixels) {
        self.events.push(format!("clear:{s}:{color:?}"));
        lock.pixels.fill(0x0064647f);
    }
}

impl Paint for RecordingHost {
    fn paint_geometry(&mut self, _: Window) -> Result<PaintGeometry> {
        self.geometry.ok_or(Unsupported::Open("MMW2"))
    }
    fn paint_buffer_lock(&mut self, _: Window) -> Result<Option<PaintPixels>> {
        self.events.push("buffer-lock".into());
        Ok(self.buffer.clone())
    }
    fn paint_buffer_unlock(&mut self, _: Window, pixels: PaintPixels) {
        self.events.push("buffer-unlock".into());
        self.buffer = Some(pixels);
    }
    fn surface_source_rect(&mut self, _: Handle) -> base::Rect {
        base::Rect(0, 0, 1, 1)
    }
    fn paint_blit(&mut self, _: Window, _: Handle, _: base::Rect, dst: base::Rect) {
        self.events.push(format!("blit:{dst:?}"));
    }
    fn world_size(&mut self) -> [f32; 2] {
        [100.0; 2]
    }
    fn main_dimensions(&mut self) -> [i32; 2] {
        [10; 2]
    }
    fn eye_ray(&mut self, screen: [i32; 2]) -> Ray {
        Ray {
            origin: [screen[0] as f32, 0.0, screen[1] as f32],
            direction: [0.0, -1.0, 0.0],
        }
    }
    fn pick_terrain_always(&mut self, ray: Ray, mode: u32) -> Result<[f32; 3]> {
        assert_eq!(mode, 1);
        Ok(ray.origin)
    }
    fn marker_service(&mut self) -> Result<Option<Handle>> {
        self.marker.ok_or(Unsupported::Open("MMW3"))
    }
    fn marker_subject(&mut self, _: Handle) -> Option<Handle> {
        Some(400)
    }
    fn marker_active(&mut self, _: Handle) -> bool {
        self.active_marker
    }
    fn marker_position(&mut self, _: Handle) -> [f32; 3] {
        [25.0, 0.0, 50.0]
    }
    fn paint_buffer_color(&mut self, _: Window, c: u32) {
        self.events.push(format!("marker:{c:08x}"));
    }
    fn paint_buffer_points(&mut self, _: Window, mode: u32, _: [[f32; 2]; 2], count: u32) {
        assert_eq!((mode, count), (3, 2));
    }
}

impl Input for RecordingHost {
    fn terrain_height_at(&mut self, _: f32, _: f32) -> Result<f32> {
        Ok(12.0)
    }
    fn set_view_target(&mut self, target: [f32; 3]) {
        self.targets.push(target);
    }
}
impl widget_controls::Foreign for RecordingHost {
    fn add_ref(&mut self, h: Handle) {
        self.events.push(format!("add:{h}"));
    }
    fn release(&mut self, h: Handle) {
        self.events.push(format!("release:{h}"));
    }
    fn base_init(&mut self, _: Handle) -> bool {
        panic!("use cGZWin runtime")
    }
    fn base_shutdown(&mut self, _: Handle) {
        panic!("use cGZWin runtime")
    }
    fn base_message(&mut self, _: Handle, _: Message) -> bool {
        panic!("use cGZWin runtime")
    }
    fn base_query(&mut self, _: Handle, _: u32, _: &mut Option<Handle>) -> bool {
        panic!("use cGZWin runtime")
    }
    fn invalidate(&mut self, h: Handle) {
        self.events.push(format!("repaint:{h}"));
    }
    fn show(&mut self, _: Handle, _: bool) {
        unreachable!()
    }
    fn capture(&mut self, _: Handle, take: bool) {
        self.events.push(format!("capture:{take}"));
    }
    fn convert_payload(&mut self, _: Handle, _: i32) -> i32 {
        unreachable!()
    }
    fn deliver(&mut self, _: Handle, _: Delivery, _: Message) -> bool {
        unreachable!()
    }
    fn drop_complete(&mut self, _: Handle, _: u32, _: i32) {
        unreachable!()
    }
    fn default_interface(&mut self, _: u32) -> Option<Handle> {
        unreachable!()
    }
    fn default_color(&mut self, _: u32) -> Option<u32> {
        unreachable!()
    }
    fn create(&mut self, _: Resource) -> Option<Handle> {
        unreachable!()
    }
    fn font_height(&mut self, _: Handle) -> i32 {
        unreachable!()
    }
    fn buffer_height(&mut self, _: Handle) -> i32 {
        unreachable!()
    }
    fn bar_value(&mut self, _: Handle) -> i32 {
        unreachable!()
    }
    fn bar_range(&mut self, _: Handle, _: i32, _: i32, _: i32) {
        unreachable!()
    }
    fn place_bar(&mut self, _: Handle, _: Rect) {
        unreachable!()
    }
    fn create_bar(&mut self, _: Handle) -> Option<Handle> {
        unreachable!()
    }
    fn destroy_bar(&mut self, _: Handle) {
        unreachable!()
    }
    fn timer_elapsed(&mut self, _: Handle) -> u32 {
        unreachable!()
    }
    fn restart_timer(&mut self, _: Handle) {
        unreachable!()
    }
    fn double_click_interval(&mut self) -> u32 {
        unreachable!()
    }
    fn begins_with(&mut self, _: &[u8], _: &[u8], _: u32) -> bool {
        unreachable!()
    }
    fn text_equal(&mut self, _: &[u8], _: &[u8]) -> bool {
        unreachable!()
    }
    fn draw(&mut self, d: Draw) {
        self.events.push(format!("draw:{d:?}"));
    }
}

pub(super) fn fixture() -> (Windows, RecordingHost, MiniMap) {
    let mut windows = Windows::new(1);
    let mut host = RecordingHost::default();
    let map = MiniMap::new(&mut windows, &mut host);
    (windows, host, map)
}

#[test]
fn constructor_layout_and_inherited_query() {
    let (mut windows, _, map) = fixture();
    assert_eq!(CLASS_ID, 0xca31_8388);
    assert_eq!(map.flags(), 7);
    assert_eq!((map.n, map.cells, map.shift), (0, [0; 2], 0));
    assert_eq!(map.zone_colors, [0; 16]);
    assert_eq!(map.terrain_palette, [0; 256]);
    assert_eq!(map.water_palette, [0; 256]);
    assert_eq!(map.dirty, [0; 16]);
    assert!(map.networks.is_empty());
    assert_eq!((map.surface, map.overlay, map.elapsed), (None, None, 0.0));
    assert!(!map.attached && !map.map_dirty && !map.surface_dirty && !map.repaint);
    assert!(!map.dragging && !map.hooked && !map.blink);
    let mut out = None;
    assert!(map.query_interface(&mut windows, INTERFACE_ID, &mut out));
    assert_eq!(out, Some(map.as_igzwin()));
    assert!(map.query_interface(&mut windows, base::IID_WIN, &mut out));
    assert_eq!(windows.get(map.window).references, 2);
    assert!(!map.query_interface(&mut windows, 123, &mut out));
    assert_eq!(out, Some(map.window));
}

#[test]
fn mm_c1_c2_c3_size_scale_and_attached_resize_quirk() {
    let (mut windows, mut host, mut map) = fixture();
    map.set_area(&mut windows, &mut host, base::Rect(0, 0, 200, 150))
        .unwrap();
    assert_eq!(map.map_size(), 128); // MM-C1
    host.cells = [256, 64];
    map.on_city_init(&mut host).unwrap();
    assert_eq!(map.scale_shift(), 1); // MM-C2; x count only
    assert_eq!(map.map.dimensions(), [128, 128]);
    assert_eq!(map.networks.len(), 1024);
    assert_eq!(map.dirty, [u32::MAX; 16]);
    assert_eq!(host.events, ["tick:true", "sub:99ef1142:true", "sub:99ef1143:true"]);
    map.set_area(&mut windows, &mut host, base::Rect(0, 0, 64, 64)).unwrap();
    assert_eq!(map.n, 64);
    assert_eq!(map.map.dimensions(), [128, 128]);
    assert_eq!(map.shift, 1);
    map.networks[0] = 3;
    map.on_city_shutdown(&mut host);
    assert!(map.map.pixels().is_none());
    assert_eq!(map.cells, [0, 0]);
    assert_eq!(map.networks[0], 3);
    host.cells = [64, 64];
    map.set_area(&mut windows, &mut host, base::Rect(0, 0, 128, 128))
        .unwrap();
    map.on_city_init(&mut host).unwrap();
    assert_eq!(map.scale_shift(), -1); // MM-C3
    assert_eq!(map.networks[0], 3);
    let count = host.events.len();
    map.on_city_init(&mut host).unwrap();
    assert_eq!(host.events.len(), count);
}
