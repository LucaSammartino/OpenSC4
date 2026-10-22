// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable cSC4AnimationCoordinator, from animation-coordinator.md §§1–4.
//!
//! This owner makes the registration, visibility, zoom, power and flag decisions.
//! The host supplies foreign primitives, never a `should_animate` decision.
//! AC2's budget predicate is deliberately an explicit unresolved result.
//! AC5–AC8 track provisional geometry, lifetime, failure/order and trigger rules.
//! This is not a native object layout or a complete-owner compatibility claim.

use std::collections::BTreeSet;

#[cfg(test)]
mod tests;

pub const MODEL_CLASSES: [u32; 2] = [0xEA12_3BE1, 0xAA12_3BF9];
pub const EXCLUDE_ANIMATION_PROPERTY: u32 = 0x4A70_D491;
pub const NEEDS_POWER_PROPERTY: u32 = 0xEA53_93ED;
pub const BUDGET_PROPERTY: u32 = 0x8A5F_88E3;
pub const MESSAGE_TARGET_IID: u32 = 0x4522_94AA;
pub const OCCUPANT_IID: u32 = 0x26BB_767E;
pub const SHOWN_FLAG: u32 = 4;
pub const ANIMATING_FLAG: u32 = 0x20;
pub const REMOVE_FLAGS: u32 = 0xC0;
pub const POWER_CHANGED: u32 = 0x8A07_AE00;
pub const BUDGET_CHANGED: u32 = 0x6A5F_D3FD;
pub const OPAQUE_REFRESH: u32 = 0x231B_BF91;
pub const VIEW_TRANSLATION_CHANGED: u32 = 0xA6B7_9612;
pub const VIEW_ZOOM_ROTATION_CHANGED: u32 = 0xA6B7_9602;
pub const SUBSCRIPTIONS: [u32; 5] = [
    POWER_CHANGED,
    BUDGET_CHANGED,
    OPAQUE_REFRESH,
    VIEW_TRANSLATION_CHANGED,
    VIEW_ZOOM_ROTATION_CHANGED,
];

/// Foreign identity, not an owned COM pointer. Native reference policy is AC6.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OccupantId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub struct Occupant {
    pub model_classes: Vec<u32>,
    pub exemplar_properties: BTreeSet<u32>,
    pub flags: u32,
    pub x_metres: f64,
    pub z_metres: f64,
}

impl Occupant {
    /// §1.2: exemplar property *presence*, independent of its stored value.
    pub fn animatable(&self) -> bool {
        !self.exemplar_properties.contains(&EXCLUDE_ANIMATION_PROPERTY)
            && self.model_classes.iter().any(|c| MODEL_CLASSES.contains(c))
    }

