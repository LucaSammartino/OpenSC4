// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
#[test]
fn malformed_frames_and_wire_are_rejected() {
    assert!(read_event(&mut &b"\xff\xff\xff\xff"[..]).is_err());
    assert!(read_event(&mut &b"\x03\0\0\0a"[..]).is_err());
    assert!(Vec::<u8>::get(&mut Reader(&u64::MAX.to_le_bytes())).is_err());
    assert!(bool::get(&mut Reader(&[2])).is_err());
}
#[test]
fn callback_misses_never_call_live_host() {
    // A non-callback frame is a hard replay miss, even if a live fallback exists.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../build-runs/trace-tests");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("miss.trace");
    let mut f = File::create(&path).unwrap();
    f.write_all(&[0, 0, 0, 0]).unwrap();
    drop(f);
    SESSIONS.with(|s| {
        s.borrow_mut().insert(
            "budget",
            Session {
                mode: Mode::Replay {
                    reader: BufReader::new(File::open(path).unwrap()),
                },
                misses: Vec::new(),
                recorded_mismatches: 0,
                comparisons: 0,
                incomplete: 0,
            },
        );
    });
    let answer = exchange("budget", "unknown", vec![], || panic!("live host must never run"));
    assert!(answer.is_empty());
    assert!(!SESSIONS
        .with(|s| s.borrow_mut().remove("budget").unwrap())
        .misses
        .is_empty());
}

#[test]
fn budget_trace_roundtrip_and_content_address_validation() {
    use crate::budget::*;
    unsafe extern "C" fn lots(_: *mut c_void, cb: fc4_lot_visitor, ctx: *mut c_void) {
        let lot = fc4_lot_info {
            has_building: 1,
            livable: 1,
            purpose: 1,
            building_wealth: 1,
            lot_wealth: 1,
            building_value: 10000,
            land_value: 100,
            size_x: 2,
            size_z: 2,
            ..Default::default()
        };
        cb.unwrap()(ctx, &lot);
    }
    let dir = test_record("budget-roundtrip", || unsafe {
        let mut h: fc4_budget_host = std::mem::zeroed();
        h.struct_size = std::mem::size_of::<fc4_budget_host>() as u32;
        h.for_each_lot = Some(lots);
        let mut scan = fc4_tax_scan {
            struct_size: std::mem::size_of::<fc4_tax_scan>() as u32,
            ..Default::default()
        };
        assert!(fc4_budget_shadow_tax(
            &h,
            0,
            false,
            0,
            [9f32; 12].as_ptr(),
            None,
            std::ptr::null_mut(),
            &mut scan
        ));
        assert!(scan.total_tax > 0);
        fc4_trace_budget_expected(
            2000,
            2,
            1,
            scan.weighted_assessed.as_ptr(),
            scan.tax_income.as_ptr(),
            scan.total_tax,
            0,
            false,
            false,
            0,
        );
    });
    let manifest = dir.join("manifest.tsv");
    assert!(run(&manifest, true, 2).unwrap());
    assert!(run(&manifest, false, 2).unwrap());
    let text = fs::read_to_string(&manifest).unwrap();
    let sha = text.split('\t').nth(2).unwrap();
    let path = dir.join(sha);
    OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"broken")
        .unwrap();
    assert!(!run(&manifest, false, 2).unwrap());
    assert!(fs::read_to_string(dir.join("replay-verdicts.tsv"))
        .unwrap()
        .contains("trace content digest mismatch"));
}
