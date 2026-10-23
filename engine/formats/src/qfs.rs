// SPDX-License-Identifier: GPL-3.0-or-later
//! QFS (RefPack) compression as stored in SimCity 4 packages.
//!
//! A compressed DBPF entry starts with a 9-byte header:
//!
//! ```text
//! u32 LE   compressed size (the whole entry, header included)
//! u8, u8   0x10 0xFB       RefPack flags and signature
//! u24 BE   uncompressed size
//! ```
//!
//! followed by a stream of control codes. Each code may copy 0..=3 (or 4..=112) literal bytes from the input
//! and then copy a run from earlier output (an LZ77 back-reference; runs may overlap their own output):
//!
//! | first byte | bytes | literals | copy length | copy distance |
//! |---|---|---|---|---|
//! | `0x00..=0x7F` | 2 | `b0 & 3` | `((b0 >> 2) & 7) + 3` (3..=10) | `((b0 & 0x60) << 3) + b1 + 1` (1..=1024) |
//! | `0x80..=0xBF` | 3 | `b1 >> 6` | `(b0 & 0x3F) + 4` (4..=67) | `((b1 & 0x3F) << 8) + b2 + 1` (1..=16384) |
//! | `0xC0..=0xDF` | 4 | `b0 & 3` | `((b0 & 0x0C) << 6) + b3 + 5` (5..=1028) | `((b0 & 0x10) << 12) + (b1 << 8) + b2 + 1` (1..=131072) |
//! | `0xE0..=0xFB` | 1 | `((b0 & 0x1F) << 2) + 4` (4..=112) | none | none |
//! | `0xFC..=0xFF` | 1 | `b0 & 3`, then end of stream | none | none |
//!
//! Acceptance *\[compat\]* (data-formats §2.7): the original accepts **only** the two signatures `10 FB` and
//! `11 FB`. `0x11` inserts three unused bytes between the signature and the uncompressed size; the size is
//! always a 24-bit big-endian field. There is no `0x80` four-byte-size form and no `0x01` compressed-size
//! field inside the RefPack header (the DBPF entry carries the compressed length as its own 4-byte prefix).
//! The stored entry is at least 7 bytes, and the 4-byte prefix is at most `0x0100_0000` (16 MiB); the 24-bit
//! uncompressed size can never exceed 16 MiB. Anything else is rejected before decoding.

use crate::error::{corrupt, Error, Result};

/// Narrow adapter for the portable persistence runtime. Resource header,
/// format dispatch, decoded-size validation and row layout stay in core.
pub struct PersistenceQfs;
impl fc4_core::persistence_runtime::ImageCodec for PersistenceQfs {
    fn decompress(&self, bytes: &[u8]) -> Option<Vec<u8>> {
        decompress(bytes).ok()
    }
    fn compress(&self, bytes: &[u8]) -> Option<Vec<u8>> {
        compress(bytes).ok()
    }
}

/// Length of the shortest SimCity 4 QFS header (4-byte prefix, 2-byte signature, 3-byte uncompressed size).
/// The `11 FB` form is 3 bytes longer (see [`header_len`]).
pub const HEADER_LEN: usize = 9;

/// Largest uncompressed size the 3-byte header field can express.
pub const MAX_UNCOMPRESSED: usize = (1 << 24) - 1;

/// Largest accepted 4-byte compressed-length prefix (data-formats §2.7: 16 MiB).
pub const MAX_PREFIX: u32 = 0x0100_0000;

const MAX_DISTANCE: usize = 131_072;
const MAX_COPY: usize = 1028;

/// True if `data` passes the §2.7 acceptance check: at least 7 bytes, signature `10 FB` or `11 FB`, and a
/// 4-byte prefix of at most 16 MiB. This is the gate the original applies before decoding a DIR-listed entry.
pub fn has_header(data: &[u8]) -> bool {
    data.len() >= 7
        && (data[4] == 0x10 || data[4] == 0x11)
        && data[5] == 0xFB
        && u32::from_le_bytes([data[0], data[1], data[2], data[3]]) <= MAX_PREFIX
}

