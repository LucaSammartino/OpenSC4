// SPDX-License-Identifier: GPL-3.0-or-later
//! win-filebrowser §§8.3.1–8.3.3 worked resource cases, without a pixel oracle.
use super::{file_browser::*, *};

#[test]
fn fbq1_loader_installs_before_reporting_the_pixel_observation() {
    let mut b = FileBrowser::new(80);
    let mut f = Recording {
        buffer_ok: true,
        fb_no_strip: true,
        fb_locks: [true, true].into(),
        ..Recording::default()
    };
    b.icons[2] = Some(900);
    // The owner reports unavailable geometry, not a native Boolean failure.
    assert_eq!(
        b.load_default_icon(2, &mut f),
        Err(Unsupported::Open("FBQ1 icon primitive trace, §8.3.3"))
    );
    assert_eq!(b.icons[2], Some(700));
    assert_eq!(f.refs, [(true, 700), (false, 900), (false, 700)]);
    assert_eq!(f.fb_buffer_formats, [(700, 18, 17, [3, 16])]);
    assert_eq!(f.fb_lock_flags, [0x8001, 0x8004]);
    assert_eq!(f.fb_keys, [0xa5ff_00ff]);
    assert_eq!(
        f.fb_resource_draws,
        [Draw::Fill {
            rect: [0, 0, 18, 17],
            color: 0xa5ff_00ff,
        }]
    );
}

#[test]
fn fbq1_full_width_selectors_failed_locks_and_loader_rejection() {
    let mut b = FileBrowser::new(80);
    for selector in [7, u32::MAX] {
        let mut f = Recording::default();
        assert_eq!(
            b.draw_default_icon(selector, Some(700), [2, 3, 20, 20], &mut f),
            Ok(true)
        );
        assert_eq!(f.fb_lock_flags, [0x8001]);
        assert_eq!(f.fb_keys, [0xa5ff_00ff]);
        assert!(f.fb_resource_draws.is_empty());
        assert!(!f.icon_calls.contains(&"unlock"));
        assert_eq!(
            f.fb_rgb,
            [
                [255, 0, 255],
                [0, 0, 0],
                [255, 248, 144],
                [191, 184, 191],
                [127, 120, 127],
                [112, 112, 112],
                [175, 176, 175],
                [255, 255, 255],
            ]
        );
    }
    for selector in 0..=6 {
        let mut f = Recording::default();
        assert_eq!(
            b.draw_default_icon(selector, Some(700), [0, 0, 18, 17], &mut f),
            Ok(true)
        );
        assert_eq!(f.fb_lock_flags, [0x8001, 0x8004]);
        assert_eq!(f.fb_keys, [0xa5ff_00ff]);
        assert!(f.fb_resource_draws.is_empty());
        assert!(!f.icon_calls.contains(&"unlock"));
    }
    let mut f = Recording::default();
    assert_eq!(b.draw_default_icon(0, None, [0; 4], &mut f), Ok(true));
    assert!(f.icon_calls.is_empty());

    // §8.3: rejection by the icon setter does not reject the loaded buffer.
    let mut f = Recording {
        buffer_ok: true,
        lock_ok: true,
        fb_buffer_factory: Some(702),
        ..Recording::default()
    };
    assert_eq!(b.load_default_icon(u32::MAX, &mut f), Ok(true));
    assert_eq!(b.icons, [None; 7]);
    assert_eq!(f.refs, [(false, 702)]);
    assert!(!f.icon_calls.contains(&"graphics"));
    assert_eq!(f.fb_copies, [(702, 701, [-18, 0, 0, 17], [0, 0, 18, 17])]);
}

