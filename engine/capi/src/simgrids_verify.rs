// SPDX-License-Identifier: GPL-3.0-or-later
//! Private, stateless verification transport. Never calls host grids. Each call
//! reconstructs an isolated Rust grid from the shadow's *retained* record;
//! native output never replaces that record after the single bootstrap.
use fc4_core::grid::{CityDims, GridElem, SimGrid, TractGrid};
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, SaveObject};

fn run<T: GridElem>(record: &[u8], op: u32, a: &[i32], data: &[u8]) -> Option<Vec<u8>> {
    let g = SimGrid::<T>::new();
    let mut input = ByteReader::new(record);
    if !g.read(&mut input) || input.position() != record.len() {
        return None;
    }
    let geom = g.geometry();
    let mut out = ByteWriter::new();
    match op {
        0 => {}, // Bootstrap roundtrip is validation, never an algorithm comparison.
        1 | 2 | 3 | 4 => {
            let (x, z, w, h) = (a[0], a[1], a[2], a[3]);
            let (tx, tz) = if op == 2 || op == 3 {
                geom.cell_to_tract(x, z)
            } else { (x, z) };
            let (ex, ez) = if op == 3 {
                geom.cell_to_tract(x.checked_add(w)?.checked_sub(1)?, z.checked_add(h)?.checked_sub(1)?)
            } else if op == 4 { (x.checked_add(w)?.checked_sub(1)?, z.checked_add(h)?.checked_sub(1)?) }
            else { (tx, tz) };
            if !geom.in_bounds(tx, tz) || !geom.in_bounds(ex, ez) || ((op == 3 || op == 4) && (w <= 0 || h <= 0)) { return None; }
            let v = match op {
                1 => g.get(x, z),
                2 => g.get_cell_value(x, z),
                3 => g.average_in_cell_rect(x, z, w, h),
                _ => g.average_in_tract_rect(x, z, w, h),
            };
            v.put(&mut out);
            return Some(out.into_bytes());
        },
        5 => { // Exact typed scalar Set.
            if !geom.in_bounds(a[0], a[1]) { return None; }
            let v = T::get(&mut ByteReader::new(data))?;
            g.set(a[0], a[1], v);
        },
        6 => g.fill(T::get(&mut ByteReader::new(data))?),
        7 => { // City dimensions supplied through public city getters only.
            if data.len() != 16 { return None; }
            let i = |n| i32::from_le_bytes(data[n..n+4].try_into().unwrap());
            let f = |n| f32::from_le_bytes(data[n..n+4].try_into().unwrap());
            g.set_tract_size(&CityDims { cells_x: i(0), cells_z: i(4), cell_width_x: f(8), cell_width_z: f(12) }, a[0]);
        },
        8 => { g.init(); },
        9 => { g.shutdown(); },
        10 => { g.set_instance_id(a[0] as u32); },
        11 => { // Ordered, typed bulk SetTractValue calls.
            let size = std::mem::size_of::<T>();
            if data.len() % (4 + size) != 0 { return None; }
            for item in data.chunks_exact(4 + size) {
                let index = u32::from_le_bytes(item[..4].try_into().unwrap()) as usize;
                if index >= geom.len() { return None; }
                let x = index / geom.count_z as usize;
                let z = index % geom.count_z as usize;
                g.set(x as i32, z as i32, T::get(&mut ByteReader::new(&item[4..]))?);
            }
        },
        12 => { // All public geometry outputs; no SDK's meaningless bool returns.
            for v in [geom.tract_size, geom.shift, geom.count_x, geom.count_z] { out.put_i32(v); }
            for v in [geom.width_x, geom.width_z, geom.inv_width_x, geom.inv_width_z] { out.put_f32(v); }
            return Some(out.into_bytes());
        },
        _ => return None,
    }
    if !g.write(&mut out) { return None; }
    Some(out.into_bytes())
}

/// Internal static-link bridge, deliberately absent from fc4.h and the public
/// ABI table. Buffers are borrowed only for this call; no Rust handle escapes.
/// Returns byte length, or -1 for refusal/panic. No fallback or resync.
#[no_mangle]
pub unsafe extern "C" fn fc4_simgrids_verify(
    kind: u32, op: u32, record: *const u8, len: u32, args: *const i32,
    data: *const u8, data_len: u32, output: *mut u8, capacity: u32,
) -> i32 {
    if record.is_null() || args.is_null() || output.is_null() || len > 64 * 1024 * 1024 || (data_len != 0 && data.is_null()) { return -1; }
    std::panic::catch_unwind(|| {
        let r = std::slice::from_raw_parts(record, len as usize);
        let a = std::slice::from_raw_parts(args, 4);
        let d = if data_len == 0 { &[] } else { std::slice::from_raw_parts(data, data_len as usize) };
        let result = match kind {
            0x49B9E602 => run::<u8>(r, op, a, d),
            0x49B9E603 => run::<i8>(r, op, a, d),
            0x49B9E604 => run::<u16>(r, op, a, d),
            0x49B9E605 => run::<i16>(r, op, a, d),
            0x49B9E606 => run::<u32>(r, op, a, d),
            0x49B9E607 => run::<i32>(r, op, a, d),
            0x49B9E608 => run::<u64>(r, op, a, d),
            0x49B9E609 => run::<i64>(r, op, a, d),
            0x49B9E60A => run::<f32>(r, op, a, d),
            0x49B9E60B => run::<f64>(r, op, a, d),
            _ => None,
        };
        match result {
            Some(b) if b.len() <= capacity as usize => {
                std::ptr::copy_nonoverlapping(b.as_ptr(), output, b.len());
                b.len() as i32
            },
            _ => -1,
        }
    }).unwrap_or(-1)
}