/// Total header length in bytes for an accepted entry: 9 for `10 FB`, 12 for `11 FB` (3 unused bytes).
fn header_len(flags: u8) -> usize {
    if flags == 0x11 {
        12
    } else {
        HEADER_LEN
    }
}

fn be24(b: &[u8]) -> usize {
    (usize::from(b[0]) << 16) | (usize::from(b[1]) << 8) | usize::from(b[2])
}

/// Parses an accepted DBPF-form QFS header (with the 4-byte prefix). Returns the uncompressed size and the
/// header length; the caller has already checked [`has_header`].
fn parse_dbpf_header(data: &[u8]) -> Result<(usize, usize)> {
    let flags = data[4];
    let len = header_len(flags);
    if data.len() < len {
        return Err(Error::Truncated {
            what: "QFS header",
            needed: len as u64,
            available: data.len() as u64,
        });
    }
    // `10 FB`: size at 6..9. `11 FB`: three unused bytes at 6..9, size at 9..12.
    let size = if flags == 0x11 {
        be24(&data[9..12])
    } else {
        be24(&data[6..9])
    };
    Ok((size, len))
}

/// The uncompressed size announced by a SimCity 4 QFS header, if `data` has an accepted one.
pub fn uncompressed_size(data: &[u8]) -> Option<usize> {
    if !has_header(data) {
        return None;
    }
    parse_dbpf_header(data).ok().map(|(size, _)| size)
}

/// Decompresses a SimCity 4 QFS entry (with the 4-byte compressed-size prefix).
///
/// The entry must pass the §2.7 acceptance check ([`has_header`]); a DIR-listed entry that does not is a read
/// error (data-formats §2.6 — there is no fallback to the stored bytes). \[choice\] The compressed-size prefix
/// is not trusted for framing: the stream is decoded from the whole slice, and a stream that runs out of input
/// exactly when the announced uncompressed size is reached is accepted even without a stop code.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 7 {
        return Err(Error::Truncated {
            what: "QFS header",
            needed: 7,
            available: data.len() as u64,
        });
    }
    if data[5] != 0xFB || (data[4] != 0x10 && data[4] != 0x11) {
        return Err(Error::BadMagic {
            what: "QFS header",
            found: data[4..6].to_vec(),
        });
    }
    let prefix = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    if prefix > MAX_PREFIX {
        return Err(corrupt(
            "QFS header",
            format!("compressed size {prefix} exceeds 16 MiB"),
        ));
    }
    let (size, header) = parse_dbpf_header(data)?;
    decode(&data[header..], size)
}

/// Decompresses a bare RefPack stream that starts with the flags byte (no SimCity 4 size prefix). Accepted
/// flags are `0x10` and `0x11` only, as in the DBPF form.
pub fn decompress_refpack(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 2 || data[1] != 0xFB || (data[0] != 0x10 && data[0] != 0x11) {
        return Err(Error::BadMagic {
            what: "QFS header",
            found: data.iter().take(2).copied().collect(),
        });
    }
    // `0x10`: size at 2..5. `0x11`: three unused bytes at 2..5, size at 5..8.
    let len = if data[0] == 0x11 { 8 } else { 5 };
    if data.len() < len {
        return Err(Error::Truncated {
            what: "QFS header",
            needed: len as u64,
            available: data.len() as u64,
        });
    }
    let size = if data[0] == 0x11 {
        be24(&data[5..8])
    } else {
        be24(&data[2..5])
    };
    decode(&data[len..], size)
}

