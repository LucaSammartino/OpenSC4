// SPDX-License-Identifier: GPL-3.0-or-later
//! How a model names its textures and how an exemplar names its model (`docs/specs/graphics-formats.md` §4).
//!
//! - [`resolve_texture`]: a material texture ID → FSH key, in the §4.1 order: the shared texture group
//!   `0x1ABE787D`, then the model's own group, then the fallback texture `0x07F94200` (and the slot's ID becomes
//!   0 so it is not looked up again); no texture when the fallback does not load either (FS13).
//! - [`model_key`]: the RKT0–RKT3 instance arithmetic of §4.2, with the zoom remap by model detail
//!   ([`remap_zoom`]) and the view rotation ([`view_rotation`]).
//! - [`occupant_models`]: all the models of an occupant (`GetAllModelInstanceIDs`, §4.2 GF7): the zoom mask
//!   [`ZOOM_MASK`], the flag-`0x80` keys from [`RKT_FLAGGED_BASE`], RKT5 and the RKT4 record walk
//!   ([`rkt4_records`]). `fc4-dat models` prints them for a shipped exemplar.

use crate::tgi::{Tgi, GROUP_TEXTURES, TYPE_FSH};

/// The instance of the fallback texture (§4.1; shipped in `SimCity_3.dat`, 32 × 32 DXT1).
pub const FALLBACK_TEXTURE_INSTANCE: u32 = 0x07F9_4200;
/// The fallback texture's key.
pub const FALLBACK_TEXTURE: Tgi = Tgi::new(TYPE_FSH, GROUP_TEXTURES, FALLBACK_TEXTURE_INSTANCE);

/// Which step of §4.1 found the texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextureStep {
    /// (`0x7AB50E44`, `0x1ABE787D`, ID).
    SharedGroup = 1,
    /// (`0x7AB50E44`, the model's group, ID).
    ModelGroup = 2,
    /// The fallback texture.
    Fallback = 3,
}

/// The result of [`resolve_texture`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureBinding {
    /// The FSH to bind.
    pub key: Tgi,
    pub step: TextureStep,
    /// The slot's texture ID afterwards: unchanged, or 0 after the fallback.
    pub slot_id: u32,
}

/// Resolves a material texture slot (§4.1). `loads` says whether an FSH key loads (exists and passes the
/// factory). `None` means the slot draws without a texture and its ID is (or becomes) 0: a slot with ID 0 is not
/// looked up, and when neither key nor the fallback loads, the original leaves the slot with no binding and its
/// ID set to 0, so it is never retried (FS13; FreeCity4 resolves it to no texture *\[choice\]*).
pub fn resolve_texture(
    model_group: u32,
    texture_id: u32,
    mut loads: impl FnMut(&Tgi) -> bool,
) -> Option<TextureBinding> {
    if texture_id == 0 {
        return None;
    }
    let shared = Tgi::new(TYPE_FSH, GROUP_TEXTURES, texture_id);
    if loads(&shared) {
        return Some(TextureBinding {
            key: shared,
            step: TextureStep::SharedGroup,
            slot_id: texture_id,
        });
    }
    let own = Tgi::new(TYPE_FSH, model_group, texture_id);
    if loads(&own) {
        return Some(TextureBinding {
            key: own,
            step: TextureStep::ModelGroup,
            slot_id: texture_id,
        });
    }
    // FS13: the fallback lookup's result is not tested by the original; a fallback that does not load leaves
    // the slot untextured.
    if !loads(&FALLBACK_TEXTURE) {
        return None;
    }
    Some(TextureBinding {
        key: FALLBACK_TEXTURE,
        step: TextureStep::Fallback,
        slot_id: 0,
    })
}

/// The texture binding cache key (§2.10): (group, instance); the type is ignored.
pub fn binding_cache_key(key: &Tgi) -> (u32, u32) {
    (key.group, key.instance)
}

