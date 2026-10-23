// SPDX-License-Identifier: GPL-3.0-or-later
use fc4_conformance::run_vector_text;

#[test]
fn lot_model_vectors_reject_discriminating_wrong_expectations() {
    // One wrong expected result per public vector operation. A dispatch-only or
    // no-op runner must not pass any of these deliberately incorrect vectors.
    let cases = [
        (
            include_str!("../vectors/lm01_foundation_steps.json"),
            "\"expect_base\": 8",
            "\"expect_base\": 24",
        ),
        (
            include_str!("../vectors/lm02_foundation_fallback.json"),
            "0x69F20003",
            "0x69F20002",
        ),
        (
            include_str!("../vectors/lm03_wall_texture.json"),
            "0x695C00CA",
            "0x695C00C9",
        ),
        (
            include_str!("../vectors/lm04_wall_facing.json"),
            "\"expect_draw\": true",
            "\"expect_draw\": false",
        ),
        (
            include_str!("../vectors/lm05_cell_mode.json"),
            "\"expect_mode\": \"alpha\"",
            "\"expect_mode\": \"opaque\"",
        ),
        (
            include_str!("../vectors/lm06_cell_orientation.json"),
            "\"expect_corners\": [[0,1],[0,0],[1,1],[1,0]]",
            "\"expect_corners\": [[0,0],[1,0],[0,1],[1,1]]",
        ),
    ];
    for (original, from, to) in cases {
        assert!(original.contains(from));
        assert!(run_vector_text(original).is_ok(), "{:#?}", run_vector_text(original));
        assert!(
            run_vector_text(&original.replacen(from, to, 1)).is_err(),
            "accepted wrong expectation: {from}"
        );
    }
}
