// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The weather C ABI against a fake 16 × 16 city: tuning through fc4_props,
//! the post-city-init pass, accessors, the moisture grid copy, persistence.

use std::cell::RefCell;
use std::ffi::c_void;

use crate::props::*;
use crate::test_streams::{istream, ostream, In, Out};
use crate::weather::*;

#[derive(Default)]
struct Game {
    posts: RefCell<Vec<(u32, u32)>>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}
unsafe extern "C" fn city_size(_: *mut c_void, x: *mut f32, z: *mut f32) {
    *x = 256.0;
    *z = 256.0;
}
unsafe extern "C" fn cell_width(_: *mut c_void, x: *mut f32, z: *mut f32) {
    *x = 16.0;
    *z = 16.0;
}
unsafe extern "C" fn hemisphere(_: *mut c_void) -> i32 {
    1
}
unsafe extern "C" fn month(_: *mut c_void) -> i32 {
    8
}
unsafe extern "C" fn yes(_: *mut c_void) -> bool {
    true
}
unsafe extern "C" fn alt_vertex(_: *mut c_void, _x: i32, _z: i32) -> f32 {
    400.0
}
unsafe extern "C" fn water(_: *mut c_void, x: i32, _z: i32) -> bool {
    x == 0 // the west edge is water
}
unsafe extern "C" fn sea(_: *mut c_void) -> f32 {
    250.0
}
unsafe extern "C" fn post(u: *mut c_void, m: u32, d2: u32) {
    game(u).posts.borrow_mut().push((m, d2));
}

fn host(g: &Game) -> fc4_weather_host {
    fc4_weather_host {
        struct_size: std::mem::size_of::<fc4_weather_host>() as u32,
        user: g as *const Game as *mut c_void,
        city_size: Some(city_size),
        city_cell_width: Some(cell_width),
        world_hemisphere: Some(hemisphere),
        sim_month: Some(month),
        water_prefs: None,
        terrain_available: Some(yes),
        altitude_at_vertex: Some(alt_vertex),
        vertex_normal: None,
        cell_has_water: Some(water),
        altitude: None,
        sea_level: Some(sea),
        post: Some(post),
    }
}

/// The shipped tuning (weather §9), as the shim would copy it.
unsafe fn shipped() -> *mut fc4_props {
    let p = fc4_props_create();
    let i = |id: u32, v: i32| assert!(fc4_props_set_i32s(p, id, &v, 1));
    let f = |id: u32, v: f32| assert!(fc4_props_set_f32s(p, id, &v, 1));
    i(0xA760_7D70, 1);
    i(0xA760_7D71, 1);
    i(0xA760_7D72, 1);
    f(0xA760_7D73, 65.0);
    f(0xA760_7D74, 0.5);
    f(0xA760_7D75, 0.5);
    i(0xA760_7D76, 10);
    f(0xA760_7D77, 0.9);
    f(0xA760_7D78, 0.082);
    f(0xA760_7D79, 0.25);
    f(0xA760_7D7A, 0.2);
    f(0xA760_7D7B, 0.2);
    f(0xA760_7D7C, 1.0);
    f(0xA760_7D7D, 0.35);
    f(0xA760_7D7E, 1.0);
    f(0xA760_7D7F, 2.5);
    assert!(fc4_props_set_u32s(p, 0xA760_7D80, &40, 1));
    f(0xA760_7D89, 60.0);
    p
}

fn state(w: *const fc4_weather) -> fc4_weather_state {
    let mut s = fc4_weather_state::default();
    assert!(unsafe { fc4_weather_get_state(w, &mut s) });
    s
}

#[test]
fn null_safety() {
    unsafe {
        assert!(fc4_weather_create(std::ptr::null(), 0).is_null());
        let w = std::ptr::null_mut();
        assert!(!fc4_weather_init(w, std::ptr::null()));
        assert!(fc4_weather_handle_message(w, 0x6695_6815, 0, 0));
        assert_eq!(fc4_weather_cell_count(w, 0), 0);
        assert_eq!(fc4_weather_moisture_grid(w, std::ptr::null_mut(), 0, std::ptr::null_mut(), std::ptr::null_mut()), 0);
        assert!(!fc4_weather_get_state(w, std::ptr::null_mut()));
        fc4_weather_destroy(w);
        assert!(!fc4_props_set_u32s(std::ptr::null_mut(), 1, &1, 1));
        assert_eq!(fc4_props_count(std::ptr::null()), 0);
        fc4_props_destroy(std::ptr::null_mut());
    }
}

