// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable cSC4AnimationTickManager, from docs/specs/animation-tick-manager.md.
//!
//! This is a typed state machine, not a native object layout or a scheduler.
//! Valid live interfaces, successful allocation, an external owner throughout
//! callbacks/Shutdown, and non-reentrant AddRef/Release effects are preconditions.
//! DoMessage may synchronously call registration, priority, time, pause, step,
//! message accessors, and tick (recursive tick returns zero). Host panics are
//! outside the contract: no exception recovery or rollback is promised (TK3).
//!
//! [choice, TK4] Tables retain capacity 193. Slot traversal and chain prepend
//! follow the published initial table, but native growth thresholds/primes and
//! transfer order are unpublished. Large-set membership is portable; its order
//! is not a claim of native compatibility. No Rust HashSet iteration is used.
//!
//! Floating arithmetic explicitly selects nearest-even / 24-bit significand
//! operations (foundations §18). Other inherited PC/RC settings are unsupported
//! compatibility domains, not inferred native bit-exact results.

use std::collections::VecDeque;

pub const CLASS_ID: u32 = 0xA9C7_3857;
pub const INTERFACE_ID: u32 = 0x86AD_10ED;
pub const MESSAGE_TYPE: u32 = 0x86AD_10EE;
pub const ALL: u32 = u32::MAX;
pub const PHASE_MASKS: [u8; 8] = [3, 5, 3, 9, 3, 5, 3, 1];
pub const PORTABLE_CAPACITY: usize = 193;

/// Exact supplied interface-pointer bits; not an allocation address or QI identity.
/// Zero/invalid target interfaces are outside the normal-call domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interface {
    Primary,
    MessageTarget,
    Serializable,
    Service,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterfaceIdentity {
    pub base: u32,
    pub interface: Interface,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timer {
    Main,
    Auto,
}

/// Only primitive external effects. Scheduling and computed outcomes stay owned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    ConstructTimer(Timer, u32),
    Restart(Timer),
    Reset(Timer),
    FrameworkSlot(u8),
    TargetAddRef(Target),
    TargetRelease(Target),
    ConstructMessage,
    MessageAddRef,
    MessageRelease,
    UnknownAddRef(InterfaceIdentity),
    UnknownRelease(InterfaceIdentity),
    DestroyStorage,
}

/// Effects must not call back into the manager. Only do_message may reenter it.
pub trait Host {
    fn effect(&mut self, effect: Effect);
    /// Raw low-u32 unit-3 stopwatch sample, with no clamping or scheduling.
    fn elapsed(&mut self, timer: Timer) -> u32;
    fn framework_present(&mut self) -> bool;
    /// Raw allocator result; failure is outside native normal-path semantics.
    fn allocate_message(&mut self, bytes: u32) -> bool;
    /// The same manager-owned mutable message is available via message[_mut].
    fn do_message(&mut self, target: Target, manager: &mut Manager) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    FrameworkMissing,
    AllocationFailure,
    DestructiveReentry,
    ExternalOwnerRequired,
    Destroyed,
    PersistenceSchema,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub kind: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    retained_unknown: Option<InterfaceIdentity>,
}

impl Message {
    pub fn retained_unknown(&self) -> Option<InterfaceIdentity> {
        self.retained_unknown
    }
}

#[derive(Debug)]
struct IdentitySet {
    slots: Vec<Vec<Target>>,
}

impl IdentitySet {
    fn new() -> Self {
        Self {
            slots: vec![Vec::new(); PORTABLE_CAPACITY],
        }
    }

    fn slot(target: Target) -> usize {
        ((target.0 >> 2) % PORTABLE_CAPACITY as u32) as usize
    }

    fn contains(&self, target: Target) -> bool {
        self.slots[Self::slot(target)].contains(&target)
    }

    fn insert(&mut self, target: Target) {
        self.slots[Self::slot(target)].insert(0, target);
    }

    fn remove(&mut self, target: Target) -> bool {
        let chain = &mut self.slots[Self::slot(target)];
        if let Some(i) = chain.iter().position(|&entry| entry == target) {
            chain.remove(i);
            true
        } else {
            false
        }
    }

    fn ordered(&self) -> Vec<Target> {
        self.slots.iter().flatten().copied().collect()
    }

