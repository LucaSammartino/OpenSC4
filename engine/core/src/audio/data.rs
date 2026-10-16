// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The sound data (`docs/specs/audio.md` §10). Section references (`§n`)
//! point at that spec.
//!
//! `Sound.dat`, the audio configuration record, the AEMS bank header and
//! module layouts, the sample-record header and the tuning keys. The counts
//! marked *measured* in the spec were read out of the shipped packages; they
//! are reproduced here as constants so a vector can assert on them.
//!
//! See the module-level `# Choices` in [`crate::audio`] for AU1 and AU9–AU12.

/// The `Sound.dat` inventory of §10.1 (**measured**): 122,372,241 bytes,
/// 2776 index entries.
pub const SOUND_DAT_BYTES: u64 = 122_372_241;
/// The number of index entries in `Sound.dat` (§10.1).
pub const SOUND_DAT_ENTRIES: u32 = 2776;

/// Type `0x2026960B` — samples, `.hit` and `.str` payloads: 1680 records
/// (§10.1). **Context-sensitive** (§10.2): the same TGI means different things
/// in different DB segments.
pub const TYPE_SAMPLE: u32 = 0x2026_960B;
/// Type `0x0B8D821A` — TKDT track definitions: 869 records (§10.1).
pub const TYPE_TKDT: u32 = 0x0B8D_821A;
/// Type `0x7B1ACFCD` — Hit lists: 214 records (§10.1).
pub const TYPE_HIT_LIST: u32 = 0x7B1A_CFCD;
/// Type `0x9D796DB4` — `.tlo`: 12 records (§10.1, §10.11).
pub const TYPE_TLO: u32 = 0x9D79_6DB4;
/// Type `0xE86B1EEF` — the DIR (compression map): 1 record (§10.1).
pub const TYPE_DIR: u32 = 0xE86B_1EEF;

/// Per-type counts of §10.1 (**measured**), in the table's order.
pub const TYPE_COUNTS: [(u32, u32, u64); 5] = [
    (TYPE_SAMPLE, 1680, 122_235_432),
    (TYPE_TKDT, 869, 72_807),
    (TYPE_HIT_LIST, 214, 5_240),
    (TYPE_TLO, 12, 970),
    (TYPE_DIR, 1, 2_176),
];

/// The payload magics of the 1680 `0x2026960B` records (§10.1, **measured**):
/// `XA\0\0` x1512, `vers` x71, `XAJ\0` x43, `XAI\0` x32, `RIFF` x21,
/// `HIT!` x1.
pub const SAMPLE_MAGICS: [(&[u8; 4], u32); 6] = [
    (b"XA\0\0", 1512),
    (b"vers", 71),
    (b"XAJ\0", 43),
    (b"XAI\0", 32),
    (b"RIFF", 21),
    (b"HIT!", 1),
];

/// The `0x2026960B` context table of §10.2: the binding is **data-driven**,
/// not hard-coded (§10.2, conformance test 18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleContext {
    /// `%DataDirectory%Sound\HitLists\` — LTEXT / strings, extension `.str`.
    HitLists,
    /// `%DataDirectory%Sound\TLO\` — Hit track data, extension `.hit`.
    Tlo,
    /// Every `[SampleDataSegments]` group — raw sample data.
    SampleData,
}

impl SampleContext {
    /// The extension this segment binds the type to (§10.2, §10.11).
    pub fn extension(self) -> &'static str {
        match self {
            SampleContext::HitLists => "str",
            SampleContext::Tlo => "hit",
            SampleContext::SampleData => "",
        }
    }
}

/// The type/extension binding the configuration record's
/// `[DBSegmentDirectoryFiles]` section declares (§10.3, **measured**), which
/// is what `cHitIniFile::ProcessDBSegmentDirectoryFilesEntry` (`0x00936707`)
/// registers (conformance test 18).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeExtBinding {
    /// The DB segment group id.
    pub group: u32,
    /// The resource type.
    pub type_: u32,
    /// The extension it is bound to.
    pub ext: &'static str,
}

