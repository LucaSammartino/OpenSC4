// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The traffic shadow's C ABI against the fake game: our own simulator
//! stands in for the original (its record captured in the plugin's capture
//! format); the shadow compares clean, touches nothing of the game's, and
//! reports a changed record.

use std::ffi::c_void;

use super::*;
use crate::pollution_shadow::{fc4_pollution_capture, fc4_pollution_capture_ref};
use crate::traffic_shadow::*;

/// The plugin's capture stream: `out` first, so the primitive writers of
/// [`ostream`] (which see an [`Out`]) work on it.
#[repr(C)]
struct Cap<'a> {
    out: Out,
    refs: Vec<fc4_pollution_capture_ref>,
    ids: Vec<u64>,
    game: &'a Game,
}

unsafe extern "C" fn cap_ref(u: *mut c_void, r: *const fc4_ref) -> bool {
    let c = &mut *(u as *mut Cap);
    let r = &*r;
    if r.token == 0 {
        c.out.bytes.extend_from_slice(&0u32.to_le_bytes());
        return true;
    }
    let i = match c.ids.iter().position(|&t| t == r.token) {
        Some(i) => i,
        None => {
            let grid = c.game.grids.borrow().contains_key(&r.token);
            c.ids.push(r.token);
            c.refs.push(fc4_pollution_capture_ref {
                grid: if grid { r.token } else { 0 },
                occupant: if grid { 0 } else { r.token },
                department: if grid { 0 } else { r.token },
                class_id: r.class_id,
                reserved: 0,
            });
            c.ids.len() - 1
        }
    };
    c.out.bytes.extend_from_slice(&(i as u32 + 1).to_le_bytes());
    c.out.bytes.extend_from_slice(&r.class_id.to_le_bytes());
    true
}

fn capture<'a>(p: *const fc4_traffic, g: &'a Game) -> Box<Cap<'a>> {
    let mut c = Box::new(Cap {
        out: Out::default(),
        refs: Vec::new(),
        ids: Vec::new(),
        game: g,
    });
    let mut os = ostream(&mut c.out);
    os.user = &mut *c as *mut Cap as *mut c_void;
    os.put_ref = Some(cap_ref);
    assert!(unsafe { fc4_traffic_write(p, &os) });
    c
}

fn view(c: &Cap) -> fc4_pollution_capture {
    fc4_pollution_capture {
        struct_size: std::mem::size_of::<fc4_pollution_capture>() as u32,
        ok: 1,
        bytes: c.out.bytes.as_ptr(),
        len: c.out.bytes.len(),
        refs: c.refs.as_ptr(),
        ref_count: c.refs.len(),
    }
}

unsafe extern "C" fn lines(ctx: *mut c_void, data: *const u8, len: usize) {
    let v = &mut *(ctx as *mut Vec<String>);
    v.push(String::from_utf8_lossy(std::slice::from_raw_parts(data, len)).into_owned());
}

/// One delivery to the stand-in original, wrapped.
unsafe fn deliver(
    s: *mut fc4_traffic_shadow,
    p: *mut fc4_traffic,
    g: &Game,
    msg_type: u32,
    out: &mut Vec<String>,
) -> i32 {
    let b = capture(p, g);
    assert_eq!(fc4_traffic_shadow_before(s, msg_type, 0, 0, 0, 2_451_545, &view(&b)), 0);
    fc4_traffic_do_message(p, msg_type, 0, 0, 0, 0);
    let a = capture(p, g);
    fc4_traffic_shadow_after(s, &view(&a), Some(lines), out as *mut Vec<String> as *mut c_void)
}

