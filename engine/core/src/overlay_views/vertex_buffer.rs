// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub type TerrainVertex = [u8; 32];
#[derive(Clone, Copy, Debug)]
pub struct DriverBuffer {
    pub interface: Handle,
    pub handle: Handle,
    pub format: u32,
    pub capacity: usize,
}
pub trait VertexDriver {
    /// Query 0x09CD86F9 and request format 11. Success returns an owned interface.
    fn acquire(&mut self, format: u32) -> Option<DriverBuffer>;
    fn release_interface(&mut self, interface: Handle);
    fn lock(&mut self, interface: Handle, handle: Handle, capacity: usize) -> Option<Handle>;
    fn copy_vertices(&mut self, lock: Handle, start: usize, vertices: &[TerrainVertex]);
    fn unlock(&mut self, interface: Handle, handle: Handle);
    fn draw_indexed(&mut self, interface: Handle, handle: Handle, first: usize, indices: &[u16]);
}
/// The single process helper. Keep this in shared process state, never in a view.
#[derive(Default)]
pub struct VertexBufferExtension {
    interface: Option<Handle>,
    handle: Handle,
    pub capacity: usize,
    pub width: usize,
    pub used_vertices: usize,
    pub used_indices: usize,
    pub indices: Vec<u16>,
    pub lock_pointer: Option<Handle>,
}
impl VertexBufferExtension {
    pub fn get(&mut self, enabled: bool, width: usize, height: usize, driver: &mut impl VertexDriver) -> Result<bool> {
        if !enabled {
            return Ok(false);
        }
        if self.interface.is_some() {
            return Err(Boundary::Undefined("reacquire live process vertex helper"));
        }
        let Some(buffer) = driver.acquire(11) else {
            return Ok(false);
        };
        let stride = width
            .checked_add(1)
            .ok_or(Boundary::Undefined("terrain row overflow"))?;
        if buffer.format != 11 || buffer.capacity < 2 * stride {
            driver.release_interface(buffer.interface);
            return Ok(false);
        }
        self.width = width;
        self.capacity = buffer.capacity;
        self.handle = buffer.handle;
        self.interface = Some(buffer.interface);
        self.indices
            .resize((buffer.capacity / stride).min(height) * width * 6, 0);
        Ok(true)
    }
    pub fn create_buffer(&mut self, driver: &mut impl VertexDriver) -> Result<bool> {
        let interface = self.interface.ok_or(Boundary::Undefined("released vertex interface"))?;
        self.lock_pointer = driver.lock(interface, self.handle, self.capacity);
        if self.lock_pointer.is_some() {
            self.used_vertices = 0;
            self.used_indices = 0;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub fn flush_buffer(&mut self, driver: &mut impl VertexDriver) -> Result<()> {
        if self.lock_pointer.is_none() {
            return Ok(());
        }
        let interface = self.interface.ok_or(Boundary::Undefined("flush after release"))?;
        driver.unlock(interface, self.handle);
        self.lock_pointer = None;
        if (self.used_indices as i32) > 0 {
            let count = self.used_indices / 3 * 3;
            driver.draw_indexed(interface, self.handle, 0, &self.indices[..count]);
        }
        self.used_vertices = 0;
        self.used_indices = 0;
        Ok(())
    }
    pub fn release(&mut self, driver: &mut impl VertexDriver) {
        if let Some(interface) = self.interface.take() {
            driver.release_interface(interface);
        }
        // Deliberately retain lock, pending counts and index vector (§3.1).
    }
    pub fn set_index_buffer<'a>(&mut self, indices: &'a [u16], byte_flag: u8) -> IndexSource<'a> {
        IndexSource {
            indices,
            per_row: byte_flag != 0,
        }
    }
    pub fn draw_prims_indexed(
        &mut self,
        row: usize,
        first_column: usize,
        end_column: usize,
        terrain: &[TerrainVertex],
        source: &IndexSource<'_>,
        driver: &mut impl VertexDriver,
    ) -> Result<()> {
        let n = end_column
            .checked_sub(first_column)
            .ok_or(Boundary::Undefined("negative column range"))?;
        let count = 2 * (n + 1);
        if self.used_vertices + count > self.capacity {
            self.flush_buffer(driver)?;
            self.create_buffer(driver)?;
        }
        let Some(lock) = self.lock_pointer else {
            return Ok(());
        };
        if self.interface.is_none() {
            return Err(Boundary::Undefined("submission after release"));
        }
        // Oversized native requests would overwrite the lock; never claim safe success.
        if self.used_vertices + count > self.capacity {
            return Err(Boundary::Undefined("oversized vertex submission"));
        }
        let stride = self.width + 1;
        let a = self.used_vertices;
        let b = a + n + 1;
        for (r, start) in [(row, a), (row + 1, b)] {
            let from = r * stride + first_column;
            let vertices = terrain
                .get(from..from + n + 1)
                .ok_or(Boundary::Undefined("terrain row source outside storage"))?;
            driver.copy_vertices(lock, start, vertices);
        }
        self.used_vertices += count;
        let from = source.offset_entries(self.width, row, first_column);
        let entries = source
            .indices
            .get(from..from + 6 * n)
            .ok_or(Boundary::Undefined("short borrowed index source"))?;
        if self.indices.len() < self.used_indices + entries.len() {
            self.indices.resize(self.used_indices + entries.len() + 16, 0);
        }
        for &q in entries {
            let rebased = if usize::from(q) <= self.width {
                i64::from(q) + a as i64 - first_column as i64
            } else {
                i64::from(q) + b as i64 - stride as i64 - first_column as i64
            };
            self.indices[self.used_indices] = rebased as u16;
            self.used_indices += 1;
        }
        Ok(())
    }
}
/// Borrowed, not copied or reference-counted. Rust lifetime prevents dangling source use.
pub struct IndexSource<'a> {
    pub indices: &'a [u16],
    pub per_row: bool,
}
impl IndexSource<'_> {
    pub fn offset_entries(&self, width: usize, row: usize, column: usize) -> usize {
        6 * column + if self.per_row { 6 * width * row } else { 0 }
    }
    pub fn offset_bytes(&self, width: usize, row: usize, column: usize) -> usize {
        self.offset_entries(width, row, column) * 2
    }
}
