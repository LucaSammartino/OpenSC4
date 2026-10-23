// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned ordinary-domain algorithm from `docs/specs/network-routines.md` §§2–8.
//! No native binding: foreign callbacks below each represent one raw operation.

use std::rc::Rc;

use crate::grid::CellRect;
use crate::zones::CellRegion;

/// Valid ordinary city widths. The clear output border may reach `side`.
#[derive(Clone, Copy, Debug)]
pub enum CitySide {
    Small,
    Medium,
    Large,
}

impl CitySide {
    fn cells(self) -> i32 {
        match self {
            Self::Small => 64,
            Self::Medium => 128,
            Self::Large => 256,
        }
    }
}

/// Portable admission failures, NOT native false results (NRT1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    InputOutsideCity,
    MalformedOrOutsideLot,
}

/// Explicit ownership choice: publishing never destroys a previous region.
/// Callers own both the current output and displaced allocations, and may take
/// them back independently. Required allocation success is a precondition.
#[derive(Default, Debug)]
#[allow(clippy::vec_box)] // Preserve each displaced allocation's identity.
pub struct RegionOutput {
    pub current: Option<Box<CellRegion>>,
    pub displaced: Vec<Box<CellRegion>>,
}

impl RegionOutput {
    fn publish(&mut self, region: CellRegion) {
        if let Some(old) = self.current.replace(Box::new(region)) {
            self.displaced.push(old);
        }
    }
}

/// Opaque u64 reference tokens match the existing occupant-manager/demolition
/// core convention. A non-null output owns one reference regardless of AL.
pub trait RawReferenceHost {
    fn query_interface(&self, occupant: u64, iid: u32, output: &mut Option<u64>) -> u8;
    fn release(&self, reference: u64);
}

/// Stateless exclusion predicate; Rc models local AddRef/Release ownership,
/// not native object layout or allocation-failure/exception semantics.
#[derive(Debug)]
pub struct ExclusionFilter;

/// Typed identity views of the SAME local filter allocation. Native base COM
/// implementations and pointer offsets are outside this portable owner.
#[derive(Debug)]
pub enum FilterInterface {
    Primary(Rc<ExclusionFilter>),
    Serializable(Rc<ExclusionFilter>),
}

impl ExclusionFilter {
    pub fn query_interface(this: &Rc<Self>, iid: u32) -> Option<FilterInterface> {
        match iid {
            1 | 0x46A8_0026 => Some(FilterInterface::Primary(Rc::clone(this))),
            0xE4FD_A3D4 => Some(FilterInterface::Serializable(Rc::clone(this))),
            _ => None,
        }
    }

    pub fn is_occupant_included<H: RawReferenceHost>(&self, host: &H, occupant: u64) -> bool {
        let mut output = None;
        let al = host.query_interface(occupant, 0xA976_4C8C, &mut output);
        if let Some(reference) = output {
            host.release(reference);
        }
        al == 0
    }

    pub fn is_type_included(&self, _occupant_type: u32) -> bool {
        true
    }

    pub fn is_property_holder_included<T: ?Sized>(&self, _holder: &T) -> bool {
        true
    }
}

/// The eleven typed foreign DemolishRegion arguments, in published order.
/// `demolished` and `cost` borrow the caller's original storage unchanged.
pub struct PreviewArgs<'a, S> {
    pub do_it: bool,
    pub region: &'a CellRegion,
    pub privilege: u32,
    pub flags: u32,
    pub dezone: bool,
    pub caller_filter: &'a Rc<ExclusionFilter>,
    pub cost: &'a mut i64,
    pub demolished: Option<&'a mut S>,
    pub clicked: Option<u64>,
    pub screen_x: i32,
    pub screen_y: i32,
}

pub trait PlacementDemolitionHost: RawReferenceHost {
    type OccupantSet;