/// Resource-key-type properties that name an occupant's model (§4.2).
pub const RKT0: u32 = 0x2781_2820;
pub const RKT1: u32 = 0x2781_2821;
pub const RKT2: u32 = 0x2781_2822;
pub const RKT3: u32 = 0x2781_2823;
/// A list of placed models, one per matching record; read by [`occupant_models`], not by [`model_key`] (GF7).
pub const RKT4: u32 = 0x2781_2824;
/// The RKT1 rule plus the occupant's state × `0x10000`; read by [`occupant_models`], not by [`model_key`] (GF7).
pub const RKT5: u32 = 0x2781_2825;
/// The zoom mask (§4.2, GF7): when an exemplar has it and its bit number *zoom* is clear, the occupant has no
/// model at that zoom. Shipped in 1,009 records (billboards `0x1E`, signs `0x18`).
pub const ZOOM_MASK: u32 = 0x0ABF_C024;
/// The alternative model keys `0x27812920` … `0x27812925` (RKT0–RKT5 + `0x100`), read instead of RKT0–RKT5 when
/// the occupant's flag [`OCCUPANT_FLAG_ALT_MODELS`] is set (§4.2, GF7).
pub const RKT_FLAGGED_BASE: u32 = 0x2781_2920;
/// The occupant flag (`IsFlagSet`, slot 32) that selects the alternative model keys (§4.2, GF7).
pub const OCCUPANT_FLAG_ALT_MODELS: u32 = 0x80;

/// The zoom remap table, indexed by zoom + 5 × detail (§4.2): detail 0 → 0, 1, 2, 3, 4; detail 1 → 0, 1, 2, 3, 3;
/// detail 2 → 0, 0, 1, 2, 3.
pub const ZOOM_REMAP: [u8; 15] = [0, 1, 2, 3, 4, 0, 1, 2, 3, 3, 0, 0, 1, 2, 3];

/// z = table[zoom + 5 × detail] for a zoom index 0–4; a detail above 2 is clamped to 2 and a zoom above 4 gives
/// `None` *\[choice\]* (§4.2, FS14: the original checks neither, and reads the next table row or past the table).
///
/// The detail is the original's model detail global: the integer render property `kModelLODOffset` (22),
/// clamped to 0–2 when it is read, which `Graphics Rules.sgr` sets from the Texture Quality option (High 0,
/// Medium 1, Low 2; registered default 0) (§4.2, GF8). FreeCity4 has no such option yet; callers pass the
/// detail they want.
pub fn remap_zoom(zoom: u32, detail: u32) -> Option<u32> {
    if zoom > 4 {
        return None;
    }
    // FS14 [choice]: the detail is clamped to 0-2, as the original clamps its global when it is set (GF8).
    let detail = detail.min(2);
    Some(u32::from(ZOOM_REMAP[(zoom + 5 * detail) as usize]))
}

/// The rotation r, 0–3, from the model function's two rotation arguments: (3a + b) mod 4, or 3a mod 4 when its
/// flag argument is set (§4.2).
pub fn view_rotation(a: u32, b: u32, flag: bool) -> u32 {
    let three_a = a.wrapping_mul(3);
    if flag {
        three_a % 4
    } else {
        three_a.wrapping_add(b) % 4
    }
}

/// The model key named by an RKT property with values `v` (type, group, instance…) at a zoom index (0–4),
/// detail (0–2) and rotation (0–3) (§4.2):
///
/// | Property | Key |
/// |---|---|
/// | RKT0 | (v[0], v[1], v[2]) for every zoom and rotation |
/// | RKT1 | (v[0], v[1], v[2] + z × `0x100` + r × `0x10`); v[2] = 0 means no model |
/// | RKT2 | (v[0], v[1], v[2 + 4z + r]) |
/// | RKT3 | (v[0], v[1], v[2 + z]) |
///
/// `None` for no model, for RKT4/RKT5 and other properties, and *\[choice\]* for too few values or a zoom above 4.
/// *\[choice\]* A detail above 2 is clamped to 2 and the rotation is taken mod 4 (FS14; the original checks
/// nothing).
pub fn model_key(property: u32, v: &[u32], zoom: u32, detail: u32, rotation: u32) -> Option<Tgi> {
    let key = |i: usize| v.get(i).map(|&inst| Tgi::new(v[0], v[1], inst));
    if property == RKT0 {
        return key(2);
    }
    let z = remap_zoom(zoom, detail)?;
    let rotation = rotation % 4;
    match property {
        RKT1 => {
            let base = *v.get(2)?;
            if base == 0 {
                return None;
            }
            let inst = base.wrapping_add(z * 0x100).wrapping_add(rotation * 0x10);
            Some(Tgi::new(v[0], v[1], inst))
        }
        RKT2 => key(2 + 4 * z as usize + rotation as usize),
        RKT3 => key(2 + z as usize),
        _ => None,
    }
}

