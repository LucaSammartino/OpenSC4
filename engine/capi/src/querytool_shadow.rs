// SPDX-License-Identifier: GPL-3.0-or-later
//! Private static-link projection of the production query pick predicates.
use fc4_core::query_tool::{accepts, Filter, Kind, Occupant, Property};

// Input flags encode HasProperty, including present false-valued properties.
// No native predicate result is supplied as a core input.
#[no_mangle]
pub extern "C" fn fc4_querytool_shadow_pick(kind: u32, properties: u32) -> u32 {
    let mut occupant = Occupant {
        kind: match kind {
            0x2781_28A0 => Kind::Building,
            0xC772_BF98 => Kind::Network,
            0x7475_8926 => Kind::Flora,
            0xA823_821E => Kind::Prop,
            0x2890_D4DE => Kind::PowerPole,
            0x6A11_BD2B => Kind::Landfill,
            _ => Kind::Other,
        },
        ..Occupant::default()
    };
    for (flag, id) in [(1, 0x8A41_6A99), (2, 0xCAA9_AB92)] {
        if properties & flag != 0 {
            occupant.properties.insert(id, Property::Bool(false));
        }
    }
    u32::from(accepts(Filter::Hover, &occupant)) | (u32::from(accepts(Filter::Describe, &occupant)) << 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copied_presence_and_raw_type_drive_both_pick_modes() {
        for kind in [0x2781_28A0, 0xC772_BF98, 0x2890_D4DE, 0x6A11_BD2B] {
            assert_eq!(fc4_querytool_shadow_pick(kind, 0), 3);
            assert_eq!(fc4_querytool_shadow_pick(kind, 2), 1);
        }
        for kind in [0x7475_8926, 0xA823_821E] {
            assert_eq!(fc4_querytool_shadow_pick(kind, 0), 0);
            assert_eq!(fc4_querytool_shadow_pick(kind, 1), 3);
            assert_eq!(fc4_querytool_shadow_pick(kind, 3), 1);
        }
        assert_eq!(fc4_querytool_shadow_pick(0, 3), 0);
    }
}
