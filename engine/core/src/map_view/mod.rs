// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors
//! Bounded portable `cSC4WinMapView` algorithms (`docs/specs/map-view.md`).
//!
//! `select_with`, `draw_with`, `tick_with` and the window commands execute the
//! settled owner contract through foreign interfaces. The older intent-only
//! methods remain useful for inspecting prefixes. See README.md for the exact
//! unresolved domains; no foreign callback supplies an owner algorithm.
//!
//! Effects are generic primitive intents, not native calls or success reports.
//! Active Update clears dirty on entry and samples its clock only at the tail.
//! All state reads use ordinary shared references; no RefCell or native pointers.

mod execution;
mod exemplar;
mod highlights;
mod overlay;
mod palette;
mod panels;
mod ring_float;
mod settled;
mod source;
mod traffic;
mod window;

pub use execution::*;
pub use exemplar::*;
pub use highlights::*;
pub use overlay::*;
pub use palette::*;
pub use panels::*;
pub use ring_float::{RingContext, RingPrecision, RingRounding};
pub use settled::*;
pub use source::*;
pub use traffic::*;
pub use window::*;

/// Refusal of an unpublished/out-of-domain contract, not a native failure code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    SixteenBitConflict,
    NonFiniteFloat,
    FloatOutOfRange,
    BufferLength,
    GridGeometry,
    NativeAllocation,
    HelperAllocation,
    ResourceKeyOverflow,
    MissingProperty(u32),
    MalformedProperty(u32),
    DuplicateProperty(u32),
    UnknownSource(u32),
    DeveloperLayer(u32),
    DeveloperLayer5QV4,
    DeveloperDebugLayout,
    RampGeneration,
    GridInterpolation,
    LotBlockingQV4,
    CrimeMarkersQV4,
    TrafficQV2,
    FilterResetQV1,
    HighlightKind(u32),
    HighlightRadius(u32),
    HighlightComparison,
    RepeatedInit,
    RepeatedHook,
    MissingNotificationValue,
    ClockRegression,
    PanelsQV3,
    GroupButtonQV3(u32),
    LegendStringRead(usize),
    LegendLayout,
    GridType,
    OverlayGeometryQV10,
    TrafficMarkersQV4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationPriority {
    /// Activation specifies registration but no numeric priority (Â§1).
    UnspecifiedActivation,
    Explicit(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    WindowGeneratorInit,
    Subscribe(u32),
    CreateMarkedCellView {
        native_bytes: u32,
    },
    InitMarkedCellView,
    IgnoreDepthBuffer,
    UseWaterAltitude,
    RegisterAnimation(AnimationPriority),
    UnregisterAnimation,
    /// Â§3.1 gives the group order relative to lookup/reset, not an order
    /// between individual subscription releases inside this group.
    DropUpdateSubscriptions {
        subscription: Option<UpdateSubscription>,
        animation_registered: bool,
    },
    ShutdownCustomView(u64),
    ReleaseCustomView(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Initialization {
    pub effects: Vec<Effect>,
    /// Native marked-cell allocation must produce a valid object (§1).
    /// The intent-only plan does not imply that an object was created.
    pub boundary: Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionStep {
    DropUpdateSubscriptions,
    ReadExemplar(ExemplarKey),
    StoreFields,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionOutcome {
    Inactive,
    CurrentView,
    Reset,
    /// The only specified native failure here: the exemplar was not found.
    MissingExemplar,
    NeedsExemplar(ExemplarKey),
    Unsupported(Unsupported),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionProgress {
    pub steps: Vec<SelectionStep>,
    pub effects: Vec<Effect>,
    pub outcome: SelectionOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateOutcome {
    Inactive,
    /// Â§4.1's map-clear intent, without inventing bitmap dimensions/clear colour.
    ClearMapIntent,
    /// No drawing from this source. Not a bitmap-clear or redraw acknowledgement.
    DrawNothing,
    Unsupported(Unsupported),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickProgress {
    pub polled_highlights: bool,
    pub highlight_boundary: Option<Unsupported>,
    pub interval_ms: u64,
    /// This concerns dirtiness already established by specified messages or
    /// source76. A false value cannot exclude an unknown highlight change.
    pub redraw_due_for_known_dirty: bool,
    pub update: Option<UpdateOutcome>,
}

/// Owned portable state. Native constructor defaults are unpublished (QV9).
/// A new portable instance is inactive, current view unknown, no resource,
/// source zero, no known dirty cause, and no native registration presumed.
/// This initialization is not a claim about native constructor values.
#[derive(Clone, Debug)]
pub struct MapView {
    active: bool,
    current_view: Option<u32>,
    requested_view: Option<u32>,
    source: u32,
    developer_debug: bool,
    config: Option<SelectionFields>,
    resource: Option<RawExemplar>,
    dirty: bool,
    last_redraw_ms: u64,
    update_subscription: Option<UpdateSubscription>,
    animation_registered: bool,
    init_planned: bool,
    highlights: Option<HighlightCollection>,
    overlay: OverlayState,
    palettes: Option<OwnedPalettes>,
    custom_view: Option<u64>,
    mode_button: Option<u32>,
    two_way_toggle: Option<bool>,
    overlay_enabled: bool,
    retained_highlights: Vec<HighlightRecord>,
    highlight_style: HighlightStyle,
    hooked_messages: Vec<u32>,
    travel_types: Vec<u8>,
    factors: ScaleFactors,
    memories: ViewMemories,
    saved_view: u32,
    queued_views: Vec<u32>,
    map_enabled: bool,
    legend_content: bool,
}

impl MapView {
    /// Explicit raw clock and notification baselines, not native defaults.
    /// No accepted parameter is a completed MapView/selection/highlight result.
    pub fn new(last_redraw_ms: u64, _legacy_notification_value: u32) -> Self {
        Self {
            active: false,
            current_view: Some(0),
            requested_view: None,
            source: 0,
            developer_debug: false,
            config: None,
            resource: None,
            dirty: false,
            last_redraw_ms: u64::from(last_redraw_ms as u32),
            update_subscription: None,
            animation_registered: false,
            init_planned: false,
            highlights: None,
            overlay: OverlayState::default(),
            palettes: None,
            custom_view: None,
            mode_button: None,
            two_way_toggle: Some(false),
            overlay_enabled: true,
            retained_highlights: Vec::new(),
            highlight_style: HighlightStyle::default(),
            hooked_messages: Vec::new(),
            travel_types: Vec::new(),
            factors: ScaleFactors::default(),
            memories: ViewMemories {
                desirability: 99,
                traffic: 0x4c,
                mode: 0,
            },
            saved_view: 0,
            queued_views: Vec::new(),
            map_enabled: false,
            legend_content: false,
        }
    }

    pub fn from_clock(renderer: &mut impl Renderer) -> Self {
        Self::new(u64::from(renderer.clock_ms()), 0)
    }

    /// Ordered Â§1 primitive intents. No native pointer/lifetime/allocation
    /// is represented, and no invented Init return value is provided.
    pub fn init(&mut self) -> Result<Initialization, Unsupported> {
        if self.init_planned {
            return Ok(Initialization {
                effects: vec![Effect::WindowGeneratorInit],
                boundary: Unsupported::NativeAllocation,
            });
        }
        self.init_planned = true;
        Ok(Initialization {
            effects: vec![
                Effect::WindowGeneratorInit,
                Effect::Subscribe(ACTIVATE),
                Effect::Subscribe(DEACTIVATE),
                Effect::Subscribe(VALUE_CHANGED),
                Effect::CreateMarkedCellView { native_bytes: 0x50 },
                Effect::InitMarkedCellView,
                Effect::IgnoreDepthBuffer,
                Effect::UseWaterAltitude,
            ],
            boundary: Unsupported::NativeAllocation,
        })
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn current_view(&self) -> Option<u32> {
        self.current_view
    }

    pub fn requested_view(&self) -> Option<u32> {
        self.requested_view
    }

    pub fn source(&self) -> u32 {
        self.source
    }

    /// Raw debug flag +0x9E0 (Â§4.1), not a host-selected developer layer.
    /// No additional dirty/notification behavior for its setter is published.
    pub fn set_developer_debug_flag(&mut self, enabled: bool) {
        self.developer_debug = enabled;
    }

    pub fn config(&self) -> Option<&SelectionFields> {
        self.config.as_ref()
    }

    pub fn raw_resource(&self) -> Option<&RawExemplar> {
        self.resource.as_ref()
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn last_redraw_ms(&self) -> u64 {
        self.last_redraw_ms
    }

    pub fn update_subscription(&self) -> Option<UpdateSubscription> {
        self.update_subscription
    }

    pub fn highlights(&self) -> Option<&HighlightCollection> {
        self.highlights.as_ref()
    }

    pub fn overlay(&mut self) -> &mut OverlayState {
        &mut self.overlay
    }

    pub fn activate(&mut self) -> Vec<Effect> {
        self.animation_registered = true;
        self.active = true;
        vec![Effect::RegisterAnimation(AnimationPriority::UnspecifiedActivation)]
    }

    pub fn deactivate(&mut self) -> Vec<Effect> {
        self.animation_registered = false;
        self.active = false;
        vec![Effect::UnregisterAnimation]
    }

    pub fn hook_update_messages(&mut self) -> Result<Vec<Effect>, Unsupported> {
        let update = subscription(self.source);
        self.animation_registered = true;
        self.update_subscription = Some(update);
        self.hooked_messages.push(update.message);
        self.hooked_messages.sort_unstable();
        let mut effects = vec![Effect::RegisterAnimation(AnimationPriority::Explicit(update.priority))];
        effects.extend(self.hooked_messages.iter().copied().map(Effect::Subscribe));
        Ok(effects)
    }

    pub fn unhook_update_messages(&mut self) -> Vec<Effect> {
        self.hooked_messages.clear();
        let effect = Effect::DropUpdateSubscriptions {
            subscription: self.update_subscription.take(),
            animation_registered: self.animation_registered,
        };
        self.animation_registered = false;
        vec![effect]
    }

    /// Supported prefix only. Raw exemplar data is an external resource, not
    /// a callback-produced configuration. Custom handles can only originate in
    /// select_with's successful foreign creation, never from this prefix.
    /// Current view is not advanced: Â§3.1 commits it only after missing stages.
    pub fn set_data_view(&mut self, view: u32, raw: Option<&RawExemplar>) -> SelectionProgress {
        let mut progress = self.begin_selection(view);
        if !matches!(progress.outcome, SelectionOutcome::NeedsExemplar(_)) {
            return progress;
        }
        let Some(raw) = raw else {
            progress.outcome = SelectionOutcome::MissingExemplar;
            return progress;
        };
        match self.store_selection(raw) {
            Ok(()) => {
                progress.steps.push(SelectionStep::StoreFields);
                progress.outcome = SelectionOutcome::Unsupported(Unsupported::RampGeneration);
            }
            Err(error) => progress.outcome = SelectionOutcome::Unsupported(error),
        }
        progress
    }

    fn begin_selection(&mut self, view: u32) -> SelectionProgress {
        let mut progress = SelectionProgress {
            steps: Vec::new(),
            effects: Vec::new(),
            outcome: SelectionOutcome::Inactive,
        };
        if !self.active {
            return progress;
        }
        if self.current_view == Some(view) {
            progress.outcome = SelectionOutcome::CurrentView;
            return progress;
        }
        progress.steps.push(SelectionStep::DropUpdateSubscriptions);
        if let Some(handle) = self.custom_view.take() {
            progress.effects.push(Effect::ShutdownCustomView(handle));
            progress.effects.push(Effect::ReleaseCustomView(handle));
        }
        progress.effects.extend(self.unhook_update_messages());
        self.requested_view = Some(view);
        if view == 0 || exemplar_key(view).is_ok_and(|key| key.instance == 0) {
            progress.outcome = SelectionOutcome::Reset;
            return progress;
        }
        let key = match exemplar_key(view) {
            Ok(key) => key,
            Err(error) => {
                progress.outcome = SelectionOutcome::Unsupported(error);
                return progress;
            }
        };
        progress.steps.push(SelectionStep::ReadExemplar(key));
        progress.outcome = SelectionOutcome::NeedsExemplar(key);
        progress
    }

    fn store_selection(&mut self, raw: &RawExemplar) -> Result<(), Unsupported> {
        let fields = raw.selection_fields()?;
        self.factors = ScaleFactors::from_scale(raw.optional_u32(SCALE)?);
        if raw.find(TRAVEL_TYPES)?.is_some() {
            self.travel_types = raw.travel_types()?;
        }
        self.source = fields.source;
        self.config = Some(fields);
        self.resource = Some(raw.clone());
        Ok(())
    }

    /// An intent or a precise unsupported stage, never a whole Update result
    /// supplied by an external host. No raw-palette parameter can bypass the
    /// owner's missing palette generation. Inactive is the only complete no-op.
    pub fn update(&self) -> UpdateOutcome {
        if !self.active {
            return UpdateOutcome::Inactive;
        }
        let layer = self.config.as_ref().map_or(1, |c| c.layer);
        let selection = match select_source(self.source, layer, self.developer_debug) {
            Ok(selection) => selection,
            Err(error) => return UpdateOutcome::Unsupported(error),
        };
        match selection {
            SourceSelection::Clear => return UpdateOutcome::ClearMapIntent,
            SourceSelection::DrawNothing => return UpdateOutcome::DrawNothing,
            SourceSelection::TrafficVolumeQV2 => {
                return UpdateOutcome::Unsupported(Unsupported::TrafficQV2);
            }
            SourceSelection::Developer {
                grid: DeveloperGrid::DebugEntriesTimes12,
                ..
            } => return UpdateOutcome::Unsupported(Unsupported::DeveloperDebugLayout),
            _ => {}
        }
        // These identify independently missing owner stages, without making
        // an assertion about native failure ordering between them.
        if let Some(config) = &self.config {
            if config.interpolate {
                return UpdateOutcome::Unsupported(Unsupported::GridInterpolation);
            }
            if config.block_by_lot.is_some() {
                return UpdateOutcome::Unsupported(Unsupported::LotBlockingQV4);
            }
        }
        if self.source == 4 {
            return UpdateOutcome::Unsupported(Unsupported::CrimeMarkersQV4);
        }
        UpdateOutcome::Unsupported(Unsupported::RampGeneration)
    }

    pub fn notify(&mut self, message: u32, value: Option<u32>) -> Result<Vec<Effect>, Unsupported> {
        match message {
            ACTIVATE => return Ok(self.activate()),
            DEACTIVATE => return Ok(self.deactivate()),
            VALUE_CHANGED => {
                // One word cannot describe this message: use notify_values(old,new).
                let _ = value;
                return Err(Unsupported::MissingNotificationValue);
            }
            _ => {}
        }
        if self.update_subscription.is_some_and(|s| s.message == message) {
            self.dirty = true;
        }
        Ok(Vec::new())
    }

    /// Second words of the message's two data records; no retained baseline.
    pub fn notify_values(&mut self, old: u32, new: u32) {
        self.dirty |= old != new;
    }

    pub fn retained_travel_types(&self) -> &[u8] {
        &self.travel_types
    }

    pub fn selection_fields(&self) -> Option<&SelectionFields> {
        self.config.as_ref()
    }

    /// Collect from raw building records only. Equality/order and therefore
    /// change-induced dirty state are deliberately not manufactured (QV7).
    pub fn update_highlights(&mut self, buildings: &[RawBuilding]) -> Result<(), Unsupported> {
        let kind = self.config.as_ref().map_or(0, |c| c.highlight_kind);
        let mut collection = collect_highlights(kind, buildings)?;
        collection
            .candidates
            .sort_by_key(|candidate| candidate.id as u32 as i32);
        self.dirty |= self
            .highlights
            .as_ref()
            .map(|old| old.candidates.iter().map(|c| c.id).collect::<Vec<_>>())
            .unwrap_or_default()
            != collection.candidates.iter().map(|c| c.id).collect::<Vec<_>>();
        self.highlights = Some(collection);
        Ok(())
    }

    /// One delivered animation tick. Highlights are polled even if the source
    /// is clean or throttled; no caller-supplied "highlights changed" answer.
    /// Only a known dirty cause establishes a due redraw while equality is
    /// unpublished. An unsupported attempt retains dirty and last-redraw time.
    pub fn tick(&mut self, now_ms: u64, buildings: &[RawBuilding]) -> Result<TickProgress, Unsupported> {
        let elapsed = u64::from((now_ms as u32).wrapping_sub(self.last_redraw_ms as u32));
        self.notify(ANIMATION_TICK, None)?;
        let highlight_boundary = self.update_highlights(buildings).err();
        let interval_ms = if self.source == 76 {
            self.dirty = true;
            250
        } else {
            1000
        };
        let due = self.dirty && elapsed >= interval_ms;
        Ok(TickProgress {
            polled_highlights: true,
            highlight_boundary,
            interval_ms,
            redraw_due_for_known_dirty: due,
            update: due.then(|| self.update()),
        })
    }
}

#[cfg(test)]
mod tests;