#[test]
fn fbq1_allocation_and_initialization_failures_precede_drawing() {
    let b = FileBrowser::new(80);
    for (factory, graphics, initialized, expected_buffer) in [
        (None, false, true, None),
        (None, true, false, Some(700)),
        (Some(702), true, false, Some(702)),
    ] {
        for button in [false, true] {
            let mut f = Recording {
                fb_buffer_factory: factory,
                no_graphics: !graphics,
                buffer_ok: initialized,
                ..Recording::default()
            };
            let mut output = None;
            let result = if button {
                b.create_button_buffer(4, &mut output, &mut f)
            } else {
                b.create_combo_arrow(&mut output, &mut f)
            };
            assert_eq!(result, Ok(false));
            assert_eq!(output, None);
            assert_eq!(
                f.refs,
                expected_buffer.map(|h| (false, h)).into_iter().collect::<Vec<_>>()
            );
            assert!(f.fb_rgb.is_empty() && f.fb_lock_flags.is_empty() && f.fb_keys.is_empty());
            assert!(f.fb_resource_draws.is_empty());
            assert_eq!(f.icon_calls.contains(&"graphics"), factory.is_none());
            let dimensions = if button { (84, 22) } else { (64, 15) };
            assert_eq!(
                f.fb_buffer_formats,
                expected_buffer
                    .map(|h| (h, dimensions.0, dimensions.1, [3, 16]))
                    .into_iter()
                    .collect::<Vec<_>>()
            );
        }
    }
    for selector in [0, 1, 4, u32::MAX] {
        let mut f = Recording {
            buffer_ok: true,
            ..Recording::default()
        };
        let mut output = None;
        assert_eq!(b.create_button_buffer(selector, &mut output, &mut f), Ok(true));
        assert_eq!(output, Some(700));
        assert_eq!(
            f.fb_rgb,
            [
                [125, 125, 125],
                [170, 170, 170],
                [255, 0, 255],
                [255, 255, 255],
                [100, 100, 100]
            ]
        );
        assert_eq!(f.fb_lock_flags, [0x8001, 0x8004]);
        assert_eq!(f.fb_keys, [0xa5ff_00ff]);
        assert!(f.fb_resource_draws.is_empty() && f.refs.is_empty());
        assert!(!f.icon_calls.contains(&"unlock"));
    }
    let mut f = Recording::default();
    let mut output = Some(99);
    assert!(matches!(
        b.create_combo_arrow(&mut output, &mut f),
        Err(Unsupported::Precondition(_))
    ));
    assert!(matches!(
        b.create_button_buffer(4, &mut output, &mut f),
        Err(Unsupported::Precondition(_))
    ));
    assert_eq!(output, Some(99));
    assert!(f.icon_calls.is_empty() && f.refs.is_empty());
}

#[test]
fn fbq1_arrow_worked_case_uses_converted_colors_and_ordered_panels() {
    let b = FileBrowser::new(80);
    let mut f = Recording {
        buffer_ok: true,
        fb_locks: [true, true, false, true, true].into(),
        ..Recording::default()
    };
    let mut output = None;
    assert_eq!(b.create_combo_arrow(&mut output, &mut f), Ok(true));
    assert_eq!(output, Some(700));
    assert_eq!(f.fb_buffer_formats, [(700, 64, 15, [3, 16])]);
    assert_eq!(f.fb_keys, [0xa5ff_00ff]);
    let mut expected = vec![Draw::Fill {
        rect: [0, 0, 64, 15],
        color: 0xa5ff_00ff,
    }];
    // §8.3.2's worked case omits origin 20, preserving the other panels.
    for points in [
        [4, 5, 11, 5],
        [5, 6, 10, 6],
        [6, 7, 9, 7],
        [7, 8, 8, 8],
        [36, 5, 43, 5],
        [37, 6, 42, 6],
        [38, 7, 41, 7],
        [39, 8, 40, 8],
        [52, 5, 59, 5],
        [53, 6, 58, 6],
        [54, 7, 57, 7],
        [55, 8, 56, 8],
    ] {
        expected.push(Draw::Line {
            points,
            color: 0xa500_0000,
            style: 1,
        });
    }
    assert_eq!(f.fb_resource_draws, expected);
    assert_eq!(
        f.icon_calls,
        [
            "factory", "graphics", "init", "rgb", "rgb", "lock", "fill", "unlock", "key", "lock", "line", "line",
            "line", "line", "unlock", "lock", "lock", "line", "line", "line", "line", "unlock", "lock", "line", "line",
            "line", "line", "unlock",
        ]
    );
    assert!(f.refs.is_empty());
}

#[test]
fn fbq1_loader_without_strip_keeps_settled_result_and_ownership() {
    for selector in [2, 7, u32::MAX] {
        let mut b = FileBrowser::new(80);
        let mut f = Recording {
            buffer_ok: true,
            fb_no_strip: true,
            ..Recording::default()
        };
        assert_eq!(b.load_default_icon(selector, &mut f), Ok(true));
        assert_eq!(f.fb_buffer_formats, [(700, 18, 17, [3, 16])]);
        assert_eq!(f.fb_keys, [0xa5ff_00ff]);
        assert_eq!(
            f.fb_lock_flags,
            if selector == 2 {
                vec![0x8001, 0x8004]
            } else {
                vec![0x8001]
            }
        );
        assert!(f.fb_copies.is_empty() && f.fb_resource_draws.is_empty());
        assert!(!f.icon_calls.contains(&"unlock"));
        if selector == 2 {
            assert_eq!(b.icons[2], Some(700));
            assert_eq!(f.refs, [(true, 700), (false, 700)]);
        } else {
            assert_eq!(b.icons, [None; 7]);
            assert_eq!(f.refs, [(false, 700)]);
        }
    }
}