    fn get_lot_manager(&self);
    fn get_lot(&self, x: i32, z: i32, check_containment: bool) -> Option<u64>;
    fn lot_bounding_rect(&self, lot: u64) -> CellRect;
    fn zone_type(&self, x: i32, z: i32) -> i8;
    fn first_occupant(&self, output: &mut Option<u64>, x: i32, z: i32, placement_filter: Option<u64>) -> u8;
    fn demolish_region(&self, args: PreviewArgs<'_, Self::OccupantSet>) -> u8;
}

/// ComputeDemolitionRegionForPlacementCells, ordinary domain only.
///
/// The slice and disjoint mutable borrows exclude malformed ranges, null and
/// aliasing outputs. Initialized, returning foreign primitives and successful
/// allocations are required. Reentry, exceptions, unbacked tokens and unknown
/// foreign filter state are not modeled (NRT1); no rollback is promised.
/// Validation errors are this portable API's admission policy, not native AL.
/// Existing network_rules callers retain their documented foreign bridge.
pub fn compute_demolition_region_for_placement_cells<H: PlacementDemolitionHost>(
    host: &H,
    city: CitySide,
    cells: &[(i32, i32)],
    output: &mut RegionOutput,
    cost: &mut i64,
    demolished: Option<&mut H::OccupantSet>,
    placement_filter: Option<u64>,
) -> Result<u8, Unsupported> {
    let (mut xmin, mut zmin) = (i32::MAX, i32::MAX);
    let (mut xhigh, mut zhigh) = (0, 0);
    let mut rects = Vec::new();
    host.get_lot_manager();
    let side = city.cells();
    for &(x, z) in cells {
        if x < 0 || z < 0 || x >= side || z >= side {
            return Err(Unsupported::InputOutsideCity);
        }
        xmin = xmin.min(x);
        zmin = zmin.min(z);
        if x >= xhigh {
            xhigh = x + 1;
        }
        if z >= zhigh {
            zhigh = z + 1;
        }
        if let Some(lot) = host.get_lot(x, z, false) {
            let mut rect = host.lot_bounding_rect(lot);
            if rect.x1 < 0
                || rect.z1 < 0
                || rect.x2 < rect.x1
                || rect.z2 < rect.z1
                || rect.x2 >= side
                || rect.z2 >= side
            {
                return Err(Unsupported::MalformedOrOutsideLot);
            }
            rect.x2 += 1;
            rect.z2 += 1;
            xmin = xmin.min(rect.x1);
            zmin = zmin.min(rect.z1);
            xhigh = xhigh.max(rect.x2);
            zhigh = zhigh.max(rect.z2);
            rects.push(rect);
        }
    }
    if cells.is_empty() || xhigh < xmin {
        return Ok(1);
    }
    // Inclusive stored highs deliberately include the extra clear border.
    output.publish(CellRegion::new(CellRect::new(xmin, zmin, xhigh, zhigh), false));
    let region = output.current.as_mut().expect("just published region");
    let mut type_mask = 0_u32;
    for rect in &rects {
        for x in rect.x1..rect.x2 {
            for z in rect.z1..rect.z2 {
                let zone = host.zone_type(x, z);
                if zone == 0 {
                    continue;
                }
                let mut occupant = None;
                let al = host.first_occupant(&mut occupant, x, z, placement_filter);
                if al == 0 {
                    region.set(x, z, true);
                    type_mask |= 1_u32 << (zone as u32 & 31);
                }
                if let Some(reference) = occupant {
                    host.release(reference);
                }
            }
        }
    }
    if type_mask & 0xFC00 != 0 {
        return Ok(0);
    }
    *cost = 0;
    let filter = Rc::new(ExclusionFilter);
    let al = host.demolish_region(PreviewArgs {
        do_it: false,
        region,
        privilege: 1,
        flags: 0x2001,
        dezone: false,
        caller_filter: &filter,
        cost,
        demolished,
        clicked: None,
        screen_x: 0,
        screen_y: 0,
    });
    drop(filter);
    drop(rects);
    Ok(al)
}
