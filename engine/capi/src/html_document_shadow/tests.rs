// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn set_observation_checks_membership_return_and_both_complete_postsets() {
    let view = |s: &[u32]| SetState {
        members: s.as_ptr(),
        count: s.len() as u32,
    };
    let before = view(&[18, 20, 32]);
    let source = view(&[20, 54, u32::MAX]);
    let union = view(&[18, 20, 32, 54, u32::MAX]);
    let check = |op, id, a: &SetState, sa: &SetState, value| unsafe {
        fc4_html_document_set_check(op, id, &before, a, &source, sa, value)
    };
    assert_eq!(check(0, 20, &before, &source, 1), 0);
    assert_eq!(check(0, 19, &before, &source, 0), 0);
    assert_eq!(check(0, 20, &before, &source, 0), 1);
    assert_eq!(check(0, 20, &union, &source, 1), 2);
    assert_eq!(check(1, 0, &union, &source, 0), 0);
    assert_eq!(check(1, 0, &before, &source, 0), 2);
    assert_eq!(check(1, 0, &union, &before, 0), 4);
    let empty = view(&[]);
    assert_eq!(
        unsafe { fc4_html_document_set_check(1, 0, &empty, &source, &source, &source, 0) },
        0
    );
    // A source aliased with the destination is still a set union, not an append.
    assert_eq!(
        unsafe { fc4_html_document_set_check(1, 0, &before, &before, &before, &before, 0) },
        0
    );
}

fn state(values: [&[u8]; 4], position: u32) -> TextState {
    let b = values.map(|s| Bytes {
        data: s.as_ptr(),
        len: s.len() as u32,
    });
    TextState {
        source: b[0],
        pending: b[1],
        tag: b[2],
        attributes: b[3],
        position,
    }
}
fn check(op: u32, before: &TextState, after: &TextState, value: u32) -> u32 {
    unsafe { fc4_html_document_text_check(op, before, after, value, None) }
}

#[test]
fn cdata_compares_return_cursor_and_every_string_without_poststate_seeding() {
    let before = state([b"ab<!--gone-->cd<T>", b"old", b"tag", b"attr"], 0);
    let after = state([b"ab<!--gone-->cd<T>", b"abcd", b"tag", b"attr"], 15);
    assert_eq!(check(0, &before, &after, 1), 0);
    assert_eq!(check(0, &before, &after, 0), 32);
    let mut changed = after;
    changed.position += 1;
    assert_eq!(check(0, &before, &changed, 1), 16);
    let mut changed = state([b"ab<!--gone-->cd<T>", b"ab cd", b"tag", b"attr"], 15);
    assert_eq!(check(0, &before, &changed, 1), 2);
    changed.pending = before.pending;
    assert_ne!(check(0, &before, &changed, 1), 0);
    for (field, mask) in [(0, 1), (2, 4), (3, 8)] {
        let mut strings: [&[u8]; 4] = [b"ab<!--gone-->cd<T>", b"abcd", b"tag", b"attr"];
        strings[field] = b"mutation";
        assert_eq!(check(0, &before, &state(strings, 15), 1), mask);
    }
}

#[test]
fn split_and_alignment_include_reference_argument_writes() {
    let before = state([b"", b"", b"a  href=\"x.html\" ", b"old"], 0);
    let after = state([b"", b"", b"A", b"href=\"x.html\""], 0);
    assert_eq!(check(3, &before, &after, 0), 0);
    assert_ne!(check(3, &before, &before, 0), 0);
    let input = state([b"", b"", b"rIgHt", b""], 0);
    let upper = state([b"", b"", b"RIGHT", b""], 0);
    assert_eq!(check(4, &input, &upper, 2), 0);
    assert_eq!(check(5, &input, &upper, 1), 0);
    assert_eq!(check(4, &input, &input, 2), 4);
    assert_eq!(check(5, &input, &upper, 2), 32);
}

#[test]
fn foreign_character_steps_are_used_for_whitespace() {
    unsafe extern "C" fn character(op: u32, _: *const u8, _: u32, i: u32) -> u32 {
        if op == 0 {
            u32::from(i < 2)
        } else {
            i + 2
        }
    }
    let before = state([b"\x81\x40x", b"", b"", b""], 0);
    let after = state([b"\x81\x40x", b"", b"", b""], 2);
    assert_eq!(
        unsafe { fc4_html_document_text_check(1, &before, &after, 0, Some(character)) },
        0
    );
    assert_ne!(check(1, &before, &after, 0), 0);
}

#[test]
fn font_getter_checks_zero_filled_tail_return_and_unchanged_owner() {
    let before = [8, 10, 12, 14, 18, 24, 36, 14, 14, 9];
    let mut output = [8, 10, 12, 14, 18, 24, 36, 0, 0];
    let check = |after: &[i32; 10], out: &[i32], value| unsafe {
        fc4_html_document_scalar_check(
            1,
            before.as_ptr(),
            after.as_ptr(),
            std::ptr::null(),
            out.len() as u32,
            out.as_ptr(),
            value,
        )
    };
    assert_eq!(check(&before, &output, 7), 0);
    output[8] = 42;
    assert_eq!(check(&before, &output, 7), 1);
    output[8] = 0;
    assert_eq!(check(&before, &output, 9), 1);
    let mut after = before;
    after[7] += 1;
    assert_eq!(check(&after, &output, 7), 2);
}

#[test]
fn short_font_setter_preserves_old_entry_three_and_slot_byte() {
    let before = [8, 10, 12, 14, 18, 24, 36, 99, 100, 9];
    let mut after = [6, 7, 8, 14, 18, 24, 36, 14, 14, 9];
    let input = [6, 7, 8];
    let check = |a: &[i32; 10]| unsafe {
        fc4_html_document_scalar_check(0, before.as_ptr(), a.as_ptr(), input.as_ptr(), 3, std::ptr::null(), 0)
    };
    assert_eq!(check(&after), 0);
    after[3] = 0;
    assert_eq!(check(&after), 2);
    after[3] = 14;
    after[9] = 0;
    assert_eq!(check(&after), 2);
}
