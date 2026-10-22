// SPDX-License-Identifier: GPL-3.0-or-later
use super::super::*;
use crate::{terrain::VertRect, view_utilities::PixelRect};
use std::collections::VecDeque;

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Cursor(u32),
    Capture,
    Release,
    Top(bool),
    Sound(u32, u32),
    Motion(ScrollMotion),
    Stop,
    LoadPicture,
    Transparent(u64, (u8, u8, u8)),
    PictureWindow(u32, u64, PixelRect),
    Flag(u64, u32, bool),
    ShowWindow(u64),
    Child(u64),
    Front(u64),
    Destroy(u64),
    DeleteChild(u32),
    ReleasePicture(u64),
    Pick(Pixel),
    PickProp(Pixel, PropFilter),
    First(Vec3, PropFilter),
    Auto(bool),
    Mark(VertRect, u32, bool),
    Clear,
    Position(PropId, Vec3),
    Text(PropId, Text),
    Dynamic(PropId, bool),
    Edit(PropId, bool),
    Alpha(PropId, f32),
    Add(u32),
    Remove(PropId),
    Show(bool),
    Color(PropId, (f32, f32, f32)),
    Height(PropId, f32),
    Orientation(PropId, u8),
    Dialog((u32, u32), u32),
    Field(Text),
    SelectAll,
    Modal,
    Focus,
    ShutdownDialog,
}
#[derive(Clone, Debug)]
pub struct Prop {
    pub info: PropInfo,
    pub position: Vec3,
    pub text: Text,
    pub bounds: (Vec3, Vec3),
}
pub struct Host {
    pub events: Vec<Event>,
    pub props: Vec<Option<Prop>>,
    pub terrain_picks: VecDeque<Option<Vec3>>,
    pub renderer_picks: VecDeque<Option<PropId>>,
    pub occupant_picks: VecDeque<Option<PropId>>,
    pub renderer: bool,
    pub city_view: bool,
    pub capturing: bool,
    pub auto_cursor: bool,
    pub cursor_found: bool,
    pub picture_found: bool,
    pub window_found: bool,
    pub child_add: bool,
    pub rotation: u32,
    pub water: bool,
    pub tile: Option<NetworkTile>,
    pub road_tile: Option<NetworkTile>,
    pub avenue: Option<Edges>,
    pub center: Vec3,
    pub modal_result: u32,
    pub modal_text: Text,
    pub script_found: bool,
    pub ground: f32,
    pub sea: f32,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            events: vec![],
            props: vec![],
            terrain_picks: VecDeque::new(),
            renderer_picks: VecDeque::new(),
            occupant_picks: VecDeque::new(),
            renderer: true,
            city_view: true,
            capturing: false,
            auto_cursor: true,
            cursor_found: true,
            picture_found: true,
            window_found: true,
            child_add: true,
            rotation: 0,
            water: false,
            tile: None,
            road_tile: None,
            avenue: None,
            center: Vec3::new(24.0, 0.0, 40.0),
            modal_result: LABEL_OK,
            modal_text: Text::from_single_bytes(b"edited"),
            script_found: true,
            ground: 3.0,
            sea: 5.0,
        }
    }
}
impl Host {
    pub fn prop(&self, id: PropId) -> &Prop {
        self.props[id.0 as usize].as_ref().expect("access to removed prop")
    }
    pub fn prop_mut(&mut self, id: PropId) -> &mut Prop {
        self.props[id.0 as usize].as_mut().expect("access to removed prop")
    }
    pub fn insert(&mut self, kind: u32, text: &[u8], position: Vec3) -> PropId {
        let id = PropId(self.props.len() as u64);
        self.props.push(Some(Prop {
            info: PropInfo {
                is_prop: true,
                prop_type: kind,
                text_decal: kind == LABEL_TYPE,
                signpost: kind == SIGN_TYPE,
            },
            position,
            text: Text::from_single_bytes(text),
            bounds: (Vec3::ZERO, Vec3::new(4.0, 6.0, 8.0)),
        }));
        id
    }
    pub fn hit(&mut self, p: Vec3) {
        self.terrain_picks.push_back(Some(p));
    }
    pub fn clear(&mut self) {
        self.events.clear();
    }
}
impl ControlHost for Host {
    fn set_cursor(&mut self, id: u32) -> bool {
        self.events.push(Event::Cursor(id));
        self.cursor_found
    }
    fn am_capturing(&self) -> bool {
        self.capturing
    }
    fn capture(&mut self) -> bool {
        self.events.push(Event::Capture);
        self.capturing = true;
        true
    }
    fn release_capture(&mut self) -> bool {
        self.events.push(Event::Release);
        self.capturing = false;
        true
    }
    fn remove_top_control(&mut self, stack: bool) {
        self.events.push(Event::Top(stack));
    }
    fn post_sound(&mut self, event: u32, id: u32) {
        self.events.push(Event::Sound(event, id));
    }
}
impl ScrollHost for Host {
    fn scroll(&mut self, m: ScrollMotion) {
        self.events.push(Event::Motion(m));
    }
    fn scroll_stop(&mut self) {
        self.events.push(Event::Stop);
    }
    fn load_picture(&mut self, key: (u32, u32, u32)) -> Option<u64> {
        assert_eq!(key, (0x856D_DBAC, 0x46A0_06B0, 0x094A_C89A));
        self.events.push(Event::LoadPicture);
        self.picture_found.then_some(7)
    }
    fn picture_size(&self, _: u64) -> (i32, i32) {
        (20, 10)
    }
    fn picture_transparent_color(&mut self, p: u64, rgb: (u8, u8, u8)) {
        self.events.push(Event::Transparent(p, rgb));
    }
    fn create_picture_window(&mut self, id: u32, p: u64, r: PixelRect) -> Option<u64> {
        self.events.push(Event::PictureWindow(id, p, r));
        self.window_found.then_some(8)
    }
    fn window_flag(&mut self, w: u64, f: u32, on: bool) {
        self.events.push(Event::Flag(w, f, on));
    }
    fn window_show(&mut self, w: u64) {
        self.events.push(Event::ShowWindow(w));
    }
    fn child_add(&mut self, w: u64) -> bool {
        self.events.push(Event::Child(w));
        self.child_add
    }
    fn window_front(&mut self, w: u64) {
        self.events.push(Event::Front(w));
    }
    fn window_destroy(&mut self, w: u64) {
        self.events.push(Event::Destroy(w));
    }
    fn child_delete_id(&mut self, id: u32) {
        self.events.push(Event::DeleteChild(id));
    }
    fn release_picture(&mut self, p: u64) {
        self.events.push(Event::ReleasePicture(p));
    }
}
impl LevelView for Host {
    fn has_city_view(&self) -> bool {
        self.city_view
    }
    fn pick_city_terrain(&mut self, at: Pixel) -> Option<Vec3> {
        self.pick_terrain(at)
    }
    fn automatic_cell_cursor(&self) -> bool {
        self.auto_cursor
    }
    fn set_automatic_cell_cursor(&mut self, on: bool) {
        self.events.push(Event::Auto(on));
        self.auto_cursor = on;
    }
    fn mark_selected(&mut self, r: VertRect, m: u32, on: bool) {
        self.events.push(Event::Mark(r, m, on));
    }
    fn clear_selection(&mut self) {
        self.events.push(Event::Clear);
    }
}
impl PropWorld for Host {
    fn renderer_available(&self) -> bool {
        self.renderer
    }
    fn pick_terrain(&mut self, at: Pixel) -> Option<Vec3> {
        self.events.push(Event::Pick(at));
        self.terrain_picks.pop_front().flatten()
    }
    fn pick_prop(&mut self, at: Pixel, f: PropFilter) -> Option<PropId> {
        self.events.push(Event::PickProp(at, f));
        self.renderer_picks
            .pop_front()
            .flatten()
            .filter(|&p| f.accepts(self.prop_info(p)))
    }
    fn first_prop_at(&mut self, p: Vec3, f: PropFilter) -> Option<PropId> {
        self.events.push(Event::First(p, f));
        self.occupant_picks
            .pop_front()
            .flatten()
            .filter(|&p| f.accepts(self.prop_info(p)))
    }
    fn prop_info(&self, p: PropId) -> PropInfo {
        self.prop(p).info
    }
    fn position(&self, p: PropId) -> Vec3 {
        self.prop(p).position
    }
    fn set_position(&mut self, p: PropId, v: Vec3) {
        self.events.push(Event::Position(p, v));
        self.prop_mut(p).position = v;
    }
    fn text(&self, p: PropId) -> Text {
        self.prop(p).text.clone()
    }
    fn set_text(&mut self, p: PropId, t: &Text) {
        self.events.push(Event::Text(p, t.clone()));
        self.prop_mut(p).text = t.clone();
    }
    fn set_dynamic(&mut self, p: PropId, on: bool) {
        self.prop(p);
        self.events.push(Event::Dynamic(p, on));
    }
    fn set_edit_mode(&mut self, p: PropId, on: bool) {
        self.prop(p);
        self.events.push(Event::Edit(p, on));
    }
    fn set_alpha(&mut self, p: PropId, a: f32) {
        self.prop(p);
        self.events.push(Event::Alpha(p, a));
    }
    fn add_city_prop(&mut self, t: u32) -> Option<PropId> {
        self.events.push(Event::Add(t));
        Some(self.insert(t, b"", Vec3::ZERO))
    }
    fn remove_prop(&mut self, p: PropId) {
        self.prop(p);
        self.events.push(Event::Remove(p));
        self.props[p.0 as usize] = None;
    }
    fn show_signs_and_labels(&mut self, on: bool) {
        self.events.push(Event::Show(on));
    }
}
impl LabelDialog for Host {
    fn load_dialog(&mut self, s: (u32, u32), id: u32) -> Option<u64> {
        self.events.push(Event::Dialog(s, id));
        self.script_found.then_some(19)
    }
    fn set_field_text(&mut self, _: u64, f: u32, t: &Text) {
        assert_eq!(f, LABEL_FIELD);
        self.events.push(Event::Field(t.clone()));
    }
    fn select_all_text(&mut self, _: u64, f: u32) {
        assert_eq!(f, LABEL_FIELD);
        self.events.push(Event::SelectAll);
    }
    fn field_text(&self, _: u64, f: u32) -> Text {
        assert_eq!(f, LABEL_FIELD);
        self.modal_text.clone()
    }
    fn run_modal(&mut self, _: u64, filter: fn(DialogMessage) -> FilterAction) -> u32 {
        assert_eq!(
            filter(DialogMessage {
                kind: 0x11,
                ..Default::default()
            }),
            FilterAction::Focus(LABEL_FIELD)
        );
        self.events.push(Event::Modal);
        self.modal_result
    }
    fn focus_city_view(&mut self) {
        self.events.push(Event::Focus);
    }
    fn shutdown_dialog(&mut self, _: u64) {
        self.events.push(Event::ShutdownDialog);
    }
}
impl LabelWorld for Host {
    fn view_rotation(&self) -> u32 {
        self.rotation
    }
    fn position_to_cell(&self, p: Vec3) -> (i32, i32) {
        ((p.x / 16.0) as i32, (p.z / 16.0) as i32)
    }
    fn cell_center(&self, _: (i32, i32)) -> Vec3 {
        self.center
    }
    fn cell_has_water(&self, _: (i32, i32)) -> bool {
        self.water
    }
    fn network_tile(&self, _: (i32, i32), m: u32) -> Option<NetworkTile> {
        match m {
            0x44B => self.tile,
            0x40B => self.road_tile,
            _ => panic!("wrong mask"),
        }
    }
    fn avenue_piece_edges(&self, _: Vec3, m: u32) -> Option<Edges> {
        assert_eq!(m, 0x40);
        self.avenue
    }
    fn set_label_color(&mut self, p: PropId, c: (f32, f32, f32)) {
        self.events.push(Event::Color(p, c));
    }
    fn set_label_height(&mut self, p: PropId, h: f32) {
        self.events.push(Event::Height(p, h));
    }
    fn set_label_orientation(&mut self, p: PropId, o: u8) {
        self.events.push(Event::Orientation(p, o));
    }
}
impl SignWorld for Host {
    fn ground_altitude(&self, _: Vec3) -> f32 {
        self.ground
    }
    fn sea_level(&self) -> f32 {
        self.sea
    }
    fn bounds_corners(&self, p: PropId) -> (Vec3, Vec3) {
        self.prop(p).bounds
    }
}

pub struct Codec;
impl CharacterCodec for Codec {
    fn decode(&self, _: u32, b: &[u8]) -> DecodedCharacter {
        match b {
            [0x81] => DecodedCharacter::Pending,
            [0x81, 0x40] => DecodedCharacter::Complete(b.to_vec()),
            [0x82] => DecodedCharacter::Unsupported,
            [_] => DecodedCharacter::Complete(b.to_vec()),
            _ => DecodedCharacter::Unsupported,
        }
    }
}