fn decode(src: &[u8], size: usize) -> Result<Vec<u8>> {
    const WHAT: &str = "QFS stream";
    // Never trust the header for the allocation beyond what the input could possibly expand to.
    let mut out: Vec<u8> = Vec::with_capacity(size.min(src.len().saturating_mul(MAX_COPY)));
    let mut pos = 0usize;
    let need = |pos: usize, n: usize| -> Result<()> {
        if pos + n > src.len() {
            Err(Error::Truncated {
                what: WHAT,
                needed: (pos + n) as u64,
                available: src.len() as u64,
            })
        } else {
            Ok(())
        }
    };
    loop {
        if pos >= src.len() {
            if out.len() == size {
                return Ok(out); // no stop code, but complete (see `decompress`)
            }
            return Err(Error::Truncated {
                what: WHAT,
                needed: size as u64,
                available: out.len() as u64,
            });
        }
        let b0 = src[pos] as usize;
        let (lits, copy, dist, stop, len) = match b0 {
            0x00..=0x7F => {
                need(pos, 2)?;
                let b1 = src[pos + 1] as usize;
                (b0 & 3, ((b0 >> 2) & 7) + 3, ((b0 & 0x60) << 3) + b1 + 1, false, 2)
            }
            0x80..=0xBF => {
                need(pos, 3)?;
                let (b1, b2) = (src[pos + 1] as usize, src[pos + 2] as usize);
                (b1 >> 6, (b0 & 0x3F) + 4, ((b1 & 0x3F) << 8) + b2 + 1, false, 3)
            }
            0xC0..=0xDF => {
                need(pos, 4)?;
                let (b1, b2, b3) = (src[pos + 1] as usize, src[pos + 2] as usize, src[pos + 3] as usize);
                (
                    b0 & 3,
                    ((b0 & 0x0C) << 6) + b3 + 5,
                    ((b0 & 0x10) << 12) + (b1 << 8) + b2 + 1,
                    false,
                    4,
                )
            }
            0xE0..=0xFB => (((b0 & 0x1F) << 2) + 4, 0, 0, false, 1),
            _ => (b0 & 3, 0, 0, true, 1),
        };
        pos += len;
        need(pos, lits)?;
        if out.len() + lits + copy > size {
            return Err(corrupt(WHAT, format!("output exceeds the announced {size} bytes")));
        }
        out.extend_from_slice(&src[pos..pos + lits]);
        pos += lits;
        if copy > 0 {
            if dist > out.len() {
                return Err(corrupt(
                    WHAT,
                    format!("back-reference {dist} before start (at output {})", out.len()),
                ));
            }
            let start = out.len() - dist;
            if dist >= copy {
                out.extend_from_within(start..start + copy);
            } else {
                for i in 0..copy {
                    let b = out[start + i];
                    out.push(b);
                }
            }
        }
        if stop {
            if out.len() != size {
                return Err(corrupt(WHAT, format!("stop code after {} of {size} bytes", out.len())));
            }
            return Ok(out);
        }
    }
}

/// Compresses `data` into a SimCity 4 QFS entry (9-byte header, `0x10 0xFB`).
///
/// Greedy LZ77 with hash chains. Fails only if `data` is longer than the 3-byte size field allows.
pub fn compress(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() > MAX_UNCOMPRESSED {
        return Err(Error::Unsupported {
            what: "QFS compress",
            detail: format!("{} bytes exceeds the 24-bit size field", data.len()),
        });
    }
    let mut out = Vec::with_capacity(data.len() / 2 + 16);
    out.extend_from_slice(&[0, 0, 0, 0, 0x10, 0xFB]);
    let n = data.len();
    out.extend_from_slice(&[(n >> 16) as u8, (n >> 8) as u8, n as u8]);

    const HASH_BITS: u32 = 16;
    const MAX_CHAIN: usize = 128;
    let hash = |i: usize| -> usize {
        let v = (data[i] as u32) << 16 | (data[i + 1] as u32) << 8 | data[i + 2] as u32;
        (v.wrapping_mul(2_654_435_761) >> (32 - HASH_BITS)) as usize
    };
    let mut head = vec![usize::MAX; 1 << HASH_BITS];
    let mut prev = vec![usize::MAX; MAX_DISTANCE];
    let insert = |i: usize, head: &mut [usize], prev: &mut [usize]| {
        if i + 3 <= n {
            let h = hash(i);
            prev[i % MAX_DISTANCE] = head[h];
            head[h] = i;
        }
    };

    let mut lit_start = 0usize;
    let mut i = 0usize;
    while i < n {
        let mut best_len = 0usize;
        let mut best_dist = 0usize;
        if i + 3 <= n {
            let mut cand = head[hash(i)];
            let mut chain = 0;
            let max_len = (n - i).min(MAX_COPY);
            while cand != usize::MAX && chain < MAX_CHAIN {
                let dist = i - cand;
                if dist > MAX_DISTANCE {
                    break;
                }
                let mut l = 0;
                while l < max_len && data[cand + l] == data[i + l] {
                    l += 1;
                }
                if l > best_len && encodable(l, dist) {
                    best_len = l;
                    best_dist = dist;
                    if l == max_len {
                        break;
                    }
                }
                let p = prev[cand % MAX_DISTANCE];
                // Entries older than the window were overwritten in the ring; stop at non-decreasing links.
                if p == usize::MAX || p >= cand {
                    break;
                }
                cand = p;
                chain += 1;
            }
        }
        if best_len >= 3 {
            let lits = flush_literals(&mut out, &data[lit_start..i]);
            emit_copy(&mut out, lits, best_len, best_dist);
            for k in i..i + best_len {
                insert(k, &mut head, &mut prev);
            }
            i += best_len;
            lit_start = i;
        } else {
            insert(i, &mut head, &mut prev);
            i += 1;
        }
    }
    let rest = flush_literals(&mut out, &data[lit_start..]);
    out.push(0xFC | rest.len() as u8);
    out.extend_from_slice(rest);
    let total = out.len() as u32;
    out[0..4].copy_from_slice(&total.to_le_bytes());
    Ok(out)
}

