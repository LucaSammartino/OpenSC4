// SPDX-License-Identifier: GPL-3.0-or-later
//! Inventory §§1.1–1.3,5.1–5.4. Native words and addresses wrap at 32 bits.
use super::{call, zero, Allocation, Event, Fault, Host, Op, Result};

#[derive(Debug)]
pub struct Pool {
    pub block_size: u32,
    pub count: u32,
    pub base: u32,
    pub descriptors: Vec<[u32; 3]>,
    pub descriptor_storage: u32,
    pub counters: [u32; 2],
}
impl Pool {
    pub fn new(size: u32, count: u32, host: &mut impl Host) -> Result<Self> {
        let block_size = size.wrapping_add(31) & !31;
        let total = block_size.wrapping_mul(count).wrapping_add(4095) & !4095;
        let actual = (total as i32).checked_div(block_size as i32).ok_or(Fault::Division)? as u32;
        let base = host.alloc(
            actual.wrapping_mul(block_size),
            Allocation::Virtual {
                flags: 0x1000,
                protection: 0x40,
            },
        );
        // General descriptor collection growth is outside the inventory; logical ordered
        // append does not prescribe an allocator policy (GI2 in the inventory questions).
        let descriptors = (0..actual)
            .map(|i| [base.wrapping_add(i.wrapping_mul(block_size)), 0, i])
            .collect();
        Ok(Self {
            block_size,
            count: actual,
            base,
            descriptors,
            descriptor_storage: 0,
            counters: [0; 2],
        })
    }
    pub fn destroy(&mut self, host: &mut impl Host) {
        host.free(
            self.base,
            Allocation::Virtual {
                flags: 0x1000,
                protection: 0x40,
            },
        );
        if self.descriptor_storage != 0 {
            host.free(self.descriptor_storage, Allocation::Heap);
        }
        self.descriptors.clear();
        self.descriptors.shrink_to_fit();
        host.observe(Event::PoolSetDestroyed);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coefficients {
    pub q: [[u32; 2]; 64],
    pub r: [[u32; 2]; 64],
}
impl Default for Coefficients {
    fn default() -> Self {
        Self {
            q: [[0; 2]; 64],
            r: [[0; 2]; 64],
        }
    }
}
impl Coefficients {
    pub fn regenerate(&mut self) {
        for i in 0..64 {
            let q = (i as u32 * 65536) / 252;
            let r = 16384 - q;
            self.q[i] = [q | (q << 16), q];
            self.r[i] = [r | (r << 16), r | 0x4000_0000];
        }
    }
}

#[derive(Debug, Default)]
pub struct WorkingArrays {
    pub records: u32,
    pub words: u32,
}
impl WorkingArrays {
    pub fn allocate(&mut self, host: &mut impl Host) -> Result<()> {
        self.records = host.alloc(90112, Allocation::Heap);
        if self.records != 0 {
            for i in 0u32..1024 {
                host.write(self.records.wrapping_add(i * 88 + 80), &[0; 8])?;
            }
        }
        self.words = host.alloc(5376, Allocation::Heap);
        zero(host, self.words, 5376)
    }
    pub fn free(&self, host: &mut impl Host) {
        host.free(self.records, Allocation::Heap);
        host.free(self.words, Allocation::Heap);
    }
}

/// 0x008DDFB0 / 0x008DDFC0 are the primitive tail calls, kept distinct from
/// the strided-copy algorithm so endpoint and completion counts remain observable.
pub fn copy_endpoint(host: &mut impl Host, source: u32, destination: u32, bytes: u32) {
    call(host, 0, Op::CopyEndpoint, &[source, destination, bytes]);
}
pub fn completion_endpoint(host: &mut impl Host) {
    call(host, 0, Op::CompletionEndpoint, &[]);
}
pub fn strided_copy(
    host: &mut impl Host,
    source: u32,
    destination: u32,
    width: i32,
    source_pitch: u32,
    destination_pitch: u32,
    height: i32,
) {
    if width <= 0 || height <= 0 {
        return;
    }
    let width = width as u32;
    if source_pitch == width && destination_pitch == width {
        copy_endpoint(host, source, destination, width.wrapping_mul(height as u32));
    } else {
        for row in 0..height as u32 {
            copy_endpoint(
                host,
                source.wrapping_add(row.wrapping_mul(source_pitch)),
                destination.wrapping_add(row.wrapping_mul(destination_pitch)),
                width,
            );
        }
    }
    completion_endpoint(host);
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Buffer {
    pub active: u8,
    pub data: u32,
    pub kind: u32,
    pub serial: u32,
    pub origin: [i32; 2],
    pub dimensions: [i32; 2],
    pub pixel_bytes: u32,
    pub pitch: Option<u32>,
    pub version: Option<u32>,
}
#[derive(Debug, Default)]
pub struct Buffers {
    pub slots: Vec<Buffer>,
    pub capacity: u32,
    pub live: u32,
    pub storage: u32,
}
#[derive(Debug, Default)]
pub struct Serial(pub u32);
impl Serial {
    pub fn next_serial(&mut self) -> u32 {
        if self.0 == 0 {
            self.0 = 1;
        }
        let n = self.0;
        self.0 = self.0.wrapping_add(1);
        n
    }
}

impl Buffers {
    pub fn valid(&self, handle: u32) -> bool {
        handle != 0 && handle <= self.capacity && self.slots[(handle - 1) as usize].active != 0
    }
    pub fn create(&mut self, kind: u32, serial: &mut Serial, host: &mut impl Host) -> Result<u32> {
        if kind > 1 {
            return Ok(0);
        }
        if self.live >= self.capacity {
            let capacity = self.capacity.wrapping_add(16);
            let storage = host.alloc(capacity.wrapping_mul(44), Allocation::Heap);
            if storage == 0 {
                return Err(Fault::NullWrite);
            }
            self.slots.resize_with(capacity as usize, Buffer::default);
            self.capacity = capacity;
            host.free(self.storage, Allocation::Heap);
            self.storage = storage;
        }
        let Some(index) = self.slots.iter().position(|b| b.active == 0) else {
            return Ok(0);
        };
        let b = &mut self.slots[index];
        self.live = self.live.wrapping_add(1);
        b.active = 1;
        b.data = 0;
        b.kind = kind;
        b.serial = serial.next_serial();
        b.origin = [0; 2];
        b.dimensions = [0; 2];
        b.pixel_bytes = if kind == 0 { 4 } else { 2 };
        // Reuse intentionally retains the old pitch/version. Newly grown slots have None.
        Ok(index as u32 + 1)
    }
    pub fn release(&mut self, handle: u32, host: &mut impl Host) -> bool {
        self.release_via(handle, host, Self::valid)
    }
    pub fn release_via(
        &mut self,
        handle: u32,
        host: &mut impl Host,
        validity: impl FnOnce(&Self, u32) -> bool,
    ) -> bool {
        if !validity(self, handle) {
            return false;
        }
        let b = &mut self.slots[(handle - 1) as usize];
        b.active = 0;
        host.free(b.data, Allocation::Heap);
        self.live = self.live.wrapping_sub(1);
        true
    }
    pub fn clear(&mut self, host: &mut impl Host) -> bool {
        self.clear_via(host, |this, handle, host| {
            this.release(handle, host);
        });
        true
    }
    pub fn clear_via<H: Host>(&mut self, host: &mut H, mut release: impl FnMut(&mut Self, u32, &mut H)) {
        for i in 0..self.capacity {
            if self.slots[i as usize].active != 0 {
                release(self, i + 1, host);
            }
        }
        host.free(self.storage, Allocation::Heap);
        self.storage = 0;
        self.capacity = 0;
        self.slots.clear();
        self.slots.shrink_to_fit();
    }
    pub fn save(&mut self, frame: Frame, args: [i32; 7], host: &mut impl Host) -> Result<bool> {
        let [handle, rx, ry, width, height, sx, sy] = args;
        let b = self
            .slots
            .get_mut((handle as u32).wrapping_sub(1) as usize)
            .ok_or(Fault::MissingSlot)?;
        if rx < 0
            || ry < 0
            || sx < 0
            || sy < 0
            || rx.wrapping_add(width) > frame.width
            || sx.wrapping_add(width) > frame.width
            || ry.wrapping_add(height) > frame.height
            || sy.wrapping_add(height) > frame.height
        {
            return Ok(false);
        }
        if b.data != 0 && (rx.wrapping_add(width) > b.dimensions[0] || ry.wrapping_add(height) > b.dimensions[1]) {
            return Ok(false);
        }
        call(host, frame.identity, Op::Synchronize, &[]);
        if b.data == 0 {
            let pitch = (frame.width as u32).wrapping_mul(b.pixel_bytes);
            let size = pitch.wrapping_mul(frame.height as u32).wrapping_add(8);
            b.data = host.alloc(size, Allocation::Heap);
            zero(host, b.data, size)?;
            b.pitch = Some(pitch);
            b.version = Some(0);
            b.origin = [0; 2];
            b.dimensions = [frame.width, frame.height];
        }
        b.version = Some(b.version.ok_or(Fault::MissingSlot)?.wrapping_add(1));
        if b.kind > 1 {
            return Ok(false);
        }
        let plane = b.kind as usize;
        strided_copy(
            host,
            address(frame.planes[plane], sx, sy, b.pixel_bytes, frame.pitches[plane]),
            address(b.data, rx, ry, b.pixel_bytes, b.pitch.ok_or(Fault::MissingSlot)?),
            width.wrapping_mul(b.pixel_bytes as i32),
            frame.pitches[plane],
            b.pitch.ok_or(Fault::MissingSlot)?,
            height,
        );
        Ok(true)
    }
    pub fn restore(&self, frame: Frame, args: [i32; 7], host: &mut impl Host) -> Result<bool> {
        let [handle, sx, sy, width, height, dx, dy] = args;
        let b = self
            .slots
            .get((handle as u32).wrapping_sub(1) as usize)
            .ok_or(Fault::MissingSlot)?;
        let u = dx.wrapping_sub(b.origin[0]);
        let v = dy.wrapping_sub(b.origin[1]);
        if dx < 0
            || dy < 0
            || dx.wrapping_add(width) > frame.width
            || dy.wrapping_add(height) > frame.height
            || u < 0
            || v < 0
            || u.wrapping_add(width) > b.dimensions[0]
            || v.wrapping_add(height) > b.dimensions[1]
        {
            return Ok(false);
        }
        call(host, frame.identity, Op::Synchronize, &[]);
        if b.kind == 0 {
            let tag = (((dx.wrapping_sub(sx) as u32) << 18) & 0x0ffc_0000)
                .wrapping_add(((dy.wrapping_sub(sy) as u32) & 0x3ff) << 8)
                .wrapping_add(b.serial << 28)
                .wrapping_add(b.version.ok_or(Fault::MissingSlot)? & 0xff);
            let left = dx >> 4;
            let right = dx.wrapping_add(width).wrapping_sub(1) >> 4;
            let top = dy >> 4;
            let bottom = dy.wrapping_add(height).wrapping_sub(1) >> 4;
            let mut tile = |x: i32, y: i32, value: u32| {
                host.write(
                    frame
                        .tile_base
                        .wrapping_add((y as u32).wrapping_mul(frame.tile_pitch))
                        .wrapping_add((x as u32).wrapping_mul(4)),
                    &value.to_le_bytes(),
                )
            };
            for y in top..=bottom {
                for x in left..=right {
                    tile(x, y, tag)?;
                }
            }
            if dx & 15 != 0 {
                for y in top..=bottom {
                    tile(left, y, 0)?;
                }
            }
            if dx.wrapping_add(width) & 15 != 0 {
                for y in top..=bottom {
                    tile(right, y, 0)?;
                }
            }
            if dy & 15 != 0 {
                for x in left..=right {
                    tile(x, top, 0)?;
                }
            }
            if dy.wrapping_add(height) & 15 != 0 {
                for x in left..=right {
                    tile(x, bottom, 0)?;
                }
            }
        }
        if b.kind <= 1 {
            let plane = b.kind as usize;
            let pitch = b.pitch.ok_or(Fault::MissingSlot)?;
            strided_copy(
                host,
                address(b.data, sx, sy, b.pixel_bytes, pitch),
                address(frame.planes[plane], dx, dy, b.pixel_bytes, frame.pitches[plane]),
                width.wrapping_mul(b.pixel_bytes as i32),
                pitch,
                frame.pitches[plane],
                height,
            );
        }
        Ok(true)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub identity: u32,
    pub width: i32,
    pub height: i32,
    pub planes: [u32; 2],
    pub pitches: [u32; 2],
    pub tile_base: u32,
    pub tile_pitch: u32,
}
pub fn address(base: u32, x: i32, y: i32, pixel_bytes: u32, pitch: u32) -> u32 {
    base.wrapping_add((y as u32).wrapping_mul(pitch))
        .wrapping_add((x as u32).wrapping_mul(pixel_bytes))
}