/// The bindings of the two `[DBSegmentDirectoryFiles]` lines of §10.3.
pub const DB_SEGMENT_DIRECTORY_FILES: [TypeExtBinding; 5] = [
    // 0x9dbdbf74=0x7b1acfcd:hls,0x2026960b:str,0x7b1acfcd:hls
    TypeExtBinding {
        group: 0x9DBD_BF74,
        type_: TYPE_HIT_LIST,
        ext: "hls",
    },
    TypeExtBinding {
        group: 0x9DBD_BF74,
        type_: TYPE_SAMPLE,
        ext: "str",
    },
    TypeExtBinding {
        group: 0x9DBD_BF74,
        type_: TYPE_HIT_LIST,
        ext: "hls",
    },
    // 0xca4d19e3=0x9d796db4:tlo,0x2026960b:hit
    TypeExtBinding {
        group: 0xCA4D_19E3,
        type_: TYPE_TLO,
        ext: "tlo",
    },
    TypeExtBinding {
        group: 0xCA4D_19E3,
        type_: TYPE_SAMPLE,
        ext: "hit",
    },
];

/// Resolve a `0x2026960B` record to its extension by DB segment (§10.2,
/// conformance test 18): `.str` in HitLists, `.hit` in TLO, raw sample data in
/// the `[SampleDataSegments]` groups. `0x9D796DB4` resolves to `.tlo`.
pub fn resolve_extension(type_: u32, context: SampleContext) -> Option<&'static str> {
    match type_ {
        TYPE_SAMPLE => Some(match context {
            SampleContext::HitLists => "str",
            SampleContext::Tlo => "hit",
            SampleContext::SampleData => "",
        }),
        TYPE_TLO => Some("tlo"),
        TYPE_HIT_LIST => Some("hls"),
        _ => None,
    }
}

/// The `0x18`-byte sample-record header of §10.4 (**measured**).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleHeader {
    /// `0x00` — the magic.
    pub magic: [u8; 4],
    /// `0x04` — size-like; ≈3.73x the record size for the XA set
    /// *[inferred: decoded PCM byte count]*.
    pub decoded_size: u32,
    /// `0x08`, `0x0A` — both 1 in every measured record.
    pub unk_08: u16,
    pub unk_0a: u16,
    /// `0x0C` — 22050.
    pub rate_0c: u32,
    /// `0x10` — 44100 (88200 on the 4-channel set).
    pub rate_10: u32,
    /// `0x14` — channels: 2 (or 4).
    pub channels: u16,
    /// `0x16` — bits: 16.
    pub bits: u16,
}

/// The size of a sample-record header (§10.4).
pub const SAMPLE_HEADER_SIZE: usize = 0x18;

/// The XA header histogram of §10.4 (**measured**): `(22050, 44100, 2ch,
/// 16bit)` x1440, `(22050, 88200, 4ch, 16bit)` x72.
pub const XA_HEADER_HISTOGRAM: [(u32, u32, u16, u16, u32); 2] =
    [(22050, 44100, 2, 16, 1440), (22050, 88200, 4, 16, 72)];

/// The audio configuration record of §10.3: type `0x00000000`, group
/// `0x8A5971C5`, instance `0xEA8A1115` (**measured**, 4012 bytes).
pub const CONFIG_TYPE: u32 = 0x0000_0000;
/// The configuration record's group (§10.3).
pub const CONFIG_GROUP: u32 = 0x8A59_71C5;
/// The configuration record's instance (§10.3).
pub const CONFIG_INSTANCE: u32 = 0xEA8A_1115;
/// The configuration record's size (§10.3, measured).
pub const CONFIG_BYTES: u32 = 4012;

/// The `[Config]` section's default 2D / 3D data-source player class ids
/// (§10.3).
pub const DEFAULT_PLAYER_2D: u32 = 0xFDED_7F31;
/// The 3D default player class id (§10.3).
pub const DEFAULT_PLAYER_3D: u32 = 0xFDED_7F34;

/// The four data-source class ids of `[SndFileDefaultDataSourceClassIds]`
/// (§10.3): the codec set.
pub const DATA_SOURCE_CLASS: [(u32, &str); 4] = [
    (0x1D07_EB4B, "XA"),
    (0x1B6B_9806, "UTK"),
    (0xBB70_51F5, "WAV"),
    (0x3CEC_2B47, "MP3"),
];

