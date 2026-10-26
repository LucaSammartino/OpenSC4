// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::stream::fc4_bytes_sink;

pub(super) struct Reader<'a>(pub &'a [u8]);
impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        if n > self.0.len() {
            return Err("truncated trace".into());
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    pub(super) fn end(&self) -> Result<(), String> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err("trailing trace bytes".into())
        }
    }
}
pub(super) trait Wire: Sized {
    fn put(&self, b: &mut Vec<u8>);
    fn get(r: &mut Reader<'_>) -> Result<Self, String>;
}
macro_rules! number {
    ($($t:ty),*) => { $(impl Wire for $t {
        fn put(&self,b:&mut Vec<u8>){ b.extend_from_slice(&self.to_le_bytes()); }
        fn get(r:&mut Reader<'_>)->Result<Self,String>{ Ok(Self::from_le_bytes(r.take(std::mem::size_of::<Self>())?.try_into().map_err(|_|"number size")?)) }
    })* };
}
number!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);
impl Wire for usize {
    fn put(&self, b: &mut Vec<u8>) {
        (*self as u64).put(b);
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        u64::get(r)?.try_into().map_err(|_| "length overflow".into())
    }
}
impl Wire for bool {
    fn put(&self, b: &mut Vec<u8>) {
        (*self as u8).put(b);
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        match u8::get(r)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("invalid bool".into()),
        }
    }
}
impl Wire for () {
    fn put(&self, _: &mut Vec<u8>) {}
    fn get(_: &mut Reader<'_>) -> Result<Self, String> {
        Ok(())
    }
}
impl<T: Wire> Wire for Vec<T> {
    fn put(&self, b: &mut Vec<u8>) {
        self.len().put(b);
        for v in self {
            v.put(b);
        }
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        let n = usize::get(r)?;
        if n > r.0.len() || n > 64 * 1024 * 1024 {
            return Err("invalid collection length".into());
        }
        (0..n).map(|_| T::get(r)).collect()
    }
}
impl<T: Wire, const N: usize> Wire for [T; N] {
    fn put(&self, b: &mut Vec<u8>) {
        for v in self {
            v.put(b);
        }
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        (0..N)
            .map(|_| T::get(r))
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| "array length".into())
    }
}
impl<T: Wire> Wire for Option<T> {
    fn put(&self, b: &mut Vec<u8>) {
        self.is_some().put(b);
        if let Some(v) = self {
            v.put(b);
        }
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        if bool::get(r)? {
            Ok(Some(T::get(r)?))
        } else {
            Ok(None)
        }
    }
}
impl Wire for String {
    fn put(&self, b: &mut Vec<u8>) {
        self.as_bytes().to_vec().put(b);
    }
    fn get(r: &mut Reader<'_>) -> Result<Self, String> {
        String::from_utf8(Vec::<u8>::get(r)?).map_err(|_| "invalid UTF-8".into())
    }
}
pub(super) fn packed<T: Wire>(v: &T) -> Vec<u8> {
    let mut b = Vec::new();
    v.put(&mut b);
    b
}

pub(super) unsafe fn put_ptr<T: Wire>(p: *const T, n: usize, b: &mut Vec<u8>) {
    (!p.is_null()).put(b);
    if !p.is_null() {
        for v in std::slice::from_raw_parts(p, n) {
            v.put(b);
        }
    }
}
pub(super) unsafe fn get_ptr<T: Wire>(p: *mut T, n: usize, r: &mut Reader<'_>) -> Result<(), String> {
    let present = bool::get(r)?;
    if present == p.is_null() {
        return Err("callback output pointer presence changed".into());
    }
    if present {
        for i in 0..n {
            p.add(i).write(T::get(r)?);
        }
    }
    Ok(())
}
unsafe fn put_raw(p: *const c_void, mem: u16, n: usize, b: &mut Vec<u8>) {
    let width = match mem {
        1..=3 | 12 | 14 => 1,
        4 | 5 | 13 => 2,
        6 | 7 | 10 => 4,
        8 | 9 | 11 => 8,
        _ => 0,
    };
    put_ptr(p as *const u8, n.saturating_mul(width), b);
}
unsafe fn put_props(p: *const crate::props::fc4_props, b: &mut Vec<u8>) {
    (!p.is_null()).put(b);
    if let Some(h) = crate::props::props_ref(p) {
        let mut out = fc4_core::stream::ByteWriter::new();
        h.write(&mut out);
        out.into_bytes().put(b);
    }
}
fn read_props(r: &mut Reader<'_>) -> Result<Option<fc4_core::property::PropertyHolder>, String> {
    if !bool::get(r)? {
        return Ok(None);
    }
    let bytes = Vec::<u8>::get(r)?;
    let mut h = fc4_core::property::PropertyHolder::new();
    if !h.read(&mut fc4_core::stream::ByteReader::new(&bytes)) {
        return Err("invalid trace property holder".into());
    }
    Ok(Some(h))
}
unsafe fn set_props(p: *mut crate::props::fc4_props, r: &mut Reader<'_>) -> Result<(), String> {
    let h = read_props(r)?;
    if h.is_some() == p.is_null() {
        return Err("property holder presence changed".into());
    }
    if let Some(h) = h {
        *(p as *mut fc4_core::property::PropertyHolder) = h;
    }
    Ok(())
}
fn new_props(r: &mut Reader<'_>) -> Result<*mut crate::props::fc4_props, String> {
    Ok(read_props(r)?.map_or(std::ptr::null_mut(), |h| Box::into_raw(Box::new(h)) as _))
}

struct Tap<'a, T, C> {
    values: &'a mut Vec<T>,
    callback: C,
    user: *mut c_void,
}
unsafe extern "C" fn tap_tokens(p: *mut c_void, v: u64) {
    let t = &mut *(p as *mut Tap<'_, u64, crate::budget::fc4_token_callback>);
    t.values.push(v);
    if let Some(f) = t.callback {
        f(t.user, v);
    }
}
unsafe extern "C" fn tap_lots(p: *mut c_void, v: *const crate::budget::fc4_lot_info) {
    let t = &mut *(p as *mut Tap<'_, crate::budget::fc4_lot_info, crate::budget::fc4_lot_visitor>);
    if let Some(v) = v.as_ref() {
        t.values.push(*v);
    }
    if let Some(f) = t.callback {
        f(t.user, v);
    }
}
unsafe extern "C" fn tap_deals(p: *mut c_void, v: *const crate::power::fc4_power_deal) {
    let t = &mut *(p as *mut Tap<'_, crate::power::fc4_power_deal, crate::power::fc4_power_deal_callback>);
    if let Some(v) = v.as_ref() {
        t.values.push(*v);
    }
    if let Some(f) = t.callback {
        f(t.user, v);
    }
}
unsafe extern "C" fn tap_bytes(p: *mut c_void, v: *const u8, n: usize) {
    let t = &mut *(p as *mut Tap<'_, Vec<u8>, crate::stream::fc4_bytes_sink>);
    t.values.push(if v.is_null() {
        Vec::new()
    } else {
        std::slice::from_raw_parts(v, n).to_vec()
    });
    if let Some(f) = t.callback {
        f(t.user, v, n);
    }
}

include!(concat!(env!("OUT_DIR"), "/trace_hosts.rs"));