#[test]
fn shadow_follows_the_stand_in_original() {
    let g = Game::new(16, 16);
    let p = new_city(&g);
    unsafe {
        assert!(fc4_traffic_init(p, 0));
        assert!(fc4_traffic_do_message(p, ts::msg::POST_CITY_INIT, 0, 0, 0, 0));
        let h = host(&g);
        let s = fc4_traffic_shadow_create(&h, 7, 0, 0);
        assert!(!s.is_null());
        let (agents, subs, posts, depts, grids) = (
            g.agents.borrow().len(),
            g.subs.borrow().len(),
            g.posts.borrow().len(),
            g.depts.borrow().len(),
            g.grids.borrow().len(),
        );
        let refs = g.live_refs();
        let mut out = Vec::new();
        assert_eq!(deliver(s, p, &g, ts::msg::SIM_BEGIN, &mut out), 0, "{out:?}");
        let mut posts_orig = g.posts.borrow().len();
        for d in 0..60u32 {
            g.day.set(d % 30 + 1);
            g.date.set(1000 + d);
            let n = deliver(s, p, &g, ts::msg::DAY, &mut out);
            assert_eq!(n, 0, "day {d}: {out:?}");
            posts_orig = g.posts.borrow().len().max(posts_orig);
        }
        assert_eq!(deliver(s, p, &g, ts::msg::MONTH, &mut out), 0, "{out:?}");
        let month = out.last().expect("the month line");
        assert!(
            month.starts_with("traffic shadow month to 2000-01-01: clean;"),
            "{month}"
        );
        // The shadow added nothing to the game: no agent, subscription,
        // department, grid or reference of its own, no post.
        assert_eq!(g.agents.borrow().len(), agents);
        assert_eq!(g.subs.borrow().len(), subs);
        assert_eq!(g.depts.borrow().len(), depts);
        assert_eq!(g.grids.borrow().len(), grids);
        assert_eq!(g.live_refs(), refs);
        assert!(g.posts.borrow().len() >= posts);
        let mut st = fc4_traffic_shadow_stats::default();
        assert!(fc4_traffic_shadow_get_stats(s, &mut st));
        assert_eq!((st.deliveries, st.compared, st.clean, st.incomplete), (62, 62, 62, 0));
        assert!(st.grid_cells > 0);
        assert_eq!(st.last_mismatches, 0);

        // A grid value the copy did not produce: one difference, named.
        let b = capture(p, &g);
        fc4_traffic_shadow_before(s, ts::msg::DAY, 0, 0, 0, 2_451_546, &view(&b));
        fc4_traffic_do_message(p, ts::msg::DAY, 0, 0, 0, 0);
        let a = capture(p, &g);
        let grid = a.refs.iter().find(|r| r.grid != 0).expect("a grid").grid;
        g_set(g.user(), grid, 0, 0, 0, g_get(g.user(), grid, 0, 0, 0) + 1.0);
        out.clear();
        let n = fc4_traffic_shadow_after(s, &view(&a), Some(lines), &mut out as *mut Vec<String> as *mut c_void);
        assert_eq!(n, 1, "{out:?}");
        assert!(out[0].contains("mismatches=1; grid "), "{out:?}");
        assert!(out[0].contains("1 of "), "{out:?}");

        // No capture: INCOMPLETE, nothing compared.
        fc4_traffic_shadow_before(s, ts::msg::DAY, 0, 0, 0, 2_451_547, std::ptr::null());
        out.clear();
        let n = fc4_traffic_shadow_after(
            s,
            std::ptr::null(),
            Some(lines),
            &mut out as *mut Vec<String> as *mut c_void,
        );
        assert_eq!(n, -1);
        assert!(out.iter().any(|l| l.contains("INCOMPLETE")), "{out:?}");

        // A message reaches the copy once it has started.
        assert!(fc4_traffic_shadow_message(s, ts::msg::BUDGET_CHANGED, 0, 0, 0, 0));
        fc4_traffic_shadow_destroy(s);
        assert!(fc4_traffic_shutdown(p));
        fc4_traffic_destroy(p);
    }
    assert_eq!(g.live_refs(), vec![]);
    // Null handles.
    unsafe {
        assert!(fc4_traffic_shadow_create(std::ptr::null(), 0, 0, 0).is_null());
        assert_eq!(
            fc4_traffic_shadow_before(std::ptr::null_mut(), 0, 0, 0, 0, 0, std::ptr::null()),
            -1
        );
        assert!(!fc4_traffic_shadow_message(std::ptr::null_mut(), 0, 0, 0, 0, 0));
        fc4_traffic_shadow_destroy(std::ptr::null_mut());
    }
}
