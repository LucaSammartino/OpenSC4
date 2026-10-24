// SPDX-License-Identifier: GPL-3.0-or-later
//! Stateless replay of published owner-only methods, not a build replay.
//! Handles are copied identities; no native references are taken.
use super::{Handle, Interface, LegacyControl};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnerState {
    pub refs: u32,
    pub control: Option<Handle>,
    pub parent: Option<Handle>,
    pub procedure: Option<Handle>,
    pub own: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Operation {
    AddRef,
    SetOwnControl(bool),
    GetControl,
    QueryInterface(u32),
}

/// QI's scalar is 0 (rejected), 1 (primary), 5 (area), or 9 (legacy).
/// The adapter translates nonzero tags to this-relative native pointers.
pub fn replay(state: OwnerState, operation: Operation) -> (OwnerState, u64) {
    let mut owner = LegacyControl {
        refs: state.refs,
        control: state.control,
        parent: state.parent,
        procedure: state.procedure,
        own: state.own,
        dead: false,
    };
    let scalar = match operation {
        Operation::AddRef => u64::from(owner.add_ref()),
        Operation::SetOwnControl(own) => {
            owner.set_own_control(own);
            0
        }
        Operation::GetControl => owner.control().unwrap_or(0),
        Operation::QueryInterface(iid) => {
            let mut out = None;
            if owner.query_interface(iid, &mut out) {
                match out {
                    Some(Interface::Primary) => 1,
                    Some(Interface::Area) => 5,
                    Some(Interface::Legacy) => 9,
                    None => unreachable!(),
                }
            } else {
                0
            }
        }
    };
    (
        OwnerState {
            refs: owner.ref_count(),
            control: owner.control(),
            parent: owner.pending_parent(),
            procedure: owner.procedure(),
            own: owner.owns_control(),
        },
        scalar,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> OwnerState {
        OwnerState {
            refs: 3,
            control: Some(0x1234),
            parent: Some(0x5678),
            procedure: Some(0x9abc),
            own: false,
        }
    }

    #[test]
    fn independent_fields_survive_owner_flag_change() {
        let before = state();
        let (after, _) = replay(before, Operation::SetOwnControl(true));
        assert_eq!(after, OwnerState { own: true, ..before });
        assert_eq!(replay(after, Operation::GetControl), (after, 0x1234));
    }

    #[test]
    fn query_reject_preserves_state_and_success_adjusts_count() {
        let before = state();
        assert_eq!(replay(before, Operation::QueryInterface(0)), (before, 0));
        for (iid, tag) in [(1, 1), (0x1a356083, 1), (0x5a356159, 5), (0x9a4527cb, 9)] {
            assert_eq!(
                replay(before, Operation::QueryInterface(iid)),
                (OwnerState { refs: 4, ..before }, tag)
            );
        }
        assert_eq!(
            replay(
                OwnerState {
                    refs: u32::MAX,
                    ..before
                },
                Operation::AddRef
            )
            .0
            .refs,
            0
        );
    }
}
