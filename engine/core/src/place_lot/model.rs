// SPDX-License-Identifier: GPL-3.0-or-later
use super::{ControlHost, Pixel, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
}
impl Rect {
    pub const INVALID: Self = Self {
        x0: -1,
        z0: -1,
        x1: -1,
        z1: -1,
    };
    pub fn width(self) -> i32 {
        self.x1 - self.x0 + 1
    }
    pub fn height(self) -> i32 {
        self.z1 - self.z0 + 1
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Terrain {
    pub cell_width: f32,
    pub width: i32,
    pub height: i32,
}
#[derive(Clone, Debug)]
pub struct Configuration {
    pub size: [i32; 2],
    pub first_building: u32,
    pub water_compatible: bool,
    /// Configuration slot 20, passed unchanged to the foreign builder.
    pub construction: u32,
}
#[derive(Clone, Debug, Default)]
pub struct BuildingInfo {
    pub groups: Vec<u32>,
    pub water_production: bool,
    pub subway_station: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Building {
    pub token: u64,
    pub position: Vec3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    Monopoly(bool),
    Building(u32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    View,
    WindowManager,
    Configurations,
    Buildings,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Open {
    WindowManagerCt2,
    TollBoothCt16,
    FeedbackCt17,
    RingFinishedCt22,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Acquire(Service),
    Release(Service),
    AcquireCursor(u64),
    ReleaseCursor(u64),
    Subscribe(u32),
    Unsubscribe(u32),
    StoreLastBuilding(u32),
    StoreLastConfiguration(u32),
    GridInit,
    GridShutdown,
    GridVisible(bool),
    RegisterView(u32),
    UnregisterView(u32),
    ClearCoverageOverlay,
    RecalculateCoverage,
    CoverageCategories(u32),
    CursorText(u32, String),
    ClearCost(bool),
    ShowCost(u32),
    PreviewMarker(u32, u32),
    FeedbackState(bool, bool, Vec3, u8),
    StopEffect(u64, u32),
    ReleaseCreatedEffect(u64),
    Withdraw(i64),
    Deposit(i64),
    Aura(u32, Vec3),
    InsertConnection(Connection, i32, i32),
    PlacementEffect(&'static str, Vec3),
    Ring(Vec3, [i32; 2], u32),
    Unsupported(Open),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Connection {
    Pipe,
    Subway,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Demolition {
    pub commit: bool,
    pub rect: Rect,
    pub privilege: u32,
    pub flags: u32,
    pub option: bool,
}
/// The tool's filter is a pure owner predicate, supplied on both dry/commit runs.
pub fn demolish_accepts(occupant_type: u32) -> bool {
    !matches!(occupant_type, 0x2890_D4DE | 0x0990_B2EA)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zone {
    pub rect: Rect,
    pub kind: u32,
    pub commit: bool,
    pub skip_funds: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coverage {
    pub point: Vec3,
    pub rect: Rect,
    pub valid: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Validation {
    pub allowed: bool,
    pub coverage: Option<Coverage>,
    pub creation_code: i32,
    pub cost: i64,
    pub text: String,
}
#[derive(Clone, Copy, Debug)]
pub struct Facing {
    pub rotation: u8,
    pub coverage: Coverage,
    pub any: bool,
    pub scores: [i32; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub rect: Rect,
    pub built: bool,
}

/// Primitive foreign interfaces. Tokens are identities, not native pointers.
/// Lookups borrow; Acquire/Release and ReleaseCreatedEffect encode obligations.
/// Implementations must not call any native place-lot owner/helper method.
pub trait Host: ControlHost {
    fn effect(&mut self, effect: Effect);
    fn has_view(&self) -> bool;
    fn on_top(&self) -> bool;
    fn translate_key(&mut self, key: u32, modifiers: u32, context: u32) -> Option<u32>;
    fn configuration(&mut self, id: u32) -> Option<Configuration>;
    fn configurations(&mut self, filter: Filter) -> Vec<u32>;
    fn building_types(&mut self) -> Vec<u32>;
    fn last_building(&self) -> u32;
    fn last_configuration(&self) -> u32;
    fn pick_list(&mut self, title: &str, rows: &[(u32, String)], initial: u32) -> Option<u32>;
    fn plop_cost(&mut self, exemplar: u32) -> Option<i64>;
    fn building_info(&mut self, exemplar: u32) -> BuildingInfo;
    fn occupant_info(&mut self, token: u64) -> BuildingInfo;
    fn terrain(&self) -> Terrain;
    fn pick_terrain(&mut self, mouse: Pixel) -> Option<Vec3>;
    fn terrain_grid_hidden(&self) -> bool;
    fn localize(&mut self, group: u32, instance: u32) -> String;
    fn development_cost(&mut self, rect: Rect) -> i64;
    fn can_create(&mut self, rect: Rect, rotation: u8, configuration: u32) -> i32;
    fn demolish(&mut self, request: Demolition, filter: fn(u32) -> bool) -> (bool, i64);
    fn network_flags(&mut self, x: i32, z: i32, mask: u32) -> u32;
    /// Only Some when IsTutorial and a current task both exist.
    fn tutorial_rectangle(&self) -> Option<Rect>;
    fn total_funds(&self) -> i64;
    /// checkSlope/skipSize are the foreign PlaceZone service's defaults: Â§3.12
    /// does not give explicit values (CT23). They are not guessed by this owner.
    fn place_zone(&mut self, zone: Zone);
    fn create_lot(&mut self, rect: Rect, rotation: u8, enabled: bool, water: bool, configuration: u32) -> Option<u64>;
    fn construct_default_lot(
        &mut self,
        lot: u64,
        configuration: u32,
        exemplar: u32,
        construction: u32,
    ) -> Option<Building>;
    fn effect_running(&self, token: u64) -> bool;
}