    fn clear(&mut self) {
        for chain in &mut self.slots {
            chain.clear();
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Operation {
    Add(Target, u32),
    Remove(Target),
}

#[derive(Debug)]
pub struct Manager {
    identity: u32,
    references: u32,
    destroyed: bool,
    initialized: bool,
    in_tick: bool,
    message: Option<Message>,
    min: u32,
    max: u32,
    timestamp: u32,
    frame: u32,
    rate: f32,
    depth: u32,
    step_delta: u32,
    step_pending: bool,
    auto_enabled: bool,
    auto_interval: u32,
    last: [u32; 4],
    buckets: [IdentitySet; 4],
    pending: VecDeque<Operation>,
    suppression: Vec<Target>,
}

/// Unsigned priority bands; ALL is special only in Find/Remove.
pub fn priority_bucket(priority: u32) -> usize {
    match priority {
        192.. => 0,
        128..=191 => 1,
        64..=127 => 2,
        _ => 3,
    }
}

/// Signed FILD, optional unsigned correction, literal FMUL, binary32 store.
/// Exact for this selected PC=24/RC=nearest domain, including large u32 inputs.
pub fn millisecond_bits(delta: u32) -> u32 {
    let literal = f32::from_bits(0x3A83_126F);
    let value = if delta < 0x8000_0000 {
        // FILD does not round the integer before FMUL.
        crate::x87::mul_i64_f32(i64::from(delta), literal)
    } else {
        let corrected = crate::x87::u32_to_float(delta as i32);
        (corrected * f64::from(literal)) as f32
    };
    value.to_bits()
}

impl Manager {
    pub fn new(identity: u32, host: &mut impl Host) -> Self {
        host.effect(Effect::ConstructTimer(Timer::Main, 3));
        host.effect(Effect::ConstructTimer(Timer::Auto, 3));
        Self {
            identity,
            references: 0,
            destroyed: false,
            initialized: false,
            in_tick: false,
            message: None,
            min: 2000,
            max: 500000,
            timestamp: 0,
            frame: 0,
            rate: 0.0,
            depth: 0,
            step_delta: 100000,
            step_pending: false,
            auto_enabled: false,
            auto_interval: 0,
            last: [0; 4],
            buckets: std::array::from_fn(|_| IdentitySet::new()),
            pending: VecDeque::new(),
            suppression: Vec::new(),
        }
    }

    pub fn query_interface(&mut self, iid: u32, out: &mut InterfaceIdentity) -> bool {
        let interface = match iid {
            1 | INTERFACE_ID => Interface::Primary,
            0x4522_94AA => Interface::MessageTarget,
            0xE4FD_A3D4 => Interface::Serializable,
            0x287F_B697 => Interface::Service,
            _ => return false,
        };
        *out = InterfaceIdentity {
            base: self.identity,
            interface,
        };
        self.add_ref();
        true
    }

    pub fn add_ref(&mut self) -> u32 {
        self.references = self.references.wrapping_add(1);
        self.references
    }

    /// Destruction releases storage only, not contained target/message references.
    /// Rust Drop likewise does not emit external Release or call Shutdown.
    pub fn release(&mut self, host: &mut impl Host) -> Result<u32, Unsupported> {
        if self.destroyed {
            return Err(Unsupported::Destroyed);
        }
        if self.references > 1 {
            self.references -= 1;
            return Ok(self.references);
        }
        if self.in_tick {
            return Err(Unsupported::DestructiveReentry);
        }
        self.references = 0;
        self.destroyed = true;
        for bucket in &mut self.buckets {
            bucket.clear();
        }
        self.pending.clear();
        self.suppression.clear();
        self.message = None;
        host.effect(Effect::DestroyStorage);
        Ok(0)
    }

    pub fn init(&mut self, host: &mut impl Host) -> Result<bool, Unsupported> {
        if self.destroyed {
            return Err(Unsupported::Destroyed);
        }
        if self.initialized {
            return Ok(true);
        }
        self.initialized = true;
        self.depth = 0;
        self.step_pending = false;
        host.effect(Effect::Restart(Timer::Main));
        if !host.framework_present() {
            return Err(Unsupported::FrameworkMissing);
        }
        host.effect(Effect::FrameworkSlot(3));
        host.effect(Effect::FrameworkSlot(9));
        if self.message.is_none() {
            if !host.allocate_message(0x2C) {
                return Err(Unsupported::AllocationFailure);
            }
            host.effect(Effect::ConstructMessage);
            host.effect(Effect::MessageAddRef);
            self.message = Some(Message {
                kind: MESSAGE_TYPE,
                data1: self.identity,
                data2: 0,
                data3: 0,
                retained_unknown: None,
            });
            self.set_retained_unknown(host, Some(self.primary_identity()));
        }
        Ok(true)
    }

    pub fn shutdown(&mut self, host: &mut impl Host) -> Result<bool, Unsupported> {
        if self.in_tick {
            return Err(Unsupported::DestructiveReentry);
        }
        if !self.initialized {
            return Ok(true);
        }
        let cycle = self.message.as_ref().and_then(|m| m.retained_unknown)
            .is_some_and(|id| id.base == self.identity);
        if self.references <= u32::from(cycle) {
            return Err(Unsupported::ExternalOwnerRequired);
        }
        self.set_retained_unknown(host, None);
        host.effect(Effect::MessageRelease);
        self.message = None;
        if host.framework_present() {
            host.effect(Effect::FrameworkSlot(10));
            host.effect(Effect::FrameworkSlot(4));
        }
        while let Some(operation) = self.pending.pop_front() {
            let (Operation::Add(target, _) | Operation::Remove(target)) = operation;
            host.effect(Effect::TargetRelease(target));
        }
        self.suppression.clear();
        for bucket in &mut self.buckets {
            for target in bucket.ordered() {
                host.effect(Effect::TargetRelease(target));
            }
            bucket.clear();
        }
        self.initialized = false;
        self.depth = 0;
        self.step_pending = false;
        host.effect(Effect::Reset(Timer::Main));
        Ok(true)
    }

    pub fn add(&mut self, host: &mut impl Host, target: Target, priority: u32) -> bool {
        if self.in_tick {
            self.queue(host, Operation::Add(target, priority), target);
        } else {
            let bucket = &mut self.buckets[priority_bucket(priority)];
            if !bucket.contains(target) {
                host.effect(Effect::TargetAddRef(target));
                bucket.insert(target);
            }
        }
        true
    }

    pub fn remove(&mut self, host: &mut impl Host, target: Target, priority: u32) -> bool {
        if self.in_tick {
            self.queue(host, Operation::Remove(target), target);
            return true;
        }
        let preferred = priority_bucket(priority);
        let found = if priority != ALL && self.buckets[preferred].remove(target) {
            true
        } else {
            (0..4).rev().any(|i| self.buckets[i].remove(target))
        };
        if found {
            host.effect(Effect::TargetRelease(target));
        }
        true
    }

    pub fn find(&self, target: Target, priority: u32) -> bool {
        if priority == ALL {
            (0..4).rev().any(|i| self.buckets[i].contains(target))
        } else {
            self.buckets[priority_bucket(priority)].contains(target)
        }
    }

    pub fn set_new_priority(&mut self, host: &mut impl Host, target: Target, priority: u32) {
        host.effect(Effect::TargetAddRef(target));
        self.remove(host, target, ALL);
        self.add(host, target, priority);
        host.effect(Effect::TargetRelease(target));
    }

    fn queue(&mut self, host: &mut impl Host, operation: Operation, target: Target) {
        host.effect(Effect::TargetAddRef(target));
        self.pending.push_back(operation);
        if !self.suppression.contains(&target) {
            self.suppression.push(target);
        }
    }

    pub fn tick(&mut self, host: &mut impl Host) -> u32 {
        if self.in_tick {
            return 0;
        }
        // Caller preconditions: initialized, live, retained by an external owner.
        assert!(self.initialized && !self.destroyed && self.message.is_some());
        self.frame = self.frame.wrapping_add(1);
        let phase = (self.frame & 7) as usize;
        self.in_tick = true;
        let previous = self.timestamp;
        self.timestamp = host.elapsed(Timer::Main);
        if phase == 0 {
            let difference = self.timestamp.wrapping_sub(previous).wrapping_add(3);
            let divisor = crate::x87::u32_to_float(difference as i32);
            let instantaneous = crate::x87::r24(1_000_000.0 / divisor);
            let change = crate::x87::r24(instantaneous - f64::from(self.rate));
            let weighted = crate::x87::r24(change * 0.125);
            // [choice] IEEE infinity/NaN on zero divisor, no invented guard.
            self.rate = crate::x87::r24(f64::from(self.rate) + weighted) as f32;
        }
        if self.auto_enabled && self.depth != 0
            && host.elapsed(Timer::Auto) >= self.auto_interval
        {
            self.step_pending = true;
            host.effect(Effect::Reset(Timer::Auto));
        }
        let mut result = 0;
        if self.depth == 0 || self.step_pending {
            result = self.min;
            for bucket in 0..4 {
                if PHASE_MASKS[phase] & (1 << bucket) == 0 {
                    continue;
                }
                // Copy only opaque identities, never messages or target refs.
                // Advance the cursor before delivery; all safe live mutations
                // are deferred until after this outer sweep.
                let mut cursor = self.buckets[bucket].ordered().into_iter();
                if let Some(first) = cursor.next() {
                    if self.step_pending {
                        self.timestamp = self.last[bucket].wrapping_add(self.step_delta);
                    }
                    let delta = self.timestamp.wrapping_sub(self.last[bucket])
                        .min(self.max).max(self.min);
                    let message = self.message.as_mut().expect("initialized message");
                    message.data2 = delta;
                    message.data3 = millisecond_bits(delta);
                    let mut next = Some(first);
                    while let Some(target) = next {
                        next = cursor.next();
                        if !self.suppression.contains(&target) {
                            let _ = host.do_message(target, self);
                        }
                    }
                    if result <= self.min {
                        result = delta;
                    }
                }
                self.last[bucket] = self.timestamp;
            }
            self.step_pending = false;
        }
        self.in_tick = false;
        while let Some(operation) = self.pending.pop_front() {
            let target = match operation {
                Operation::Add(target, priority) => {
                    self.add(host, target, priority);
                    target
                }
                Operation::Remove(target) => {
                    self.remove(host, target, ALL);
                    target
                }
            };
            host.effect(Effect::TargetRelease(target));
        }
        self.suppression.clear();
        result
    }

    pub fn set_bounds(&mut self, min: u32, max: u32) {
        self.min = min.max(1);
        self.max = max.max(self.min);
    }

    pub fn pause(&mut self, host: &mut impl Host, pause: bool) {
        if pause {
            if self.auto_enabled && self.depth as i32 <= 0 {
                host.effect(Effect::Restart(Timer::Auto));
            }
            self.depth = self.depth.wrapping_add(1);
        } else {
            self.depth = self.depth.wrapping_sub(1);
        }
    }

    pub fn is_paused(&self) -> bool {
        self.depth as i32 > 0
    }

    pub fn enable_auto_step(&mut self, host: &mut impl Host, enable: bool, interval: u32) {
        self.auto_interval = interval;
        if enable != self.auto_enabled {
            self.auto_enabled = enable;
            host.effect(if enable { Effect::Restart(Timer::Auto) } else { Effect::Reset(Timer::Auto) });
        }
    }

    pub fn set_step_delta(&mut self, delta: u32) {
        self.step_delta = delta;
    }

    pub fn step(&mut self, host: &mut impl Host) {
        if self.is_paused() {
            self.step_pending = true;
        } else {
            self.pause(host, true);
        }
    }

    pub fn message(&self) -> Option<&Message> { self.message.as_ref() }
    pub fn message_mut(&mut self) -> Option<&mut Message> { self.message.as_mut() }

    /// Standard-message retained-field assignment: AddRef new before Release old.
    /// Foreign identities are primitive host refs, never QI-canonicalized.
    pub fn set_retained_unknown(&mut self, host: &mut impl Host, new: Option<InterfaceIdentity>) {
        assert!(self.message.is_some(), "retained-field assignment requires a message");
        let old = self.message.as_ref().and_then(|m| m.retained_unknown);
        if let Some(identity) = new {
            if identity.base == self.identity { self.add_ref(); }
            host.effect(Effect::UnknownAddRef(identity));
        }
        if let Some(identity) = old {
            // Caller guarantees this cannot be final Release/reenter (TK3).
            if identity.base == self.identity { self.references = self.references.wrapping_sub(1); }
            host.effect(Effect::UnknownRelease(identity));
        }
        if let Some(message) = self.message.as_mut() { message.retained_unknown = new; }
    }

    pub fn primary_identity(&self) -> InterfaceIdentity {
        InterfaceIdentity { base: self.identity, interface: Interface::Primary }
    }
    pub fn timestamp(&self) -> u32 { self.timestamp }
    pub fn frame(&self) -> u32 { self.frame }
    pub fn bucket_timestamps(&self) -> [u32; 4] { self.last }
    pub fn smoothed_rate(&self) -> f32 { self.rate }
    pub fn bounds(&self) -> (u32, u32) { (self.min, self.max) }
    pub fn pause_depth_bits(&self) -> u32 { self.depth }
    pub fn pending_step(&self) -> bool { self.step_pending }
    pub fn reference_count(&self) -> u32 { self.references }
    pub fn in_tick(&self) -> bool { self.in_tick }
    pub fn is_initialized(&self) -> bool { self.initialized }
    /// Auxiliary message receiver reads type but never invokes tick.
    pub fn receive_message(&self, message: &Message) -> bool { let _ = message.kind; true }
    pub fn read(&mut self) -> Result<(), Unsupported> { Err(Unsupported::PersistenceSchema) }
    pub fn write(&self) -> Result<(), Unsupported> { Err(Unsupported::PersistenceSchema) }
}