/// One RKT0–RKT3 rule on values `v` (type, group, …) without [`model_key`]'s "no model" mapping: the instance,
/// 0 included (an RKT1 base of 0 gives 0, §4.2 GF7). `z` is the remapped zoom (unused by RKT0). `None` only
/// *\[choice\]* for too few values, a missing zoom or another kind.
fn rule_instance(kind: u32, v: &[u32], z: Option<u32>, r: u32) -> Option<u32> {
    match kind {
        RKT0 => v.get(2).copied(),
        RKT1 => {
            let base = *v.get(2)?;
            let z = z?;
            Some(if base == 0 {
                0
            } else {
                base.wrapping_add(z * 0x100).wrapping_add(r * 0x10)
            })
        }
        RKT2 => v.get(2 + 4 * z? as usize + r as usize).copied(),
        RKT3 => v.get(2 + z? as usize).copied(),
        _ => None,
    }
}

/// The values each RKT4 record kind takes after its five-value header (§4.2, GF7): 3 for RKT0 and RKT1, 30 for
/// RKT2 (T, G and 28 instances, of which the rule uses the first 20) and 5 for RKT3. `None` for any other kind.
pub fn rkt4_record_len(kind: u32) -> Option<usize> {
    match kind {
        RKT0 | RKT1 => Some(3),
        RKT2 => Some(30),
        RKT3 => Some(5),
        _ => None,
    }
}

/// One RKT4 record (§4.2, GF7): the header (selector, x, y, z in signed 16.16, kind) and where its values
/// (T, G, …) start in the property's array.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rkt4Record {
    pub selector: u32,
    pub offset: [u32; 3],
    pub kind: u32,
    /// The index of the record's first value (T) in the property's array.
    pub values_at: usize,
}

/// How an RKT4 walk ended (§4.2, GF7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rkt4End {
    /// Fewer than five values were left (none, when the records fill the array exactly).
    Complete { leftover: usize },
    /// A record's kind is not RKT0–RKT3, at this index (the records before it are kept).
    BadKind { at: usize, kind: u32 },
    /// The record at this index runs past the end of the array.
    PastEnd { at: usize },
}

/// Walks an RKT4 array as `GetAllModelInstanceIDs` does (§4.2, GF7): it needs more than four values, starts a
/// record only while at least five are left, and stops at a kind that is not RKT0–RKT3 or at a record that runs
/// past the end, keeping the records before it.
pub fn rkt4_records(v: &[u32]) -> (Vec<Rkt4Record>, Rkt4End) {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while v.len() - pos >= 5 {
        let kind = v[pos + 4];
        let Some(len) = rkt4_record_len(kind) else {
            return (out, Rkt4End::BadKind { at: pos, kind });
        };
        if pos + 5 + len > v.len() {
            return (out, Rkt4End::PastEnd { at: pos });
        }
        out.push(Rkt4Record {
            selector: v[pos],
            offset: [v[pos + 1], v[pos + 2], v[pos + 3]],
            kind,
            values_at: pos + 5,
        });
        pos += 5 + len;
    }
    (
        out,
        Rkt4End::Complete {
            leftover: v.len() - pos,
        },
    )
}

/// A signed 16.16 fixed-point offset as the walk converts it: the int32 × 1/65536 (§4.2, GF7). The product is
/// exact, so one rounding of the value to f32 is the result.
pub fn fixed_16_16(x: u32) -> f32 {
    (x as i32) as f32 * (1.0 / 65536.0)
}

