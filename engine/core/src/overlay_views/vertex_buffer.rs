// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

/// The buffer owns its acquired driver reference. Drop releases only that reference.
pub trait VertexBuffer: Send + Sync {
    fn format(&self) -> u32;
    fn capacity(&self) -> usize;
    fn lock(&mut self) -> bool;
    fn write_vertices(&mut self, offset: usize, vertices: &[[u8; 32]]);
    fn unlock(&mut self);
    fn draw_triangles(&mut self, indices: &[u32]) -> bool;
}

pub trait VertexBufferDriver {
    fn create(&self, interface: u32, format: u32) -> Option<Box<dyn VertexBuffer>>;
}

pub trait TerrainRows {
    /// Raw terrain vertex bytes, never an overlay-owner result.
    fn vertex_bytes(&self, row: usize, column: usize) -> [u8; 32];
}

#[allow(non_camel_case_types)]
pub struct cSTEVertBufExtn {
    buffer: Box<dyn VertexBuffer>,
    width: usize,
    height: usize,
    used_vertices: usize,
    index_capacity: usize,
    indices: Vec<u32>,
    source: Option<Vec<[u32; 6]>>,
    locked: bool,
}

impl cSTEVertBufExtn {
    pub fn get_vertex_buffer_extension(
        driver: &dyn VertexBufferDriver,
        use_vertex_buffers: bool,
        width: usize,
        height: usize,
    ) -> Result<Option<Self>, Error> {
        if !use_vertex_buffers {
            return Ok(None);
        }
        let stride = width.checked_add(1).ok_or(Error::Overflow)?;
        let minimum = stride.checked_mul(2).ok_or(Error::Overflow)?;
        let Some(buffer) = driver.create(0x09CD86F9, 11) else {
            return Ok(None);
        };
        if buffer.format() != 11 || buffer.capacity() < minimum {
            return Ok(None);
        }
        // §3 literally says rows fitting, not rows minus one. Native formula is asked in OVV7.
        let rows = (buffer.capacity() / stride).min(height);
        let index_capacity = rows
            .checked_mul(width)
            .and_then(|n| n.checked_mul(6))
            .ok_or(Error::Overflow)?;
        Ok(Some(Self {
            buffer,
            width,
            height,
            used_vertices: 0,
            index_capacity,
            indices: Vec::new(),
            source: None,
            locked: false,
        }))
    }

    pub fn index_capacity(&self) -> usize {
        self.index_capacity
    }
    pub fn used_vertices(&self) -> usize {
        self.used_vertices
    }
    pub fn pending_indices(&self) -> &[u32] {
        &self.indices
    }
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Terrain owns the source topology. Each entry is its six absolute row-major indices.
    pub fn set_index_buffer(&mut self, source: Vec<[u32; 6]>) -> Result<(), Error> {
        if source.len() != self.width.checked_mul(self.height).ok_or(Error::Overflow)? {
            return Err(Error::InvalidInput);
        }
        self.source = Some(source);
        Ok(())
    }

    pub fn create_buffer(&mut self) -> Result<(), Error> {
        if self.locked {
            self.flush_buffer()?;
        }
        if !self.buffer.lock() {
            return Err(Error::LockFailed);
        }
        self.used_vertices = 0;
        self.indices.clear();
        self.locked = true;
        Ok(())
    }

    pub fn copy_terrain_rows(
        &self,
        terrain: &dyn TerrainRows,
        row: usize,
        column: usize,
        n: usize,
    ) -> Result<Vec<[u8; 32]>, Error> {
        if row >= self.height || column.checked_add(n).ok_or(Error::Overflow)? > self.width {
            return Err(Error::OutOfBounds);
        }
        let count = n.checked_add(1).and_then(|v| v.checked_mul(2)).ok_or(Error::Overflow)?;
        let mut bytes = Vec::with_capacity(count);
        for r in row..=row + 1 {
            for c in column..=column + n {
                bytes.push(terrain.vertex_bytes(r, c));
            }
        }
        Ok(bytes)
    }

    pub fn draw_prims_indexed_using_vert_buf(
        &mut self,
        terrain: &dyn TerrainRows,
        row: usize,
        column: usize,
        n: usize,
    ) -> Result<(), Error> {
        if n == 0 {
            return Ok(());
        }
        if row >= self.height || column.checked_add(n).ok_or(Error::Overflow)? > self.width {
            return Err(Error::OutOfBounds);
        }
        let stride = self.width + 1;
        let row_vertices = n.checked_add(1).ok_or(Error::Overflow)?;
        let vertices_needed = row_vertices.checked_mul(2).ok_or(Error::Overflow)?;
        let indices_needed = n.checked_mul(6).ok_or(Error::Overflow)?;
        if vertices_needed > self.buffer.capacity() || indices_needed > self.index_capacity {
            return Err(Error::OutOfBounds);
        }
        let source = self.source.as_ref().ok_or(Error::MissingIndexSource)?;
        let mut local = Vec::with_capacity(indices_needed);
        // Rebase each coordinate, not by subtracting a single base: copied rows are compact.
        for cell in &source[row * self.width + column..row * self.width + column + n] {
            for &index in cell {
                let r = (index as usize / stride).checked_sub(row).ok_or(Error::OutOfBounds)?;
                let c = (index as usize % stride)
                    .checked_sub(column)
                    .ok_or(Error::OutOfBounds)?;
                if r > 1 || c > n {
                    return Err(Error::OutOfBounds);
                }
                local.push(r * row_vertices + c);
            }
        }
        let bytes = self.copy_terrain_rows(terrain, row, column, n)?;
        if self.used_vertices + vertices_needed > self.buffer.capacity()
            || self.indices.len() + indices_needed > self.index_capacity
        {
            self.flush_buffer()?;
        }
        if !self.locked {
            self.create_buffer()?;
        }
        let rebased: Vec<u32> = local
            .into_iter()
            .map(|i| u32::try_from(i + self.used_vertices).map_err(|_| Error::Overflow))
            .collect::<Result<_, _>>()?;
        self.buffer.write_vertices(self.used_vertices, &bytes);
        self.indices.extend(rebased);
        self.used_vertices += vertices_needed;
        Ok(())
    }

    pub fn flush_buffer(&mut self) -> Result<(), Error> {
        if !self.locked {
            return Ok(());
        }
        self.buffer.unlock();
        self.locked = false;
        let count = self.indices.len() / 3 * 3;
        let succeeded = count == 0 || self.buffer.draw_triangles(&self.indices[..count]);
        self.indices.clear();
        self.used_vertices = 0;
        if succeeded {
            Ok(())
        } else {
            Err(Error::DrawFailed)
        }
    }

    pub fn release_vertex_buffer_extension(self) {
        // Drop releases; it never implicitly draws pending terrain (OVV7).
    }
}

impl Drop for cSTEVertBufExtn {
    fn drop(&mut self) {
        // A core safety rule: balance our lock before releasing our owned buffer.
        // The native release-while-locked contract remains OVV7, not a promotion claim.
        if self.locked {
            self.buffer.unlock();
            self.locked = false;
        }
    }
}
