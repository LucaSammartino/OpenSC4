// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent literal ledgers from font-renderer.md section 6, written before production code.
use fc4_core::font_renderer::{draw, lock, unlock, Event, Lifetime, Memory, Record, Rect, Refusal, Release, Target};

struct Buffer {
    width: i32,
    height: i32,
    depth: u32,
    format: [u32; 2],
    pitch: u32,
    pixels: Vec<u8>,
    callbacks: Vec<(&'static str, u32)>,
    raw: u8,
}

impl Buffer {
    fn new(depth: u32, pitch: u32, bytes: usize) -> Self {
        Self {
            width: 8,
            height: 6,
            depth,
            format: [7, 16],
            pitch,
            pixels: vec![0; bytes],
            callbacks: Vec::new(),
            raw: 0,
        }
    }
}

impl Target for Buffer {
    fn width(&mut self) -> i32 {
        self.callbacks.push(("width", 0));
        self.width
    }
    fn height(&mut self) -> i32 {
        self.callbacks.push(("height", 0));
        self.height
    }
    fn depth(&mut self) -> u32 {
        self.callbacks.push(("depth", 0));
        self.depth
    }
    fn format(&mut self, words: &mut [u32; 2]) -> u8 {
        self.callbacks.push(("format", 0));
        *words = self.format;
        self.raw
    }
    fn pitch(&mut self) -> u32 {
        self.callbacks.push(("pitch", 0));
        self.pitch
    }
    fn pixels(&mut self) -> &mut [u8] {
        self.callbacks.push(("pixels", 0));
        &mut self.pixels
    }
    fn lock(&mut self, flags: u32) -> u8 {
        self.callbacks.push(("lock", flags));
        self.raw
    }
    fn unlock(&mut self, flags: u32) -> u8 {
        self.callbacks.push(("unlock", flags));
        self.raw
    }
}

fn rect(l: i32, t: i32, r: i32, b: i32) -> Rect {
    Rect {
        left: l,
        top: t,
        right: r,
        bottom: b,
    }
}

fn record(source: &[u8], destination: Rect, pitch: u32) -> Record<'_> {
    Record {
        destination,
        source,
        source_pitch: pitch,
        source_width: 1,
        source_height: 1,
        character: 0x20,
    }
}

fn reads(events: &[Event], kind: Memory) -> Vec<usize> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Read { memory, offset, .. } if *memory == kind => Some(*offset),
            _ => None,
        })
        .collect()
}

fn writes(events: &[Event]) -> Vec<(usize, u32)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Write { offset, value, .. } => Some((*offset, *value)),
            _ => None,
        })
        .collect()
}

#[test]
fn zero_count_and_control_character_have_no_target_or_colour_access() {
    let mut events = Vec::new();
    assert_eq!(draw(None, &[], 0, 3, &[], None, &mut events), Ok(true));
    assert!(events.is_empty());
    let mut skipped = record(&[], rect(0, 0, 1, 1), 0);
    skipped.character = 0x1f;
    assert_eq!(draw(None, &[skipped], 1, 3, &[], None, &mut events), Ok(true));
    assert_eq!(
        events,
        vec![Event::Record {
            index: 0,
            width: 1,
            height: 1,
            character: 0x1f
        }]
    );
}

