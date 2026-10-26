// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::regionalcity_owner::{
    fc4_regionalcity_owner_create, fc4_regionalcity_owner_destroy, fc4_regionalcity_owner_get,
};

fn cache() -> Cache {
    Cache {
        difficulty: 2,
        serial: 42,
        established: true,
        mayor_rating: -17,
        tax: [9.5; 12],
        tutorial: 123,
        residential: true,
        population: 500,
        workforce: 0.48,
        populations: std::ptr::null(),
        population_count: 0,
        points: std::ptr::null(),
        point_count: 0,
    }
}

#[test]
fn cache_replaces_maps_computes_jobs_and_preserves_record_only_fields() {
    unsafe {
        let owner = fc4_regionalcity_owner_create(0);
        {
            let mut c = (*owner).0.lock().unwrap();
            c.populations.insert(99, 999);
            c.potential.insert(99, 1500);
            c.extrapolated.insert(99, 1000);
            c.world = [1.0, 2.0, 3.0];
            c.birth_date = 66;
            c.budget = 123.25;
            c.income = 44.5;
        }
        let population = [
            Population {
                kind: 0x3111,
                count: 13,
                jobs: [999.0, 0.5, 0.25, 0.25],
            },
            Population {
                kind: 0x4101,
                count: 5,
                jobs: [999.0, 0.5, 0.5, 0.5],
            },
        ];
        let points = [
            Point {
                kind: 0,
                position: [10.0, 20.0, 30.0],
                livable: true,
            },
            Point {
                kind: 0,
                position: [99.0; 3],
                livable: false,
            },
            Point {
                kind: 1,
                position: [40.0; 3],
                livable: true,
            },
        ];
        let mut input = cache();
        input.populations = population.as_ptr();
        input.population_count = 2;
        input.points = points.as_ptr();
        input.point_count = 3;
        assert!(fc4_regionalcity_owner_cache(owner, &input));
        {
            let c = (*owner).0.lock().unwrap();
            assert_eq!((c.population, c.commercial_jobs, c.industrial_jobs), (500, 13, 7));
            assert_eq!(c.workforce, 0.48);
            assert!(!c.populations.contains_key(&99));
            assert_eq!(c.potential[&99], 1500);
            assert_eq!(c.extrapolated[&99], 1000);
            assert_eq!(c.world, [1.0, 2.0, 3.0]);
            assert_eq!(c.birth_date, 66);
            assert_eq!((c.budget, c.income), (123.25, 44.5));
            assert_eq!(c.points[&0], [[10.0, 20.0, 30.0]]);
            assert_eq!(c.points[&1], [[40.0; 3]]);
            assert_eq!(c.tax_rates, [9.5; 12]);
            assert_eq!((c.serial, c.difficulty, c.tutorial_id), (42, 2, 123));
        }
        assert!(fc4_regionalcity_owner_event(owner, 1));
        {
            let c = (*owner).0.lock().unwrap();
            assert_eq!(c.potential[&0x3111], 1000);
            assert_eq!(c.extrapolated[&0x3111], 13);
        }
        // A missing residential service leaves its two cached values alone.
        input.residential = false;
        input.population = 0;
        input.tutorial = 555;
        assert!(fc4_regionalcity_owner_cache(owner, &input));
        let c = (*owner).0.lock().unwrap();
        assert_eq!((c.population, c.workforce, c.tutorial_id), (500, 0.48, 123));
        drop(c);
        fc4_regionalcity_owner_destroy(owner);
    }
}

#[test]
fn preferences_rebuild_path_and_invalid_cache_keeps_old_state() {
    unsafe {
        let owner = fc4_regionalcity_owner_create(0);
        (*owner).0.lock().unwrap().file_path = b"C:/Regions/Berlin/old.sc4".to_vec();
        assert!(fc4_regionalcity_owner_preferences(
            owner,
            b"New\0ignored".as_ptr(),
            11,
            b"Mayor".as_ptr(),
            5,
            1
        ));
        let c = (*owner).0.lock().unwrap();
        assert_eq!(c.city_name, b"New");
        assert_eq!(c.mayor_name, b"Mayor");
        assert_eq!(c.file_path, b"C:/Regions/Berlin/New.sc4");
        drop(c);
        let mut input = cache();
        input.population_count = 1;
        assert!(!fc4_regionalcity_owner_cache(owner, &input));
        assert_eq!((*owner).0.lock().unwrap().difficulty, 1);
        assert!(!fc4_regionalcity_owner_preferences(
            owner,
            std::ptr::null(),
            1,
            std::ptr::null(),
            0,
            2
        ));
        fc4_regionalcity_owner_destroy(owner);
    }
}

struct Output {
    owner: *const Owner,
    points: Vec<[f32; 3]>,
    calls: u32,
}
unsafe extern "C" fn emit(context: *mut c_void, points: *const [f32; 3], count: u32) {
    let output = &mut *context.cast::<Output>();
    let mut value = 0.0;
    // Getter re-entry proves emission happens after unlocking the owner.
    assert!(fc4_regionalcity_owner_get(output.owner, 4, 0, &mut value));
    output.points = std::slice::from_raw_parts(points, count as usize).to_vec();
    output.calls += 1;
}
#[test]
fn poi_unknown_preserves_output_and_known_empty_replaces_it_without_lock_reentry() {
    unsafe {
        let owner = fc4_regionalcity_owner_create(0);
        let mut output = Output {
            owner,
            points: vec![[99.0; 3]],
            calls: 0,
        };
        let context = (&mut output as *mut Output).cast();
        assert!(!fc4_regionalcity_owner_points(owner, 77, context, emit));
        assert_eq!(output.points, [[99.0; 3]]);
        assert_eq!(output.calls, 0);
        (*owner).0.lock().unwrap().points.insert(0, Vec::new());
        assert!(fc4_regionalcity_owner_points(owner, 0, context, emit));
        assert!(output.points.is_empty());
        assert_eq!(output.calls, 1);
        fc4_regionalcity_owner_destroy(owner);
    }
}
