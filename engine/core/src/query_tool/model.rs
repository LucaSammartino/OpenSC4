// SPDX-License-Identifier: GPL-3.0-or-later
use super::{Filter, Open};
use crate::view_input_controls::{ControlHost, Pixel, Vec3};
use std::collections::BTreeMap;

pub type Token = u64;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub x: i32,
    pub z: i32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub min: Cell,
    pub max: Cell,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Kind {
    Building,
    Network,
    Flora,
    Prop,
    PowerPole,
    Landfill,
    #[default]
    Other,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Property {
    U32(u32),
    Bool(bool),
    Text(String),
    Words(Vec<u32>),
    Key(u32, u32),
    Curve(Vec<(f64, f64)>),
}
#[derive(Clone, Debug, Default)]
pub struct Occupant {
    pub kind: Kind,
    pub cell: Cell,
    pub position: Vec3,
    pub highlight: u32,
    pub network_interface: bool,
    pub pole_interface: bool,
    pub network_types: Vec<u8>,
    pub vehicle: bool,
    pub pedestrian: bool,
    pub groups: Vec<u32>,
    pub name: String,
    pub properties: BTreeMap<u32, Property>,
    pub network_piece: Option<NetworkPiece>,
    pub network_tile: Option<NetworkTile>,
}
#[derive(Clone, Debug)]
pub struct NetworkPiece {
    pub id: u32,
    pub flip: bool,
    pub rotation: i32,
    pub model_y: f64,
    pub floors: [f64; 4],
}
#[derive(Clone, Debug)]
pub struct NetworkTile {
    pub id: u32,
    pub rotation: i32,
    pub flipped: i32,
}
impl Occupant {
    pub fn bool_property(&self, p: u32) -> bool {
        matches!(self.properties.get(&p), Some(Property::Bool(true)))
    }
    pub fn u32_property(&self, p: u32) -> Option<u32> {
        match self.properties.get(&p) {
            Some(Property::U32(v)) => Some(*v),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Lot {
    pub building: Option<Token>,
    pub occupants: Vec<Token>,
    pub headlines: Vec<String>,
    pub rect: Rect,
    pub centre: Cell,
    pub condition: u32,
    pub state: u32,
    pub abandon_reason: u32,
    pub historical: bool,
    pub wealth: u32,
    pub building_type: u32,
    pub capacity: u32,
    pub fallback_capacity: u16,
    pub occupancy: f64,
    pub config_name: String,
    pub config_id: u32,
}

/// A simulator primitive, NOT a dialog filler. Values are unformatted source
/// values; all combinations, rounding, thresholds and write-back live here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Read {
    FireEfficiency,
    FireFunding,
    FireCost,
    FireDispatch,
    PoliceEfficiency,
    PoliceFunding,
    PoliceCost,
    PoliceDispatch,
    Arrests,
    Crimes,
    JailedFraction,
    JailCapacity,
    JailCost,
    AirportUsage,
    AirportCapacity,
    AirportNoise,
    AirportCost,
    SeaportUsage,
    SeaportCapacity,
    SeaportCost,
    PollutionAir,
    PollutionWater,
    GarbageEfficiency,
    IncineratorUsed,
    IncineratorCurrent,
    GarbageScaling,
    GarbageLandfilled,
    LandfillUsage,
    LandfillCapacity,
    LandfillBudget,
    SchoolPresent,
    HospitalPresent,
    SchoolGrade,
    SchoolStaffFunding,
    SchoolStaffCost,
    SchoolBusFunding,
    SchoolBusCost,
    SchoolServed,
    HospitalGrade,
    HospitalStaffFunding,
    HospitalStaffCost,
    HospitalBusFunding,
    HospitalBusCost,
    HospitalServed,
    EstimatedStaff,
    EstimatedCapacity,
    PowerEfficiency,
    PowerProduced,
    PowerUsed,
    PowerMaximum,
    PowerCost,
    PowerFunding,
    PowerCondition,
    WasteToEnergyUsed,
    WaterEfficiency,
    WaterCurrent,
    WaterUsed,
    WaterMaximum,
    WaterCost,
    WaterPolluted,
    WaterCondition,
    Powered,
    Watered,
    StationPresent,
    StationAvailable,
    StationUsage,
    StationMaximum,
    StationCost,
    TransitFunding,
    TransitLocalCost,
    TransitDepartmentFunding,
    RecyclingMaximum,
    RecyclingCurrent,
    GarbageRecycled,
    GarbageProduced,
    RecyclingCenters,
    LandValue,
    MayorRating,
    BuildingTypeWealth,
    DeveloperType,
    PipeDistressed,
    PipeCount,
    Aura,
    BudgetCost(u32),
    RatingCurve(u8, u8),
    RatingMinimum(u8),
    RatingMaximum(u8),
    TrafficCapacity1,
    TrafficCapacity2,
    TrafficVolumeByte,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Write {
    FireFunding,
    PoliceFunding,
    SchoolFunding,
    HospitalFunding,
    PowerFunding,
    Name,
    Historical,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Funding(f64, f64),
    Text(String),
    Bool(bool),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    Occupant(Token),
    Cell(Cell),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Acquire(Token),
    Release(Token),
    AcquireCursor(u64),
    ReleaseCursor(u64),
    AcquireView,
    ReleaseView,
    AcquireWindowManager,
    ReleaseWindowManager,
    Subscribe(u32),
    Unsubscribe(u32),
    BindTipCallback,
    StartTipTimer(u32),
    StopTipTimer,
    DestroyTipTimer,
    CalloutSetup(Token, [u32; 2], [i32; 4]),
    AddChild(Token),
    RemoveChild(Token),
    Hide(Token),
    Show(Token),
    MoveCallout(Token, Pixel),
    CalloutText(Token, String, String),
    CalloutImages(Token, u32, u32, f32),
    ClearCursorText(u32),
    CursorText(u32, String, String),
    Highlight(Token, u32),
    ReleaseOccupantList,
    PostAt(u32, Pixel),
    Residential59(Cell),
    ModifierHit(Token, Open),
    Unsettled(Open),
    SendTipMessage(u32),
    PostMessage(u32),
    RecalcCoverage,
    Categories(u32),
    CoverageTimer(bool, u32),
    AttachProc(Token, Subject),
    DetachProc(Token),
    PrepareControl(Token),
    ClearFilter(Token),
    PublishDialog(Option<(Token, Subject)>),
    DestroyWindow(Token),
    FocusView,
    Sound(u32, Option<Token>),
    Write(Subject, Write, Value),
    Text(Token, String),
    Meter(Token, f32),
    Slider(Token, i32),
    Toggle(Token, bool),
    SliderProperties(Token, u32, i64, Vec<u32>),
    MoneyChild(Token, u32, String),
    ResizeMeter(Token, f32),
    EndDialog(Token, u32),
    UnregisterPath(Token),
    ShutdownPath(Token),
    ClearPaths(Token),
}

/// All identities returned by world/pick/children are borrowed. The owner emits
/// Acquire/Release only for its persistent holds. create_* transfers one created
/// obligation, discharged by Release. Native adapters must use the appropriate
/// created/acquired release macros. No native query-owner method may implement a
/// host primitive. Read records are copied foreign simulator data, not closures.
pub trait World {
    // Numeric primitive preconditions: finite values, representable destination
    // integers and non-overflowing budget sums. CT19 requests the native policy
    // outside that domain; Rust casts are not claimed as game behaviour.
    fn occupant(&self, id: Token) -> Occupant;
    fn lot(&self, id: Token) -> Lot;
    fn lot_for_occupant(&self, id: Token) -> Option<Token>;
    fn lot_at(&self, cell: Cell) -> Option<Token>;
    fn position_to_cell(&self, point: Vec3) -> Cell;
    fn read(&self, read: Read, subject: Subject) -> f64;
    fn zone(&self, cell: Cell) -> u32;
    fn cell_has_water(&self, cell: Cell) -> bool;
    fn localize(&self, group: u32, instance: u32) -> String;
    fn money(&self, amount: i64) -> String;
    fn decimal(&self, amount: f64) -> String;
    /// Native int64 values must not pass through a floating-point conversion.
    fn money_value(&self, read: Read, subject: Subject) -> i64;
    fn debug_query_enabled(&self) -> bool;
}
pub trait Host: World + ControlHost {
    fn effect(&mut self, effect: Effect);
    fn has_view(&self) -> bool;
    fn terrain_query_enabled(&self) -> bool;
    fn default_point(&self) -> Vec3;
    fn pick(&mut self, pixel: Pixel, filter: Filter, predicate: fn(Filter, &Occupant) -> bool) -> Option<Token>;
    fn pick_terrain(&mut self, pixel: Pixel) -> Option<Vec3>;
    fn create_callout(&mut self) -> Option<Token>;
    fn visible(&self, window: Token) -> bool;
    fn cursor_position(&self, relative_to_view: bool) -> Pixel;
    fn day(&self) -> u32;
    fn active_dialog(&self) -> Option<(Token, Subject)>;
    fn categories(&self) -> u32;
    fn occupant_categories(&self, id: Token) -> u32;
    fn create_dialog(&mut self, script: Script) -> Option<Token>;
    fn supports_proc(&self, window: Token) -> bool;
    fn children(&self, window: Token) -> Vec<Token>;
    fn control_id(&self, window: Token) -> u32;
    /// Run the modal message pump, passing notifications back through
    /// `dialog_notification` and daily messages through QueryTool::message.
    fn modal(&mut self, window: Token, subject: Subject) -> u32;
    fn control_value(&self, window: Token) -> Value;
    fn slider_properties(&self, window: Token) -> (u32, i64, Vec<u32>);
    fn child_by_id(&self, window: Token, id: u32) -> Option<Token>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Script {
    pub resource_type: u32,
    pub group: u32,
    pub instance: u32,
    pub flags: u32,
}
impl Script {
    pub fn query(instance: u32) -> Self {
        Self {
            resource_type: 0,
            group: 0x96A0_06B0,
            instance,
            flags: 0x1000_0005,
        }
    }
}

pub(crate) fn loc(world: &impl World, instance: u32) -> String {
    world.localize(0xEA55_24EB, instance)
}
