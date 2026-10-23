// SPDX-License-Identifier: GPL-3.0-or-later
//! SC4DrawContext, implemented solely from draw-context.md sections 1-9, 13.
//!
//! Driver/camera/light/instance interfaces are external calls only. Packing,
//! state transitions, composition, material decisions and mesh ordering live
//! here. No RefCell or borrowed game reference is owned by this implementation.
//! DC3-DC7 document missing contracts; this module is not promoted to a plugin.

mod boxes;
mod cache;
mod numeric;
mod draw;
mod state;
mod textures;
mod transforms;
mod types;

pub use state::{PackInput, PackedState, BOX_STATE, DEFAULT_STATE, HIGHLIGHT_STATE};
pub use types::*;

use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractGap {
    /// DC3: no Begin/End bodies, signatures or lifecycle contract are published.
    BeginEnd,
    /// DC4: section 1.1 elides bytes of both owned combiner records.
    DefaultCombiner,
    HighlightCombiner,
    /// DC5: exact default stage-record values are not published.
    StageDefaults,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    NotInitialized,
    MissingCamera,
    MissingLighting,
    MissingView,
    InvalidStage,
    InvalidValue,
    AddressOverflow,
    Allocation,
    /// Outside the declared portable numeric/control qualification.
    UnsupportedNumeric,
    Provider(u32),
    Gap(ContractGap),
}

/// Caller qualification, never an inference about the game's ambient x87 word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericControl {
    Unqualified,
    NearestEven64Gradual,
}

pub const HIGHLIGHT_COLORS: [Rgba; 11] = [
    [1.0, 1.0, 1.0, 1.0],
    [0.0, 1.0, 0.0, 1.0],
    [1.0, 0.0, 0.0, 1.0],
    [0.2, 0.2, 0.4, 0.6],
    [0.66, 0.0, 0.0, 0.7],
    [0.66, 0.0, 0.0, 1.0],
    [0.5, 0.0, 0.0, 1.0],
    [0.0, 0.37, 0.0, 1.0],
    [0.5, 0.0, 0.0, 1.0],
    [0.0, 0.7, 0.2, 0.3],
    [0.7, 0.0, 0.2, 0.3],
];

/// Renderer-populated scalar fields, section 1. Opaque values stay opaque.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewState {
    pub ints_08_0c: [i32; 2],
    pub zoom: f32,
    pub zoom_int: i32,
    pub rotation: i32,
    pub value_1c: u32,
    pub values_28_2c: [u32; 2],
    /// Unused borrowed renderer/camera-control identities, section 1 +0x34/+0x40.
    /// DC3: no reference acquisition/release semantics are published for these.
    pub renderer_link: usize,
    pub camera_control_link: usize,
    pub zero_fields_48_4c: [u32; 2],
}

pub struct DrawContext<D: Driver> {
    driver: D,
    initialized: bool,
    properties: Properties,
    view_state: ViewState,
    camera: Option<Arc<dyn Camera>>,
    lighting: Option<Arc<dyn Lighting>>,
    view: Option<Arc<dyn View>>,
    driver_info: DriverInfo,
    word: u32,
    default_state: PackedState,
    highlight_state: PackedState,
    highlights: [Rgba; 11],
    default_combiner: [Option<u8>; 12],
    highlight_combiner: [Option<u8>; 12],
    default_shade: Rgb,
    shade_override: bool,
    lighting_enabled: bool,
    numeric_control: NumericControl,
    depth_offset: i32,
    stages: Vec<Stage>,
    model_view_valid: bool,
    model_counter: u8,
    composed_counter: u8,
    model_transform: Transform,
    composed: Transform,
    matrix: Matrix,
    stack: Vec<PackedState>,
    index_base: usize,
}

impl<D: Driver> DrawContext<D> {
    pub fn new(driver: D, properties: Properties) -> Self {
        Self {
            driver,
            initialized: false,
            properties,
            view_state: ViewState::default(),
            camera: None,
            lighting: None,
            view: None,
            driver_info: DriverInfo {
                stage_count: 0,
                bytes: [0; 2],
            },
            word: 0,
            default_state: PackedState { value: 0, mask: 0 },
            highlight_state: PackedState { value: 0, mask: 0 },
            highlights: [[0.0; 4]; 11],
            default_combiner: CONSTRUCTOR_COMBINER_BYTES,
            highlight_combiner: CONSTRUCTOR_COMBINER_BYTES,
            default_shade: [1.0; 3],
            shade_override: false,
            lighting_enabled: false,
            numeric_control: NumericControl::Unqualified,
            depth_offset: 0,
            stages: Vec::new(),
            model_view_valid: false,
            model_counter: 0,
            composed_counter: 0,
            model_transform: Transform::ZERO,
            composed: Transform::ZERO,
            matrix: [0.0; 16],
            stack: Vec::new(),
            index_base: 0,
        }
    }