/// A model or mask property's values as unsigned 32-bit integers, for [`occupant_models`]: the integer types up
/// to 32 bits, converted (signed ones as two's complement); `None` for any other type *\[choice\]* (shipped
/// model keys are Uint32 arrays). GF18 (re-answers11): the original reads the raw 32-bit words of any stored type
/// (`vec<u32>`, no conversion, a scalar giving an empty vector); the choice is accepted.
pub fn property_u32s(p: &crate::exemplar::Property) -> Option<Vec<u32>> {
    use crate::exemplar::Values;
    Some(match &p.values {
        Values::Uint32(v) => v.clone(),
        Values::Sint32(v) => v.iter().map(|&x| x as u32).collect(),
        Values::Uint16(v) => v.iter().map(|&x| u32::from(x)).collect(),
        Values::Sint16(v) => v.iter().map(|&x| x as u32).collect(),
        Values::Uint8(v) => v.iter().map(|&x| u32::from(x)).collect(),
        Values::Sint8(v) => v.iter().map(|&x| x as u32).collect(),
        _ => return None,
    })
}

/// The models of an occupant at one zoom (`GetAllModelInstanceIDs`, §4.2 GF7).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OccupantModels {
    /// The model property read (`None`: masked out at this zoom, or none of the six keys).
    pub property: Option<u32>,
    /// The type and group written to the out pointers; `None` when nothing was written (an RKT4 with no record
    /// for the state leaves them as they were).
    pub type_group: Option<(u32, u32)>,
    /// One instance per model, 0 included (no model at that slot).
    pub instances: Vec<u32>,
    /// The offset of each instance, in world units.
    pub offsets: Vec<[f32; 3]>,
}