#[test]
fn literal_five_by_three_clip_uses_source_27_to_55_and_target_40_76_112() {
    let mut target = Buffer::new(32, 36, 216);
    let source = [128; 56];
    let item = record(&source, rect(-2, -1, 7, 5), 12);
    let mut events = Vec::new();
    assert_eq!(
        draw(
            Some(&mut target),
            &[item],
            1,
            1,
            &0x89abcdefu32.to_le_bytes(),
            Some(rect(1, 1, 6, 4)),
            &mut events
        ),
        Ok(true)
    );
    assert_eq!(
        reads(&events, Memory::Source),
        vec![27, 28, 29, 30, 31, 39, 40, 41, 42, 43, 51, 52, 53, 54, 55]
    );
    assert_eq!(
        writes(&events),
        vec![
            (40, 0x89abcdef),
            (44, 0x89abcdef),
            (48, 0x89abcdef),
            (52, 0x89abcdef),
            (56, 0x89abcdef),
            (76, 0x89abcdef),
            (80, 0x89abcdef),
            (84, 0x89abcdef),
            (88, 0x89abcdef),
            (92, 0x89abcdef),
            (112, 0x89abcdef),
            (116, 0x89abcdef),
            (120, 0x89abcdef),
            (124, 0x89abcdef),
            (128, 0x89abcdef)
        ]
    );
    assert!(reads(&events, Memory::Target).is_empty());
    assert_eq!(
        target.callbacks,
        vec![("width", 0), ("height", 0), ("depth", 0), ("pitch", 0), ("pixels", 0)]
    );
    assert_eq!(
        &events[1..7],
        &[
            Event::Width(8),
            Event::Height(6),
            Event::Depth(32),
            Event::Read {
                memory: Memory::Colour,
                offset: 0,
                bytes: 4,
                value: 0x89abcdef
            },
            Event::Pitch(36),
            Event::PixelBase,
        ]
    );
}

#[test]
fn empty_reversed_edge_and_unsupported_depth_only_query_dimensions_and_optional_format() {
    let cases = [
        (rect(8, 1, 10, 3), None, 32),
        (rect(2, 2, 4, 4), Some(rect(4, 0, 8, 6)), 32),
        (rect(5, 1, 3, 4), None, 32),
        (rect(0, 6, 1, 7), None, 16),
        (rect(0, 0, 2, 2), None, 8),
        (rect(0, 2, 2, 1), None, 24),
    ];
    for (destination, clip, depth) in cases {
        for mode in [1, 3, 4] {
            let mut target = Buffer::new(depth, 0, 0);
            let mut events = Vec::new();
            assert_eq!(
                draw(
                    Some(&mut target),
                    &[record(&[], destination, 0)],
                    1,
                    mode,
                    &[],
                    clip,
                    &mut events
                ),
                Ok(true)
            );
            let mut expected = vec![("width", 0), ("height", 0), ("depth", 0)];
            if mode == 3 {
                expected.push(("format", 0));
            }
            assert_eq!(target.callbacks, expected);
            assert!(reads(&events, Memory::Colour).is_empty());
            assert!(writes(&events).is_empty());
        }
    }
}

