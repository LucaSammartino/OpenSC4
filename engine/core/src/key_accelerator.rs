// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable cRZWinKeyAccelerator table and published live-object behavior.
//! Source: docs/specs/widget-toolkit.md §13; keyboard messages: §7.3.
//!
//! WT7 leaves construction, QueryInterface reference/output effects and native
//! deleting-destructor flags unpublished. This is not a full native owner or ABI
//! adapter: callers supply a live reference count; interface membership is only
//! a predicate. Rust Drop owns allocation cleanup. No window algorithm lives here.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

pub const IID_UNKNOWN: u32 = 1;
pub const IID_KEY_ACCELERATOR: u32 = 0xe2c1_b3c4;
pub const MESSAGE_SERVER_SERVICE: u32 = 0xc108_5722;
pub const KEY_DOWN: u32 = 5;
pub const KEY_UP: u32 = 6;

/// Exactly the four dwords copied by the accelerator, not the larger messages
/// used by other window methods. No native pointer is read by this module.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Message {
    pub type_id: u32,
    pub data: [u32; 3],
}

/// Primitive foreign service operations. The provider cannot look up bindings,
/// pack keys, choose a match or decide whether a keyboard message is handled.
pub trait MessageHost {
    type Server: Copy;

    /// Return a borrowed service identity. The core takes its own reference.
    fn find_service(&mut self, service: u32) -> Option<Self::Server>;
    fn add_ref(&mut self, server: Self::Server);
    fn release(&mut self, server: Self::Server);
    /// Synchronous MessageSend (server slot 3); return only after delivery.
    fn message_send(&mut self, server: Self::Server, message: Message);
}

/// Safe slice equivalent of slot 13's return value and in/out count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyCounts {
    pub written: usize,
    pub total: usize,
}

pub struct KeyAccelerator<H: MessageHost> {
    bindings: BTreeMap<u32, Message>,
    initialized: bool,
    server: Option<H::Server>,
    references: u32,
    host: H,
}

impl<H: MessageHost> KeyAccelerator<H> {
    /// Explicit empty, uninitialized live state. The count is supplied rather
    /// than inferred from an undocumented native constructor (WT7).
    pub fn new(host: H, live_references: NonZeroU32) -> Self {
        Self {
            bindings: BTreeMap::new(),
            initialized: false,
            server: None,
            references: live_references.get(),
            host,
        }
    }

    /// Slot 0's published membership decision only, not a COM acquisition.
    pub fn supports_interface(iid: u32) -> bool {
        matches!(iid, IID_UNKNOWN | IID_KEY_ACCELERATOR)
    }

    /// Slot 1 on live counts. None is a safe Rust rejection of overflow or a
    /// terminal handle, not claimed native overflow behavior (WT7).
    pub fn add_ref(&mut self) -> Option<u32> {
        if self.is_destroyed() {
            return None;
        }
        self.references = self.references.checked_add(1)?;
        Some(self.references)
    }

    /// Slot 2: final release runs Shutdown before deleting owned table values.
    /// The Rust handle remains terminal so an embedding cannot reuse freed state.
    pub fn release(&mut self) -> Option<u32> {
        match self.references {
            0 => None,
            1 => {
                self.shutdown();
                self.bindings.clear();
                self.references = 0;
                Some(0)
            }
            _ => {
                self.references -= 1;
                Some(self.references)
            }
        }
    }

    pub fn reference_count(&self) -> u32 {
        self.references
    }

