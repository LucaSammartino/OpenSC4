// SPDX-License-Identifier: GPL-3.0-or-later
//! Antialiasing lookup table (drawing-fonts.md §10, `BuildAntialiasingLookupArray` `0x008993BB`).

/// The sharpness exponent k: L = log10(2.0), 1.0 / L, then pow(float(sharpness) + 1.0, that) in
/// double, stored as float.
pub fn sharpness_exponent(sharpness: f32) -> f32 {
    let l = 2.0f64.log10();
    let inv = 1.0 / l;
    (sharpness as f64 + 1.0).powf(inv) as f32
}

/// Build the table: empty when gamma == 1.0 and sharpness == 0.0, else 256 entries. (The caller
/// clears the grey cache in the non-empty case.)
pub fn build_table(gamma: f32, sharpness: f32) -> Vec<u8> {
    if gamma == 1.0 && sharpness == 0.0 {
        return Vec::new();
    }
    let k = sharpness_exponent(sharpness);
    let inv255: f32 = 1.0 / 255.0;
    let mut t = Vec::with_capacity(256);
    for i in 0..256u32 {
        let mut v = i as f32 * inv255;
        if gamma != 1.0 {
            v = v.powf(gamma);
        }
        if sharpness != 0.0 {
            v = v.min(1.0);
            v = if i <= 127 {
                (2.0 * v).powf(k) * 0.5
            } else {
                1.0 - (2.0 * (1.0 - v)).powf(k) * 0.5
            };
        }
        // `__ftol` truncates the x87 product; the product is formed in double here (literal
        // reading, FN20: the spec gives `truncate(v · 255)` without intermediate precision).
        t.push((v as f64 * 255.0) as i32 as u8);
    }
    t
}
