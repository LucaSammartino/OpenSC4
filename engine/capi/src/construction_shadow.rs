// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link transport for LR14/T79 live construction getters.
use fc4_core::lot_records::construction::ConstructionOccupant;

// The six inputs come only from the native bounding-box getter, never position.
// No native references, record Read, Init or lifecycle callbacks are involved.
#[no_mangle]
pub unsafe extern "C" fn fc4_construction_shadow_position(bounds: *const f32, out: *mut f32) -> bool {
    if bounds.is_null() || out.is_null() {
        return false;
    }
    let b = std::slice::from_raw_parts(bounds, 6);
    let mut owner = ConstructionOccupant::default();
    owner.rec.world_bounds = ([b[0], b[1], b[2]], [b[3], b[4], b[5]]);
    std::ptr::copy_nonoverlapping(owner.position().as_ptr(), out, 3);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_bounds_are_the_only_inputs() {
        let bounds = [-8.0, 0.25, 100.0, 12.0, 1.75, 200.0];
        let mut position = [f32::NAN; 3];
        assert!(unsafe { fc4_construction_shadow_position(bounds.as_ptr(), position.as_mut_ptr()) });
        assert_eq!(position, [2.0, 1.0, 150.0]);
        assert!(!unsafe { fc4_construction_shadow_position(std::ptr::null(), position.as_mut_ptr()) });
        assert!(!unsafe { fc4_construction_shadow_position(bounds.as_ptr(), std::ptr::null_mut()) });
    }
}
