// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::widget_controls::{Delivery, Draw, Message, Rect, Resource};

#[derive(Default)]
pub(super) struct RecordingHost {
    events: Vec<String>,
    cells: [usize; 2],
}

impl base::Host for RecordingHost {}
impl Host for RecordingHost {
    fn tuning_ramp(&mut self, _: [u32; 3], _: u32) -> Result<Option<Vec<(u32, u32)>>> { Ok(None) }
    fn city_cells(&mut self) -> [usize; 2] { self.cells }
    fn animation_target(&mut self, _: Window, register: bool) { self.events.push(format!("tick:{register}")); }
    fn subscribe(&mut self, _: Window, message: u32, subscribe: bool) { self.events.push(format!("sub:{message:08x}:{subscribe}")); }
}
impl widget_controls::Foreign for RecordingHost {
    fn add_ref(&mut self, h: Handle) { self.events.push(format!("add:{h}")); }
    fn release(&mut self, h: Handle) { self.events.push(format!("release:{h}")); }
    fn base_init(&mut self, _: Handle) -> bool { panic!("use cGZWin runtime") }
    fn base_shutdown(&mut self, _: Handle) { panic!("use cGZWin runtime") }
    fn base_message(&mut self, _: Handle, _: Message) -> bool { panic!("use cGZWin runtime") }
    fn base_query(&mut self, _: Handle, _: u32, _: &mut Option<Handle>) -> bool { panic!("use cGZWin runtime") }
    fn invalidate(&mut self, h: Handle) { self.events.push(format!("repaint:{h}")); }
    fn show(&mut self, _: Handle, _: bool) { unreachable!() }
    fn capture(&mut self, _: Handle, take: bool) { self.events.push(format!("capture:{take}")); }
    fn convert_payload(&mut self, _: Handle, _: i32) -> i32 { unreachable!() }
    fn deliver(&mut self, _: Handle, _: Delivery, _: Message) -> bool { unreachable!() }
    fn drop_complete(&mut self, _: Handle, _: u32, _: i32) { unreachable!() }
    fn default_interface(&mut self, _: u32) -> Option<Handle> { unreachable!() }
    fn default_color(&mut self, _: u32) -> Option<u32> { unreachable!() }
    fn create(&mut self, _: Resource) -> Option<Handle> { unreachable!() }
    fn font_height(&mut self, _: Handle) -> i32 { unreachable!() }
    fn buffer_height(&mut self, _: Handle) -> i32 { unreachable!() }
    fn bar_value(&mut self, _: Handle) -> i32 { unreachable!() }
    fn bar_range(&mut self, _: Handle, _: i32, _: i32, _: i32) { unreachable!() }
    fn place_bar(&mut self, _: Handle, _: Rect) { unreachable!() }
    fn create_bar(&mut self, _: Handle) -> Option<Handle> { unreachable!() }
    fn destroy_bar(&mut self, _: Handle) { unreachable!() }
    fn timer_elapsed(&mut self, _: Handle) -> u32 { unreachable!() }
    fn restart_timer(&mut self, _: Handle) { unreachable!() }
    fn double_click_interval(&mut self) -> u32 { unreachable!() }
    fn begins_with(&mut self, _: &[u8], _: &[u8], _: u32) -> bool { unreachable!() }
    fn text_equal(&mut self, _: &[u8], _: &[u8]) -> bool { unreachable!() }
    fn draw(&mut self, d: Draw) { self.events.push(format!("draw:{d:?}")); }
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
    map.set_area(&mut windows, &mut host, base::Rect(0, 0, 200, 150)).unwrap();
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
    map.set_area(&mut windows, &mut host, base::Rect(0, 0, 128, 128)).unwrap();
    map.on_city_init(&mut host).unwrap();
    assert_eq!(map.scale_shift(), -1); // MM-C3
    assert_eq!(map.networks[0], 3);
    let count = host.events.len();
    map.on_city_init(&mut host).unwrap();
    assert_eq!(host.events.len(), count);
}