#[test]
fn init_pass_accessors_and_save() {
    let g = Game::default();
    let h = host(&g);
    unsafe {
        let w = fc4_weather_create(&h, 0x1234_5678);
        assert!(!w.is_null());
        let t = shipped();
        assert_eq!(fc4_props_count(t), 18);
        assert!(fc4_weather_init(w, t));
        fc4_props_destroy(t); // copied
        let s = state(w);
        assert_eq!((s.initialized, s.count_x, s.count_z, s.cells_per_week), (1, 16, 16, 256));
        assert_eq!(s.ambient_temp, 65.0);
        assert_eq!(fc4_weather_cell_width(w, 0), 16.0);
        assert_eq!(fc4_weather_cell_count(w, 1), 16);
        assert_eq!(fc4_weather_season_month(w), 8);
        assert_eq!(fc4_weather_season(w), 2);
        assert_eq!(fc4_weather_humidity(w, 1.0, 2.0), 0.6);

        // PostCityInit: the complete simulation (month 1 water scan, ambient
        // values of season month 8, SeasonChanged, two weekly passes).
        assert!(fc4_weather_handle_message(w, 0x26D3_1EC1, 0, 0));
        let s = state(w);
        assert_eq!(s.post_init_done, 1);
        assert_eq!(s.last_season, 2);
        assert_eq!(s.ambient_temp, 81.25);
        let posts = g.posts.borrow().clone();
        assert!(posts.contains(&(0x6716_A4E5, 2)), "{posts:?}");
        assert!(posts.iter().any(|p| p.0 == 0x09B6_D58E));
        assert!(posts.iter().any(|p| p.0 == 0x6716_A4E6));
        // The onshore wind blows from the west edge (the water) into the city.
        let mut v = [0.0f32; 2];
        let speed = fc4_weather_ambient_wind(w, v.as_mut_ptr());
        assert_eq!(speed, 3.0); // reset to the default at the season change
        assert!(v[0].abs() > 0.0 || v[1].abs() > 0.0);

        // The moisture grid copy, x-major; water cells are wet.
        let (mut nx, mut nz) = (0, 0);
        assert_eq!(fc4_weather_moisture_grid(w, std::ptr::null_mut(), 0, &mut nx, &mut nz), 256);
        assert_eq!((nx, nz), (16, 16));
        let mut grid = vec![0u8; 256];
        assert_eq!(fc4_weather_moisture_grid(w, grid.as_mut_ptr(), grid.len(), &mut nx, &mut nz), 256);
        assert_eq!(fc4_weather_moisture_value(w, 5.0 * 16.0 + 1.0, 3.0 * 16.0), grid[5 * 16 + 3]);
        assert_eq!(fc4_weather_moisture_type(w, 80.0, 48.0), grid[5 * 16 + 3] >> 4);
        assert!(grid[0] > grid[8 * 16 + 8], "water edge {} vs inland {}", grid[0], grid[8 * 16 + 8]);

        // Accessors (§4).
        let (mut a, mut b) = (0, 0);
        fc4_weather_cell_to_standard_city_cell(w, 3, 7, &mut a, &mut b);
        assert_eq!((a, b), (3, 3)); // [quirk]
        assert_eq!(fc4_weather_position_to_cell(w, 40.0, 70.0, &mut a, &mut b), 0);
        assert_eq!((a, b), (2, 4));
        let (mut x, mut z) = (0.0, 0.0);
        fc4_weather_cell_to_position(w, 2, 3, &mut x, &mut z);
        assert_eq!((x, z), (32.0, 48.0));

        // Save, load into a fresh object, save again: byte-identical.
        let mut o = Out::default();
        assert!(fc4_weather_write(w, &ostream(&mut o)));
        let w2 = fc4_weather_create(&h, 1);
        let mut i = In::new(&o.bytes);
        assert!(fc4_weather_read(w2, &istream(&mut i)));
        assert_eq!(i.pos, o.bytes.len());
        assert_eq!(state(w2).loaded_from_save, 1);
        let mut o2 = Out::default();
        assert!(fc4_weather_write(w2, &ostream(&mut o2)));
        assert_eq!(o.bytes, o2.bytes);
        // A truncated record fails.
        let mut i = In::new(&o.bytes[..40]);
        assert!(!fc4_weather_read(w2, &istream(&mut i)));

        assert!(fc4_weather_shutdown(w));
        assert_eq!(state(w).initialized, 0);
        fc4_weather_destroy(w);
        fc4_weather_destroy(w2);
    }
}
