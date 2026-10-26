// SPDX-License-Identifier: GPL-3.0-or-later
//! Replay from entry state and captured foreign outcomes, never owner post-state.
//! Handles are copied identities; no native references are taken.
use super::{Foreign, Globals, Handle, Interface, LegacyControl, Properties, Result};

pub mod recording;

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
    ImplementInterface([u32; 4]),
    AreaStub,
}

impl OwnerState {
    pub fn owner(self) -> LegacyControl {
        LegacyControl {
            refs: self.refs,
            control: self.control,
            parent: self.parent,
            procedure: self.procedure,
            own: self.own,
            dead: false,
        }
    }

    pub fn capture(owner: &LegacyControl) -> Self {
        Self {
            refs: owner.ref_count(),
            control: owner.control(),
            parent: owner.pending_parent(),
            procedure: owner.procedure(),
            own: owner.owns_control(),
        }
    }
}

/// Process maps from the published registrations, without creating a vocabulary
/// or a second native widget. Foreign factory/QI results remain host inputs.
pub fn build_globals() -> Globals {
    let mut g = Globals::default();
    for t in super::vocabulary::TABLES {
        if t.kind != super::Kind::Common {
            g.classes.entry(t.class).or_insert(t.iid);
            g.interpreters.entry(t.iid).or_insert(t.kind);
        }
    }
    g
}

pub fn replay_build(
    state: OwnerState,
    p: Properties<'_>,
    f: &mut impl Foreign,
) -> Result<(OwnerState, bool)> {
    let mut owner = state.owner();
    let value = owner.set_parameters(p, &build_globals(), f)?;
    Ok((OwnerState::capture(&owner), value))
}

/// QI's scalar is 0 (rejected), 1 (primary), 5 (area), or 9 (legacy).
/// The adapter translates nonzero tags to this-relative native pointers.
pub fn replay(state: OwnerState, operation: Operation) -> (OwnerState, u64) {
    let mut owner = state.owner();
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
        Operation::ImplementInterface(args) => u64::from(owner.implement_interface(args)),
        Operation::AreaStub => u64::from(owner.area_stub(&[])),
    };
    (OwnerState::capture(&owner), scalar)
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
        assert_eq!(
            after,
            OwnerState {
                own: true,
                ..before
            }
        );
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
