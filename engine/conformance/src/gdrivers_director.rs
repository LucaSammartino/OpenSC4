// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent GD1–GD65 paper fixtures, plus public-boundary discriminators.
//! Expected values in the JSON and explicit call-order predicates below come from
//! the two published specs, never a reference renderer or prior implementation.
use crate::json::Json;
use fc4_core::gdrivers_director::{self as gd, aggregate::*, capture, products::*, software::*, *};
use std::collections::{BTreeMap, BTreeSet};

type R<T> = std::result::Result<T, String>;
#[derive(Clone, Debug)]
struct Invocation {
    target: u32,
    op: Op,
    args: Vec<u32>,
    data: Vec<u8>,
}
struct Fixture {
    next: u32,
    memory: BTreeMap<u32, Vec<u8>>,
    allocs: Vec<(u32, Allocation, u32)>,
    free: Vec<u32>,
    calls: Vec<Invocation>,
    events: Vec<Event>,
    writes: Vec<(u32, Vec<u8>)>,
    fail: BTreeSet<u32>,
    fail_virtual: bool,
    strings: BTreeMap<u32, (u32, Vec<u8>)>,
    children: BTreeMap<u32, Vec<u32>>,
    unload: BTreeMap<u32, u32>,
    lookup: BTreeMap<u32, (u32, u32)>,
    qi: u32,
    qi_write: Option<u32>,
    framework: u32,
    initialized: bool,
    format: u32,
    init_ok: u32,
    lock_ok: u32,
    viewport: [u32; 4],
    dimensions: [u32; 2],
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            next: 0x10000,
            memory: BTreeMap::new(),
            allocs: vec![],
            free: vec![],
            calls: vec![],
            events: vec![],
            writes: vec![],
            fail: BTreeSet::new(),
            fail_virtual: false,
            strings: BTreeMap::new(),
            children: BTreeMap::new(),
            unload: BTreeMap::new(),
            lookup: BTreeMap::new(),
            qi: 1,
            qi_write: Some(900),
            framework: 400,
            initialized: true,
            format: 9,
            init_ok: 1,
            lock_ok: 1,
            viewport: [10, 20, 8, 6],
            dimensions: [8, 6],
        }
    }
}
impl Fixture {
    fn source(&mut self, identity: u32, bytes: &[u8]) {
        let address = self.alloc(bytes.len() as u32 + 1, Allocation::Heap);
        self.write(address, bytes).unwrap();
        self.strings.insert(identity, (address, bytes.to_vec()));
    }
    fn reset(&mut self) {
        self.calls.clear();
        self.events.clear();
        self.free.clear();
        self.allocs.clear();
        self.writes.clear();
    }
    fn ops(&self) -> Vec<Op> {
        self.calls.iter().map(|c| c.op).collect()
    }
    fn args(&self, op: Op) -> Vec<Vec<u32>> {
        self.calls
            .iter()
            .filter(|c| c.op == op)
            .map(|c| c.args.clone())
            .collect()
    }
    fn targets(&self, op: Op) -> Vec<u32> {
        self.calls.iter().filter(|c| c.op == op).map(|c| c.target).collect()
    }
    fn word(&mut self, address: u32) -> u32 {
        let mut b = [0; 4];
        self.read(address, &mut b).unwrap();
        u32::from_le_bytes(b)
    }
}
impl Host for Fixture {
    fn alloc(&mut self, bytes: u32, kind: Allocation) -> u32 {
        if self.fail.contains(&bytes) || (self.fail_virtual && matches!(kind, Allocation::Virtual { .. })) {
            self.allocs.push((bytes, kind, 0));
            return 0;
        }
        let address = self.next;
        self.next = self.next.wrapping_add(bytes.max(1)).wrapping_add(256);
        self.memory.insert(address, vec![0xa5; bytes as usize]);
        self.allocs.push((bytes, kind, address));
        address
    }
    fn free(&mut self, address: u32, _kind: Allocation) {
        self.free.push(address);
    }
    fn read(&mut self, address: u32, bytes: &mut [u8]) -> gd::Result<()> {
        let Some((base, storage)) = self.memory.range(..=address).next_back() else {
            return Err(Fault::NullRead);
        };
        let start = (address - base) as usize;
        let source = storage.get(start..start + bytes.len()).ok_or(Fault::NullRead)?;
        bytes.copy_from_slice(source);
        Ok(())
    }
    fn write(&mut self, address: u32, bytes: &[u8]) -> gd::Result<()> {
        let Some((base, storage)) = self.memory.range_mut(..=address).next_back() else {
            return Err(Fault::NullWrite);
        };
        let start = (address - base) as usize;
        let dest = storage.get_mut(start..start + bytes.len()).ok_or(Fault::NullWrite)?;
        dest.copy_from_slice(bytes);
        self.writes.push((address, bytes.to_vec()));
        Ok(())
    }
    fn observe(&mut self, event: Event) {
        self.events.push(event);
    }
    fn enumerate_child(&mut self, target: u32, context: u32, callback: &mut dyn FnMut(u32, u32, u32) -> bool) {
        self.call(target, Op::ChildEnumerate, &[context], &[], &mut []);
        callback(target, 0, context);
    }
    fn call(&mut self, target: u32, op: Op, args: &[u32], data: &[u8], out: &mut [u32]) -> u32 {
        self.calls.push(Invocation {
            target,
            op,
            args: args.to_vec(),
            data: data.to_vec(),
        });
        match op {
            Op::Framework => self.framework,
            Op::StringLength => self.strings[&target].1.len() as u32,
            Op::StringData => self.strings[&target].0,
            Op::ChildCount => self.children.get(&target).map_or(0, |v| v.len() as u32),
            Op::ChildAt => self.children[&target][args[0] as usize],
            Op::ChildInitialize => {
                // This is a fixture for a foreign child with its own two children;
                // it models only that fixture's published call trace, no parent output.
                for child in self.children.get(&target).cloned().unwrap_or_default() {
                    self.call(child, op, args, data, &mut []);
                }
                0
            }
            Op::ChildCanUnload => self.unload.get(&target).copied().unwrap_or(1),
            Op::ChildLookup => {
                let (r, v) = self.lookup.get(&target).copied().unwrap_or((0, 0));
                if v != 0 {
                    out[0] = v;
                }
                r
            }
            Op::QueryInterface | Op::BranchQueryInterface | Op::InterfaceFactory => {
                if let Some(v) = self.qi_write {
                    out[0] = v;
                }
                self.qi
            }
            Op::BranchAddRef => 27,
            Op::BranchRelease => 26,
            Op::ForeignDx7Factory => 700,
            Op::ExtensionSelector => 27,
            Op::ExtensionBool => 256,
            Op::BitmapInitialized => u32::from(self.initialized),
            Op::BitmapFormat => {
                out[0] = self.format;
                0
            }
            Op::BitmapWidth => self.dimensions[0],
            Op::BitmapHeight => self.dimensions[1],
            Op::BitmapInitialize => {
                if self.init_ok as u8 != 0 {
                    self.initialized = true;
                }
                self.init_ok
            }
            Op::BitmapLock => self.lock_ok,
            Op::BitmapCreate | Op::ComBitmapCreate => {
                out[0] = 800;
                0
            }
            Op::GraphicsService | Op::ComService => 77,
            Op::GlViewport => {
                out.copy_from_slice(&self.viewport);
                0
            }
            Op::GlReadPixels => {
                let bytes = (args[2] * args[3] * 3) as usize;
                let rgb: Vec<u8> = (1..=bytes).map(|n| n as u8).collect();
                self.write(args[6], &rgb).unwrap();
                0
            }
            Op::CurrentDc => 51,
            Op::CurrentContext => 52,
            Op::ModeRecord => {
                out[0] = args[0];
                0
            }
            Op::BitmapStorageRequest => 66,
            Op::BitmapStorageAddress => 0x9000,
            _ => 0,
        }
    }
}
fn director(host: &mut Fixture) -> Director {
    Director::new(100, 104, 108, host)
}
fn frame() -> Frame {
    Frame {
        identity: 400,
        width: 64,
        height: 64,
        planes: [0x2000, 0x4000],
        pitches: [256, 128],
        tile_base: 0,
        tile_pitch: 16,
    }
}
fn prepared(host: &mut Fixture) -> (Buffers, Serial, Frame) {
    let mut buffers = Buffers::default();
    let mut serial = Serial::default();
    buffers.create(0, &mut serial, host).unwrap();
    buffers.slots[0].data = 0x8000;
    buffers.slots[0].dimensions = [64, 64];
    buffers.slots[0].pitch = Some(256);
    buffers.slots[0].version = Some(2);
    let mut f = frame();
    f.tile_base = host.alloc(64, Allocation::Heap);
    (buffers, serial, f)
}
fn b(value: bool) -> u32 {
    u32::from(value)
}
fn scenario(id: u32) -> gd::Result<Vec<u32>> {
    let mut h = Fixture::default();
    let mut p = Products::default();
    let result = match id {
        1 => {
            let mut s = Singleton::default();
            let a = s.access(100, 104, 108, &mut h);
            let c = s.access(100, 104, 108, &mut h);
            let d = s.access(100, 104, 108, &mut h);
            vec![
                b(a == c && c == d),
                s.director.as_ref().unwrap().refs(),
                h.events.len() as u32,
                h.targets(Op::ExitRegister).len() as u32,
                b(h.events
                    == vec![
                        Event::Register(OPENGL_CLASS, 0x87e1d0, 1),
                        Event::Register(SOFTWARE_CLASS, 0x8dcd30, 1),
                        Event::Register(FOREIGN_DX7_CLASS, 0x886700, 1),
                    ]),
            ]
        }
        2 => {
            let d = director(&mut h);
            let mut ids = vec![];
            d.enumerate(9, &mut h, &mut |id, _, _| {
                ids.push(id);
                false
            });
            vec![
                d.id(),
                d.refs(),
                d.service(),
                d.framework(),
                h.calls.len() as u32,
                b(ids == [SOFTWARE_CLASS, OPENGL_CLASS, FOREIGN_DX7_CLASS]),
            ]
        }
        3 => {
            let mut d = director(&mut h);
            vec![
                b(d.initialize(0, 0, &mut h)?),
                d.service(),
                h.calls.len() as u32,
                d.path().len() as u32,
            ]
        }
        4 => {
            let mut d = director(&mut h);
            h.source(8, b"abc");
            h.framework = 0;
            h.reset();
            let r = d.initialize(5, 8, &mut h)?;
            vec![
                b(r),
                d.service(),
                d.framework(),
                b(d.path() == b"abc"),
                b(h.ops() == [Op::Framework, Op::StringLength, Op::StringData]),
            ]
        }
        5 => {
            let mut d = director(&mut h);
            h.source(8, b"one");
            h.source(9, b"two");
            h.reset();
            let a = d.initialize(1, 8, &mut h)?;
            let c = d.initialize(2, 9, &mut h)?;
            let z = d.initialize(0, 0, &mut h)?;
            vec![
                b(a),
                b(c),
                b(z),
                d.service(),
                b(d.path() == b"two"),
                b(h.targets(Op::Framework) == [1, 2]),
                b(!h.ops().contains(&Op::Release) && !h.ops().contains(&Op::AddRef)),
            ]
        }
        6 => {
            let d = director(&mut h);
            let mut out = 123;
            let r = d.get_class(99, 1, &mut out, &mut h, &mut p)?;
            vec![b(r), out, h.calls.len() as u32, h.allocs.len() as u32]
        }
        7 => {
            let d = director(&mut h);
            let mut result = vec![];
            for class in [OPENGL_CLASS, SOFTWARE_CLASS, FOREIGN_DX7_CLASS] {
                let mut out = 123;
                result.extend([b(d.get_class(class, 17, &mut out, &mut h, &mut p)?), out]);
            }
            result.extend([
                h.targets(Op::QueryInterface).len() as u32,
                b(!h.ops().contains(&Op::AddRef) && !h.ops().contains(&Op::Release)),
            ]);
            result
        }
        8 => {
            let d = director(&mut h);
            h.qi = 0;
            h.qi_write = None;
            let mut out = 123;
            let r = d.get_class(OPENGL_CLASS, 17, &mut out, &mut h, &mut p)?;
            vec![
                b(r),
                out,
                b(h.ops() == [Op::QueryInterface, Op::AddRef, Op::Release]),
                b(h.calls.iter().all(|c| c.target == h.calls[0].target)),
            ]
        }
        9 => {
            let d = director(&mut h);
            h.fail.insert(420);
            let mut out = 123;
            let r = d.get_class(OPENGL_CLASS, 1, &mut out, &mut h, &mut p);
            vec![
                b(r == Err(Fault::NullFactory)),
                out,
                h.calls.len() as u32,
                p.opengl.len() as u32,
            ]
        }
        10 => {
            let mut d = director(&mut h);
            d.add_director(10, &mut h);
            d.add_director(20, &mut h);
            h.lookup.insert(10, (0, 55));
            h.lookup.insert(20, (1, 66));
            h.reset();
            let mut out = 123;
            vec![
                b(d.get_class(OPENGL_CLASS, 1, &mut out, &mut h, &mut p)?),
                out,
                b(h.targets(Op::ChildLookup) == [10, 20]),
                h.allocs.len() as u32,
            ]
        }
        11 => {
            let d = director(&mut h);
            let mut values = vec![];
            d.enumerate(42, &mut h, &mut |class, zero, context| {
                values.extend([class, zero, context]);
                false
            });
            values.push(h.calls.len() as u32);
            values
        }
        12 => {
            let d = director(&mut h);
            let mut out = 0;
            d.get_class(OPENGL_CLASS, 1, &mut out, &mut h, &mut p)?;
            d.get_class(OPENGL_CLASS, 1, &mut out, &mut h, &mut p)?;
            vec![
                p.opengl.len() as u32,
                b(h.targets(Op::QueryInterface)[0] != h.targets(Op::QueryInterface)[1]),
            ]
        }
        13 => {
            let mut d = director(&mut h);
            let mut values = vec![];
            for iid in [1, 0x03fa40bf, 0xa21ee941] {
                let mut out = 0;
                values.extend([b(d.query_interface(iid, Some(&mut out))?), out]);
            }
            values.push(d.refs());
            values
        }
        14 => {
            let mut d = director(&mut h);
            d.set_reference_fixture(3);
            let mut out = 123;
            vec![b(d.query_interface(99, Some(&mut out))?), out, d.refs()]
        }
        15 => {
            let mut d = director(&mut h);
            d.set_reference_fixture(3);
            vec![d.add_ref(), d.release(), d.refs()]
        }
        16 => {
            let mut d = director(&mut h);
            let mut v = vec![d.release(), d.release(), d.add_ref(), d.release()];
            let mut n = 0;
            d.enumerate(0, &mut h, &mut |_, _, _| {
                n += 1;
                true
            });
            v.push(n);
            v
        }
        17 => {
            let mut d = director(&mut h);
            d.set_reference_fixture(u32::MAX);
            vec![d.add_ref(), d.remove_ref(), h.free.len() as u32]
        }
        18 => {
            let mut d = director(&mut h);
            let a = d.can_unload(&mut h);
            d.add_ref();
            vec![b(a), b(d.can_unload(&mut h)), h.calls.len() as u32]
        }
        19 => {
            let mut d = director(&mut h);
            d.add_director(10, &mut h);
            d.add_director(20, &mut h);
            h.unload.insert(10, 0);
            h.reset();
            let a = d.can_unload(&mut h);
            d.add_ref();
            let c = d.can_unload(&mut h);
            vec![b(a), b(c), b(h.targets(Op::ChildCanUnload) == [10])]
        }
        20 => {
            let mut d = director(&mut h);
            let mut v = vec![];
            for phase in 0..2 {
                if phase == 1 {
                    h.source(9, b"x");
                    d.initialize(4, 9, &mut h)?;
                    h.reset();
                }
                v.extend([b(d.primary_noop_one(33)), b(d.primary_noop())]);
                for slot in 3..=10 {
                    v.push(b(d.secondary_callback(slot) == Some(true)));
                }
            }
            v.extend([d.refs(), h.calls.len() as u32, h.allocs.len() as u32]);
            v
        }
        21 => {
            let mut d = director(&mut h);
            d.library_path(7, &mut h)?;
            h.source(8, b"A\0B");
            d.initialize(1, 8, &mut h)?;
            d.library_path(7, &mut h)?;
            let c: Vec<_> = h.calls.iter().filter(|c| c.op == Op::StringAssign).collect();
            vec![c[0].args[0], c[1].args[0], b(c[1].data == b"A\0B")]
        }
        22 => {
            let mut d = director(&mut h);
            h.source(8, b"old");
            d.initialize(7, 8, &mut h)?;
            h.source(8, b"new");
            vec![
                d.service(),
                d.framework(),
                b(d.path() == b"old"),
                h.targets(Op::AddRef).len() as u32,
            ]
        }
        23 => {
            let mut d = director(&mut h);
            let r = d.initialize(7, 0, &mut h);
            vec![
                b(r == Err(Fault::NullRead)),
                d.service(),
                d.framework(),
                b(h.ops() == [Op::Framework]),
            ]
        }
        24 => {
            let mut d = director(&mut h);
            d.add_director(10, &mut h);
            d.add_director(20, &mut h);
            h.source(8, b"path");
            h.reset();
            let r = d.initialize(7, 8, &mut h)?;
            vec![
                b(r),
                b(h.targets(Op::ChildInitialize) == [10, 20]),
                b(h.args(Op::ChildInitialize) == [vec![7, 8], vec![7, 8]]),
            ]
        }
        25 => {
            let mut d = director(&mut h);
            let changed = d.register(OPENGL_CLASS, 55, 2, &mut h)?;
            let mut out = 0;
            d.get_class(OPENGL_CLASS, 1, &mut out, &mut h, &mut p)?;
            let mut n = 0;
            d.enumerate(0, &mut h, &mut |_, _, _| {
                n += 1;
                false
            });
            vec![
                b(changed),
                n,
                p.opengl.len() as u32,
                b(!h.targets(Op::QueryInterface).contains(&55)),
            ]
        }
        26 => {
            let mut d = director(&mut h);
            d.add_director(10, &mut h);
            d.add_director(20, &mut h);
            h.reset();
            let mut ids = vec![];
            d.enumerate(42, &mut h, &mut |id, z, c| {
                ids.extend([id, z, c]);
                false
            });
            ids.push(b(h.targets(Op::ChildEnumerate) == [10, 20]));
            ids
        }
        27 => {
            let mut d = director(&mut h);
            d.register(88, 99, 2, &mut h)?;
            h.qi = 0;
            h.qi_write = Some(55);
            let mut out = 123;
            vec![
                b(d.get_class(88, 18, &mut out, &mut h, &mut p)?),
                out,
                b(h.ops() == [Op::InterfaceFactory]),
                b(h.calls[0].target == 99 && h.calls[0].args == [18]),
            ]
        }
        28 => {
            let mut v = vec![];
            for flag in [0, 2, 1, 3] {
                let mut d = director(&mut h);
                h.reset();
                let r = d.delete(flag, &mut h);
                v.extend([
                    r,
                    h.free.len() as u32,
                    b(h.events
                        == [
                            Event::BaseDispatch,
                            Event::RegistryDestroyed,
                            Event::ChildrenDestroyed,
                            Event::PathDestroyed,
                        ]),
                    h.calls.len() as u32,
                ]);
            }
            v
        }
        29 => {
            let mut s = Singleton::default();
            s.access(100, 104, 108, &mut h);
            let d = s.director.as_mut().unwrap();
            d.add_ref();
            let mut out = 0;
            d.get_class(OPENGL_CLASS, 1, &mut out, &mut h, &mut p)?;
            h.reset();
            s.exit(&mut h);
            vec![
                s.guard as u32,
                s.director.as_ref().unwrap().refs(),
                p.opengl.len() as u32,
                h.free.len() as u32,
                h.calls.len() as u32,
                h.events.len() as u32,
            ]
        }
        30 => {
            let mut d = director(&mut h);
            h.source(8, b"path");
            d.initialize(7, 8, &mut h)?;
            h.children.insert(30, vec![10, 20]);
            h.reset();
            d.add_director(30, &mut h);
            vec![
                b(h.targets(Op::ChildInitialize) == [30, 10, 20, 10, 20]),
                b(d.children() == [10, 20, 30]),
                b(h.events == [Event::AppendChild(10), Event::AppendChild(20), Event::AppendChild(30)]),
                h.targets(Op::AddRef).len() as u32,
                b(h.calls
                    .iter()
                    .filter(|c| c.op == Op::ChildInitialize)
                    .all(|c| c.args == [7, 108] && c.data == b"path")),
            ]
        }
        31 => {
            let mut d = director(&mut h);
            let a = d.heap_allocated_size();
            d.register(99, 77, 1, &mut h)?;
            h.source(8, b"x");
            d.initialize(7, 8, &mut h)?;
            let mut out = 0;
            d.get_class(OPENGL_CLASS, 1, &mut out, &mut h, &mut p)?;
            vec![a, d.heap_allocated_size()]
        }
        32 => {
            let a = p.factory(0x87e1d0, &mut h)?;
            let c = p.factory(0x87e1d0, &mut h)?;
            let d = p.factory(0x8dcd30, &mut h)?;
            let e = p.factory(0x8dcd30, &mut h)?;
            vec![
                b(a != c && d != e),
                p.opengl.len() as u32,
                p.software.len() as u32,
                b(h.calls.is_empty()),
                b(p.opengl[&(a - 12)].name == OPENGL_NAME),
                b(!p.software[&(d - 8)].input),
                p.opengl[&(a - 12)].selected[0],
                b(p.opengl[&(a - 12)].lights == [[0; 4]; 8]),
                p.software[&(d - 8)].state.mask,
                p.software[&(d - 8)].state.bits,
            ]
        }
        33 => {
            h.fail.extend([420, 1048]);
            vec![
                p.factory(0x87e1d0, &mut h)?,
                p.factory(0x8dcd30, &mut h)?,
                p.opengl.len() as u32,
                p.software.len() as u32,
                h.events.len() as u32,
            ]
        }
        34 => {
            h.fail.insert(90112);
            let mut a = WorkingArrays::default();
            a.allocate(&mut h)?;
            vec![
                a.records,
                b(h.memory[&a.words].iter().all(|b| *b == 0)),
                h.free.len() as u32,
            ]
        }
        35 => {
            h.fail.insert(5376);
            let mut a = WorkingArrays::default();
            let r = a.allocate(&mut h);
            vec![
                b(r == Err(Fault::NullWrite)),
                b(a.records != 0),
                a.words,
                h.free.len() as u32,
                h.word(a.records),
                h.word(a.records + 80),
                h.word(a.records + 84),
            ]
        }
        36 => {
            let pool = Pool::new(1025, 2, &mut h)?;
            vec![
                pool.block_size,
                pool.count,
                h.allocs[0].0,
                pool.descriptors[1][0] - pool.descriptors[0][0],
                pool.descriptors[2][2],
                b(matches!(
                    h.allocs[0].1,
                    Allocation::Virtual {
                        flags: 0x1000,
                        protection: 0x40
                    }
                )),
            ]
        }
        37 => {
            h.fail_virtual = true;
            let pool = Pool::new(1024, 128, &mut h)?;
            vec![
                pool.base,
                pool.count,
                pool.descriptors[0][0],
                pool.descriptors[1][0],
                pool.descriptors[127][0],
            ]
        }
        38 => {
            let mut c = Coefficients {
                q: [[u32::MAX; 2]; 64],
                r: [[u32::MAX; 2]; 64],
            };
            c.regenerate();
            vec![
                c.q[0][0],
                c.q[1][1],
                c.r[1][0],
                c.r[1][1],
                c.q[63][0],
                c.q[63][1],
                c.r[63][0],
                c.r[63][1],
                b((0..64).all(|i| c.q[i][1] + (c.r[i][1] & !0x40000000) == 16384)),
            ]
        }
        39 => vec![
            extension_selector(1, 1, &mut h),
            extension_selector(1, 3, &mut h),
            b(h.args(Op::ExtensionSelector) == [vec![2]]),
        ],
        40 => vec![
            b(extension_five(1, [1, 2, 3, 4, 5, 6, 7], &mut h)?),
            b(h.args(Op::ExtensionFive) == [vec![1, 2, 3, 4, 5]]),
        ],
        41 => {
            ambient([1f32.to_bits(), 2f32.to_bits(), 3f32.to_bits(), 4f32.to_bits()], &mut h);
            h.args(Op::GlAmbient)[0].clone()
        }
        42 => {
            combined_light(2, [11, 22, 33], &mut h);
            combined_light(2, [0, 22, 0], &mut h);
            h.args(Op::GlLight).into_iter().flatten().collect()
        }
        43 => {
            for word in [
                (-1f32).to_bits(),
                (-0f32).to_bits(),
                f32::NAN.to_bits(),
                f32::INFINITY.to_bits(),
            ] {
                combined_material([0; 4], word, &mut h);
            }
            h.calls
                .iter()
                .filter(|c| c.op == Op::GlMaterial)
                .map(|c| u32::from_le_bytes(c.data.clone().try_into().unwrap()))
                .collect()
        }
        44 => {
            let r = capture::front([-2, 1, 5, 2], 800, &mut h)?;
            let read = h.args(Op::GlReadPixels)[0].clone();
            let pixels = h.args(Op::BitmapPixel);
            vec![
                r,
                read[0],
                read[1],
                read[2],
                read[3],
                pixels[0][0],
                pixels[0][1],
                pixels[0][2],
                pixels[3][0],
                pixels[3][1],
                pixels[3][2],
                b(h.args(Op::BitmapUnlock) == [vec![0x8080]]),
                h.free.len() as u32,
            ]
        }
        45 => {
            h.format = 8;
            let r = capture::front([0, 0, 2, 2], 800, &mut h)?;
            vec![
                r,
                h.free.len() as u32,
                h.targets(Op::GlReadPixels).len() as u32,
                h.targets(Op::Release).len() as u32,
            ]
        }
        46 => {
            h.fail.insert(12);
            let a = capture::front([0, 0, 2, 2], 800, &mut h)?;
            let n = h.targets(Op::GlReadPixels).len();
            h.fail.clear();
            h.initialized = false;
            h.init_ok = 0;
            h.reset();
            let c = capture::front([0, 0, 2, 2], 800, &mut h)?;
            vec![
                a,
                n as u32,
                c,
                h.targets(Op::GlReadPixels).len() as u32,
                h.free.len() as u32,
                h.targets(Op::Release).len() as u32,
            ]
        }
        47 => {
            let mut o = OpenGl::new(100, &mut h)?;
            let mut v = vec![];
            for name in [b"NVIDIA".as_slice(), b"GeForce", b"NVIDIA GeForce", b"nViDiA gEfOrCe"] {
                o.name = name.to_vec();
                v.extend([b(o.name_predicate()), b(o.name == name)]);
            }
            v
        }
        48 => {
            let mut o = OpenGl::new(100, &mut h)?;
            o.selected[0] = 3;
            h.reset();
            let a = o.mode(9, -1, 0, &mut h);
            let n = h.calls.len();
            let c = o.mode(9, -1, 0, &mut h);
            vec![
                b(a),
                b(c),
                n as u32,
                h.calls.len() as u32,
                o.selected[0],
                b(h.ops()
                    == [
                        Op::BranchReset,
                        Op::CurrentDc,
                        Op::CurrentContext,
                        Op::MakeCurrent,
                        Op::DeleteContext,
                        Op::ReleaseDc,
                    ]),
                b(h.args(Op::MakeCurrent) == [vec![51, 0]]),
                b(h.args(Op::DeleteContext) == [vec![52]]),
            ]
        }
        49 => {
            let mut o = OpenGl::new(100, &mut h)?;
            o.auxiliary = 7;
            h.reset();
            let a = o.mode(9, 2, 0, &mut h);
            let first = o.auxiliary;
            let c = o.mode(9, 3, 99, &mut h);
            vec![
                b(a),
                first,
                b(c),
                o.auxiliary,
                o.selected[0],
                b(h.ops()
                    == [
                        Op::WindowAssociate,
                        Op::ModeRecord,
                        Op::ModePrepare,
                        Op::ModeCreate,
                        Op::WindowAssociate,
                        Op::ModeRecord,
                        Op::ModePrepare,
                        Op::ModeCreate,
                    ]),
                b(h.args(Op::WindowAssociate) == [vec![(-21i32) as u32, 100], vec![(-21i32) as u32, 100]]),
            ]
        }
        50 => {
            let mut v = vec![];
            for flag in [0, 2, 1] {
                let mut o = OpenGl::new(100, &mut h)?;
                h.reset();
                v.extend([
                    o.delete(flag, &mut h),
                    b(h.free.contains(&100)),
                    b(h.ops() == [Op::ReferenceBaseCleanup]),
                ]);
            }
            let mut o = OpenGl::new(100, &mut h)?;
            h.reset();
            v.extend([
                o.finalize(&mut h),
                b(h.ops() == [Op::DriverSlot80, Op::ReferenceBaseCleanup]),
                b(h.free.last() == Some(&100)),
            ]);
            v
        }
        51 => {
            let mut bs = Buffers::default();
            let mut s = Serial::default();
            let a = bs.create(0, &mut s, &mut h)?;
            let c = bs.create(1, &mut s, &mut h)?;
            let d = bs.create(2, &mut s, &mut h)?;
            vec![
                a,
                c,
                d,
                bs.capacity,
                bs.live,
                bs.slots[0].pixel_bytes,
                bs.slots[1].pixel_bytes,
                h.allocs.len() as u32,
                b(bs.slots[0].pitch.is_none() && bs.slots[0].version.is_none()),
            ]
        }
        52 => {
            let mut bs = Buffers::default();
            let mut s = Serial::default();
            bs.create(0, &mut s, &mut h)?;
            let serial = bs.slots[0].serial;
            let a = bs.release(1, &mut h);
            let c = bs.valid(1);
            let d = bs.release(1, &mut h);
            let e = bs.create(0, &mut s, &mut h)?;
            vec![
                b(a),
                b(c),
                b(d),
                e,
                b(bs.valid(1)),
                b(serial != bs.slots[0].serial),
                b(!bs.valid(u32::MAX)),
                h.free.len() as u32,
            ]
        }
        53 => {
            let mut bs = Buffers::default();
            let mut s = Serial::default();
            for _ in 0..9 {
                bs.create(0, &mut s, &mut h)?;
            }
            for handle in [1, 3, 4, 6, 7, 8] {
                bs.release(handle, &mut h);
            }
            let mut order = vec![];
            h.reset();
            bs.clear_via(&mut h, |this, handle, host| {
                order.push(handle);
                this.release(handle, host);
            });
            order.extend([bs.storage, bs.capacity, bs.live, h.free.len() as u32]);
            order
        }
        54 => {
            h.fail.insert(704);
            let mut bs = Buffers::default();
            let mut s = Serial::default();
            let r = bs.create(0, &mut s, &mut h);
            vec![
                b(r == Err(Fault::NullWrite)),
                bs.capacity,
                bs.live,
                s.0,
                h.free.len() as u32,
            ]
        }
        55 => {
            strided_copy(&mut h, 100, 200, 8, 8, 8, 3);
            let mut v: Vec<u32> = h.args(Op::CopyEndpoint).into_iter().flatten().collect();
            v.push(h.targets(Op::CompletionEndpoint).len() as u32);
            h.reset();
            strided_copy(&mut h, 100, 200, 8, 12, 16, 3);
            v.extend(h.args(Op::CopyEndpoint).into_iter().flatten());
            v.push(h.targets(Op::CompletionEndpoint).len() as u32);
            h.reset();
            strided_copy(&mut h, 100, 200, 0, 8, 8, 3);
            v.push(h.calls.len() as u32);
            v
        }
        56 => {
            let mut bs = Buffers::default();
            let mut s = Serial::default();
            bs.create(0, &mut s, &mut h)?;
            h.reset();
            let r = bs.save(frame(), [1, 0, 0, 0, 2, 0, 0], &mut h)?;
            vec![
                b(r),
                bs.slots[0].version.unwrap(),
                bs.slots[0].pitch.unwrap(),
                h.allocs[0].0,
                b(h.ops() == [Op::Synchronize]),
                b(h.memory[&bs.slots[0].data].iter().all(|b| *b == 0)),
            ]
        }
        57 => {
            let mut bs = Buffers::default();
            let mut s = Serial::default();
            bs.create(0, &mut s, &mut h)?;
            h.fail.insert(16392);
            h.reset();
            let r = bs.save(frame(), [1, 0, 0, 0, 2, 0, 0], &mut h);
            vec![
                b(r == Err(Fault::NullWrite)),
                b(h.ops() == [Op::Synchronize]),
                h.free.len() as u32,
                b(bs.slots[0].version.is_none()),
            ]
        }
        58 => {
            let (mut bs, _, f) = prepared(&mut h);
            h.reset();
            let r = bs.restore(f, [1, 0, 0, 32, 32, 0, 0], &mut h)?;
            let mut v = vec![
                b(r),
                h.word(f.tile_base),
                h.word(f.tile_base + 4),
                h.word(f.tile_base + 16),
                h.word(f.tile_base + 20),
                h.writes.len() as u32,
            ];
            bs.slots[0].kind = 1;
            bs.slots[0].pixel_bytes = 2;
            h.reset();
            v.push(b(bs.restore(f, [1, 0, 0, 32, 32, 0, 0], &mut h)?));
            v.push(h.writes.len() as u32);
            v
        }
        59 => {
            let (bs, _, f) = prepared(&mut h);
            h.reset();
            bs.restore(f, [1, 1, 0, 31, 32, 1, 0], &mut h)?;
            vec![
                h.word(f.tile_base),
                h.word(f.tile_base + 4),
                h.word(f.tile_base + 16),
                h.word(f.tile_base + 20),
                h.writes.len() as u32,
            ]
        }
        60 => {
            let (bs, _, f) = prepared(&mut h);
            h.reset();
            let r = bs.restore(f, [1, -1, 0, 16, 16, 0, 0], &mut h)?;
            vec![
                b(r),
                h.args(Op::CopyEndpoint)[0][0],
                b(h.ops().first() == Some(&Op::Synchronize)),
            ]
        }
        61 => {
            let (mut bs, _, f) = prepared(&mut h);
            bs.slots[0].kind = 2;
            h.reset();
            let a = bs.save(f, [1, 0, 0, 0, 0, 0, 0], &mut h)?;
            let c = bs.restore(f, [1, 0, 0, 0, 0, 0, 0], &mut h)?;
            vec![
                b(a),
                b(c),
                bs.slots[0].version.unwrap(),
                b(h.ops() == [Op::Synchronize, Op::Synchronize]),
                h.writes.len() as u32,
            ]
        }
        62 => {
            let mut v = vec![];
            h.format = 8;
            let r = capture::software(frame(), [0, 0, 2, 2], 800, &mut h)?;
            v.extend([r, h.targets(Op::Release).len() as u32]);
            for init_failure in [true, false] {
                h.reset();
                h.format = 9;
                h.initialized = !init_failure;
                h.init_ok = 0;
                h.lock_ok = 0;
                let r = capture::software(frame(), [0, 0, 2, 2], 800, &mut h)?;
                v.extend([r, h.targets(Op::Release).len() as u32]);
            }
            v
        }
        63 => {
            let mut a: Sequence<ARecord> = Sequence::default();
            let mut x = ARecord::partial_default();
            x.key = Some(1);
            a.insert(0, 1, &x, &mut h)?;
            x.key = Some(2);
            a.insert(1, 1, &x, &mut h)?;
            x.key = Some(3);
            a.insert(2, 1, &x, &mut h)?;
            // Explicit public fixture capacity, not a dependence on the growth algorithm.
            a.capacity = 8;
            h.reset();
            a.insert_from(0, 2, 1, &mut h)?;
            let mut v: Vec<u32> = a.records.iter().map(|r| r.key.unwrap()).collect();
            v.extend([a.capacity, h.free.len() as u32]);
            a.records.truncate(3);
            a.capacity = 3;
            h.reset();
            a.insert(1, 5, &x, &mut h)?;
            v.extend([a.capacity, a.records.len() as u32, h.allocs[0].0, h.free.len() as u32]);
            v
        }
        64 => {
            let mut a = Aggregate::new(&mut h)?;
            let mut x = ARecord::partial_default();
            x.entries[0] = Some(777);
            a.a.resize(2, &x, &mut h)?;
            a.a.records[0].entries[0] = Some(555);
            h.reset();
            a.a.resize(1, &x, &mut h)?;
            let before = h.free.len();
            a.destroy(&mut h);
            vec![
                before as u32,
                b(h.free.contains(&555)),
                b(h.free.contains(&777)),
                b(a.a.records[0].entries == [Some(0); 9]),
                a.a.capacity,
            ]
        }
        65 => {
            let key = p.factory(0x8dcd30, &mut h)? - 8;
            let s = p.software.get_mut(&key).unwrap();
            s.buffers.create(0, &mut p.serial, &mut h)?;
            s.trailing_buffer = 111;
            s.strings = vec![222, 0, 333];
            s.string_storage = 444;
            s.tile_tags = 555;
            s.owned_buffer = 666;
            s.held_interface = 777;
            s.pool.descriptor_storage = 888;
            let arrays = [s.arrays.records, s.arrays.words];
            let buffers = s.buffers.storage;
            let aggregate = s.aggregate.a.storage;
            let pool = s.pool.base;
            h.reset();
            let r = s.delete(2, &mut h);
            vec![
                b(r == key),
                b(h.ops() == [Op::BranchStop, Op::Release, Op::ReferenceBaseCleanup]),
                b(h.free
                    == [
                        arrays[0], arrays[1], 0, buffers, 111, 222, 333, 444, 555, 666, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        aggregate, pool, 888,
                    ]),
                b(!h.free.contains(&key)),
                b(h.targets(Op::Release) == [777]),
            ]
        }
        66 => {
            let absent = extension_bool(0, &mut h);
            let present = extension_bool(1, &mut h);
            extension_one(2, 99, &mut h)?;
            extension_seven(3, [1, 2, 3, 4, 5, 6, 7], &mut h)?;
            vec![
                b(absent),
                b(present),
                b(h.ops() == [Op::ExtensionBool, Op::ExtensionOne, Op::ExtensionSeven]),
                b(extension_one(0, 1, &mut h) == Err(Fault::NullRead)),
                b(extension_five(0, [0; 7], &mut h) == Err(Fault::NullRead)),
                b(extension_seven(0, [0; 7], &mut h) == Err(Fault::NullRead)),
                b(h.args(Op::ExtensionSeven) == [vec![1, 2, 3, 4, 5, 6, 7]]),
            ]
        }
        67 => {
            lighting(0, &mut h);
            lighting(128, &mut h);
            light_enabled(u32::MAX, 255, &mut h);
            light(2, u32::MAX, 99, &mut h);
            light(2, 2, 99, &mut h);
            material(5, 99, &mut h);
            material(4, 0, &mut h);
            combined_material([11, 22, 33, 44], (-1f32).to_bits(), &mut h);
            vec![
                b(h.ops()
                    == [
                        Op::GlDisable,
                        Op::GlEnable,
                        Op::GlEnable,
                        Op::GlLight,
                        Op::GlMaterial,
                        Op::GlMaterial,
                        Op::GlMaterial,
                        Op::GlMaterial,
                        Op::GlMaterial,
                    ]),
                b(h.calls[2].args == [0x3fff]),
                b(h.args(Op::GlLight) == [vec![0x4002, 0x1202, 99]]),
                b(h.args(Op::GlMaterial)
                    == [
                        vec![0x404, 0x1601, 0],
                        vec![0x404, 0x1200, 11],
                        vec![0x404, 0x1201, 22],
                        vec![0x404, 0x1202, 33],
                        vec![0x404, 0x1600, 44],
                    ]),
            ]
        }
        68 => {
            let o = OpenGl::new(100, &mut h)?;
            let mut d = Descriptor {
                words: [7; 10],
                flags: 0x80,
                ..Descriptor::default()
            };
            let mut global = 0;
            o.descriptor(&mut d, &mut global);
            let mut v = vec![
                d.flags,
                d.words[4],
                d.words[5],
                d.words[3],
                global,
                b(d.name == Some(DESCRIPTOR_NAME)),
            ];
            o.callback_descriptor(&mut d);
            v.extend([
                d.words[0],
                d.words[1],
                d.words[8],
                b(d.name_word9 == Some(DESCRIPTOR_NAME)),
                o.class_id(),
                o.initial_matrix_word(0),
            ]);
            v
        }
        69 => {
            let mut v = vec![];
            for (entry, adjustment) in [
                (0x87e0b0, 4),
                (0x87e0c0, 4),
                (0x87e0d0, 8),
                (0x87e0e0, 8),
                (0x87e120, 16),
                (0x87e130, 16),
                (0x87e150, 20),
                (0x8dcbf0, 4),
                (0x8dcc60, 12),
                (0x8dcc70, 12),
            ] {
                let mut out = 123;
                let r = p.thunk(entry, 100 + adjustment, 9, &mut out, 0, &mut h)?;
                let c = h.calls.last().unwrap();
                v.extend([r, out, c.target, c.args.len() as u32]);
            }
            v
        }
        70 => {
            let mut a: Sequence<ARecord> = Sequence::default();
            let x = ARecord::partial_default();
            a.insert(0, 3, &x, &mut h)?;
            a.reallocate(1, 2, &x, 1, &mut h)?;
            let mut bs: Sequence<BRecord> = Sequence::default();
            let value = BRecord {
                owned: Some(555),
                opaque: [Some(9); 44],
            };
            bs.insert(0, 3, &value, &mut h)?;
            bs.capacity = 8;
            h.reset();
            bs.insert_from(2, 2, 0, &mut h)?;
            bs.reallocate(1, 5, &value, 0, &mut h)?;
            vec![
                a.records.len() as u32,
                a.capacity,
                bs.records.len() as u32,
                bs.capacity,
                h.allocs[0].0,
                b(!h.free.contains(&555)),
                b(x.values[8].is_none()),
                b(x.opaque.iter().all(Option::is_none)),
            ]
        }
        71 => {
            let mut r = Registry::new();
            for class in [1, 194, 387] {
                r.insert(Registration {
                    class,
                    factory: class,
                    kind: 1,
                })?;
            }
            let mut v: Vec<u32> = r.entries().map(|e| e.class).collect();
            v.push(b(!r.insert(Registration {
                class: 194,
                factory: 0,
                kind: 2,
            })?));
            v.push(r.find(194).unwrap().factory);
            let mut bs = Buffers::default();
            let mut s = Serial(u32::MAX);
            bs.create(0, &mut s, &mut h)?;
            v.push(bs.slots[0].serial);
            bs.create(0, &mut s, &mut h)?;
            v.push(bs.slots[1].serial);
            v.push(s.0);
            v
        }
        72 => {
            h.initialized = false;
            let a = capture::software(frame(), [1, 2, 2, 3], 0, &mut h)?;
            let copies = h.args(Op::CopyEndpoint);
            let mut v = vec![
                a,
                b(h.args(Op::ComBitmapCreate) == [vec![0xc470d325, 0x20732180]]),
                copies.len() as u32,
                copies[0][0],
                copies[0][1],
                copies[0][2],
                copies[1][0],
                copies[1][1],
                b(h.args(Op::BitmapStorageRequest) == [vec![256, 3]]),
                b(h.args(Op::BitmapUnlock) == [vec![0x8040]]),
            ];
            h.reset();
            h.initialized = true;
            h.lock_ok = 0;
            v.extend([
                capture::front([0, 0, 2, 2], 800, &mut h)?,
                h.targets(Op::Release).len() as u32,
                h.free.len() as u32,
            ]);
            v
        }
        73 => {
            let mut d = director(&mut h);
            let mut out = 123;
            d.register(88, 99, 7, &mut h)?;
            let a = d.get_class(88, 1, &mut out, &mut h, &mut p)?;
            let q = d.query_interface(1, None);
            let refs = d.refs();
            let path = d.library_path(0, &mut h);
            let pool = Pool::new(0, 1, &mut h);
            let (bs, _, f) = prepared(&mut h);
            h.reset();
            let save = bs.restore(f, [1, 0, 0, 1, 1, -1, 0], &mut h)?;
            vec![
                b(a),
                out,
                b(q == Err(Fault::NullWrite)),
                refs,
                b(path == Err(Fault::NullRead)),
                b(matches!(pool, Err(Fault::Division))),
                b(save),
                h.calls.len() as u32,
            ]
        }
        74 => {
            let gl = p.factory(0x87e1d0, &mut h)? - 12;
            let sw = p.factory(0x8dcd30, &mut h)? - 8;
            h.reset();
            let mut output = 123;
            let a = p.thunk(0x87e140, gl + 20, 9, &mut output, 2, &mut h)?;
            let c = p.thunk(0x8dcc50, sw + 12, 9, &mut output, 0, &mut h)?;
            let mut values = vec![
                b(a == gl),
                b(c == sw),
                output,
                b(!h.free.contains(&gl) && !h.free.contains(&sw)),
            ];
            let mut coefficients = Coefficients::default();
            let mut s = Software::new(999, true, &mut coefficients, &mut h)?;
            h.reset();
            values.extend([
                b(s.input),
                s.class_id(),
                s.finalize(&mut h),
                b(h.ops() == [Op::DriverSlot80, Op::BranchStop, Op::ReferenceBaseCleanup]),
                b(h.free.last() == Some(&999)),
            ]);
            values
        }
        75 => {
            strided_copy(&mut h, 100, 200, -1, 4, 4, 3);
            strided_copy(&mut h, 100, 200, 4, 4, 4, -1);
            let mut values = vec![h.calls.len() as u32];
            strided_copy(&mut h, 100, 200, 4, (-8i32) as u32, (-12i32) as u32, 2);
            values.extend(h.args(Op::CopyEndpoint).into_iter().flatten());
            values.push(h.targets(Op::CompletionEndpoint).len() as u32);
            h.reset();
            strided_copy(&mut h, 100, 200, 0x40000000, 0x40000000, 0x40000000, 4);
            values.extend(h.args(Op::CopyEndpoint).into_iter().flatten());
            values.push(h.targets(Op::CompletionEndpoint).len() as u32);
            values
        }
        76 => {
            let (mut bs, _, f) = prepared(&mut h);
            h.reset();
            let reject = bs.save(f, [1, -1, 0, 2, 2, 0, 0], &mut h)?;
            let no_calls = h.calls.len();
            bs.release(1, &mut h);
            h.reset();
            let inactive = bs.save(f, [1, 0, 0, 0, 0, 0, 0], &mut h)?;
            let version = bs.slots[0].version.unwrap();
            let bad = bs.save(f, [0, 0, 0, 0, 0, 0, 0], &mut h);
            bs.slots[0].data = 0;
            bs.slots[0].kind = 2;
            h.reset();
            let unsupported = bs.save(f, [1, 0, 0, 0, 0, 0, 0], &mut h)?;
            vec![
                b(reject),
                no_calls as u32,
                b(inactive),
                version,
                b(!bs.valid(1)),
                b(bad == Err(Fault::MissingSlot)),
                b(unsupported),
                bs.slots[0].version.unwrap(),
                h.allocs[0].0,
                b(h.ops() == [Op::Synchronize]),
            ]
        }
        77 => {
            let (mut bs, _, f) = prepared(&mut h);
            bs.slots[0].serial = u32::MAX;
            bs.slots[0].version = Some(0x1ff);
            h.reset();
            let result = bs.restore(f, [1, -1, -2, 16, 16, 17, 17], &mut h)?;
            let offsets: Vec<_> = h.writes.iter().map(|w| w.0 - f.tile_base).collect();
            let mut values = vec![b(result)];
            values.extend(
                h.writes[..4]
                    .iter()
                    .map(|w| u32::from_le_bytes(w.1.clone().try_into().unwrap())),
            );
            values.extend([
                h.writes.len() as u32,
                b(offsets == [20, 24, 36, 40, 20, 36, 24, 40, 20, 24, 36, 40]),
                h.word(f.tile_base + 20),
                h.word(f.tile_base + 24),
                h.word(f.tile_base + 36),
                h.word(f.tile_base + 40),
            ]);
            values
        }
        78 => {
            let mut a = Aggregate::new(&mut h)?;
            for (i, pointer) in a.a.records[0].entries.iter_mut().enumerate() {
                *pointer = Some(101 + i as u32);
            }
            a.b.insert(
                0,
                1,
                &BRecord {
                    owned: Some(201),
                    opaque: [None; 44],
                },
                &mut h,
            )?;
            a.b.insert(
                1,
                1,
                &BRecord {
                    owned: Some(202),
                    opaque: [None; 44],
                },
                &mut h,
            )?;
            let a_storage = a.a.storage;
            let b_storage = a.b.storage;
            h.reset();
            a.destroy(&mut h);
            let mut expected = vec![201, 202];
            expected.extend(101..=109);
            expected.extend([b_storage, a_storage]);
            let mut values = vec![
                b(h.free == expected),
                b(a.a.records[0].entries == [Some(0); 9]),
                a.a.capacity,
                a.b.capacity,
                b(a.b.records[0].owned == Some(201)),
                b(a.a.storage == a_storage && a.b.storage == b_storage),
            ];
            let mut sequence: Sequence<ARecord> = Sequence::default();
            let fill = ARecord::partial_default();
            sequence.insert(0, 1, &fill, &mut h)?;
            let old = sequence.storage;
            h.reset();
            h.fail.insert(612);
            let failure = sequence.insert(1, 2, &fill, &mut h);
            values.extend([
                b(failure == Err(Fault::NullWrite)),
                b(sequence.storage == old),
                sequence.records.len() as u32,
                sequence.capacity,
                h.free.len() as u32,
            ]);
            values
        }
        79 => {
            let mut registry = Registry::new();
            for class in 0..193 {
                registry.insert(Registration {
                    class,
                    factory: class,
                    kind: 1,
                })?;
            }
            let failure = registry.insert(Registration {
                class: 193,
                factory: 99,
                kind: 1,
            });
            let mut bs = Buffers::default();
            let mut serial = Serial::default();
            bs.create(0, &mut serial, &mut h)?;
            h.reset();
            let validity_calls = std::cell::Cell::new(0);
            let result = bs.release_via(1, &mut h, |_, _| {
                validity_calls.set(validity_calls.get() + 1);
                false
            });
            let mut releases = vec![];
            bs.clear_via(&mut h, |_, handle, _| releases.push(handle));
            vec![
                b(failure == Err(Fault::UnspecifiedGrowth)),
                registry.entries().count() as u32,
                b(registry.find(193).is_none()),
                b(result),
                validity_calls.get(),
                b(releases == [1]),
                bs.live,
                bs.capacity,
                bs.storage,
                h.free.len() as u32,
            ]
        }
        80 => {
            let mut d = director(&mut h);
            d.add_director(10, &mut h);
            h.lookup.insert(10, (256, 55));
            h.unload.insert(10, 256);
            h.qi = 256;
            h.qi_write = None;
            h.reset();
            let mut out = 123;
            let local = d.get_class(OPENGL_CLASS, 1, &mut out, &mut h, &mut p)?;
            let unload = d.can_unload(&mut h);
            let mut values = vec![
                b(local),
                out,
                b(unload),
                b(h.ops()
                    == [
                        Op::ChildLookup,
                        Op::QueryInterface,
                        Op::AddRef,
                        Op::Release,
                        Op::ChildCanUnload,
                    ]),
            ];
            d.register(88, 99, 2, &mut h)?;
            h.reset();
            let alternate = d.get_class(88, 1, &mut out, &mut h, &mut p)?;
            values.extend([b(alternate), b(h.ops() == [Op::ChildLookup, Op::InterfaceFactory]), out]);
            values
        }
        _ => return Err(Fault::MissingSlot),
    };
    Ok(result)
}
pub(crate) fn exec(name: &str, op: &Json) -> Option<R<()>> {
    if name != "gd_paper" {
        return None;
    }
    Some((|| {
        let id = op.get("id").and_then(Json::as_i64).ok_or("missing gd id")?;
        let id = u32::try_from(id).map_err(|_| "gd id outside u32")?;
        let actual = scenario(id).map_err(|e| format!("GD{id}: unexpected fault {e:?}"))?;
        let actual = Json::Array(actual.into_iter().map(|n| Json::Int(i64::from(n))).collect());
        let expected = op.get("expect").ok_or("missing gd expectation")?;
        if &actual == expected {
            Ok(())
        } else {
            Err(format!("GD{id}: expected {expected:?}, got {actual:?}"))
        }
    })())
}

#[cfg(test)]
mod tests {
    #[test]
    fn rejects_changed_gd_expectation() {
        let vector = r#"{"format":1,"name":"gd-negative","steps":[{"op":"gd_paper","id":17,"expect":[1,0,0]}]}"#;
        assert!(crate::run_vector_text(vector).unwrap_err().contains("GD17"));
    }
}