#[test]
fn threshold_and_raw_palette_indices_use_four_byte_entries_at_all_supported_depths() {
    // Expected ledgers are section 6.2 literals; no production arithmetic is used as oracle.
    for (mode, coverage, expected16, expected32) in [
        (
            1,
            [0, 127, 128, 255],
            [0x3344u32, 0x3344, 0xcdef, 0xcdef],
            [0x11223344u32, 0x11223344, 0x89abcdef, 0x89abcdef],
        ),
        (
            2,
            [0, 1, 128, 255],
            [0x3344u32, 0x5678, 0xcdef, 0x3210],
            [0x11223344u32, 0x12345678, 0x89abcdef, 0x76543210],
        ),
    ] {
        let mut palette = vec![0; 1024];
        for (index, value) in [
            (0, 0xdeadbeefu32),
            (1, 0x12345678),
            (128, 0x89abcdef),
            (255, 0x76543210),
        ] {
            palette[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        let foreground = 0x89abcdefu32.to_le_bytes();
        let colours = if mode == 1 { &foreground[..] } else { &palette[..] };
        for depth in [16, 24, 32] {
            let bytes = if depth == 16 { 2 } else { 4 };
            let mut target = Buffer::new(depth, 16, 16);
            for pixel in target.pixels.chunks_mut(bytes) {
                pixel.copy_from_slice(&0x11223344u32.to_le_bytes()[..bytes]);
            }
            let mut events = Vec::new();
            assert_eq!(
                draw(
                    Some(&mut target),
                    &[record(&coverage, rect(0, 0, 4, 1), 4)],
                    1,
                    mode,
                    colours,
                    None,
                    &mut events
                ),
                Ok(true)
            );
            let expected = if depth == 16 { &expected16 } else { &expected32 };
            for (index, value) in expected.iter().enumerate() {
                assert_eq!(
                    &target.pixels[index * bytes..index * bytes + bytes],
                    &value.to_le_bytes()[..bytes]
                );
            }
            assert_eq!(
                reads(&events, Memory::Colour),
                if mode == 1 { vec![0] } else { vec![0, 4, 512, 1020] }
            );
            assert!(reads(&events, Memory::Target).is_empty());
            assert_eq!(writes(&events).len(), if mode == 1 { 2 } else { 3 });
        }
    }
}

#[test]
fn packed_word_literals_and_quantization_boundaries() {
    // f=31,d=0: q=0 => 0, q=3 => 2, q=4 => 3, q=31 => 30.
    let cases = [
        (7, 0, 0xffff, 0, 0, false),
        (7, 0, 0xffff, 1, 0xf000, true),
        (7, 0, 0xffff, 7, 0xf000, true),
        (7, 0, 0xffff, 255, 0xfeee, true),
        (4, 0x8000, 0, 1, 0, true),
        (6, 0x8000, 0, 1, 0, true),
        (4, 0, 0x7fff, 255, 0x7bde, true),
        (6, 0, 0x7fff, 255, 0x7bde, true),
        (5, 0x20, 0, 1, 0, true),
        (5, 0, 0xffff, 255, 0xf79e, true),
        (4, 0, 0x7fff, 31, 0x0842, true),
        (4, 0, 0x7fff, 32, 0x0c63, true),
        (3, 0x1234, 0xffff, 255, 0x1234, false),
    ];
    for (format, background, foreground, coverage, expected, stored) in cases {
        let mut target = Buffer::new(16, 2, 2);
        target.format = [format, 32]; // Second format word does not replace queried depth.
        target.pixels.copy_from_slice(&(background as u16).to_le_bytes());
        let mut events = Vec::new();
        assert_eq!(
            draw(
                Some(&mut target),
                &[record(&[coverage], rect(0, 0, 1, 1), 1)],
                1,
                3,
                &(foreground as u32).to_le_bytes(),
                None,
                &mut events
            ),
            Ok(true)
        );
        assert_eq!(target.pixels, (expected as u16).to_le_bytes());
        assert_eq!(writes(&events).len(), usize::from(stored));
        assert_eq!(reads(&events, Memory::Target).len(), usize::from(stored));
    }
}

#[test]
fn literal_dword_results_include_actual_zero_coverage_store_and_other_modes() {
    for depth in [24, 32] {
        for mode in [0, 3, 4, u32::MAX] {
            for (background, foreground, coverage, expected) in [
                (0x11223344u32, 0xffeeddccu32, 0, 0x11223344),
                (0x11223344, 0xffeeddcc, 128, 0x88888888),
                (0, 0xffffffff, 128, 0x80808080),
                (0, 0xffffffff, 255, 0xfefefefe),
            ] {
                let mut target = Buffer::new(depth, 4, 4);
                target.pixels.copy_from_slice(&background.to_le_bytes());
                let mut events = Vec::new();
                assert_eq!(
                    draw(
                        Some(&mut target),
                        &[record(&[coverage], rect(0, 0, 1, 1), 1)],
                        1,
                        mode,
                        &foreground.to_le_bytes(),
                        None,
                        &mut events
                    ),
                    Ok(true)
                );
                assert_eq!(writes(&events), vec![(0, expected)]);
                assert_eq!(reads(&events, Memory::Target), vec![0]);
                assert_eq!(target.pixels, expected.to_le_bytes());
            }
        }
    }
}

#[test]
fn pitch_quantization_and_fresh_record_origins() {
    for (depth, pitch, offsets) in [(16, 7, vec![2, 8]), (24, 15, vec![4, 16]), (32, 15, vec![4, 16])] {
        let mut target = Buffer::new(depth, pitch, 32);
        let mut events = Vec::new();
        let source = [128, 0, 0, 128];
        let first = record(&source, rect(1, 0, 2, 2), 3);
        assert_eq!(
            draw(Some(&mut target), &[first], 1, 1, &[1, 0, 0, 0], None, &mut events),
            Ok(true)
        );
        assert_eq!(writes(&events).iter().map(|p| p.0).collect::<Vec<_>>(), offsets);
        assert_eq!(reads(&events, Memory::Source), vec![0, 3]);
    }
    let mut target = Buffer::new(32, 32, 64);
    let mut events = Vec::new();
    let source = [128; 4];
    assert_eq!(
        draw(
            Some(&mut target),
            &[
                record(&source, rect(-1, 0, 1, 1), 4),
                record(&source, rect(0, 1, 1, 2), 4)
            ],
            2,
            1,
            &[1, 0, 0, 0],
            None,
            &mut events
        ),
        Ok(true)
    );
    assert_eq!(reads(&events, Memory::Source), vec![1, 0]);
    assert_eq!(writes(&events), vec![(0, 1), (32, 1)]);
}

#[test]
fn other_mode_word_default_format_is_noop_even_at_full_coverage() {
    let mut target = Buffer::new(16, 2, 2);
    target.pixels.copy_from_slice(&0x1234u16.to_le_bytes());
    let mut events = Vec::new();
    assert_eq!(
        draw(
            Some(&mut target),
            &[record(&[255], rect(0, 0, 1, 1), 1)],
            1,
            4,
            &[255; 4],
            None,
            &mut events
        ),
        Ok(true)
    );
    assert_eq!(target.pixels, [0x34, 0x12]);
    assert!(writes(&events).is_empty());
    assert!(reads(&events, Memory::Target).is_empty());
    assert!(!target.callbacks.iter().any(|p| p.0 == "format"));
}

#[test]
fn callback_failure_and_raw_al_forwarding_have_no_renderer_lock_state() {
    let mut target = Buffer::new(32, 4, 0);
    for raw in [0, 1, 0x80, 0xff] {
        target.raw = raw;
        assert_eq!(lock(&mut target), raw);
        assert_eq!(unlock(&mut target), raw);
    }
    assert_eq!(
        target.callbacks,
        vec![
            ("lock", 0x8080),
            ("unlock", 0x8080),
            ("lock", 0x8080),
            ("unlock", 0x8080),
            ("lock", 0x8080),
            ("unlock", 0x8080),
            ("lock", 0x8080),
            ("unlock", 0x8080)
        ]
    );
}

#[test]
fn safe_lifetime_models_signed_release_and_query_out_preservation() {
    let mut state = Lifetime::new();
    let mut out = 77;
    assert!(!state.query(0x9999, &mut out, 42).unwrap());
    assert_eq!(out, 77);
    assert_eq!(state.count(), Some(0));
    assert!(state.query(0xa8bc2a75, &mut out, 42).unwrap());
    assert_eq!(out, 42);
    assert_eq!(state.add_ref(), Ok(2));
    assert_eq!(state.release(), Ok(Release::Retained(1)));
    assert_eq!(state.release(), Ok(Release::Freed));
    assert!(state.add_ref().is_err());
    assert!(state.release().is_err());
    assert_eq!(Lifetime::new().release(), Ok(Release::Freed));
    let mut high = Lifetime::from_count(0xffffffff);
    assert_eq!(high.release(), Ok(Release::Freed));
    let mut wrap = Lifetime::from_count(0xffffffff);
    assert_eq!(wrap.add_ref(), Ok(0));
    let mut base = Lifetime::new();
    assert!(base.query(1, &mut out, 123).unwrap());
    assert_eq!(out, 123);
    assert_eq!(base.count(), Some(1));
}

#[test]
fn safety_extensions_refuse_invalid_backing_separately_from_native_vectors() {
    let mut events = Vec::new();
    assert_eq!(draw(None, &[], 1, 1, &[], None, &mut events), Err(Refusal::Records));
    assert_eq!(
        draw(
            None,
            &[record(&[128], rect(0, 0, 1, 1), 1)],
            1,
            1,
            &[],
            None,
            &mut events
        ),
        Err(Refusal::Target)
    );
    for (source, colours, target_bytes, expected_memory) in [
        (&[128][..], &[][..], 4, Memory::Colour),
        (&[][..], &[0; 4][..], 4, Memory::Source),
        (&[128][..], &[0; 1024][..], 3, Memory::Target),
        (&[255][..], &[0; 512][..], 4, Memory::Colour),
    ] {
        let mut target = Buffer::new(32, 4, target_bytes);
        let error = draw(
            Some(&mut target),
            &[record(source, rect(0, 0, 1, 1), 1)],
            1,
            2,
            colours,
            None,
            &mut Vec::new(),
        );
        assert!(matches!(error,Err(Refusal::Backing { memory, .. }) if memory==expected_memory));
    }
    let mut target = Buffer::new(32, 4, 4);
    assert_eq!(
        draw(
            Some(&mut target),
            &[record(&[128], rect(i32::MIN, 0, 1, 1), 1)],
            1,
            1,
            &[0; 4],
            None,
            &mut Vec::new()
        ),
        Err(Refusal::Geometry)
    );
    assert_eq!(
        draw(
            Some(&mut target),
            &[record(&[128], rect(0, 0, 1, 1), 0)],
            1,
            1,
            &[0; 4],
            None,
            &mut Vec::new()
        ),
        Err(Refusal::Geometry)
    );
}

#[test]
fn independent_asymmetric_word_components_use_dyadic_floor() {
    // Hand ledgers: 31*29/32=28, 31*28/32=27; (2+11)/2=6,
    // (3+12)/2=7, (4+13)/2=8. Forced bits are applied after component lookup.
    for (format, background, foreground, coverage, expected) in [
        (4, 0x7fffu16, 0u32, 31, 0x739cu16),
        (4, 0x7fff, 0, 32, 0x6f7b),
        (4, 0x7fff, 0, 128, 0x3def),
        (5, 0xffff, 0, 128, 0x7bcf),
        (7, 0x1234, 0xabcd, 128, 0xf678),
        (7, 0, 0x0111, 255, 0xf000),
    ] {
        let mut target = Buffer::new(16, 2, 2);
        target.format[0] = format;
        target.pixels.copy_from_slice(&background.to_le_bytes());
        let mut events = Vec::new();
        assert_eq!(
            draw(
                Some(&mut target),
                &[record(&[coverage], rect(0, 0, 1, 1), 1)],
                1,
                3,
                &foreground.to_le_bytes(),
                None,
                &mut events
            ),
            Ok(true)
        );
        assert_eq!(writes(&events), vec![(0, u32::from(expected))]);
    }
}

#[test]
fn safety_refusal_is_not_rollback_or_signed_count_rejection() {
    let mut target = Buffer::new(32, 8, 4);
    let mut events = Vec::new();
    assert_eq!(
        draw(
            Some(&mut target),
            &[record(&[128, 128], rect(0, 0, 2, 1), 2)],
            1,
            1,
            &0x89abcdefu32.to_le_bytes(),
            None,
            &mut events
        ),
        Err(Refusal::Backing {
            memory: Memory::Target,
            offset: 4,
            bytes: 4
        })
    );
    assert_eq!(target.pixels, 0x89abcdefu32.to_le_bytes());
    assert_eq!(writes(&events), vec![(0, 0x89abcdef)]);
    assert_eq!(reads(&events, Memory::Source), vec![0, 1]);

    let mut skipped = record(&[], rect(0, 0, 0, 0), 0);
    skipped.character = 0;
    events.clear();
    assert_eq!(
        draw(None, &[skipped], u32::MAX, 0, &[], None, &mut events),
        Err(Refusal::Records)
    );
    assert_eq!(
        events,
        vec![Event::Record {
            index: 0,
            width: 1,
            height: 1,
            character: 0
        }]
    );

    // FR2 address overflow is refused, not wrapped into an unrelated part of a window.
    let mut target = Buffer::new(32, i32::MAX as u32, 4);
    assert_eq!(
        draw(
            Some(&mut target),
            &[record(&[128], rect(2, 2, 3, 3), 1)],
            1,
            1,
            &[0; 4],
            None,
            &mut Vec::new()
        ),
        Err(Refusal::Geometry)
    );
}