    pub fn shown_animatable(&self) -> bool {
        self.flags & SHOWN_FLAG != 0 && self.animatable()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Block {
    pub x: usize,
    pub z: usize,
}

/// [choice, AC5] Half-open bounds. Empty and out-of-grid rectangles are allowed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlockRect {
    pub min_x: usize,
    pub min_z: usize,
    pub max_x: usize,
    pub max_z: usize,
}

impl BlockRect {
    pub fn contains(self, block: Block) -> bool {
        self.min_x <= block.x && block.x < self.max_x && self.min_z <= block.z && block.z < self.max_z
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldRect {
    pub min_x: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_z: f64,
}

impl WorldRect {
    /// §1.1: 256 m blocks. AC5: floor lower/ceil upper implements overlap;
    /// half-open edges, malformed and non-finite inputs are provisional choices.
    pub fn blocks(self, side: usize) -> Result<BlockRect, Error> {
        if ![self.min_x, self.min_z, self.max_x, self.max_z]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err(Error::InvalidGeometry);
        }
        if self.max_x <= self.min_x || self.max_z <= self.min_z {
            return Ok(BlockRect::default());
        }
        let lower = |v: f64| (v / 256.0).floor().clamp(0.0, side as f64) as usize;
        let upper = |v: f64| (v / 256.0).ceil().clamp(0.0, side as f64) as usize;
        Ok(BlockRect {
            min_x: lower(self.min_x),
            min_z: lower(self.min_z),
            max_x: upper(self.max_x),
            max_z: upper(self.max_z),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    NotInitialized,
    EmptyGrid,
    GridAllocation,
    InvalidGeometry,
    ForeignUnavailable,
    MissingMessageTarget,
    /// AC2: no budget eligibility decision is published; preserve animation state.
    UnspecifiedBudgetTest(OccupantId),
}

/// The host owns foreign interface mechanics. Each returned lease must release
/// only its own acquisition on Drop. No AddRef is inferred for borrowed IDs.
/// AC6: native subscription, service and occupant reference counts are unpublished.
pub trait Host {
    type Services;
    type Subscription;
    type Target;

    /// Tick/occupant/power/budget/lot services of §1.1, as one owned lease.
    fn acquire_services(&mut self) -> Result<Self::Services, Error>;
    fn subscribe(&mut self, message: u32) -> Result<Self::Subscription, Error>;
    fn city_cells_x(&mut self) -> Result<u32, Error>;
    fn view_zoom(&mut self) -> Result<u32, Error>;
    fn visible_world_rect(&mut self) -> Result<WorldRect, Error>;
    /// Raw global byte; its meaning remains opaque (AC1).
    fn animation_detail_byte(&mut self) -> Result<u8, Error>;
    fn occupant(&mut self, id: OccupantId) -> Result<Occupant, Error>;
    /// Power simulator slot 31, not a coordinator eligibility callback.
    fn powered(&mut self, id: OccupantId) -> Result<bool, Error>;
    /// QI for MESSAGE_TARGET_IID, returned as an owned query lease.
    fn message_target(&mut self, id: OccupantId) -> Result<Option<Self::Target>, Error>;
    fn tick_add(&mut self, target: &Self::Target) -> Result<(), Error>;
    fn tick_remove(&mut self, target: &Self::Target, flags: u32) -> Result<(), Error>;
    /// Update only flag 0x20, preserving all other occupant flags.
    fn set_animating_flag(&mut self, id: OccupantId, on: bool);
}

/// Typed message payloads. Native decoding is outside this unbound core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Message {
    /// Blocks under the changed rectangle. Its native units/edges remain AC5;
    /// callers must not pretend this is already a decoded native message.
    PowerChanged(BlockRect),
    /// Occupant obtained through IID 0x26BB767E.
    BudgetChanged(OccupantId),
    OpaqueRefresh,
    ZoomRotation {
        zoom: u32,
        old_rotation: u32,
        new_rotation: u32,
    },
    Translation,
    Other(u32),
}

/// Immutable getters and exclusively borrowed mutations: no RefCell in owner
/// state. A future native wrapper must provide synchronization and AC7 reentry.
pub struct AnimationCoordinator<H: Host> {
    // Drop subscriptions before their services and the host, including on Drop.
    subscriptions: Vec<H::Subscription>,
    services: Option<H::Services>,
    host: H,
    initialized: bool,
    zoom: u32,
    minimum_zoom: u32,
    visible: BlockRect,
    blocks: Vec<BTreeSet<OccupantId>>,
    side: usize,
    registered_count: usize,
    enabled: bool,
}

impl<H: Host> AnimationCoordinator<H> {
    pub fn new(host: H) -> Self {
        Self {
            subscriptions: Vec::new(),
            services: None,
            host,
            initialized: false,
            zoom: 4,
            minimum_zoom: 2,
            visible: BlockRect::default(),
            blocks: Vec::new(),
            side: 0,
            registered_count: 0,
            enabled: true,
        }
    }

    pub fn host(&self) -> &H {
        &self.host
    }

    /// Foreign test/adaptor access; not a coordinator decision provider.
    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }

    pub fn initialized(&self) -> bool {
        self.initialized
    }

    pub fn zoom(&self) -> u32 {
        self.zoom
    }

    pub fn minimum_zoom(&self) -> u32 {
        self.minimum_zoom
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn visible_blocks(&self) -> BlockRect {
        self.visible
    }

    pub fn blocks_per_side(&self) -> usize {
        self.side
    }

    pub fn registered_count(&self) -> usize {
        self.registered_count
    }

    pub fn block_occupant_count(&self, block: Block) -> usize {
        self.index(block).map_or(0, |i| self.blocks[i].len())
    }

    /// §1.1 once-only initialization. [choice, AC7] Failure drops newly acquired
    /// leases, leaves the owner uninitialized and permits a later retry.
    pub fn init(&mut self) -> Result<bool, Error> {
        if self.initialized {
            return Ok(false);
        }
        let services = self.host.acquire_services()?;
        let mut subscriptions = Vec::new();
        for id in SUBSCRIPTIONS {
            subscriptions.push(self.host.subscribe(id)?);
        }
        let zoom = self.host.view_zoom()?;
        let side = (self.host.city_cells_x()? / 16) as usize;
        let count = side.checked_mul(side).ok_or(Error::GridAllocation)?;
        let mut blocks = Vec::new();
        blocks.try_reserve_exact(count).map_err(|_| Error::GridAllocation)?;
        blocks.resize_with(count, BTreeSet::new);
        let visible = self.host.visible_world_rect()?.blocks(side)?;
        self.services = Some(services);
        self.subscriptions = subscriptions;
        self.zoom = zoom;
        self.side = side;
        self.blocks = blocks;
        self.visible = visible;
        self.initialized = true;
        Ok(true)
    }

    /// §1.1: unsubscribe, release services, empty grid. No invented tick removals
    /// or flag clearing on shutdown; the original ownership/teardown is AC6.
    pub fn shutdown(&mut self) {
        self.subscriptions.clear();
        self.services = None;
        self.blocks.clear();
        self.side = 0;
        self.registered_count = 0;
        self.initialized = false;
    }

    fn require_init(&self) -> Result<(), Error> {
        if self.initialized {
            Ok(())
        } else {
            Err(Error::NotInitialized)
        }
    }

    fn index(&self, block: Block) -> Option<usize> {
        (block.x < self.side && block.z < self.side).then(|| block.z * self.side + block.x)
    }

    /// §2: position / 256 m, clamped to the square city grid.
    /// [choice, AC5] floor finite inputs; zero-sized grids reject registration.
    pub fn occupant_block(&self, occupant: &Occupant) -> Result<Block, Error> {
        if self.side == 0 {
            return Err(Error::EmptyGrid);
        }
        if !occupant.x_metres.is_finite() || !occupant.z_metres.is_finite() {
            return Err(Error::InvalidGeometry);
        }
        let coordinate = |v: f64| (v / 256.0).floor().clamp(0.0, (self.side - 1) as f64) as usize;
        Ok(Block {
            x: coordinate(occupant.x_metres),
            z: coordinate(occupant.z_metres),
        })
    }

    /// §2: duplicates affect neither set cardinality nor the registered count.
    pub fn set_occupant_animation(&mut self, id: OccupantId, on: bool) -> Result<(), Error> {
        self.require_init()?;
        let occupant = self.host.occupant(id)?;
        if !occupant.animatable() {
            return Ok(());
        }
        let block = self.occupant_block(&occupant)?;
        let index = self.index(block).ok_or(Error::EmptyGrid)?;
        if on {
            if self.blocks[index].insert(id) {
                self.registered_count += 1;
            }
        } else if self.blocks[index].remove(&id) {
            self.registered_count -= 1;
        }
        // [choice, AC7] Set membership is committed before foreign callbacks.
        self.reevaluate_snapshot(id, &occupant, on, block)
    }

    /// §2 helper 0x00446290. The coordinator performs the power decision itself.
    /// AC2 has no threshold/default; do not outsource its unknown budget decision.
    pub fn may_animate(&mut self, id: OccupantId, occupant: &Occupant) -> Result<bool, Error> {
        if occupant.exemplar_properties.contains(&NEEDS_POWER_PROPERTY) && !self.host.powered(id)? {
            return Ok(false);
        }
        if occupant.exemplar_properties.contains(&BUDGET_PROPERTY) {
            return Err(Error::UnspecifiedBudgetTest(id));
        }
        Ok(true)
    }

    fn reevaluate_snapshot(
        &mut self,
        id: OccupantId,
        occupant: &Occupant,
        on: bool,
        block: Block,
    ) -> Result<(), Error> {
        let should_animate = self.enabled
            && on
            && self.zoom >= self.minimum_zoom
            && self.visible.contains(block)
            && self.may_animate(id, occupant)?;
        if should_animate != (occupant.flags & ANIMATING_FLAG != 0) {
            self.activate(id, should_animate)?;
        }
        Ok(())
    }

    /// §2 helper 0x00446150: target query, manager add/remove, then flag change.
    /// [choice, AC7] Missing target/failed manager call preserves the flag. The
    /// query lease is dropped even on failure, never releasing a borrowed target.
    fn activate(&mut self, id: OccupantId, on: bool) -> Result<(), Error> {
        let target = self.host.message_target(id)?.ok_or(Error::MissingMessageTarget)?;
        if on {
            self.host.tick_add(&target)?;
        } else {
            self.host.tick_remove(&target, REMOVE_FLAGS)?;
        }
        self.host.set_animating_flag(id, on);
        Ok(())
    }

    fn registered(&self, id: OccupantId) -> bool {
        self.blocks.iter().any(|block| block.contains(&id))
    }

    /// Budget-change target reevaluation (§3, 0x004465D0).
    /// [choice, AC8] Unregistered targets cannot become registered via a message.
    pub fn reevaluate_occupant(&mut self, id: OccupantId) -> Result<(), Error> {
        self.require_init()?;
        let occupant = self.host.occupant(id)?;
        if !occupant.animatable() {
            return Ok(());
        }
        let block = self.occupant_block(&occupant)?;
        self.reevaluate_snapshot(id, &occupant, self.registered(id), block)
    }

    /// §3, 0x00446510. [choice, AC7] Stable identity order, stop at first error.
    pub fn refresh_block(&mut self, block: Block) -> Result<(), Error> {
        self.require_init()?;
        if let Some(index) = self.index(block) {
            let occupants: Vec<_> = self.blocks[index].iter().copied().collect();
            for id in occupants {
                self.reevaluate_occupant(id)?;
            }
        }
        Ok(())
    }

    /// §3 rectangle helper 0x00445F10, after native rectangle normalization (AC5).
    pub fn refresh_rectangle(&mut self, rect: BlockRect) -> Result<(), Error> {
        self.require_init()?;
        for z in rect.min_z.min(self.side)..rect.max_z.min(self.side) {
            for x in rect.min_x.min(self.side)..rect.max_x.min(self.side) {
                self.refresh_block(Block { x, z })?;
            }
        }
        Ok(())
    }

    /// §3 helper 0x00446760; opaque message 0x231BBF91 means exactly this.
    pub fn refresh_all(&mut self) -> Result<(), Error> {
        self.refresh_rectangle(BlockRect {
            min_x: 0,
            min_z: 0,
            max_x: self.side,
            max_z: self.side,
        })
    }

    /// §1.1: sample the opaque byte only with registered occupants, >= 2 enables.
    /// AC8: the spec does not identify this helper's call sites. Keep this an
    /// explicit operation; do not assign an invented meaning to OPAQUE_REFRESH.
    pub fn refresh_animation_enabled(&mut self) -> Result<(), Error> {
        self.require_init()?;
        if self.registered_count != 0 {
            let enabled = self.host.animation_detail_byte()? >= 2;
            if self.enabled != enabled {
                self.enabled = enabled;
                self.refresh_all()?;
            }
        }
        Ok(())
    }

    /// §3 helpers 0x004469B0/0x00446D20: only leaving and entering blocks.
    fn move_visible_rectangle(&mut self, next: BlockRect) -> Result<(), Error> {
        let previous = self.visible;
        self.visible = next;
        // [choice, AC7] All leaving blocks first, then entering, row-major.
        for entering in [false, true] {
            for z in 0..self.side {
                for x in 0..self.side {
                    let block = Block { x, z };
                    let old = previous.contains(block);
                    let new = next.contains(block);
                    if old != new && new == entering {
                        self.refresh_block(block)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// §3 DoMessage. False means an unknown message, not a native return value.
    /// Native handled/error return conventions and payload decoding remain AC7/8.
    pub fn do_message(&mut self, message: Message) -> Result<bool, Error> {
        self.require_init()?;
        match message {
            Message::PowerChanged(rect) => {
                if self.registered_count != 0 {
                    self.refresh_rectangle(rect)?;
                }
            }
            Message::BudgetChanged(id) => self.reevaluate_occupant(id)?,
            Message::OpaqueRefresh => self.refresh_all()?,
            Message::ZoomRotation {
                zoom,
                old_rotation,
                new_rotation,
            } => {
                let old_zoom = self.zoom;
                self.zoom = zoom;
                let was_close = old_zoom >= self.minimum_zoom;
                let is_close = zoom >= self.minimum_zoom;
                if self.registered_count != 0
                    && (was_close != is_close || (was_close && is_close && old_rotation != new_rotation))
                {
                    self.visible = self.host.visible_world_rect()?.blocks(self.side)?;
                    self.refresh_all()?;
                }
            }
            Message::Translation => {
                let rect = self.host.visible_world_rect()?.blocks(self.side)?;
                self.move_visible_rectangle(rect)?;
            }
            Message::Other(_) => return Ok(false),
        }
        Ok(true)
    }
}
