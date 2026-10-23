// SPDX-License-Identifier: GPL-3.0-or-later
//! Byte-aged embedded simulator, written from docs/specs/game-of-life.md Â§Â§2â€“5.
//!
//! This is a portable representation, not a native constructor, COM owner or ABI.
//! `Result` reports unsupported transport/domain conditions, never native success.
//! GL1 (foreign allocation/RNG), GL2 (reentry/lifetime), GL3 (additional inline
//! operations/control reachability) remain partial. Hosts supply primitives only.
//! Callbacks borrow a read-only partial publication; mutation, destruction and
//! reentry are outside the contract. Ordinary Rust drop releases memory without
//! pretending that the parent Shutdown protocol ran.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    Arithmetic,
    Allocation,
    MissingBuffer,
    Coordinates,
    StageLength,
    RandomPrimitive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Current,
    Alternate,
}

/// Stable logical allocation identity, independent of the published role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Allocation(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Controls {
    pub suppress: u8,
    pub edit_mode: u32,
}

/// Raw identities, not owned references. Installing these is portable wiring,
/// not evidence for native standalone setters (GL3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Callbacks {
    pub live: Option<u64>,
    pub dead: Option<u64>,
    pub context: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Free(Role, Option<Allocation>),
    Null(Role),
    Dimensions(u32, u32),
    Allocate(Role, Option<Allocation>, usize),
    Store(Allocation, usize, u8),
    Read(Allocation, usize, u8),
    Swap,
    ClearLive,
    ClearDead,
    ClearContext,
}

#[derive(Clone, Copy, Debug)]
pub struct BufferView<'a> {
    pub allocation: Allocation,
    pub bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
pub struct View<'a> {
    pub width: u32,
    pub height: u32,
    pub current: Option<BufferView<'a>>,
    pub alternate: Option<BufferView<'a>>,
    pub controls: Controls,
    pub callbacks: Callbacks,
}

/// Foreign primitives and observers. No hook computes a final grid or result.
/// `allocate` supplies an individual null/non-null result; bytes are owned here.
/// Trace free(null) is a ledger entry, not a claim of native null-free tolerance.
pub trait Host {
    fn allocate(&mut self, _bytes: usize) -> bool {
        true
    }
    fn rng_construct(&mut self, _seed: u32) -> Result<(), Unsupported> {
        Err(Unsupported::RandomPrimitive)
    }
    fn rng_draw(&mut self, _bound: u32) -> Result<u32, Unsupported> {
        Err(Unsupported::RandomPrimitive)
    }
    fn trace(&mut self, _event: Event) {}
    fn callback(&mut self, _identity: u64, _x: u32, _y: u32, _context: Option<u64>, _view: View<'_>) -> u32 {
        0
    }
    fn view_clear(&mut self) {}
}

struct Buffer {
    id: Allocation,
    bytes: Vec<u8>,
}

impl Buffer {
    fn view(&self) -> BufferView<'_> {
        BufferView {
            allocation: self.id,
            bytes: &self.bytes,
        }
    }

    fn store(&mut self, index: usize, value: u8, host: &mut impl Host) {
        self.bytes[index] = value;
        host.trace(Event::Store(self.id, index, value));
    }

    fn read(&self, index: usize, host: &mut impl Host) -> u8 {
        let value = self.bytes[index];
        host.trace(Event::Read(self.id, index, value));
        value
    }
}

#[derive(Default)]
pub struct GameOfLife {
    controls: Controls,
    callbacks: Callbacks,
    width: u32,
    height: u32,
    current: Option<Buffer>,
    alternate: Option<Buffer>,
    next_allocation: u64,
}

