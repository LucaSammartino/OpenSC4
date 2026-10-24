// SPDX-License-Identifier: GPL-3.0-or-later
//! Batching and vertex-buffer lifetime, renderer-driver §§8.2–8.3,14.1.
use super::{backend::*, vertex};

#[derive(Debug, Default)]
pub struct VertexBuffer {
    pub token: u32,
    pub capacity: u32,
    pub cursor: u32,
    pub mapped: u32,
    pub locked: bool,
    pub stride: u32,
    pub format: u32,
    pub flags: u32,
    pub discard: bool,
}
impl VertexBuffer {
    pub fn copy(&self, host: &mut impl Device) -> Self {
        acquire(host, self.token);
        Self {
            token: self.token,
            capacity: self.capacity,
            cursor: self.cursor,
            mapped: self.mapped,
            locked: self.locked,
            stride: self.stride,
            format: self.format,
            flags: self.flags,
            discard: self.discard,
        }
    }
    pub fn assign(&mut self, source: &Self, host: &mut impl Device) {
        if self.token != source.token {
            acquire(host, source.token);
            release(host, &mut self.token);
        }
        *self = Self {
            token: source.token,
            capacity: source.capacity,
            cursor: source.cursor,
            mapped: source.mapped,
            locked: source.locked,
            stride: source.stride,
            format: source.format,
            flags: source.flags,
            discard: source.discard,
        };
    }
    pub fn recreate(&mut self, capacity: u32, dd: u32, d3d: u32, host: &mut impl Device) -> u32 {
        self.cursor = 0;
        self.mapped = 0;
        self.locked = false;
        self.discard = false;
        release(host, &mut self.token);
        self.capacity = capacity;
        if capacity == 0 {
            return 0;
        }
        if failed(dx(host, dd, Call::RestoreAll, &[])) {
            return FAIL;
        }
        let mut out = [0];
        let hr = host.dx(
            d3d,
            Call::CreateVertexBuffer,
            &[capacity, self.stride, vertex::fvf(self.format), self.flags],
            &[],
            &mut out,
        );
        if failed(hr) {
            self.capacity = 0;
        } else {
            self.token = out[0];
        }
        hr
    }
    pub fn reserve(&mut self, count: u32, wrap: bool, dd: u32, d3d: u32, host: &mut impl Device) -> Option<(u32, u32)> {
        if self.locked || self.token == 0 || count > self.capacity {
            return None;
        }
        let discard = self.cursor.checked_add(count).map_or(true, |n| n > self.capacity);
        if discard && !wrap {
            return None;
        }
        if discard {
            self.cursor = 0;
            self.discard = true;
        }
        let mut out = [0];
        let mut hr = host.dx(
            self.token,
            Call::LockVertexBuffer,
            &[if self.discard { 0x2021 } else { 0x1021 }],
            &[],
            &mut out,
        );
        if hr == LOST {
            let capacity = self.capacity;
            if failed(self.recreate(capacity, dd, d3d, host)) {
                return None;
            }
            self.discard = true;
            hr = host.dx(self.token, Call::LockVertexBuffer, &[0x2021], &[], &mut out);
        }
        if failed(hr) {
            return None;
        }
        self.discard = false;
        if out[0] == 0 {
            self.locked = true;
            self.unlock(dd, d3d, host);
            return None;
        }
        let first = self.cursor;
        self.cursor += count;
        self.mapped = out[0];
        self.locked = true;
        Some((first, out[0].wrapping_add(first.wrapping_mul(self.stride))))
    }
    pub fn unlock(&mut self, dd: u32, d3d: u32, host: &mut impl Device) -> u32 {
        if !self.locked {
            return 0;
        }
        self.locked = false;
        self.mapped = 0;
        let hr = dx(host, self.token, Call::UnlockVertexBuffer, &[]);
        if failed(hr) {
            let cap = self.capacity;
            self.recreate(cap, dd, d3d, host);
        }
        hr
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub primitive: u32,
    pub first: u32,
    pub count: u32,
    pub indices: Option<Vec<u16>>,
}
#[derive(Debug, Default)]
pub struct PrimitiveManager {
    pub buffer: VertexBuffer,
    pub entries: Vec<Entry>,
    pub started: bool,
    pub refs: u32,
    pub scratch_used: usize,
    pub scratch_capacity: usize,
    dd: u32,
    d3d: u32,
    device: u32,
}
impl PrimitiveManager {
    pub fn destroy(&mut self, host: &mut impl Device) {
        self.stop(host);
        self.entries = Vec::new();
        self.scratch_capacity = 0;
    }
    pub fn init(&mut self, dd: u32, d3d: u32, device: u32, host: &mut impl Device) -> u32 {
        self.stop(host);
        self.dd = dd;
        self.d3d = d3d;
        self.device = device;
        self.buffer.format = 11;
        self.buffer.stride = 32;
        let mut capabilities = [0];
        host.dx(device, Call::GetCaps, &[], &[], &mut capabilities);
        self.buffer.flags = if capabilities[0] & 0x80000 != 0 {
            0x10000
        } else {
            0x10800
        };
        let hr = self.buffer.recreate(65536 / 32, dd, d3d, host);
        self.started = !failed(hr);
        hr
    }
    pub fn stop(&mut self, host: &mut impl Device) {
        self.entries.clear();
        self.scratch_used = 0;
        self.started = false;
        release(host, &mut self.buffer.token);
        self.buffer.capacity = 0;
        self.buffer.locked = false;
        self.buffer.mapped = 0;
        self.buffer.cursor = 0;
        self.buffer.discard = false;
    }
    pub fn flush(&mut self, host: &mut impl Device) -> u32 {
        self.scratch_used = 0;
        if !self.started && !self.entries.is_empty() {
            return FAIL;
        }
        let mut result = 0;
        for e in self.entries.drain(..) {
            let bytes: Vec<u8> = e
                .indices
                .as_ref()
                .map_or_else(Vec::new, |v| v.iter().flat_map(|n| n.to_le_bytes()).collect());
            let hr = host.dx(
                self.device,
                if e.indices.is_some() {
                    Call::DrawIndexedBuffer
                } else {
                    Call::DrawBuffer
                },
                &[
                    self.buffer.token,
                    e.primitive,
                    e.first,
                    e.count,
                    (bytes.len() / 2) as u32,
                ],
                &bytes,
                &mut [],
            );
            result = hr;
        }
        self.scratch_used = 0;
        result
    }
    pub fn indices(&mut self, n: usize, host: &mut impl Device) {
        if self.scratch_used + n > self.scratch_capacity {
            self.flush(host);
            self.scratch_capacity = self.scratch_capacity.max(n).max(1).next_power_of_two();
        }
        self.scratch_used += n;
    }
    #[allow(clippy::too_many_arguments)]
    pub fn queue(
        &mut self,
        format: u32,
        primitive: u32,
        stride: u32,
        minimum: u32,
        count: u32,
        source: u32,
        indices: Option<Vec<u16>>,
        host: &mut impl Device,
    ) -> Result<u32> {
        if !self.started {
            return Ok(FAIL);
        }
        if stride > 32 {
            let data: Vec<u8> = indices
                .as_ref()
                .map_or_else(Vec::new, |v| v.iter().flat_map(|i| i.to_le_bytes()).collect());
            return Ok(host.dx(
                self.device,
                if indices.is_some() {
                    Call::DrawIndexed
                } else {
                    Call::Draw
                },
                &[vertex::fvf(format), primitive, stride, minimum, count, source],
                &data,
                &mut [],
            ));
        }
        if self.entries.is_empty() || self.buffer.cursor.saturating_add(count) > self.buffer.capacity {
            self.flush(host);
        }
        let (first, destination) = match self.buffer.reserve(count, true, self.dd, self.d3d, host) {
            Some(v) => v,
            None => return Ok(0),
        };
        let f = vertex::pack(format);
        if stride % 4 != 0
            || !matches!(
                f,
                0x80000001 | 0x80000101 | 0x80004001 | 0x80008001 | 0x80004101 | 0x80008101
            )
            || (f == 0x80008101 && stride != 32)
        {
            self.buffer.unlock(self.dd, self.d3d, host);
            // §8.3: unsupported layouts still consume the reservation and queue.
        } else {
            for n in 0..count {
                let mut bytes = vec![0; vertex::stride(f) as usize];
                let read = host.read(
                    source.wrapping_add((minimum.wrapping_add(n)).wrapping_mul(stride)),
                    &mut bytes,
                );
                if let Err(error) = read {
                    self.buffer.unlock(self.dd, self.d3d, host);
                    return Err(error.into());
                }
                let mut cursor = 0;
                for (kind, offset, size) in [(0, 0, 12), (5, 12, 4), (7, 16, 8), (7, 24, 8)] {
                    let present = match (kind, offset) {
                        (7, 24) => vertex::elements(f, 7)? > 1,
                        _ => vertex::elements(f, kind)? > 0,
                    };
                    if present {
                        let write =
                            host.write(destination.wrapping_add(n * 32 + offset), &bytes[cursor..cursor + size]);
                        if let Err(error) = write {
                            self.buffer.unlock(self.dd, self.d3d, host);
                            return Err(error.into());
                        }
                        cursor += size;
                    } else if kind == 5 {
                        if let Err(error) =
                            host.write(destination.wrapping_add(n * 32 + offset), &u32::MAX.to_le_bytes())
                        {
                            self.buffer.unlock(self.dd, self.d3d, host);
                            return Err(error.into());
                        }
                    }
                }
            }
        }
        self.buffer.unlock(self.dd, self.d3d, host);
        let indices = indices.map(|v| {
            v.into_iter()
                .map(|i| i.wrapping_sub(minimum as u16))
                .collect::<Vec<_>>()
        });
        if let Some(last) = self.entries.last_mut() {
            if last.primitive == primitive
                && matches!(primitive, 1 | 2 | 4)
                && last.indices.is_some() == indices.is_some()
                && last.first + last.count == first
            {
                if let (Some(a), Some(b)) = (&mut last.indices, &indices) {
                    a.extend(b.iter().map(|i| i.wrapping_add(last.count as u16)));
                }
                last.count += count;
                return Ok(0);
            }
        }
        self.entries.push(Entry {
            primitive,
            first,
            count,
            indices,
        });
        Ok(0)
    }
}
pub fn expanded(mode: u32, input: &[u16]) -> Result<(u32, Option<Vec<u16>>)> {
    let (primitive, indices, declared) = expanded_writes(mode, input, input.len(), true)?;
    Ok((
        primitive,
        indices.map(|mut v| {
            v.truncate(declared);
            v
        }),
    ))
}
/// The complete scratch writes and the separately declared submission count.
/// `input` may include readable sentinel indices after the logical `count`.
pub fn expanded_writes(
    mode: u32,
    input: &[u16],
    count: usize,
    indexed: bool,
) -> Result<(u32, Option<Vec<u16>>, usize)> {
    let n = count;
    if mode > 7 {
        return Err(Error::InvalidInput);
    }
    if matches!(mode, 0 | 3 | 4 | 5) {
        return Ok(([4, 5, 6, 1, 2, 3, 4, 4][mode as usize], None, n as u16 as usize));
    }
    let mut out = Vec::new();
    match mode {
        1 | 2 => {
            if n < 3 {
                return Err(Error::InvalidInput);
            }
            for k in 0..n - 2 {
                let positions = if mode == 2 {
                    [0, k + 1, k + 2]
                } else if k % 2 == 0 {
                    [k, k + 1, k + 2]
                } else {
                    [k + 1, k, k + 2]
                };
                out.extend(positions.map(|i| input[i]));
            }
        }
        6 => {
            let declared = (3 * (n / 2)) as u16 as usize;
            let groups = if indexed { declared.div_ceil(6) } else { n.div_ceil(4) };
            for q in (0..groups * 4).step_by(4) {
                for i in [q, q + 1, q + 2, q, q + 2, q + 3] {
                    out.push(if indexed {
                        *input.get(i).ok_or(Error::InvalidInput)?
                    } else {
                        i as u16
                    });
                }
            }
            return Ok((4, Some(out), declared));
        }
        7 => {
            if n < 2 {
                return Err(Error::InvalidInput);
            }
            for i in (0..n - 2).step_by(2) {
                for k in [i, i + 1, i + 3, i, i + 3, i + 2] {
                    out.push(if indexed {
                        *input.get(k).ok_or(Error::InvalidInput)?
                    } else {
                        k as u16
                    });
                }
            }
        }
        _ => unreachable!(),
    }
    // Native interface truncates the generated count, not each queue allocation.
    let declared = if mode == 7 {
        (3 * n.saturating_sub(2)) as u16 as usize
    } else {
        out.len() as u16 as usize
    };
    Ok((4, Some(out), declared))
}