/// The `0x0B8D821A` groups in `Sound.dat` are exactly `[TrackDefSegments]`
/// (§10.3, measured).
pub const TRACK_DEF_SEGMENTS: [u32; 7] = [
    0xAA4D_1920,
    0xEA4D_192A,
    0xAA4D_1930,
    0x2A4D_1937,
    0x2A4D_193D,
    0xCA4D_1943,
    0xCA4D_1948,
];

/// The resource factory registered for type `0x2026960B` at `0x00457EDA`
/// (§10.5). The CLSID `0x00436EB4` is **not** in the class-id table (AU10).
pub const RESOURCE_FACTORY_CLSID: u32 = 0x0043_6EB4;
/// `cRZStringResource::QueryInterface` answers this IID with `this+0x14`
/// (§10.5).
pub const IID_STRING_RESOURCE: u32 = 0x456B_8F1D;

// ---- AEMS banks (§10.7) -----------------------------------------------------

/// The AEMS bank header of §10.7 (**measured** from bank `0x8C0AE396`,
/// 1,629,888 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AemsBankHeader {
    /// `0x00` — magic `"AB"` (`41 42`).
    pub magic: [u8; 2],
    /// `0x02` — the **format version** (9). `<8` -> module stride `0x14`,
    /// `>=8` -> stride `0x1C`; `>4` enables the `+0x34` relocation.
    pub version: u8,
    /// `0x04` — the total image size.
    pub image_size: u32,
    /// `0x08` — a constant, `0x00184945` in all 43 banks.
    pub constant: u32,
    /// `0x0C` — the module count.
    pub module_count: u32,
    /// `0x10` — the module table offset (`0x38` in all 43).
    pub module_table_offset: u32,
    /// `0x14` — the bank id.
    pub bank_id: u16,
    /// `0x16` — unknown (`0x18`).
    pub unk_16: u16,
    /// `0x18` — count of `0xC`-byte records at the `+0x2C` table.
    pub record_count: u16,
    /// `0x1C` — the offset of SNDBANK blob #1.
    pub sndbank1_offset: u32,
    /// `0x20` — trailing region #1 size.
    pub region1_size: u32,
    /// `0x24` — offset of SNDBANK blob #2.
    pub sndbank2_offset: u32,
    /// `0x28` — trailing region #2 size.
    pub region2_size: u32,
    /// `0x2C` — offset of the `0xC`-byte record table.
    pub record_table_offset: u32,
    /// `0x34` — optional extra table, relocated only when `version > 4`.
    pub extra_table: u32,
}

/// The size of the AEMS bank header (§10.7).
pub const AEMS_BANK_HEADER_SIZE: usize = 0x38;

/// The module-record stride for a bank of version >= 8 (§10.7).
pub const AEMS_MODULE_STRIDE_V8: usize = 0x1C;
/// The module-record stride for a bank of version < 8 (§10.7).
pub const AEMS_MODULE_STRIDE_V7: usize = 0x14;

impl AemsBankHeader {
    /// The module-record stride this bank's version selects (§10.7).
    pub fn module_stride(&self) -> usize {
        if self.version >= 8 {
            AEMS_MODULE_STRIDE_V8
        } else {
            AEMS_MODULE_STRIDE_V7
        }
    }

    /// Whether the `+0x34` extra table is relocated: only when `version > 4`
    /// (§10.7).
    pub fn relocates_extra_table(&self) -> bool {
        self.version > 4
    }

    /// `size - region2 - region1`: the size of the header + module-table
    /// prefix (§10.7). `SNDAEMSI_addmodulebank` copies that many bytes and
    /// appends the decompressed bank blobs at that offset.
    pub fn prefix_size(&self) -> u32 {
        self.image_size
            .wrapping_sub(self.region2_size)
            .wrapping_sub(self.region1_size)
    }
}

/// The module record of §10.7 (stride `0x1C` for version >= 8).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AemsModule {
    /// `0x00` — the module id.
    pub module_id: u16,
    /// `0x02` — the instance count.
    pub instance_count: u16,
    /// `0x04` — the record (node) count.
    pub record_count: u16,
    /// `0x06` — the update period.
    pub update_period: u16,
    /// `0x0C` — the record table offset.
    pub record_table_offset: u32,
    /// `0x14`, `0x18` — unknown (`0x32`, `0x26`).
    pub unk_14: u32,
    pub unk_18: u32,
}