    pub fn init(&mut self) -> Result<(), Error> {
        if self.initialized {
            return Ok(());
        }
        let info = self.driver.info()?;
        let mut stages = Vec::new();
        stages
            .try_reserve_exact(info.stage_count.max(2))
            .map_err(|_| Error::Allocation)?;
        stages.resize_with(info.stage_count.max(2), Stage::unpublished_default);
        self.driver_info = info;
        self.stages = stages;
        self.highlights = HIGHLIGHT_COLORS;
        self.default_state = DEFAULT_STATE;
        self.highlight_state = HIGHLIGHT_STATE;
        self.default_combiner = DEFAULT_COMBINER_BYTES;
        self.highlight_combiner = HIGHLIGHT_COMBINER_BYTES;
        self.initialized = true;
        Ok(())
    }

    pub fn begin(&mut self) -> Result<(), Error> {
        Err(Error::Gap(ContractGap::BeginEnd))
    }

    pub fn end(&mut self) -> Result<(), Error> {
        Err(Error::Gap(ContractGap::BeginEnd))
    }

    pub fn driver(&self) -> &D {
        &self.driver
    }
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    pub fn state_word(&self) -> u32 {
        self.word
    }
    pub fn driver_info(&self) -> DriverInfo {
        self.driver_info
    }
    pub fn stages(&self) -> &[Stage] {
        &self.stages
    }
    pub fn highlights(&self) -> &[Rgba; 11] {
        &self.highlights
    }
    pub fn default_packed_state(&self) -> PackedState {
        self.default_state
    }
    pub fn highlight_packed_state(&self) -> PackedState {
        self.highlight_state
    }
    pub fn default_combiner_bytes(&self) -> &[Option<u8>; 12] {
        &self.default_combiner
    }
    pub fn highlight_combiner_bytes(&self) -> &[Option<u8>; 12] {
        &self.highlight_combiner
    }
    pub fn model_view_valid(&self) -> bool {
        self.model_view_valid
    }
    pub fn composed_transform(&self) -> Transform {
        self.composed
    }
    pub fn model_transform(&self) -> Transform {
        self.model_transform
    }
    pub fn model_view_matrix(&self) -> Matrix {
        self.matrix
    }
    pub fn shade_override(&self) -> bool {
        self.shade_override
    }
    pub fn default_shade(&self) -> Rgb {
        self.default_shade
    }
    pub fn depth_offset(&self) -> i32 {
        self.depth_offset
    }
    pub fn index_base(&self) -> usize {
        self.index_base
    }
    pub fn view_state(&self) -> ViewState {
        self.view_state
    }
    pub fn properties(&self) -> Properties {
        self.properties
    }

    /// Renderer input population, not inferred same-owner setter bodies.
    pub fn set_view_state(&mut self, state: ViewState) {
        self.view_state = state;
    }
    pub fn set_properties(&mut self, properties: Properties) {
        self.properties = properties;
    }
    pub fn set_lighting(&mut self, lighting: Option<Arc<dyn Lighting>>) {
        self.lighting = lighting;
    }
    pub fn set_view(&mut self, view: Option<Arc<dyn View>>) {
        self.view = view;
    }
    pub fn set_default_shade(&mut self, shade: Rgb) {
        self.default_shade = shade;
    }
    pub fn set_numeric_control(&mut self, control: NumericControl) {
        self.numeric_control = control;
    }

    fn qualified_numeric(&self) -> Result<(), Error> {
        if self.numeric_control == NumericControl::NearestEven64Gradual {
            Ok(())
        } else {
            Err(Error::UnsupportedNumeric)
        }
    }

    /// Core stack storage helpers. The spec names the stack but does not publish
    /// original push/pop entry points or underflow return channels (DC3).
    pub fn push_packed_state(&mut self, state: PackedState) {
        self.stack.push(state);
    }
    pub fn pop_packed_state(&mut self) -> Option<PackedState> {
        self.stack.pop()
    }

    fn ready(&self) -> Result<(), Error> {
        if self.initialized {
            Ok(())
        } else {
            Err(Error::NotInitialized)
        }
    }
    fn emit(&mut self, command: Command) -> Result<(), Error> {
        self.driver.submit_returned(command).map(|_ignored_register| ())
    }
    fn stage(&self, stage: usize) -> Result<&Stage, Error> {
        self.ready()?;
        self.stages.get(stage).ok_or(Error::InvalidStage)
    }
    fn reaches_driver(&self, stage: usize) -> bool {
        stage == 0 || self.properties.use_second_stage
    }
    fn select(&mut self, stage: usize) -> Result<(), Error> {
        self.emit(Command::SelectStage(stage))
    }
}

#[cfg(test)]
mod tests;
