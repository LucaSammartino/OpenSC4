// SPDX-License-Identifier: GPL-3.0-or-later
use super::{composition::*, conversion::*, *};

#[derive(Default)]
struct Environment {
    wide: Vec<u16>,
    calls: Vec<String>,
}
impl CodePages for Environment {
    fn system_code_page(&mut self) -> i32 {
        self.calls.push("system".into());
        932
    }
    fn multi_byte_to_wide(&mut self, p: i32, flags: u32, s: &[u8], d: &mut [u16]) -> usize {
        self.calls.push(format!("mb:{p}:{flags}:{s:?}:{}", d.len()));
        let n = self.wide.len().min(d.len());
        d[..n].copy_from_slice(&self.wide[..n]);
        n
    }
    fn wide_to_multi_byte(&mut self, p: i32, flags: u32, s: &[u16], d: &mut [u8], default: u8) -> usize {
        self.calls.push(format!("wc:{p}:{flags}:{s:?}:{}:{default}", d.len()));
        if d.len() < s.len() {
            return 0;
        }
        for (out, &word) in d.iter_mut().zip(s) {
            *out = word as u8;
        }
        s.len()
    }
    fn composition_tail(&mut self) -> u16 {
        0x0e33
    }
    fn temporary_buffer(&mut self, bytes: u32, heap: bool) {
        self.calls.push(format!("temp:{bytes}:{heap}"));
    }
    fn free_temporary(&mut self) {
        self.calls.push("free".into());
    }
}
fn bytes(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

#[test]
fn te5_same_buffer_equal_selector_ignores_capacity() {
    let mut cap = 1;
    assert_eq!(
        convert(&[], 5, 16, &mut [], &mut cap, 16, true, &mut Environment::default()).unwrap(),
        5
    );
    assert_eq!(cap, 5);
}
#[test]
fn te5_same_buffer_distinct_selector_keeps_capacity() {
    let mut cap = 7;
    assert_eq!(
        convert(
            &[0xc2, 0xa0],
            2,
            8,
            &mut [],
            &mut cap,
            16,
            true,
            &mut Environment::default()
        )
        .unwrap(),
        0
    );
    assert_eq!(cap, 7);
}
fn decoding(input: &[u8], expected: &[u16], consumed: usize, capacity: usize) {
    let mut output = vec![0x55; capacity * 2 + 1];
    let mut cap = capacity;
    assert_eq!(
        convert(
            input,
            input.len(),
            8,
            &mut output,
            &mut cap,
            16,
            false,
            &mut Environment::default()
        )
        .unwrap(),
        consumed
    );
    assert_eq!(cap, expected.len());
    assert_eq!(&output[..cap * 2], bytes(expected));
    assert_eq!(output[cap * 2], 0x55);
}
#[test]
fn te5_invalid_utf8_preserves_prefix() {
    decoding(&[0x41, 0xc0, 0xaf], &[0x41], 1, 3);
}
#[test]
fn te5_utf8_surrogate_word() {
    decoding(&[0xed, 0xa0, 0x80], &[0xd800], 3, 1);
}
#[test]
fn te5_utf8_four_byte_low_word() {
    decoding(&[0xf0, 0x9f, 0x98, 0x80], &[0xf600], 4, 1);
}
#[test]
fn te5_utf8_five_byte_low_word() {
    decoding(&[0xf8, 0x88, 0x80, 0x80, 0x80], &[0], 5, 1);
}
#[test]
fn te5_encoder_consumes_unwritten_word() {
    let mut output = [0x55; 4];
    let mut cap = 1;
    assert_eq!(
        convert(
            &bytes(&[0x41, 0x800]),
            2,
            16,
            &mut output,
            &mut cap,
            8,
            false,
            &mut Environment::default()
        )
        .unwrap(),
        2
    );
    assert_eq!((output, cap), ([0x41, 0x55, 0x55, 0x55], 1));
    cap = 0;
    assert_eq!(
        convert(
            &bytes(&[0x800]),
            1,
            16,
            &mut [],
            &mut cap,
            8,
            false,
            &mut Environment::default()
        )
        .unwrap(),
        1
    );
}
#[test]
fn te5_encoder_lone_surrogate() {
    let mut output = [0; 3];
    let mut cap = 3;
    assert_eq!(
        convert(
            &bytes(&[0xd800]),
            1,
            16,
            &mut output,
            &mut cap,
            8,
            false,
            &mut Environment::default()
        )
        .unwrap(),
        1
    );
    assert_eq!((output, cap), ([0xed, 0xa0, 0x80], 3));
}
#[test]
fn te5_two_stage_prefix_is_not_rolled_back() {
    let mut env = Environment {
        wide: vec![0x41, 0x800, 0x42],
        ..Environment::default()
    };
    let mut output = [0x55; 3];
    let mut cap = 1;
    assert_eq!(
        convert(&[0x81, 0x82, 0x83], 3, 932, &mut output, &mut cap, 8, false, &mut env).unwrap(),
        0
    );
    assert_eq!((output, cap), ([0x41, 0x55, 0x55], 0));
    assert_eq!(env.calls, ["temp:10:false", "mb:932:0:[129, 130, 131]:10"]);
}
#[test]
fn te5_thai_below_mark_context() {
    assert_eq!((thai_level(0xe38, 0x41, 0), thai_level(0xe38, 0xe01, 0)), (0, 5));
    let e = TextEdit::new(
        1,
        2,
        0,
        Encoding {
            selector: 8,
            system_selector: 0,
        },
    );
    struct Native(Environment);
    impl CodePages for Native {
        fn system_code_page(&mut self) -> i32 {
            0
        }
        fn multi_byte_to_wide(&mut self, p: i32, f: u32, s: &[u8], d: &mut [u16]) -> usize {
            self.0.multi_byte_to_wide(p, f, s, d)
        }
        fn wide_to_multi_byte(&mut self, p: i32, f: u32, s: &[u16], d: &mut [u8], b: u8) -> usize {
            self.0.wide_to_multi_byte(p, f, s, d, b)
        }
        fn composition_tail(&mut self) -> u16 {
            0
        }
    }
    impl TextForeign for Native {
        fn correction(&mut self, _: u32) {}
        fn invalidate(&mut self, _: Handle) {}
        fn notify(&mut self, _: Handle, _: u32) {}
    }
    let mut f = Native(Environment::default());
    let mut mark = vec![0xe0, 0xb8, 0xb8];
    assert_eq!(e.fix_composition(&mut mark, b"A", 1, &mut f).unwrap(), 3);
    assert!(mark.is_empty());
    mark = vec![0xe0, 0xb8, 0xb8];
    assert_eq!(e.fix_composition(&mut mark, &[0xe0, 0xb8, 0x81], 3, &mut f).unwrap(), 0);
}
#[test]
fn te5_thai_tone_following_context() {
    assert_eq!((thai_level(0xe48, 0xe01, 0), thai_level(0xe48, 0xe01, 0xe33)), (3, 2));
}
#[test]
fn te5_fourth_word_is_read_with_count_three() {
    assert_eq!(thai_group(&[0xe01, 0xe48, 0xe34, 0xe33], 3).unwrap(), 4);
    assert_eq!(
        forward(
            &bytes(&[0xe01, 0xe48, 0xe34, 0xe33]),
            3,
            16,
            &mut Environment::default()
        )
        .unwrap(),
        4
    );
    assert!(thai_group(&[0xe01, 0xe48, 0xe34], 3).is_err());
}
#[test]
fn te5_sara_am_forward_backward_asymmetry() {
    let input = bytes(&[0xe01, 0xe33]);
    let mut env = Environment::default();
    assert_eq!(forward(&input, 2, 16, &mut env).unwrap(), 2);
    assert_eq!(backward(&input, 2, 16, &mut env).unwrap(), 1);
}
#[test]
fn te5_utf8_base_and_decoration_both_directions() {
    let input = [0xe0, 0xb8, 0x81, 0xe0, 0xb9, 0x88];
    let mut env = Environment::default();
    assert_eq!(forward(&input, 6, 8, &mut env).unwrap(), 6);
    assert_eq!(backward(&input, 6, 8, &mut env).unwrap(), 6);
}

#[test]
fn te5_character_adapter_count_and_ascii_shortcuts() {
    let mut env = Environment::default();
    let mut count = 0;
    assert_eq!(character(&[], &mut count, 8, &mut env).unwrap(), 0);
    count = 7;
    assert_eq!(character(b"A", &mut count, 8, &mut env).unwrap(), 0x41);
    assert_eq!(count, 7);
    count = 9;
    assert_eq!(previous_character(b"A", &mut count, 100, &mut env).unwrap(), 0x41);
    assert_eq!(count, 9);
    count = 1;
    assert_eq!(character(&[0x7f], &mut count, 8, &mut env).unwrap(), 0x7f);
    count = 1;
    assert_eq!(character(&[0x41, 0], &mut count, 16, &mut env).unwrap(), 0x41);
    count = 2;
    assert_eq!(previous_character(&[0xc0, 0xaf], &mut count, 8, &mut env).unwrap(), 0);
    assert_eq!(count, 0);
}