impl GameOfLife {
    /// Portable zero-origin representation of documented parent-initialized fields.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view(&self) -> View<'_> {
        View {
            width: self.width,
            height: self.height,
            current: self.current.as_ref().map(Buffer::view),
            alternate: self.alternate.as_ref().map(Buffer::view),
            controls: self.controls,
            callbacks: self.callbacks,
        }
    }

    /// Portable boundary wiring; does not assert native callable setter APIs.
    pub fn wire(&mut self, controls: Controls, callbacks: Callbacks) {
        self.controls = controls;
        self.callbacks = callbacks;
    }

    fn length(width: u32, height: u32) -> Result<usize, Unsupported> {
        // [choice] Refuse overflow, including the specified edge-coordinate sums;
        // no native maximum or fault/success channel is inferred (GL1).
        width.checked_mul(2).ok_or(Unsupported::Arithmetic)?;
        height.checked_mul(2).ok_or(Unsupported::Arithmetic)?;
        let length = width.checked_mul(height).ok_or(Unsupported::Arithmetic)?;
        usize::try_from(length).map_err(|_| Unsupported::Arithmetic)
    }

    fn free_buffers(&mut self, host: &mut impl Host) {
        host.trace(Event::Free(Role::Current, self.current.as_ref().map(|b| b.id)));
        self.current = None;
        host.trace(Event::Null(Role::Current));
        host.trace(Event::Free(Role::Alternate, self.alternate.as_ref().map(|b| b.id)));
        self.alternate = None;
        host.trace(Event::Null(Role::Alternate));
    }

    fn allocate(&mut self, role: Role, length: usize, host: &mut impl Host) -> Result<Option<Buffer>, Unsupported> {
        if !host.allocate(length) {
            host.trace(Event::Allocate(role, None, length));
            return Ok(None);
        }
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).map_err(|_| Unsupported::Allocation)?;
        bytes.resize(length, 0);
        self.next_allocation = self.next_allocation.checked_add(1).ok_or(Unsupported::Arithmetic)?;
        let id = Allocation(self.next_allocation);
        host.trace(Event::Allocate(role, Some(id), length));
        Ok(Some(Buffer { id, bytes }))
    }

    pub fn init(&mut self, width: u32, height: u32, flag: u32, host: &mut impl Host) -> Result<(), Unsupported> {
        self.free_buffers(host);
        self.width = if width == 0 { 16 } else { width };
        self.height = if height == 0 { 16 } else { height };
        host.trace(Event::Dimensions(self.width, self.height));
        let length = Self::length(self.width, self.height)?;
        self.alternate = self.allocate(Role::Alternate, length, host)?;
        self.current = self.allocate(Role::Current, length, host)?;
        if flag as u8 != 0 {
            self.randomize(host)?;
        } else {
            self.clear_buffers(host);
        }
        self.backed()
    }

    fn backed(&self) -> Result<(), Unsupported> {
        if self.current.is_some() && self.alternate.is_some() {
            Ok(())
        } else {
            Err(Unsupported::MissingBuffer)
        }
    }

    fn clear_buffers(&mut self, host: &mut impl Host) {
        for buffer in [&mut self.alternate, &mut self.current].into_iter().flatten() {
            for index in 0..buffer.bytes.len() {
                buffer.store(index, 0, host);
            }
        }
    }

    /// Narrow inline parent key boundary, not a full input-control implementation.
    pub fn clear_key_boundary(&mut self, host: &mut impl Host) -> bool {
        self.clear_buffers(host);
        host.view_clear();
        true
    }

    pub fn randomize(&mut self, host: &mut impl Host) -> Result<(), Unsupported> {
        self.backed()?;
        host.rng_construct(u32::MAX)?;
        let p = self.alternate.as_mut().ok_or(Unsupported::MissingBuffer)?;
        let c = self.current.as_mut().ok_or(Unsupported::MissingBuffer)?;
        for y in 0..self.height {
            for x in 0..self.width {
                let index = (y * self.width + x) as usize;
                let value = host.rng_draw(2)? as u8;
                p.store(index, value, host);
                c.store(index, value, host);
            }
        }
        Ok(())
    }

    fn index(&self, x: u32, y: u32) -> Result<usize, Unsupported> {
        if x >= self.width || y >= self.height {
            return Err(Unsupported::Coordinates);
        }
        let index = y
            .checked_mul(self.width)
            .and_then(|base| base.checked_add(x))
            .ok_or(Unsupported::Arithmetic)?;
        usize::try_from(index).map_err(|_| Unsupported::Arithmetic)
    }

    /// The bool models only AL. A false result can still change current storage.
    pub fn edit(&mut self, x: u32, y: u32, host: &mut impl Host) -> Result<bool, Unsupported> {
        let index = self.index(x, y)?;
        self.backed()?;
        let c = self.current.as_mut().ok_or(Unsupported::MissingBuffer)?;
        let p = self.alternate.as_mut().ok_or(Unsupported::MissingBuffer)?;
        match self.controls.edit_mode {
            0 => {
                c.store(index, 1, host);
                if p.bytes[index] == 0 {
                    p.store(index, 2, host);
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            1 => {
                c.store(index, 0, host);
                if p.bytes[index] != 0 {
                    p.store(index, 0, host);
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    /// Portable readout of the parent's current-nonzero test, not native GetCell.
    pub fn current_nonzero(&self, x: u32, y: u32) -> Result<bool, Unsupported> {
        let index = self.index(x, y)?;
        Ok(self.current.as_ref().ok_or(Unsupported::MissingBuffer)?.bytes[index] != 0)
    }

    pub fn advance(&mut self, host: &mut impl Host) -> Result<(), Unsupported> {
        if self.controls.suppress != 0 {
            return Ok(());
        }
        self.backed()?;
        std::mem::swap(&mut self.current, &mut self.alternate);
        host.trace(Event::Swap);
        for y in 0..self.height {
            for x in 0..self.width {
                let w = self.width;
                let h = self.height;
                let interior = x > 0 && y > 0 && x < w - 1 && y < h - 1;
                let west = (x + w - 1) % w;
                let east = (x + w + 1) % w;
                let north = (y + h - 1) % h;
                let south = (y + h + 1) % h;
                let neighbors = if interior {
                    [
                        (west, south),
                        (east, south),
                        (west, north),
                        (east, north),
                        (west, y),
                        (east, y),
                        (x, north),
                        (x, south),
                    ]
                } else {
                    [
                        (x, south),
                        (x, north),
                        (east, south),
                        (east, north),
                        (west, south),
                        (west, north),
                        (east, y),
                        (west, y),
                    ]
                };
                let p = self.alternate.as_mut().ok_or(Unsupported::MissingBuffer)?;
                let mut count = 0;
                for (nx, ny) in neighbors {
                    count += u8::from(p.read((ny * w + nx) as usize, host) != 0);
                }
                let index = (y * w + x) as usize;
                let source = p.read(index, host);
                let (destination, live, age_source) = match (count, source) {
                    (3, 0) => (1, true, false),
                    (2, 0) => (0, false, false),
                    (2 | 3, 191) => (191, true, false),
                    (2 | 3, s) => (s.wrapping_add(1), true, true),
                    _ => (0, false, false),
                };
                let c = self.current.as_mut().ok_or(Unsupported::MissingBuffer)?;
                // [choice] Both writes precede callbacks; relative C/P store
                // order is unpublished. Use table order C then P pending GL4.
                c.store(index, destination, host);
                if age_source {
                    p.store(index, destination, host);
                }
                let callback = if live { self.callbacks.live } else { self.callbacks.dead };
                if let Some(identity) = callback {
                    let _ = host.callback(identity, x, y, self.callbacks.context, self.view());
                }
            }
        }
        Ok(())
    }

    /// Timing is unused; normal AL is true even when generation is suppressed.
    pub fn tick_boundary(&mut self, _timing: u32, host: &mut impl Host) -> Result<bool, Unsupported> {
        host.view_clear();
        self.advance(host)?;
        Ok(true)
    }

    /// Only the admitted initialized-parent branch is modeled. Null-free trace
    /// entries describe calls, not safety of a foreign free or repeated Shutdown.
    pub fn shutdown_boundary(&mut self, parent_initialized: bool, host: &mut impl Host) {
        if parent_initialized {
            self.callbacks.live = None;
            host.trace(Event::ClearLive);
            self.callbacks.dead = None;
            host.trace(Event::ClearDead);
            self.callbacks.context = None;
            host.trace(Event::ClearContext);
            self.free_buffers(host);
        }
    }

    /// Explicit buffer-only ledger. Ordinary Rust Drop does not invoke this.
    pub fn destruction_boundary(&mut self, host: &mut impl Host) {
        self.free_buffers(host);
    }

    /// Conditional portable injection for stage vectors, not an owner-reachable
    /// constructor or final-grid dependency. Allows height zero to test boundaries.
    pub fn from_stage(width: u32, height: u32, current: Vec<u8>, alternate: Vec<u8>) -> Result<Self, Unsupported> {
        let length = Self::length(width, height)?;
        if width == 0 || current.len() != length || alternate.len() != length {
            return Err(Unsupported::StageLength);
        }
        Ok(Self {
            width,
            height,
            current: Some(Buffer {
                id: Allocation(1),
                bytes: current,
            }),
            alternate: Some(Buffer {
                id: Allocation(2),
                bytes: alternate,
            }),
            next_allocation: 2,
            ..Self::new()
        })
    }
}

#[cfg(test)]
mod tests;
