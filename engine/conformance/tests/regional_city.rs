// SPDX-License-Identifier: GPL-3.0-or-later
//! Published regional-city statistics and foundations §14 vectors 20–22.
use fc4_core::neighbors::{deal_type, DealData, NeighborConnection, NeighborDeal};
use fc4_core::regional_city::{DealRegion, Lifecycle, ReadContext, RegionalCity};
use fc4_core::stream::{ByteReader, ByteWriter};

struct Host;
impl Lifecycle for Host {
    fn subscribe(&mut self, _: u32, _: bool) {}
}
impl DealRegion for Host {
    fn current_city(&self) -> Option<u64> {
        Some(1)
    }
    fn city_at(&self, p: (i32, i32)) -> Option<u64> {
        match p {
            (0, 0) => Some(1),
            (1, 0) => Some(2),
            (2, 0) => Some(3),
            _ => None,
        }
    }
}
#[test]
fn regional_city_statistics() {
    let mut c = RegionalCity::new(2);
    c.potential.insert(1, 1500);
    c.extrapolated.insert(1, 1800);
    assert_eq!(c.get_allowable_extrapolation(1), 0);
    c.extrapolated.insert(1, 1200);
    assert_eq!(c.get_allowable_extrapolation(1), 300);
    c.populations = [(1, 500), (2, 2000), (3, 0)].into();
    c.reset_growth();
    assert_eq!((c.potential[&1], c.potential[&2], c.potential[&3]), (1000, 2200, 0));
    c.set_produced(1, 10.0);
    c.set_imported(1, 5.0);
    c.set_demanded(1, 8.0);
    c.set_exported(1, 2.0);
    assert_eq!(c.get_commodity_balance(1), 5.0);
    c.set_exported(1, -3.0);
    assert_eq!(c.get_exported(1), 0.0);
    c.tax_rates[6] = 9.0;
    assert_eq!(c.get_tax_rate(0x3320), 9.0);
    assert_eq!(c.get_tax_rate(0x9999), 7.0);
}
#[test]
fn foundations_record_and_deal_lifecycle() {
    let mut c = RegionalCity::new(2);
    c.established = true;
    c.connections = vec![NeighborConnection::with(2, -1, 5), NeighborConnection::with(9, 64, 10)];
    c.commodities.remove(&3);
    let d = NeighborDeal::from_data(DealData {
        city_side: (0, 0),
        other_side: (1, 0),
        deal_type: deal_type::BUY_POWER,
        amount: 100,
        flags: 0,
        ..DealData::default()
    });
    c.other_deals.push(d.clone());
    c.local_deals.push(NeighborDeal::new());
    c.points = [(0, vec![[1.0, 2.0, 3.0]]), (1, vec![[4.0, 5.0, 6.0]])].into();
    let mut out = ByteWriter::new();
    assert!(c.write_record(&mut out));
    let raw = RegionalCity::decode_record(&mut ByteReader::new(out.bytes()), 2).unwrap();
    let mut again = ByteWriter::new();
    assert!(raw.write_record(&mut again));
    assert_eq!(out.bytes(), again.bytes());
    let ctx = ReadContext {
        application_default_difficulty: 2,
        default_city_name: b"New City",
        localized_city_name: None,
        localized_mayor_name: None,
        map_tutorial: |v| v,
    };
    let obj = RegionalCity::read_record(&mut ByteReader::new(out.bytes()), &ctx, &mut Host).unwrap();
    assert_eq!(obj.other_deals.len(), 2);
    assert!(obj.local_deals.is_empty());
    assert!(obj.other_deals.iter().all(|d| d.is_silent()));
    c.local_deals.clear();
    c.other_deals = vec![d.clone()];
    c.update_import_export(1, &Host);
    assert_eq!(c.get_imported(1), 100.0);
    c.update_import_export(2, &Host);
    assert_eq!(c.get_exported(1), 100.0);
    d.set_flags(0x80, true);
    c.update_import_export(1, &Host);
    assert_eq!(c.get_imported(1), 0.0);
    c.update_local_deals(2, &Host);
    assert_eq!(c.local_deals.len(), 1);
    assert!(c.other_deals.is_empty());
    c.return_local_deals();
    assert_eq!(c.other_deals.len(), 1);
}