/// `GetAllModelInstanceIDs` (§4.2, GF7) on a property holder: `get(id)` gives a property's values as unsigned
/// 32-bit integers. `zoom` is the zoom index (0–4) before the detail remap, `rotation` the r of [`view_rotation`]
/// (taken mod 4), `flagged` the occupant's flag `0x80` and `state` its state (`GetOccupantOrientationAndState`).
///
/// 1. If the holder has [`ZOOM_MASK`] and its bit number `zoom` is clear: no models.
/// 2. With b = RKT0, or [`RKT_FLAGGED_BASE`] when flagged, the first of b + 0 … b + 5 the holder has is read:
///    b + 0 … b + 3 one instance by its rule, b + 5 the RKT1 rule plus `state` × `0x10000`, b + 4 every record
///    whose selector equals `state` (its instance, 0 included, and its offset; the last match's T and G).
///
/// *\[choice\]* (as [`model_key`]): a zoom above 4 gives no models from the RKT1–RKT3 rules (RKT0 ignores it);
/// a rule with too few values adds no instance (GF17, re-answers11: the original appends an instance read past the
/// array or the remap table; the choice is accepted). *\[literal reading\]* The mask's bit number is the zoom index
/// as passed; a mask with no value counts as 0. *\[inferred\]* RKT0–RKT3 and RKT5 write their own T and G.
pub fn occupant_models<'a>(
    get: impl Fn(u32) -> Option<&'a [u32]>,
    zoom: u32,
    detail: u32,
    rotation: u32,
    flagged: bool,
    state: u32,
) -> OccupantModels {
    let mut out = OccupantModels::default();
    if let Some(mask) = get(ZOOM_MASK) {
        let m = mask.first().copied().unwrap_or(0);
        if zoom >= 32 || (m >> zoom) & 1 == 0 {
            return out;
        }
    }
    let z = remap_zoom(zoom, detail);
    let r = rotation % 4;
    let base = if flagged { RKT_FLAGGED_BASE } else { RKT0 };
    let Some((k, v)) = (0..6).find_map(|k| get(base + k).map(|v| (k, v))) else {
        return out;
    };
    out.property = Some(base + k);
    let tg = |v: &[u32]| v.get(1).map(|&g| (v[0], g));
    match k {
        0..=3 => {
            out.type_group = tg(v);
            if let Some(i) = rule_instance(RKT0 + k, v, z, r) {
                out.instances.push(i);
                out.offsets.push([0.0; 3]);
            }
        }
        5 => {
            out.type_group = tg(v);
            if let Some(i) = rule_instance(RKT1, v, z, r) {
                out.instances.push(i.wrapping_add(state.wrapping_mul(0x1_0000)));
                out.offsets.push([0.0; 3]);
            }
        }
        _ => {
            for rec in rkt4_records(v).0.iter().filter(|rec| rec.selector == state) {
                // The rule reads from the record's values on, so an RKT3 record at zooms 3-4 reads into the
                // next record (the [quirk] of §4.2).
                let vals = &v[rec.values_at..];
                if let Some(i) = rule_instance(rec.kind, vals, z, r) {
                    out.instances.push(i);
                    out.offsets.push(rec.offset.map(fixed_16_16));
                    out.type_group = tg(vals);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §8.3 test 38.
    #[test]
    fn t38_texture_lookup() {
        let g = 0xBADB_57F1;
        let x = 0x1234_5678;
        let shared = Tgi::new(TYPE_FSH, GROUP_TEXTURES, x);
        let own = Tgi::new(TYPE_FSH, g, x);
        let b = resolve_texture(g, x, |k| *k == shared || *k == own).unwrap();
        assert_eq!((b.key, b.step, b.slot_id), (shared, TextureStep::SharedGroup, x));
        let b = resolve_texture(g, x, |k| *k == own).unwrap();
        assert_eq!((b.key, b.step, b.slot_id), (own, TextureStep::ModelGroup, x));
        let mut asked = Vec::new();
        let b = resolve_texture(g, x, |k| {
            asked.push(*k);
            *k == FALLBACK_TEXTURE
        })
        .unwrap();
        assert_eq!((b.key, b.step, b.slot_id), (FALLBACK_TEXTURE, TextureStep::Fallback, 0));
        assert_eq!(asked, [shared, own, FALLBACK_TEXTURE]);
        assert_eq!(FALLBACK_TEXTURE, Tgi::new(0x7AB5_0E44, 0x1ABE_787D, 0x07F9_4200));
        // FS13: when the fallback does not load either, the slot has no texture.
        let mut asked = Vec::new();
        let b = resolve_texture(g, x, |k| {
            asked.push(*k);
            false
        });
        assert_eq!(b, None);
        assert_eq!(asked, [shared, own, FALLBACK_TEXTURE]);
        // ID 0 is never looked up.
        assert_eq!(resolve_texture(g, 0, |_| panic!("looked up")), None);
        // The binding cache ignores the type.
        assert_eq!(
            binding_cache_key(&Tgi::new(1, 2, 3)),
            binding_cache_key(&Tgi::new(9, 2, 3))
        );
    }

    /// §8.3 test 39.
    #[test]
    fn t39_rkt1() {
        let v = [0x5AD0_E817, 0xBADB_57F1, 0x1234_0000];
        let inst = |detail| model_key(RKT1, &v, 4, detail, 2).map(|k| k.instance);
        assert_eq!(inst(0), Some(0x1234_0420));
        assert_eq!(inst(1), Some(0x1234_0320));
        assert_eq!(inst(2), Some(0x1234_0320));
        assert_eq!(
            model_key(RKT1, &v, 4, 0, 2).unwrap(),
            Tgi::new(0x5AD0_E817, 0xBADB_57F1, 0x1234_0420)
        );
        assert_eq!(model_key(RKT1, &[0x5AD0_E817, 0xBADB_57F1, 0], 4, 0, 2), None);
    }

    /// §8.3 test 40.
    #[test]
    fn t40_rkt2() {
        let mut v = vec![0x5AD0_E817, 0xBADB_57F1];
        v.extend((0..20).map(|i| 0x1000 + i));
        assert_eq!(model_key(RKT2, &v, 1, 0, 3).map(|k| k.instance), Some(0x1000 + 7));
        assert_eq!(model_key(RKT2, &v, 4, 0, 3).map(|k| k.instance), Some(0x1000 + 19));
        assert_eq!(model_key(RKT2, &v[..21], 4, 0, 3), None);
    }

    /// §8.3 test 41.
    #[test]
    fn t41_rkt3() {
        let mut v = vec![0x5AD0_E817, 0xBADB_57F1];
        v.extend((0..5).map(|i| 0x2000 + i));
        assert_eq!(model_key(RKT3, &v, 2, 2, 0).map(|k| k.instance), Some(0x2000 + 1));
        assert_eq!(model_key(RKT3, &v, 2, 0, 0).map(|k| k.instance), Some(0x2000 + 2));
    }

    #[test]
    fn rkt_helpers() {
        let v = [1, 2, 3];
        for zoom in 0..5 {
            for r in 0..4 {
                assert_eq!(model_key(RKT0, &v, zoom, 0, r), Some(Tgi::new(1, 2, 3)));
            }
        }
        assert_eq!(model_key(RKT4, &v, 0, 0, 0), None);
        assert_eq!(model_key(RKT5, &v, 0, 0, 0), None);
        assert_eq!(model_key(RKT1, &v, 5, 0, 0), None);
        assert_eq!(model_key(RKT1, &v[..2], 0, 0, 0), None);
        // FS14 [choice]: a detail above 2 is clamped, a rotation taken mod 4.
        let big = [1, 2, 0x1000];
        assert_eq!(model_key(RKT1, &big, 4, 3, 1), model_key(RKT1, &big, 4, 2, 1));
        assert_eq!(model_key(RKT1, &big, 4, u32::MAX, 1).map(|k| k.instance), Some(0x1310));
        assert_eq!(model_key(RKT1, &big, 4, 0, 6).map(|k| k.instance), Some(0x1420));
        assert_eq!(remap_zoom(4, 7), Some(3));
        assert_eq!(remap_zoom(5, 0), None);
        let table: Vec<Vec<u32>> = (0..3)
            .map(|d| (0..5).map(|z| remap_zoom(z, d).unwrap()).collect())
            .collect();
        assert_eq!(table, [vec![0, 1, 2, 3, 4], vec![0, 1, 2, 3, 3], vec![0, 0, 1, 2, 3]]);
        assert_eq!(view_rotation(1, 2, false), 1);
        assert_eq!(view_rotation(1, 2, true), 3);
        assert_eq!(view_rotation(u32::MAX, 0, false), 1); // -1: 3 × -1 = -3 ≡ 1 (mod 4)
    }

    /// §4.2 GF7 worked example: `Street2x6x2_$$$LuxerySedan2_1D71`'s RKT4.
    #[test]
    fn gf7_rkt4_luxery_sedan() {
        let (t, g) = (0x5AD0_E817, 0xBADB_57F1);
        let rkt4 = [0, 0, 0, 0, RKT1, t, g, 0x1D71_0000, 1, 0, 0, 0, RKT0, t, g, 0];
        let get = |id| (id == RKT4).then_some(&rkt4[..]);
        let m = occupant_models(get, 4, 0, 2, false, 0);
        assert_eq!(m.property, Some(RKT4));
        assert_eq!(m.instances, [0x1D71_0420]);
        assert_eq!(m.offsets, [[0.0; 3]]);
        assert_eq!(m.type_group, Some((t, g)));
        let m = occupant_models(get, 4, 0, 2, false, 1);
        assert_eq!(m.instances, [0]);
        // No record for state 2: nothing, and T and G are not written.
        let m = occupant_models(get, 4, 0, 2, false, 2);
        assert_eq!((m.instances.len(), m.type_group), (0, None));
        // Flagged, the holder has no 0x2781292x key: no model.
        assert_eq!(occupant_models(get, 4, 0, 2, true, 0).property, None);
        assert_eq!(rkt4_records(&rkt4).1, Rkt4End::Complete { leftover: 0 });
    }

    /// §4.2 GF7 worked example: the street-light banner with flag 0x80 reads `0x27812924`.
    #[test]
    fn gf7_rkt4_flagged_banner_offsets() {
        let (t, g) = (0x5AD0_E817, 0xBADB_57F1);
        let mut v = vec![1, 0, 0, 0, RKT1, t, g, 0x2B29_0000];
        v.extend([1, 0xFFFE_6667, 0x0007_8000, 0, RKT0, t, g, 0x29DA_0000]);
        v.extend([1, 0x0001_9999, 0x0007_8000, 0, RKT0, t, g, 0x29DA_0000]);
        v.extend([0, 0, 0, 0, RKT1, t, g, 0x2B29_0000]);
        let unflagged = [0, 0, 0, 0, RKT0, t, g, 0x1111_0000];
        let get = |id| match id {
            0x2781_2924 => Some(&v[..]),
            RKT4 => Some(&unflagged[..]),
            _ => None,
        };
        let m = occupant_models(get, 4, 0, 0, true, 1);
        assert_eq!(m.property, Some(RKT_FLAGGED_BASE + 4));
        assert_eq!(m.instances, [0x2B29_0400, 0x29DA_0000, 0x29DA_0000]);
        assert_eq!(m.offsets[0], [0.0; 3]);
        assert_eq!(m.offsets[1], [-1.599_990_8, 7.5, 0.0]);
        assert_eq!(m.offsets[2], [1.599_990_8, 7.5, 0.0]);
        assert_eq!(fixed_16_16(0xFFFE_6667).to_bits(), (-104_857.0f32 / 65536.0).to_bits());
        assert_eq!(occupant_models(get, 4, 0, 0, true, 0).instances, [0x2B29_0400]);
        // Not flagged: the plain RKT4.
        assert_eq!(occupant_models(get, 4, 0, 0, false, 0).instances, [0x1111_0000]);
        // A stray value in the second header (the four shipped street lights): the walk stops after record 1.
        let mut stray = v.clone();
        stray[12] = 0x54000;
        let get = |id| (id == 0x2781_2924).then_some(&stray[..]);
        assert_eq!(occupant_models(get, 4, 0, 0, true, 1).instances, [0x2B29_0400]);
        assert_eq!(occupant_models(get, 4, 0, 0, true, 0).instances.len(), 0);
        assert_eq!(rkt4_records(&stray).1, Rkt4End::BadKind { at: 8, kind: 0x54000 });
    }

    /// §4.2 GF7: the zoom mask, RKT5, the key order, and the RKT4 walk's limits.
    #[test]
    fn gf7_mask_rkt5_order_and_walk() {
        let (t, g) = (0x5AD0_E817, 0xBADB_57F1);
        let rkt5 = [t, g, 0x1690_0000];
        let rkt0 = [t, g, 0x7777];
        let sign_mask = [0x18u32];
        let get = |id| match id {
            RKT5 => Some(&rkt5[..]),
            ZOOM_MASK => Some(&sign_mask[..]),
            _ => None,
        };
        // Zooms 3 and 4 only.
        for zoom in 0..3 {
            assert_eq!(occupant_models(get, zoom, 0, 0, false, 0), OccupantModels::default());
        }
        let m = occupant_models(get, 3, 0, 1, false, 2);
        assert_eq!(m.property, Some(RKT5));
        assert_eq!(m.instances, [0x1690_0310 + 0x2_0000]);
        assert_eq!(m.type_group, Some((t, g)));
        // RKT5 with base 0: 0 from the RKT1 rule, then state × 0x10000.
        let zero = [t, g, 0];
        let get0 = |id| (id == RKT5).then_some(&zero[..]);
        assert_eq!(occupant_models(get0, 4, 0, 0, false, 3).instances, [0x3_0000]);
        // The first key the holder has wins (RKT0 before RKT5), and only it is read.
        let both = |id| match id {
            RKT0 => Some(&rkt0[..]),
            RKT5 => Some(&rkt5[..]),
            _ => None,
        };
        let m = occupant_models(both, 4, 0, 0, false, 0);
        assert_eq!((m.property, m.instances), (Some(RKT0), vec![0x7777]));
        // The walk: four values are not a record; a record past the end stops it.
        assert_eq!(rkt4_records(&[0, 0, 0, 0]), (vec![], Rkt4End::Complete { leftover: 4 }));
        let short = [0, 0, 0, 0, RKT2, t, g, 1, 2, 3];
        assert_eq!(rkt4_records(&short), (vec![], Rkt4End::PastEnd { at: 0 }));
        // An RKT3 record at zoom 4 reads two values into the next record (the quirk).
        let mut v = vec![0, 0, 0, 0, RKT3, t, g, 0x30, 0x31, 0x32];
        v.extend([0, 0x55, 0, 0, RKT0, t, g, 0x99]);
        let get3 = |id| (id == RKT4).then_some(&v[..]);
        assert_eq!(occupant_models(get3, 4, 0, 0, false, 0).instances, [0x55, 0x99]);
        assert_eq!(rkt4_record_len(RKT2), Some(30));
    }
}
