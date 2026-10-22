// SPDX-License-Identifier: GPL-3.0-or-later
//! The two attributed factory products, not complete driver implementations.
use super::{
    aggregate::Aggregate,
    call,
    software::{Buffers, Coefficients, Pool, Serial, WorkingArrays},
    Allocation, Event, Fault, Host, Op, Result, OPENGL_CLASS, SOFTWARE_CLASS,
};
use std::collections::BTreeMap;

pub const OPENGL_NAME: &[u8] = b"Maxis 3D GDriver\nOpenGL\n1.1\n";
pub const DESCRIPTOR_NAME: &str = "GDriverClass--OpenGL";

#[derive(Debug)]
pub struct OpenGl {
    pub identity: u32,
    pub name: Vec<u8>,
    pub name_storage: u32,
    pub owned_buffer: u32,
    pub strings: Vec<u32>,
    pub string_storage: u32,
    pub selected: [u32; 13],
    pub lights: [[u32; 4]; 8],
    pub scalars: [u32; 4],
    pub auxiliary: u32,
}
impl OpenGl {
    pub fn new(identity: u32, host: &mut impl Host) -> Result<Self> {
        host.observe(Event::ProductDispatch(OPENGL_CLASS));
        let mut selected = [0; 13];
        selected[0] = u32::MAX;
        let name_storage = host.alloc(OPENGL_NAME.len() as u32 + 1, Allocation::Heap);
        if name_storage == 0 {
            return Err(Fault::NullWrite);
        }
        host.write(name_storage, OPENGL_NAME)?;
        host.write(name_storage.wrapping_add(OPENGL_NAME.len() as u32), &[0])?;
        Ok(Self {
            identity,
            name: OPENGL_NAME.to_vec(),
            name_storage,
            owned_buffer: 0,
            strings: Vec::new(),
            string_storage: 0,
            selected,
            lights: [[0; 4]; 8],
            scalars: [1.0f32.to_bits(), 1.0f32.to_bits(), 1.0f32.to_bits(), 0.5f32.to_bits()],
            auxiliary: 0,
        })
    }
    /// All specified initial matrix words are zero; the specification prescribes no
    /// portable matrix count/layout. No invented matrix shape is exposed (GI1).
    pub const fn initial_matrix_word(&self, _word: u32) -> u32 {
        0
    }
    pub const fn class_id(&self) -> u32 {
        OPENGL_CLASS
    }
    pub fn name_predicate(&self) -> bool {
        let copy: Vec<u8> = self.name.iter().map(u8::to_ascii_lowercase).collect();
        copy.len() >= 6
            && copy.windows(6).position(|w| w == b"nvidia") == Some(0)
            && copy.len() >= 7
            && copy.windows(7).position(|w| w == b"geforce") == Some(0)
    }
    pub fn descriptor(&self, descriptor: &mut Descriptor, canonical_global: &mut u32) -> bool {
        descriptor.name = Some(DESCRIPTOR_NAME);
        descriptor.flags |= 0x0608_0000;
        descriptor.words[4] = 600;
        descriptor.words[5] = 800;
        *canonical_global = self.identity;
        true
    }
    pub fn callback_descriptor(&self, descriptor: &mut Descriptor) -> bool {
        descriptor.words[0] = 0x23;
        descriptor.words[1] = 0x0087_ee10;
        descriptor.name_word9 = Some(DESCRIPTOR_NAME);
        true
    }
    pub fn mode(&mut self, window: u32, selector: i32, auxiliary: u32, host: &mut impl Host) -> bool {
        if selector == -1 {
            if self.selected[0] == u32::MAX {
                return true;
            }
            call(host, self.identity, Op::BranchReset, &[]);
            // 0x00407990 is a no-op, not a memset.
            let dc = call(host, 0, Op::CurrentDc, &[]);
            let context = call(host, 0, Op::CurrentContext, &[]);
            call(host, 0, Op::MakeCurrent, &[dc, 0]);
            call(host, 0, Op::DeleteContext, &[context]);
            call(host, window, Op::ReleaseDc, &[dc]);
            self.selected[0] = u32::MAX;
        } else {
            if auxiliary != 0 {
                self.auxiliary = auxiliary;
            }
            call(host, window, Op::WindowAssociate, &[(-21i32) as u32, self.identity]);
            host.call(
                self.identity,
                Op::ModeRecord,
                &[selector as u32],
                &[],
                &mut self.selected,
            );
            call(host, self.identity, Op::ModePrepare, &[]);
            call(host, self.identity, Op::ModeCreate, &[selector as u32]);
        }
        true
    }
    pub fn destroy(&mut self, host: &mut impl Host) {
        host.observe(Event::ProductDispatch(OPENGL_CLASS));
        destroy_strings(&self.strings, self.string_storage, host);
        if self.name_storage != 0 {
            host.free(self.name_storage, Allocation::Heap);
        }
        if self.owned_buffer != 0 {
            host.free(self.owned_buffer, Allocation::Heap);
        }
        call(host, self.identity, Op::ReferenceBaseCleanup, &[]);
    }
    pub fn delete(&mut self, flags: u32, host: &mut impl Host) -> u32 {
        self.destroy(host);
        if flags & 1 != 0 {
            host.free(self.identity, Allocation::Heap);
        }
        self.identity
    }
    pub fn finalize(&mut self, host: &mut impl Host) -> u32 {
        call(host, self.identity, Op::DriverSlot80, &[]);
        self.delete(1, host);
        0
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Descriptor {
    pub words: [u32; 10],
    pub flags: u32,
    pub name: Option<&'static str>,
    pub name_word9: Option<&'static str>,
}

#[derive(Debug)]
pub struct Software {
    pub identity: u32,
    pub input: bool,
    pub pool: Pool,
    pub aggregate: Aggregate,
    pub selected: [u32; 13],
    pub arrays: WorkingArrays,
    pub state: KnownWord,
    pub buffers: Buffers,
    pub trailing_buffer: u32,
    pub strings: Vec<u32>,
    pub string_storage: u32,
    pub tile_tags: u32,
    pub owned_buffer: u32,
    pub held_interface: u32,
}
impl Software {
    pub fn new(identity: u32, input: bool, coefficients: &mut Coefficients, host: &mut impl Host) -> Result<Self> {
        host.observe(Event::ProductDispatch(SOFTWARE_CLASS));
        let pool = Pool::new(1024, 128, host)?;
        // Literal construction sequence of §1. The precise subobject placement of the
        // §5.6 aggregate relative to these steps is not given; see GI1.
        let aggregate = Aggregate::new(host)?;
        let mut selected = [0; 13];
        selected[0] = u32::MAX;
        let mut arrays = WorkingArrays::default();
        arrays.allocate(host)?;
        coefficients.regenerate();
        Ok(Self {
            identity,
            input,
            pool,
            aggregate,
            selected,
            arrays,
            state: KnownWord { bits: 0, mask: 0x2000 },
            buffers: Buffers::default(),
            trailing_buffer: 0,
            strings: Vec::new(),
            string_storage: 0,
            tile_tags: 0,
            owned_buffer: 0,
            held_interface: 0,
        })
    }
    pub const fn class_id(&self) -> u32 {
        SOFTWARE_CLASS
    }
    pub fn destroy(&mut self, host: &mut impl Host) {
        host.observe(Event::ProductDispatch(SOFTWARE_CLASS));
        call(host, self.identity, Op::BranchStop, &[]);
        self.arrays.free(host);
        self.buffers.clear(host);
        if self.trailing_buffer != 0 {
            host.free(self.trailing_buffer, Allocation::Heap);
        }
        destroy_strings(&self.strings, self.string_storage, host);
        if self.tile_tags != 0 {
            host.free(self.tile_tags, Allocation::Heap);
        }
        if self.owned_buffer != 0 {
            host.free(self.owned_buffer, Allocation::Heap);
        }
        if self.held_interface != 0 {
            call(host, self.held_interface, Op::Release, &[]);
        }
        self.aggregate.destroy(host);
        self.pool.destroy(host);
        call(host, self.identity, Op::ReferenceBaseCleanup, &[]);
    }
    pub fn delete(&mut self, flags: u32, host: &mut impl Host) -> u32 {
        self.destroy(host);
        if flags & 1 != 0 {
            host.free(self.identity, Allocation::Heap);
        }
        self.identity
    }
    pub fn finalize(&mut self, host: &mut impl Host) -> u32 {
        call(host, self.identity, Op::DriverSlot80, &[]);
        self.delete(1, host);
        0
    }
}
/// Bits not covered by mask have no initialization promise in the source contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownWord {
    pub bits: u32,
    pub mask: u32,
}
fn destroy_strings(strings: &[u32], storage: u32, host: &mut impl Host) {
    for pointer in strings {
        if *pointer != 0 {
            host.free(*pointer, Allocation::Heap);
        }
    }
    if storage != 0 {
        host.free(storage, Allocation::Heap);
    }
}

/// Process-shared serial and coefficient tables, explicitly owned by the caller to
/// avoid hidden Rust globals. All factories using this process scope share them.
#[derive(Default, Debug)]
pub struct Products {
    pub opengl: BTreeMap<u32, OpenGl>,
    pub software: BTreeMap<u32, Software>,
    pub coefficients: Coefficients,
    pub serial: Serial,
}
impl Products {
    pub fn factory(&mut self, entry: u32, host: &mut impl Host) -> Result<u32> {
        let (bytes, adjustment) = match entry {
            0x0087_e1d0 => (420, 12),
            0x008d_cd30 => (1048, 8),
            _ => return Ok(call(host, entry, Op::ForeignDx7Factory, &[])),
        };
        let storage = host.alloc(bytes, Allocation::Heap);
        if storage == 0 {
            return Ok(0);
        }
        if entry == 0x0087_e1d0 {
            self.opengl.insert(storage, OpenGl::new(storage, host)?);
        } else {
            self.software
                .insert(storage, Software::new(storage, false, &mut self.coefficients, host)?);
        }
        Ok(storage.wrapping_add(adjustment))
    }
    /// Inventory §§3.5,5.7: only identity-adjusting entries live here; primary driver
    /// QI/refcount operations are separately owned and remain primitive foreign calls.
    pub fn thunk(
        &mut self,
        entry: u32,
        identity: u32,
        iid: u32,
        output: &mut u32,
        flags: u32,
        host: &mut impl Host,
    ) -> Result<u32> {
        let (adjustment, op) = match entry {
            0x0087_e0b0 => (4, Op::BranchQueryInterface),
            0x0087_e0c0 => (4, Op::BranchRelease),
            0x0087_e0d0 => (8, Op::BranchQueryInterface),
            0x0087_e0e0 => (8, Op::BranchRelease),
            0x0087_e120 => (16, Op::BranchQueryInterface),
            0x0087_e130 => (16, Op::BranchRelease),
            0x0087_e150 => (20, Op::BranchRelease),
            0x008d_cbf0 => (4, Op::BranchQueryInterface),
            0x008d_cc60 => (12, Op::BranchRelease),
            0x008d_cc70 => (12, Op::BranchAddRef),
            0x0087_e140 => {
                let key = identity.wrapping_sub(20);
                return Ok(self.opengl.get_mut(&key).ok_or(Fault::MissingSlot)?.delete(flags, host));
            }
            0x008d_cc50 => {
                let key = identity.wrapping_sub(12);
                return Ok(self
                    .software
                    .get_mut(&key)
                    .ok_or(Fault::MissingSlot)?
                    .delete(flags, host));
            }
            _ => return Err(Fault::MissingSlot),
        };
        let arguments = [iid];
        let arguments = if op == Op::BranchQueryInterface {
            &arguments[..]
        } else {
            &[]
        };
        Ok(host.call(
            identity.wrapping_sub(adjustment),
            op,
            arguments,
            &[],
            std::slice::from_mut(output),
        ))
    }
}

pub fn extension_bool(callback: u32, host: &mut impl Host) -> bool {
    callback != 0 && call(host, callback, Op::ExtensionBool, &[]) != 0
}
pub fn extension_selector(callback: u32, selector: u32, host: &mut impl Host) -> u32 {
    if callback == 0 || selector > 2 {
        0
    } else {
        call(host, callback, Op::ExtensionSelector, &[selector + 1])
    }
}
pub fn extension_one(callback: u32, word: u32, host: &mut impl Host) -> Result<bool> {
    if callback == 0 {
        return Err(Fault::NullRead);
    }
    call(host, callback, Op::ExtensionOne, &[word]);
    Ok(true)
}
pub fn extension_five(callback: u32, words: [u32; 7], host: &mut impl Host) -> Result<bool> {
    if callback == 0 {
        return Err(Fault::NullRead);
    }
    call(host, callback, Op::ExtensionFive, &words[..5]);
    Ok(true)
}
pub fn extension_seven(callback: u32, words: [u32; 7], host: &mut impl Host) -> Result<bool> {
    if callback == 0 {
        return Err(Fault::NullRead);
    }
    call(host, callback, Op::ExtensionSeven, &words);
    Ok(true)
}
pub fn lighting(enabled: u8, host: &mut impl Host) {
    call(
        host,
        0,
        if enabled == 0 { Op::GlDisable } else { Op::GlEnable },
        &[0x0b50],
    );
}
pub fn light_enabled(index: u32, enabled: u8, host: &mut impl Host) {
    call(
        host,
        0,
        if enabled == 0 { Op::GlDisable } else { Op::GlEnable },
        &[0x4000u32.wrapping_add(index)],
    );
}
pub fn ambient(words: [u32; 4], host: &mut impl Host) {
    call(
        host,
        0,
        Op::GlAmbient,
        &[0x0b53, words[0], words[2], words[1], words[3]],
    );
}
pub fn light(index: u32, selector: u32, pointer: u32, host: &mut impl Host) {
    if selector <= 2 {
        call(
            host,
            0,
            Op::GlLight,
            &[0x4000u32.wrapping_add(index), 0x1200 + selector, pointer],
        );
    }
}
pub fn combined_light(index: u32, pointers: [u32; 3], host: &mut impl Host) {
    for (i, pointer) in pointers.iter().enumerate() {
        if *pointer != 0 {
            light(index, i as u32, pointers[0], host);
        }
    }
}
pub fn material(selector: u32, pointer: u32, host: &mut impl Host) {
    if let Some(property) = [0x1200, 0x1201, 0x1202, 0x1600, 0x1601].get(selector as usize) {
        call(host, 0, Op::GlMaterial, &[0x0404, *property, pointer]);
    }
}
pub fn combined_material(pointers: [u32; 4], shininess: u32, host: &mut impl Host) {
    for (i, pointer) in pointers.iter().enumerate() {
        if *pointer != 0 {
            material(i as u32, *pointer, host);
        }
    }
    if f32::from_bits(shininess) >= 0.0 {
        // Immediate byte data denotes the address of this binary32 argument, not a
        // foreign vector pointer. Preserve its bits, including negative zero.
        host.call(0, Op::GlMaterial, &[0x0404, 0x1601], &shininess.to_le_bytes(), &mut []);
    }
}
