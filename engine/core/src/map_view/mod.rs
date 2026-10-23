// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors
//! Bounded portable `cSC4WinMapView` algorithms (`docs/specs/map-view.md`).
//!
//! This is not a complete SetDataView/Update/UpdateHighlights implementation.
//! Nonzero selection stores coherent raw fields, then stops at unpublished
//! owner ramp generation. None stops at QV1's default-filter boundary. No
//! callback can supply these missing owner results. Supplied-palette, raw-grid,
//! highlight-collection and overlay helpers remain independent partial helpers.
//!
//! Effects are generic primitive intents, not native calls or success reports.
//! Partial updates never acknowledge a redraw, clear dirty, or move its clock.
//! All state reads use ordinary shared references; no RefCell or native pointers.

mod exemplar;
mod highlights;
mod overlay;
mod palette;
mod source;

pub use exemplar::*;
pub use highlights::*;
pub use overlay::*;
pub use palette::*;
pub use source::*;

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
    CrimeHotspot,
    RepeatedInit,
    RepeatedHook,
    MissingNotificationValue,
    ClockRegression,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationPriority {
    /// Activation specifies registration but no numeric priority (§1).
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
    /// §3.1 gives the group order relative to lookup/reset, not an order
    /// between individual subscription releases inside this group.
    DropUpdateSubscriptions {
        subscription: Option<UpdateSubscription>,
        animation_registered: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Initialization {
    pub effects: Vec<Effect>,
    /// Native allocation/initialization failure behavior has no contract.
    /// The plan does not imply that a native marked-cell view was created.
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
    /// The only specified native failure here: the exemplar was not found.
    MissingExemplar,
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
    /// §4.1's map-clear intent, without inventing bitmap dimensions/clear colour.
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
    notification_value: u32,
    update_subscription: Option<UpdateSubscription>,
    animation_registered: bool,
    init_planned: bool,
    highlights: Option<HighlightCollection>,
    overlay: OverlayState,
}

impl MapView {
    /// Explicit raw clock and notification baselines, not native defaults.
    /// No accepted parameter is a completed MapView/selection/highlight result.
    pub fn new(last_redraw_ms: u64, notification_value: u32) -> Self {
        Self {
            active: false,
            current_view: None,
            requested_view: None,
            source: 0,
            developer_debug: false,
            config: None,
            resource: None,
            dirty: false,
            last_redraw_ms,
            notification_value,
            update_subscription: None,
            animation_registered: false,
            init_planned: false,
            highlights: None,
            overlay: OverlayState::default(),
        }
    }

    /// Ordered §1 primitive intents. No native pointer/lifetime/allocation
    /// is represented, and no invented Init return value is provided.
    pub fn init(&mut self) -> Result<Initialization, Unsupported> {
        if self.init_planned {
            return Err(Unsupported::RepeatedInit);
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

    /// Raw debug flag +0x9E0 (§4.1), not a host-selected developer layer.
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
        vec![Effect::RegisterAnimation(
            AnimationPriority::UnspecifiedActivation,
        )]
    }

    pub fn deactivate(&mut self) -> Vec<Effect> {
        self.animation_registered = false;
        self.active = false;
        vec![Effect::UnregisterAnimation]
    }

    pub fn hook_update_messages(&mut self) -> Result<Vec<Effect>, Unsupported> {
        // QV9: duplicate registration/ref-count semantics are unpublished.
        if self.update_subscription.is_some() {
            return Err(Unsupported::RepeatedHook);
        }
        let update = subscription(self.source);
        self.animation_registered = true;
        self.update_subscription = Some(update);
        Ok(vec![
            Effect::RegisterAnimation(AnimationPriority::Explicit(update.priority)),
            Effect::Subscribe(update.message),
        ])
    }

    pub fn unhook_update_messages(&mut self) -> Vec<Effect> {
        let effect = Effect::DropUpdateSubscriptions {
            subscription: self.update_subscription.take(),
            animation_registered: self.animation_registered,
        };
        self.animation_registered = false;
        vec![effect]
    }

    /// Supported prefix only. Raw exemplar data is an external resource, not
    /// a callback-produced configuration. No custom view is ever created by
    /// this prefix, so no unowned pointer shutdown/release can be fabricated.
    /// Current view is not advanced: §3.1 commits it only after missing stages.
    pub fn set_data_view(&mut self, view: u32, raw: Option<&RawExemplar>) -> SelectionProgress {
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
        progress.effects = self.unhook_update_messages();
        self.requested_view = Some(view);
        if view == 0 {
            // [literal, QV1/QV9] The default occupant filter is the first
            // unknown none-stage decision. Do not skip to later resets,
            // clearing state/highlights, Update or a current-view commit.
            progress.outcome = SelectionOutcome::Unsupported(Unsupported::FilterResetQV1);
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
        let Some(raw) = raw else {
            progress.outcome = SelectionOutcome::MissingExemplar;
            return progress;
        };
        match raw.selection_fields() {
            Ok(fields) => {
                // Unsupported malformed fields do not define a native partial
                // write order. Only the complete coherent supported prefix
                // is stored (QV6); remaining properties stay raw/uninterpreted.
                self.source = fields.source;
                self.config = Some(fields);
                self.resource = Some(raw.clone());
                progress.steps.push(SelectionStep::StoreFields);
                progress.outcome = SelectionOutcome::Unsupported(Unsupported::RampGeneration);
            }
            Err(error) => progress.outcome = SelectionOutcome::Unsupported(error),
        }
        progress
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
                let value = value.ok_or(Unsupported::MissingNotificationValue)?;
                if self.notification_value != value {
                    self.notification_value = value;
                    self.dirty = true;
                }
            }
            _ => {}
        }
        if self
            .update_subscription
            .is_some_and(|s| s.message == message)
        {
            self.dirty = true;
        }
        Ok(Vec::new())
    }

    /// Collect from raw building records only. Equality/order and therefore
    /// change-induced dirty state are deliberately not manufactured (QV7).
    pub fn update_highlights(&mut self, buildings: &[RawBuilding]) -> Result<(), Unsupported> {
        let kind = self.config.as_ref().map_or(0, |c| c.highlight_kind);
        self.highlights = Some(collect_highlights(kind, buildings)?);
        Err(Unsupported::HighlightComparison)
    }

    /// One delivered animation tick. Highlights are polled even if the source
    /// is clean or throttled; no caller-supplied "highlights changed" answer.
    /// Only a known dirty cause establishes a due redraw while equality is
    /// unpublished. An unsupported attempt retains dirty and last-redraw time.
    pub fn tick(
        &mut self,
        now_ms: u64,
        buildings: &[RawBuilding],
    ) -> Result<TickProgress, Unsupported> {
        // [literal bounded clock, QV9] No guessed wrapping/subtraction width.
        let elapsed = now_ms
            .checked_sub(self.last_redraw_ms)
            .ok_or(Unsupported::ClockRegression)?;
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
