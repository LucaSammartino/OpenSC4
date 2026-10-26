// SPDX-License-Identifier: GPL-3.0-or-later
// Dependency-free SHA-256 (FIPS 180-4), used for content and source identities.
use std::io::{self, Read};
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98,
    0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8,
    0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819,
    0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
    0xc67178f2,
];
fn block(h: &mut [u32; 8], b: &[u8]) {
    let mut w = [0u32; 64];
    for (i, c) in b.chunks_exact(4).enumerate() {
        w[i] = u32::from_be_bytes(c.try_into().unwrap());
    }
    for i in 16..64 {
        let x = w[i - 15];
        let y = w[i - 2];
        w[i] = w[i - 16]
            .wrapping_add(x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3))
            .wrapping_add(w[i - 7])
            .wrapping_add(y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10));
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut z] = *h;
    for i in 0..64 {
        let t = z
            .wrapping_add(e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25))
            .wrapping_add((e & f) ^ (!e & g))
            .wrapping_add(K[i])
            .wrapping_add(w[i]);
        let u = (a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22)).wrapping_add((a & b) ^ (a & c) ^ (b & c));
        z = g;
        g = f;
        f = e;
        e = d.wrapping_add(t);
        d = c;
        c = b;
        b = a;
        a = t.wrapping_add(u);
    }
    for (x, v) in h.iter_mut().zip([a, b, c, d, e, f, g, z]) {
        *x = x.wrapping_add(v);
    }
}
pub(super) fn sha256(mut input: impl Read) -> io::Result<String> {
    let mut h = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut b = [0u8; 64];
    let mut count = 0u64;
    loop {
        let mut n = 0;
        while n < 64 {
            let got = input.read(&mut b[n..])?;
            if got == 0 {
                break;
            }
            n += got;
        }
        count += n as u64;
        if n == 64 {
            block(&mut h, &b);
            continue;
        }
        b[n] = 0x80;
        b[n + 1..].fill(0);
        if n >= 56 {
            block(&mut h, &b);
            b.fill(0);
        }
        b[56..].copy_from_slice(&count.wrapping_mul(8).to_be_bytes());
        block(&mut h, &b);
        break;
    }
    Ok(h.iter().map(|v| format!("{v:08x}")).collect())
}

#[allow(dead_code)] // Also compiled as a build-script module, where this is used.
pub(super) fn source_hash(root: &std::path::Path) -> io::Result<String> {
    fn files(path: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> io::Result<()> {
        for entry in std::fs::read_dir(path)? {
            let path = entry?.path();
            if path.is_dir() {
                files(&path, out)?;
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
        Ok(())
    }
    let mut paths = vec![root.join("Cargo.toml"), root.join("Cargo.lock")];
    for name in ["core", "capi", "formats"] {
        paths.push(root.join(name).join("Cargo.toml"));
        files(&root.join(name).join("src"), &mut paths)?;
    }
    paths.push(root.join("capi/build.rs"));
    paths.sort();
    let mut bytes = Vec::new();
    for path in paths {
        let name = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        let body = std::fs::read(&path)?;
        bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(&(body.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&body);
    }
    sha256(bytes.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nist_vectors() {
        assert_eq!(
            sha256(&b""[..]).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256(&b"abc"[..]).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256(&vec![b'a'; 1_000_000][..]).unwrap(),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }
}