    pub fn is_destroyed(&self) -> bool {
        self.references == 0
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Slot 3. Literal once-only success, even if no service is available (WT8).
    /// Repeated calls do not retry until Shutdown clears the initialized flag.
    pub fn init(&mut self) -> bool {
        if self.is_destroyed() {
            return false;
        }
        if !self.initialized {
            self.server = self.host.find_service(MESSAGE_SERVER_SERVICE);
            if let Some(server) = self.server {
                self.host.add_ref(server);
            }
            self.initialized = true;
        }
        true
    }

    /// Slot 4: keep all bindings; release only the reference Init acquired.
    pub fn shutdown(&mut self) -> bool {
        if self.is_destroyed() {
            return false;
        }
        if self.initialized {
            if let Some(server) = self.server.take() {
                self.host.release(server);
            }
            self.initialized = false;
        }
        true
    }

    /// Slot 5: input code is stored verbatim, with first binding winning.
    /// Terminal-handle rejection is a Rust safety boundary; live calls return true.
    pub fn register(&mut self, code: u32, message: Message) -> bool {
        if self.is_destroyed() {
            return false;
        }
        self.bindings.entry(code).or_insert(message);
        true
    }

    /// Slot 6.
    pub fn register_type(&mut self, code: u32, type_id: u32) -> bool {
        self.register(code, Message { type_id, data: [0; 3] })
    }

    /// Slot 7: exact code only, no modifier normalization.
    pub fn unregister(&mut self, code: u32) -> bool {
        self.bindings.remove(&code).is_some()
    }

    /// Slot 8: only the stored side is masked; retained modifier bit 3 differs
    /// from the three optional presence bits, and from packing's dropped 0x70.
    pub fn is_present(&self, code: u32, with_mod_bits: bool, with_up_bit: bool) -> bool {
        let mask = !(if with_mod_bits { 0 } else { 0x0007_0000 }
            | if with_up_bit { 0 } else { 0x1000_0000 });
        self.bindings.keys().any(|stored| stored & mask == code)
    }

    /// Slot 9. WT8: use the printed full-u32 OR/shift literally; no additional
    /// vk truncation or modifier cleanup beyond the published expression.
    pub fn test_key(&self, vk: u32, modifiers: u32, is_down: bool) -> Option<Message> {
        if matches!(vk, 0x10..=0x12) || modifiers == u32::MAX {
            return None;
        }
        let up_bit = if is_down { 0 } else { 0x1000 };
        let code = vk | (((modifiers & 0xffff_ff8f) | up_bit) << 16);
        self.bindings.get(&code).copied()
    }

    /// Slot 10.
    pub fn test_key_type(&self, vk: u32, modifiers: u32, is_down: bool) -> Option<u32> {
        self.test_key(vk, modifiers, is_down).map(|m| m.type_id)
    }

    /// Slot 11: character messages and other input types never accelerate.
    pub fn test_message(&self, message: Message) -> Option<Message> {
        let is_down = match message.type_id {
            KEY_DOWN => true,
            KEY_UP => false,
            _ => return None,
        };
        self.test_key(message.data[0], message.data[1], is_down)
    }

    /// Slot 12.
    pub fn test_message_type(&self, message: Message) -> Option<u32> {
        self.test_message(message).map(|m| m.type_id)
    }

    /// Slot 13: unsigned key order, bounded writes, unbounded total match count.
    /// A slice replaces raw pointer/capacity validity; no native null rules added.
    pub fn keys_for_type(&self, type_id: u32, codes: &mut [u32]) -> KeyCounts {
        let mut counts = KeyCounts { written: 0, total: 0 };
        for (&code, message) in &self.bindings {
            if message.type_id == type_id {
                counts.total += 1;
                if counts.written < codes.len() {
                    codes[counts.written] = code;
                    counts.written += 1;
                }
            }
        }
        counts
    }

    /// Slot 14: true on a hit even without a held server. Initialization is not
    /// an enable/disable flag for lookup, registration, presence or processing.
    pub fn process(&mut self, message: Message) -> bool {
        let Some(bound) = self.test_message(message) else {
            return false;
        };
        if let Some(server) = self.server {
            self.host.message_send(server, bound);
        }
        true
    }
}

impl<H: MessageHost> Drop for KeyAccelerator<H> {
    fn drop(&mut self) {
        // Rust ownership fallback, not native slot 15's unpublished flag contract.
        self.shutdown();
    }
}

#[cfg(test)]
#[path = "key_accelerator/tests.rs"]
mod tests;
