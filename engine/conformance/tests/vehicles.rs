// SPDX-License-Identifier: GPL-3.0-or-later
//! vehicles.md §11 test 4; tests 1-3 belong to emergency/fire-truck owners.
use fc4_core::vehicles::{find_leader, find_leader_in_tile, LeaderCandidate};

#[test]
fn vh11_4_nearest_eligible_leader_ignores_height() {
    let candidates = vec![
        LeaderCandidate {
            id: 1,
            position: [1.0, 0.0, 0.0],
            removed: true,
            may_follow: true,
            straight: true,
            lane_bits: 0,
            ahead: true,
        },
        LeaderCandidate {
            id: 2,
            position: [1.0, 0.0, 0.0],
            removed: false,
            may_follow: false,
            straight: true,
            lane_bits: 0,
            ahead: true,
        },
        LeaderCandidate {
            id: 3,
            position: [1.0, 0.0, 0.0],
            removed: false,
            may_follow: true,
            straight: false,
            lane_bits: 8,
            ahead: true,
        },
        LeaderCandidate {
            id: 4,
            position: [2.0, 10000.0, 0.0],
            removed: false,
            may_follow: true,
            straight: false,
            lane_bits: 4,
            ahead: true,
        },
        LeaderCandidate {
            id: 5,
            position: [3.0, 0.0, 0.0],
            removed: false,
            may_follow: true,
            straight: true,
            lane_bits: 8,
            ahead: true,
        },
    ];
    assert_eq!(find_leader_in_tile([0.0; 3], 4, &candidates), Some(4));
    // Route order wins over global proximity when choosing between tiles.
    let farther = LeaderCandidate {
        id: 6,
        position: [20.0, 0.0, 0.0],
        ..candidates[4].clone()
    };
    assert_eq!(
        find_leader([0.0; 3], 4, &[vec![farther], candidates]),
        Some(6)
    );
}