/// `cSC4AudioAEMSSoundBank` object size (§10.7): `0x24` = 36 bytes.
pub const AEMS_SOUNDBANK_SIZE: usize = 0x24;

/// Where the `"AB"` magic is checked (AU1): **not** in `LoadFromFile` or
/// `SNDAEMSI_addmodulebank`. This module therefore does not require it.
/// Returns whether the magic is present, without failing on a mismatch.
pub fn aems_magic_present(header: &AemsBankHeader) -> bool {
    header.magic == *b"AB"
}

// ---- tuning (§10.8) ---------------------------------------------------------

/// The audio settings exemplar every tuning reader pulls: type `0x6534284A`,
/// group `0xCA25875D`, instance `0x6A359C5C` (§10.8, **measured**, 641 bytes).
pub const TUNING_TYPE: u32 = 0x6534_284A;
/// The exemplar's group (§10.8).
pub const TUNING_GROUP: u32 = 0xCA25_875D;
/// The exemplar's instance (§10.8).
pub const TUNING_INSTANCE: u32 = 0x6A35_9C5C;

/// Tuning key `0x6A946A50` -> `+0x70` response curve, default `(0, 1024.0)`.
pub const TUNING_CURVE_KEY: u32 = 0x6A94_6A50;
/// Tuning key `0xAABAA0D3` -> `+0x64`, default **3**.
pub const TUNING_VOICE_KEY: u32 = 0xAABA_A0D3;
/// Tuning key `0x0A946A57` -> `+0x68`, default **10**.
pub const TUNING_ONE_SHOT_KEY: u32 = 0x0A94_6A57;
/// Tuning key `0x4A946A5D` -> `+0x6C`, default **30**.
pub const TUNING_UPDATE_KEY: u32 = 0x4A94_6A5D;

/// The measured value of `0x6A946A50`: `(0,1024) (2,950) (3,900) (4,850)
/// (5,800)` (§10.8).
pub const SHIPPED_CURVE: [(f32, f32); 5] = [(0.0, 1024.0), (2.0, 950.0), (3.0, 900.0), (4.0, 850.0), (5.0, 800.0)];

/// The default of `0x6A946A50` when the read fails: `(0, 1024.0)` (§10.8).
pub const DEFAULT_CURVE: [(f32, f32); 1] = [(0.0, 1024.0)];

/// The four values `cSC4AudioSoundManager::_ReadTuningValues` reads (§10.8).
///
/// **No clamps** are applied in any of the three readers *[compat]*.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundManagerTuning {
    /// `+0x64`, key `0xAABAA0D3`. A voice-count gate used by `_StartSound`.
    pub voice_gate: u32,
    /// `+0x68`, key `0x0A946A57`. The other voice-count gate.
    pub one_shot: u32,
    /// `+0x6C`, key `0x4A946A5D`.
    pub update_period: u32,
    /// `+0x70`, key `0x6A946A50` — the `cSC4ResponseCurve`.
    pub curve: [(f32, f32); 5],
}

