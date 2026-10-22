// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn independent_road_and_avenue_vectors() {
    for (edges, orientation, offset) in [
        ([0,2,0,2], 0, (0.0,0.0)),
        ([1,3,0,0], 2, (-4.0,-4.0)),
        ([2,1,2,0], 1, (0.0,0.0)),
        ([0,0,0,2], 0, (0.0,0.0)),
    ] { assert_eq!(road_center_and_orientation(edges), (orientation, offset)); }
    for (edges, orientation, offset) in [
        ([1,1,1,1], 2, (0.0,0.0)),
        ([3,0,0,0], 3, (-8.0,8.0)),
        ([2,1,1,2], 2, (-8.0,8.0)),
        ([4,2,0,0], 0, (-8.0,0.0)),
    ] { assert_eq!(avenue_center_and_orientation(edges), (orientation, offset)); }
}