fn encodable(len: usize, dist: usize) -> bool {
    (len <= 10 && dist <= 1024) || ((4..=67).contains(&len) && dist <= 16384) || (len >= 5 && dist <= MAX_DISTANCE)
}

/// Writes literal runs in multiples of four and returns the 0..=3 bytes left for the next code.
fn flush_literals<'a>(out: &mut Vec<u8>, mut lits: &'a [u8]) -> &'a [u8] {
    while lits.len() >= 4 {
        let n = (lits.len() & !3).min(112);
        out.push(0xE0 | ((n - 4) >> 2) as u8);
        out.extend_from_slice(&lits[..n]);
        lits = &lits[n..];
    }
    lits
}

fn emit_copy(out: &mut Vec<u8>, lits: &[u8], len: usize, dist: usize) {
    let l = lits.len();
    let d = dist - 1;
    if len <= 10 && dist <= 1024 {
        out.push((((d >> 3) & 0x60) | ((len - 3) << 2) | l) as u8);
        out.push(d as u8);
    } else if len <= 67 && dist <= 16384 {
        out.push(0x80 | (len - 4) as u8);
        out.push(((l << 6) | (d >> 8)) as u8);
        out.push(d as u8);
    } else {
        let c = len - 5;
        out.push((0xC0 | ((d >> 16) << 4) | ((c >> 8) << 2) | l) as u8);
        out.push((d >> 8) as u8);
        out.push(d as u8);
        out.push(c as u8);
    }
    out.extend_from_slice(lits);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// xorshift64*: deterministic pseudo-random inputs without extra dependencies.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    fn sample(rng: &mut Rng) -> Vec<u8> {
        let len = match rng.below(4) {
            0 => rng.below(16),
            1 => rng.below(300),
            2 => rng.below(5000),
            _ => rng.below(200_000),
        } as usize;
        let alphabet = 1 + rng.below(256) as u8 as u64;
        let mut v = Vec::with_capacity(len);
        while v.len() < len {
            if !v.is_empty() && rng.below(3) == 0 {
                // repeat an earlier run, sometimes overlapping, sometimes far back
                let dist = 1 + rng.below(v.len().min(150_000) as u64) as usize;
                let n = (1 + rng.below(1200) as usize).min(len - v.len());
                let start = v.len() - dist;
                for k in 0..n {
                    let b = v[start + k];
                    v.push(b);
                }
            } else {
                v.push(rng.below(alphabet) as u8);
            }
        }
        v
    }

    #[test]
    fn round_trip_property() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for case in 0..300 {
            let data = sample(&mut rng);
            let c = compress(&data).unwrap();
            assert!(has_header(&c));
            assert_eq!(u32::from_le_bytes([c[0], c[1], c[2], c[3]]) as usize, c.len());
            assert_eq!(uncompressed_size(&c), Some(data.len()));
            let d = decompress(&c).unwrap_or_else(|e| panic!("case {case}: {e}"));
            assert_eq!(d, data, "case {case} len {}", data.len());
        }
    }

    #[test]
    fn round_trip_edge_cases() {
        let mut cases: Vec<Vec<u8>> = vec![
            vec![],
            vec![7],
            vec![1, 2, 3],
            vec![0; 100_000],
            b"abcabcabcabc".to_vec(),
        ];
        cases.push((0..=255u8).cycle().take(300_000).collect());
        for data in cases {
            let c = compress(&data).unwrap();
            assert_eq!(decompress(&c).unwrap(), data);
        }
        assert!(compress(&vec![0; 100_000]).unwrap().len() < 1000);
    }

    #[test]
    fn hand_built_stream() {
        // "abcd" literal (E0), then copy 6 from distance 4 with 1 literal 'x' (short code), stop with 0 literals.
        let mut s = vec![0, 0, 0, 0, 0x10, 0xFB, 0, 0, 11];
        s.extend_from_slice(&[0xE0, b'a', b'b', b'c', b'd']);
        s.extend_from_slice(&[(6 - 3) << 2 | 1, 3, b'x']);
        s.push(0xFC);
        // distance 4 counts from after the literal 'x': output "abcdx" -> copies "bcdxbc"
        assert_eq!(decompress(&s).unwrap(), b"abcdxbcdxbc");
    }

    #[test]
    fn all_code_forms_decode() {
        // medium and long codes built by hand against the documented bit layout
        let mut data = Vec::new();
        for i in 0..20000u32 {
            data.push((i * 7 % 251) as u8);
        }
        let mut s = Vec::new();
        s.extend_from_slice(&[0, 0, 0, 0, 0x10, 0xFB]);
        let total = 112 + 20 + 1000;
        s.extend_from_slice(&[(total >> 16) as u8, (total >> 8) as u8, total as u8]);
        s.push(0xFB); // 112 literals
        s.extend_from_slice(&data[..112]);
        // medium: len 20, dist 100, 0 literals
        s.extend_from_slice(&[0x80 | (20 - 4), (99 >> 8) as u8, 99]);
        // long: len 1000, dist 50, 0 literals
        let c = 1000 - 5;
        s.extend_from_slice(&[0xC0 | ((c >> 8) << 2) as u8, 0, 49, c as u8]);
        s.push(0xFC);
        let out = decompress(&s).unwrap();
        assert_eq!(out.len(), total);
        assert_eq!(&out[..112], &data[..112]);
        for k in 112..132 {
            assert_eq!(out[k], out[k - 100]);
        }
        for k in 132..total {
            assert_eq!(out[k], out[k - 50]);
        }
    }

    #[test]
    fn errors_not_panics() {
        assert!(matches!(decompress(&[1, 2, 3]), Err(Error::Truncated { .. })));
        assert!(matches!(
            decompress(&[0, 0, 0, 0, 0x10, 0xFA, 0, 0, 1]),
            Err(Error::BadMagic { .. })
        ));
        // back-reference before the start
        let s = [0, 0, 0, 0, 0x10, 0xFB, 0, 0, 5, 0x08, 0x00];
        assert!(decompress(&s).is_err());
        // output overflow
        let s = [0, 0, 0, 0, 0x10, 0xFB, 0, 0, 1, 0xE0, 1, 2, 3, 4, 0xFC];
        assert!(decompress(&s).is_err());
        // stop too early
        let s = [0, 0, 0, 0, 0x10, 0xFB, 0, 0, 9, 0xFD, 1];
        assert!(decompress(&s).is_err());
        // fuzz: random garbage behind a valid header never panics
        let mut rng = Rng(42);
        for _ in 0..2000 {
            let mut s = vec![0, 0, 0, 0, 0x10, 0xFB, 0, 0x10, 0];
            for _ in 0..rng.below(64) {
                s.push(rng.next() as u8);
            }
            let _ = decompress(&s);
        }
    }

    #[test]
    fn header_flags() {
        // `11 FB`: three unused bytes, then the 3-byte uncompressed size.
        let s = [0x11, 0xFB, 0, 0, 9, 0, 0, 2, 0xFE, b'h', b'i'];
        assert_eq!(decompress_refpack(&s).unwrap(), b"hi");
        // `10 FB`: the size follows the signature directly.
        let s = [0x10, 0xFB, 0, 0, 2, 0xFE, b'h', b'i'];
        assert_eq!(decompress_refpack(&s).unwrap(), b"hi");
        // Any other flag byte is rejected (data-formats §2.7): no `0x80` four-byte-size form.
        for bad in [0x90u8, 0x30, 0x12, 0x00, 0x81, 0x13] {
            let s = [bad, 0xFB, 0, 0, 2, 0xFE, b'h', b'i'];
            assert!(
                matches!(decompress_refpack(&s), Err(Error::BadMagic { .. })),
                "flags {bad:#04X} must be rejected"
            );
        }
    }

    /// The §10.2 conformance vectors (data-formats §10.2, tests 11–17), as byte slices.
    #[test]
    fn conformance_vectors() {
        // 11: short copy — `E0 ABCD` (4 literals) then `14 03` copies 6 from distance 4, stop `FC`.
        let v11: &[u8] = &[
            0x11, 0x00, 0x00, 0x00, 0x10, 0xFB, 0x00, 0x00, 0x0C, 0xE0, b'A', b'B', b'C', b'D', 0x14, 0x03, 0xFC,
        ];
        assert!(has_header(v11));
        assert_eq!(uncompressed_size(v11), Some(12));
        assert_eq!(decompress(v11).unwrap(), b"ABCDABCDABCD");

        // 12: long copy — `C4 00 03 27` copies 300 bytes from distance 4; result 304 bytes.
        let v12: &[u8] = &[
            0x13, 0x00, 0x00, 0x00, 0x10, 0xFB, 0x00, 0x01, 0x30, 0xE0, b'A', b'B', b'C', b'D', 0xC4, 0x00, 0x03, 0x27,
            0xFC,
        ];
        let out = decompress(v12).unwrap();
        assert_eq!(out.len(), 304);
        assert!(out.chunks_exact(4).all(|c| c == b"ABCD"));

        // 13: `11 FB` — the 3 bytes after the signature are skipped, size 4, result "ABCD".
        let v13: &[u8] = &[
            0x12, 0x00, 0x00, 0x00, 0x11, 0xFB, 0x00, 0x00, 0x0E, 0x00, 0x00, 0x04, 0xE0, b'A', b'B', b'C', b'D', 0xFC,
        ];
        assert_eq!(uncompressed_size(v13), Some(4));
        assert_eq!(decompress(v13).unwrap(), b"ABCD");

        // 14: rejected signatures.
        for bad in [
            &[0x07, 0, 0, 0, 0x10, 0xFC, 0][..],
            &[0x07, 0, 0, 0, 0x90, 0xFB, 0][..],
            &[0x07, 0, 0, 0, 0x30, 0xFB, 0][..],
        ] {
            assert!(!has_header(bad), "{bad:02X?} must be rejected");
            assert!(decompress(bad).is_err());
        }
        // Fewer than 7 stored bytes.
        assert!(!has_header(&[0x10, 0xFB, 0, 0, 4, 0xE0]));
        assert!(decompress(&[0x10, 0xFB, 0, 0, 4, 0xE0]).is_err());

        // 17: prefix above 16 MiB fails even though the rest is valid.
        let mut v17 = v11.to_vec();
        v17[0..4].copy_from_slice(&0x0100_0001u32.to_le_bytes());
        assert!(!has_header(&v17));
        assert!(matches!(decompress(&v17), Err(Error::Corrupt { .. })));
        // exactly 16 MiB is accepted by the check (the size field, not the prefix, gates allocation)
        let mut v17b = v11.to_vec();
        v17b[0..4].copy_from_slice(&0x0100_0000u32.to_le_bytes());
        assert!(has_header(&v17b));
        assert_eq!(decompress(&v17b).unwrap(), b"ABCDABCDABCD");
    }
}