impl Default for SoundManagerTuning {
    /// The constructor defaults of §10.8: 3, 10, 30 and the curve
    /// `(0, 1024.0)`.
    fn default() -> Self {
        SoundManagerTuning {
            voice_gate: 3,
            one_shot: 10,
            update_period: 30,
            curve: [DEFAULT_CURVE[0], (0.0, 0.0), (0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
        }
    }
}

impl SoundManagerTuning {
    /// The shipped exemplar values of §10.8 (**measured**).
    pub fn shipped() -> Self {
        SoundManagerTuning {
            voice_gate: 3,
            one_shot: 10,
            update_period: 30,
            curve: SHIPPED_CURVE,
        }
    }
}

/// The five keys `cSC4AudioAEMSEventSystem::ReadTuningValues` (`0x004E1EA0`)
/// reads (§10.8).
pub const AEMS_TUNING_KEYS: [(u32, u32); 3] = [
    (0xCBFB_03FA, 1),  // -> +0x110, default 1.0f
    (0xCBFB_03FF, 1),  // -> +0x114, default 1.0f
    (0xEBFB_0404, 37), // -> +0x118, default 0x25
];

// ---- `cSC4SoundDescription` (§10.9) -----------------------------------------

/// `cSC4SoundDescription` (§10.9): object size `0x1C` (28 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoundDescription {
    /// `+0x0C` — unused / flags. Read `& 1` by `Read` (`0x005BE790`); all 305
    /// shipped records have it 0 and nothing reads it (§9.4).
    pub unused: u32,
    /// `+0x10` — the sound id, bound to a name by `cGroupSoundCommand::Parse`.
    pub sound_id: u32,
    /// `+0x14` — `1.0 / locationUpdateRate` (0 when rate <= 0), default `0.5`.
    pub inv_location_rate: f32,
    /// `+0x18` — the length.
    pub length: u32,
}

/// The default of `+0x14` at construction: `0x3F000000` = 0.5f (§10.9).
pub const DEFAULT_INV_LOCATION_RATE: f32 = 0.5;

impl SoundDescription {
    /// A description as the constructor leaves it (§10.9).
    pub fn new() -> Self {
        SoundDescription {
            unused: 0,
            sound_id: 0,
            inv_location_rate: DEFAULT_INV_LOCATION_RATE,
            length: 0,
        }
    }

    /// `cGroupSoundCommand::Parse`'s `-locationUpdateRate` (§10.9):
    /// `+0x14 = (rate > 0) ? 1.0f/rate : 0`.
    pub fn set_location_update_rate(&mut self, rate: f32) {
        self.inv_location_rate = if rate > 0.0 { 1.0 / rate } else { 0.0 };
    }
}

impl Default for SoundDescription {
    fn default() -> Self {
        Self::new()
    }
}

/// The ignore bit of the component record `+0x10` (§9.3). The text parser
/// derives it as `bit0 = (length == 0)`, a rule that **does not hold in the
/// shipped binary** for three of the four exceptions (§9.3, §14.17).
pub fn ignore_bit_from_length(length: u32) -> u32 {
    if length == 0 {
        1
    } else {
        0
    }
}

// ---- the `.tlo` records (§10.11, AU11) --------------------------------------

/// The 16-byte header of a `.tlo` record (§10.11, **measured** on the smallest
/// record, instance `0xA9225D22`, 48 bytes decompressed): `u32 0, u32 count,
/// u32 0, u32 0`, then `count` variable-length entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TloHeader {
    /// `+0x00` — always 0 in the measured record.
    pub unk_00: u32,
    /// `+0x04` — the entry count.
    pub count: u32,
    /// `+0x08`, `+0x0C` — always 0.
    pub unk_08: u32,
    pub unk_0c: u32,
}

/// The `.tlo` header size (§10.11).
pub const TLO_HEADER_SIZE: usize = 16;

/// The `.tlo` entry stride is **not** constant: `(size-16)/count` varies from
/// 32 to 40 across the 12 records (§10.11). AU11 is open, so entries are
/// treated as opaque and the stride is reported, not assumed.
pub fn tlo_entry_stride(record_size: usize, count: u32) -> Option<usize> {
    if count == 0 {
        return None;
    }
    (record_size - TLO_HEADER_SIZE).checked_div(count as usize)
}

// ---- the Hit list format (§10.10) -------------------------------------------

