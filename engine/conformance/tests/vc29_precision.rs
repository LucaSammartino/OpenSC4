// SPDX-License-Identifier: GPL-3.0-or-later
//! Foundations §18.4: identical inputs across the Lua re-entry precision boundary.
use fc4_core::lot_developer::{area_slope_with_precision, CellRect};
use fc4_core::regional_city::RegionalCity;
use fc4_core::x87::Precision::{Bits24, Bits53};

#[test]
fn vc29_balance_boundary_and_slope_float_sink() {
    let mut city = RegionalCity::new(2);
    city.set_produced(3, 16_777_216.0);
    city.set_imported(3, 1.0);
    city.set_demanded(3, 16_777_216.0);
    for (pc, balance, slope_bits) in [
        (Bits24, 0.0, 0x40ED_B6DB),
        (Bits53, 1.0, 0x40ED_B6DC),
        (Bits53, 1.0, 0x40ED_B6DC),
        (Bits24, 0.0, 0x40ED_B6DB),
    ] {
        assert_eq!(city.get_commodity_balance_with_precision(3, pc), balance);
        let mut altitude = |x, z| match (x, z) {
            (0, 0) => 0.0,
            (3, 4) => 13.0,
            _ => 6.0,
        };
        assert_eq!(
            area_slope_with_precision(CellRect::new(0, 0, 2, 3), &mut altitude, pc).to_bits(),
            slope_bits
        );
    }
}
