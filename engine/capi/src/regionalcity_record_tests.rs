// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::regionalcity_owner::{fc4_regionalcity_owner_create, fc4_regionalcity_owner_destroy};
use crate::stream::{fc4_bytes_sink, fc4_ref};
use fc4_core::neighbors::{NeighborConnection, NeighborDeal};
use fc4_core::regional_city::RegionalCity;
use fc4_core::stream::{ByteReader, ByteWriter};
use std::{cell::Cell, rc::Rc};

struct Element {
    refs: Cell<usize>,
    live: Rc<Cell<usize>>,
    connection: Option<Rc<NeighborConnection>>,
    deal: Option<Rc<NeighborDeal>>,
}
unsafe extern "C" fn retain(p: *mut c_void) {
    let e = &*p.cast::<Element>();
    e.refs.set(e.refs.get() + 1);
}
unsafe extern "C" fn release(p: *mut c_void) {
    let e = &*p.cast::<Element>();
    e.refs.set(e.refs.get() - 1);
    if e.refs.get() == 0 {
        e.live.set(e.live.get() - 1);
        drop(Box::from_raw(p.cast::<Element>()));
    }
}
unsafe extern "C" fn word(_: *mut c_void) -> u32 {
    0
}
unsafe extern "C" fn amount(_: *mut c_void) -> i32 {
    0
}
unsafe extern "C" fn pair(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = 0;
    *z = 0;
}
unsafe extern "C" fn set_pair(_: *mut c_void, _: i32, _: i32) {}
unsafe extern "C" fn repair(_: *mut c_void, _: i32, _: i32, _: i32, _: i32, _: i32, _: i32) {}
unsafe extern "C" fn flags(_: *mut c_void, _: u32, _: bool) -> bool {
    true
}
unsafe extern "C" fn read(ctx: *mut c_void, kind: u32, stream: *const fc4_istream) -> *mut c_void {
    let live = &*ctx.cast::<Rc<Cell<usize>>>();
    let mut inp = CIn {
        stream: &*stream,
        decode: |_, _| None,
    };
    let (connection, deal) = if kind == 0 {
        let c = NeighborConnection::new();
        if !c.read(&mut inp) {
            return std::ptr::null_mut();
        }
        (Some(c), None)
    } else {
        let d = NeighborDeal::new();
        if !d.read(&mut inp) {
            return std::ptr::null_mut();
        }
        (None, Some(d))
    };
    live.set(live.get() + 1);
    Box::into_raw(Box::new(Element {
        refs: Cell::new(1),
        live: live.clone(),
        connection,
        deal,
    }))
    .cast()
}
unsafe extern "C" fn write(_: *mut c_void, kind: u32, p: *mut c_void, stream: *const fc4_ostream) -> i32 {
    let e = &*p.cast::<Element>();
    let mut out = COut(&*stream);
    i32::from(if kind == 0 {
        e.connection.as_ref().unwrap().write(&mut out)
    } else {
        e.deal.as_ref().unwrap().write(&mut out)
    })
}
fn calls(live: &mut Rc<Cell<usize>>) -> RecordCallbacks {
    RecordCallbacks {
        context: (live as *mut Rc<Cell<usize>>).cast(),
        read,
        write,
        connections: ConnectionCallbacks {
            retain,
            release,
            conn_type: word,
            cell: pair,
            set_cell: set_pair,
            repair,
        },
        deals: DealCallbacks {
            retain,
            release,
            city_side: pair,
            other_side: pair,
            set_city_side: set_pair,
            set_other_side: set_pair,
            deal_type: word,
            amount,
            flags: word,
            set_flags: flags,
        },
    }
}
macro_rules! primitive {
    ($get:ident, $put:ident, $ty:ty) => {
        unsafe extern "C" fn $get(p: *mut c_void, out: *mut $ty) -> bool {
            match (&mut *p.cast::<ByteReader<'_>>()).$get() {
                Some(v) => {
                    *out = v;
                    true
                }
                None => false,
            }
        }
        unsafe extern "C" fn $put(p: *mut c_void, v: $ty) -> bool {
            (&mut *p.cast::<ByteWriter>()).$put(v)
        }
    };
}
primitive!(get_u8, put_u8, u8);
primitive!(get_u16, put_u16, u16);
primitive!(get_u32, put_u32, u32);
primitive!(get_u64, put_u64, u64);
primitive!(get_f32, put_f32, f32);
primitive!(get_f64, put_f64, f64);
unsafe extern "C" fn get_bytes(p: *mut c_void, out: *mut u8, len: usize) -> bool {
    let Some(b) = (&mut *p.cast::<ByteReader<'_>>()).get_bytes(len) else {
        return false;
    };
    std::ptr::copy_nonoverlapping(b.as_ptr(), out, len);
    true
}
unsafe extern "C" fn get_str(p: *mut c_void, sink: fc4_bytes_sink, ctx: *mut c_void) -> bool {
    let Some(b) = (&mut *p.cast::<ByteReader<'_>>()).get_str() else {
        return false;
    };
    sink.unwrap()(ctx, b.as_ptr(), b.len());
    true
}
unsafe extern "C" fn get_ref(_: *mut c_void, _: u32, _: *mut fc4_ref) -> bool {
    false
}
unsafe extern "C" fn skip(p: *mut c_void, n: usize) -> bool {
    (&mut *p.cast::<ByteReader<'_>>()).skip(n)
}
unsafe extern "C" fn in_error(p: *mut c_void) -> i32 {
    (&*p.cast::<ByteReader<'_>>()).error()
}
unsafe extern "C" fn put_bytes(p: *mut c_void, b: *const u8, n: usize) -> bool {
    (&mut *p.cast::<ByteWriter>()).put_bytes(std::slice::from_raw_parts(b, n))
}
unsafe extern "C" fn put_str(p: *mut c_void, b: *const u8, n: usize) -> bool {
    (&mut *p.cast::<ByteWriter>()).put_str(std::slice::from_raw_parts(b, n))
}
unsafe extern "C" fn put_ref(_: *mut c_void, _: *const fc4_ref) -> bool {
    false
}
unsafe extern "C" fn out_error(p: *mut c_void) -> i32 {
    (&*p.cast::<ByteWriter>()).error()
}
fn input(b: &mut ByteReader<'_>) -> fc4_istream {
    fc4_istream {
        struct_size: std::mem::size_of::<fc4_istream>() as u32,
        user: (b as *mut ByteReader<'_>).cast(),
        get_u8: Some(get_u8),
        get_u16: Some(get_u16),
        get_u32: Some(get_u32),
        get_u64: Some(get_u64),
        get_f32: Some(get_f32),
        get_f64: Some(get_f64),
        get_bytes: Some(get_bytes),
        get_str: Some(get_str),
        get_ref: Some(get_ref),
        skip: Some(skip),
        error: Some(in_error),
    }
}
fn output(b: &mut ByteWriter) -> fc4_ostream {
    fc4_ostream {
        struct_size: std::mem::size_of::<fc4_ostream>() as u32,
        user: (b as *mut ByteWriter).cast(),
        put_u8: Some(put_u8),
        put_u16: Some(put_u16),
        put_u32: Some(put_u32),
        put_u64: Some(put_u64),
        put_f32: Some(put_f32),
        put_f64: Some(put_f64),
        put_bytes: Some(put_bytes),
        put_str: Some(put_str),
        put_ref: Some(put_ref),
        error: Some(out_error),
    }
}
#[test]
fn host_record_roundtrip_and_every_truncation_release_owned_elements() {
    let mut source = RegionalCity::new(2);
    source.city_name = b"A\0B".to_vec();
    source.connections = vec![NeighborConnection::with(2, -1, 5), NeighborConnection::with(9, 64, 10)];
    source.other_deals.push(NeighborDeal::new());
    source.local_deals.push(NeighborDeal::new());
    source.points.insert(1, vec![[1.0, -2.0, 3.0]]);
    let mut wire = ByteWriter::new();
    assert!(source.write_record(&mut wire));
    let wire = wire.bytes();
    let mut live = Rc::new(Cell::new(0));
    let callbacks = calls(&mut live);
    unsafe {
        let owner = fc4_regionalcity_owner_create(1);
        let mut reader = ByteReader::new(wire);
        assert!(fc4_regionalcity_owner_decode(owner, &input(&mut reader), 1, &callbacks));
        assert_eq!(reader.remaining(), 0);
        assert_eq!(live.get(), 4);
        let mut roundtrip = ByteWriter::new();
        assert!(fc4_regionalcity_owner_write(owner, &output(&mut roundtrip), &callbacks));
        assert_eq!(roundtrip.bytes(), wire);
        for n in 0..wire.len() {
            let mut reader = ByteReader::new(&wire[..n]);
            assert!(
                !fc4_regionalcity_owner_decode(owner, &input(&mut reader), 1, &callbacks),
                "prefix {n}"
            );
            assert_eq!(live.get(), 4, "prefix {n} leaked an element or replaced state");
        }
        let mut reader = ByteReader::new(wire);
        assert!(fc4_regionalcity_owner_decode(owner, &input(&mut reader), 1, &callbacks));
        assert_eq!(live.get(), 4, "successful replacement releases old elements");
        fc4_regionalcity_owner_destroy(owner);
        assert_eq!(live.get(), 0);
    }
}