/// A Hit list (`0x7B1ACFCD`, group `0x9DBDBF74`) is `u32 version = 1; u32
/// count; count x u32 instance-id` (§10.10, **measured**). The entries are
/// instance ids of `.hit` records; the high byte carries instance flags
/// *[inferred]*.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HitList {
    /// The version: 1.
    pub version: u32,
    /// The entries.
    pub entries: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Conformance test 18: type + extension binding, not a constant.
    #[test]
    fn type_extension_binding() {
        assert_eq!(resolve_extension(TYPE_SAMPLE, SampleContext::HitLists), Some("str"));
        assert_eq!(resolve_extension(TYPE_SAMPLE, SampleContext::Tlo), Some("hit"));
        assert_eq!(resolve_extension(TYPE_SAMPLE, SampleContext::SampleData), Some(""));
        assert_eq!(resolve_extension(TYPE_TLO, SampleContext::Tlo), Some("tlo"));
        assert_eq!(resolve_extension(TYPE_TKDT, SampleContext::Tlo), None);
    }

    /// The `[DBSegmentDirectoryFiles]` lines of §10.3 carry `0x9D796DB4:tlo`.
    #[test]
    fn db_segment_binds_tlo() {
        let b = DB_SEGMENT_DIRECTORY_FILES.iter().find(|b| b.type_ == TYPE_TLO).unwrap();
        assert_eq!(b.ext, "tlo");
        assert_eq!(b.group, 0xCA4D_19E3);
    }

    /// The `Sound.dat` inventory of §10.1.
    #[test]
    fn sound_dat_inventory() {
        assert_eq!(SOUND_DAT_ENTRIES, 2776);
        let total: u32 = TYPE_COUNTS.iter().map(|c| c.1).sum();
        assert_eq!(total, 2776, "the five type counts sum to the entry count");
        let sample: u32 = SAMPLE_MAGICS.iter().map(|m| m.1).sum();
        assert_eq!(sample, 1680, "the payload magics sum to the type count");
    }

    /// Conformance test 19: the tuning keys and the shipped values.
    #[test]
    fn tuning_keys_and_values() {
        assert_eq!(SoundManagerTuning::default().voice_gate, 3);
        assert_eq!(SoundManagerTuning::default().one_shot, 10);
        assert_eq!(SoundManagerTuning::default().update_period, 30);
        let shipped = SoundManagerTuning::shipped();
        assert_eq!(shipped.curve[0], (0.0, 1024.0));
        assert_eq!(shipped.curve[4], (5.0, 800.0));
        assert_ne!(TUNING_CURVE_KEY, TUNING_VOICE_KEY);
        assert_ne!(TUNING_VOICE_KEY, TUNING_ONE_SHOT_KEY);
        assert_ne!(TUNING_ONE_SHOT_KEY, TUNING_UPDATE_KEY);
    }

    /// §10.7: the version byte selects the module stride and gates the extra
    /// table.
    #[test]
    fn aems_version_gates() {
        let mut h = AemsBankHeader {
            magic: *b"AB",
            version: 9,
            image_size: 1_629_888,
            constant: 0x0018_4945,
            module_count: 1,
            module_table_offset: 0x38,
            bank_id: 30989,
            unk_16: 0x18,
            record_count: 0,
            sndbank1_offset: 0x0E40,
            region1_size: 1_626_240,
            sndbank2_offset: 0,
            region2_size: 0,
            record_table_offset: 0x834,
            extra_table: 0,
        };
        assert!(aems_magic_present(&h));
        assert_eq!(h.module_stride(), 0x1C);
        assert!(h.relocates_extra_table());
        assert_eq!(h.prefix_size(), 3648, "size - region2 - region1 (§10.7)");

        h.version = 7;
        assert_eq!(h.module_stride(), 0x14);
        assert!(h.relocates_extra_table(), "version > 4");
        h.version = 4;
        assert!(!h.relocates_extra_table());
    }

    /// §10.9: `-locationUpdateRate` stores the reciprocal.
    #[test]
    fn sound_description_rate() {
        let mut d = SoundDescription::new();
        assert_eq!(d.inv_location_rate, 0.5);
        d.set_location_update_rate(2.0);
        assert_eq!(d.inv_location_rate, 0.5);
        d.set_location_update_rate(0.0);
        assert_eq!(d.inv_location_rate, 0.0);
        d.set_location_update_rate(-1.0);
        assert_eq!(d.inv_location_rate, 0.0);
    }

    /// §9.3 / §14.17: the text parser's rule is `bit0 = (length == 0)`.
    #[test]
    fn ignore_bit_from_length_rule() {
        assert_eq!(ignore_bit_from_length(0), 1);
        assert_eq!(ignore_bit_from_length(50), 0);
    }

    /// AU11: the `.tlo` stride is reported, not assumed.
    #[test]
    fn tlo_stride_is_variable() {
        assert_eq!(tlo_entry_stride(48, 1), Some(32));
        assert_eq!(tlo_entry_stride(56, 1), Some(40));
        assert_eq!(tlo_entry_stride(48, 0), None);
    }
}
