// SPDX-License-Identifier: GPL-3.0-or-later
//! Process backing buffers and owner copy decisions (render3d §12).
use super::{BackingLimits, BackingSwitch, Handle, Render3d};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyOperation {
    Save,
    Restore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferCopy {
    pub source: [i32; 2],
    pub size: [i32; 2],
    pub destination: [i32; 2],
}

/// Only driver/service operations; no renderer methods are delegated.
pub trait BackingServices {
    fn backing_service_present(&self) -> bool;
    fn create_backing_buffer(&mut self, kind: u32) -> Option<Handle>;
    fn free_backing_buffer(&mut self, buffer: Handle);
    fn driver_viewport_size(&mut self) -> [i32; 2];
    fn copy_backing_buffer(&mut self, operation: CopyOperation, buffer: Handle, copy: BufferCopy) -> bool;
}

/// The application supplies one shared instance for the process. Keeping this
/// separate from Render3d prevents buffers and the switch becoming city state.
#[derive(Debug, Default)]
pub struct BackingStore {
    buffers: [Option<Handle>; 2],
    enabled: bool,
}

impl BackingSwitch for BackingStore {
    fn backing_buffer_present(&self) -> bool {
        self.buffers[0].is_some()
    }
    fn backing_enabled(&self) -> bool {
        self.enabled
    }
    fn store_backing_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl BackingStore {
    pub fn buffers(&self) -> [Option<Handle>; 2] {
        self.buffers
    }

    pub fn create(&mut self, host: &mut impl BackingServices) -> bool {
        if !host.backing_service_present() {
            return false;
        }
        for (kind, buffer) in self.buffers.iter_mut().enumerate() {
            if buffer.is_none() {
                *buffer = host.create_backing_buffer(kind as u32);
            }
        }
        let [Some(first), Some(second)] = self.buffers else {
            self.destroy(host);
            return false;
        };
        let size = host.driver_viewport_size();
        let copy = BufferCopy {
            source: [0; 2],
            size,
            destination: [0; 2],
        };
        for buffer in [first, second] {
            for operation in [CopyOperation::Save, CopyOperation::Restore] {
                if !host.copy_backing_buffer(operation, buffer, copy) {
                    self.destroy(host);
                    return false;
                }
            }
        }
        true
    }

    pub fn destroy(&mut self, host: &mut impl BackingServices) -> bool {
        if !host.backing_service_present() {
            return false;
        }
        for buffer in &mut self.buffers {
            if let Some(buffer) = buffer.take() {
                host.free_backing_buffer(buffer);
            }
        }
        true
    }

    fn copy(&self, host: &mut impl BackingServices, operation: CopyOperation, copy: BufferCopy) -> bool {
        if !host.backing_service_present() {
            return false;
        }
        let [Some(first), Some(second)] = self.buffers else {
            return false;
        };
        host.copy_backing_buffer(operation, first, copy);
        host.copy_backing_buffer(operation, second, copy);
        true
    }

    pub fn restore(&self, host: &mut impl BackingServices, copy: BufferCopy) -> bool {
        self.copy(host, CopyOperation::Restore, copy)
    }
}

impl Render3d {
    pub fn has_backing_store(&self, globals: &BackingStore) -> bool {
        globals.backing_buffer_present()
    }

    pub fn save_backing_store(
        &self,
        globals: &BackingStore,
        host: &mut impl BackingServices,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> bool {
        let mut copy = BufferCopy {
            source: [x, y],
            size: [width, height],
            destination: [x, y],
        };
        if self.state.shake_offsets != [0; 2] {
            for axis in 0..2 {
                let offset = self.state.shake_offsets[axis];
                copy.source[axis] = copy.source[axis].wrapping_sub(offset);
                if copy.source[axis] < 0 {
                    copy.destination[axis] = copy.destination[axis].wrapping_add(offset);
                    copy.size[axis] = copy.size[axis].wrapping_sub(offset);
                    copy.source[axis] = 0;
                    if copy.size[axis] <= 0 {
                        return true;
                    }
                } else if copy.source[axis].wrapping_add(copy.size[axis]) > self.state.viewport_size[axis] {
                    copy.size[axis] = self.state.viewport_size[axis].wrapping_sub(copy.source[axis]);
                    if copy.size[axis] <= 0 {
                        return true;
                    }
                }
            }
        }
        if copy.size[0] <= 0 || copy.size[1] <= 0 {
            return false;
        }
        if !self.state.allows_partial_copy && copy.size != self.state.viewport_size {
            return false;
        }
        globals.copy(host, CopyOperation::Save, copy)
    }

    pub fn restore_backing_store(&self, globals: &BackingStore, host: &mut impl BackingServices) -> bool {
        let mut copy = BufferCopy {
            source: [0; 2],
            size: self.state.viewport_size,
            destination: [0; 2],
        };
        for axis in 0..2 {
            let offset = self.state.shake_offsets[axis];
            if offset >= 0 {
                copy.destination[axis] = offset;
            } else {
                copy.source[axis] = offset.wrapping_neg();
            }
            copy.size[axis] = copy.size[axis].wrapping_sub(offset.wrapping_abs());
        }
        globals.restore(host, copy)
    }

    pub fn reevaluate_backing_store_object_flags(&mut self, limits: BackingLimits) {
        if let Some(grid) = &mut self.grid {
            grid.reevaluate_backing_store_flags(self.state.draw_zoom, limits);
        }
    }
}
