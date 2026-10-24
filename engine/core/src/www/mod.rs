//! Portable partial WWW/FTP owner, from `docs/specs/web-network*.md` only.
//!
//! No native glue, default replacement or HTTP transport.
//! `Unsupported` is a contract boundary, never a native false return. Reference
//! effects are explicit: dropping Rust storage must not silently balance native leaks.

pub mod cache;
pub mod cookies;
pub mod ftp;
pub mod http;
pub mod http_date;
pub mod http_wire;
pub mod listing;
pub mod notification;
pub mod service;
pub mod worker;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unsupported {
    NativeInputDomain(&'static str),
    OwnedBody(&'static str),
    PrimitiveContract(&'static str),
}

pub type Contract<T> = Result<T, Unsupported>;

/// 00823000: one decoding pass over length-bearing bytes (including embedded NUL).
pub fn percent_decode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%' && i + 2 < input.len() {
            let hex = |b: u8| match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(b - b'a' + 10),
                b'A'..=b'F' => Some(b - b'A' + 10),
                _ => None,
            };
            out.push(match (hex(input[i + 1]), hex(input[i + 2])) {
                (Some(a), Some(b)) => a * 16 + b,
                _ => b' ',
            });
            i += 3;
        } else {
            out.push(input[i]);
            i += 1;
        }
    }
    out
}

/// 00823310. Copy semantics also permit callers to replace their original buffer.
pub fn native_path(input: &[u8]) -> Vec<u8> {
    let mut copy = input.to_vec();
    if copy.len() >= 3 && copy[0] == b'/' && copy[2] == b'|' {
        copy[2] = b':';
        copy.remove(0);
    }
    percent_decode(&copy)
        .into_iter()
        .map(|b| if b == b'/' { b'\\' } else { b })
        .collect()
}

/// 00812b10: narrow Windows Splitpath filename + extension, no URL decoding.
pub fn leaf(path: &[u8]) -> &[u8] {
    let start = path
        .iter()
        .rposition(|b| matches!(b, b'/' | b'\\'))
        .map_or(if path.get(1) == Some(&b':') { 2 } else { 0 }, |n| n + 1);
    &path[start..]
}

/// 008f736e: classification uses the URL path extension, not a MIME string.
pub fn path_preference(path: &[u8]) -> i32 {
    let name = leaf(path);
    let extension = name.iter().rposition(|b| *b == b'.').map(|i| &name[i..]);
    if extension.is_some_and(|ext| {
        [b".html".as_slice(), b".htm", b".shtml", b".asp", b".plg"]
            .iter()
            .any(|known| ext.eq_ignore_ascii_case(known))
    }) {
        1
    } else {
        3
    }
}

pub fn directory(path: &[u8]) -> Vec<u8> {
    let mut out = path.to_vec();
    if out.last().is_some_and(|b| !matches!(b, b'/' | b'\\')) {
        out.push(b'\\');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_native_path_vectors() {
        for (input, expected) in [
            (b"/C|/a%20b".as_slice(), b"C:\\a b".as_slice()),
            (b"/1|/x", b"1:\\x"),
            (b"a%ZZb%2", b"a b%2"),
            (b"a%252Fb+c%2F", b"a%2Fb+c\\"),
            (b"%00+", b"\0+"),
        ] {
            assert_eq!(native_path(input), expected);
        }
        assert_eq!(leaf(b"C:dir/a.bin"), b"a.bin");
        assert_eq!(leaf(b"out/"), b"");
        assert_eq!(path_preference(b"x.HTML"), 1);
        assert_eq!(path_preference(b"x.html.bin"), 3);
        assert_eq!(directory(b""), b"");
        assert_eq!(directory(b"out/"), b"out/");
    }
}
