//! The network-occupant family (docs/specs/network-occupants.md).
//!
//! What is here:
//! - [`identity`]: §1 for all thirteen classes (class IDs, occupant types,
//!   `QueryInterface`, `GetGZCLSID`, `IsOfType`, `IsManagedGlobally`).
//! - [`piece`]: the §2 network piece behaviour for `cSC4NetworkOccupant`,
//!   `…WithPreBuiltModel`, `…BridgeOccupant` and `…TunnelOccupant` (flags,
//!   usable message, `Init`/`Shutdown`, path connections, orientation, the
//!   occupied cell). Not the §2.3 record, the geometry or the drawing.
//! - [`power_line`]: §5, `cSC4PowerLineOccupant` slots and its v2 record.
//! - [`power_pole`]: published §4 state, controls, static-word cost and Init
//!   state prefix; full owner remains open (NO33-NO35).
//! - [`pylon`]: §6 record, call/drag state, published lifecycle (NO36-NO37).
//! - [`connection`]: §3.6 record, hidden triangles and Shutdown (UG1/UG5).
//!
//! - [`record`], [`xform`]: the §2.3 base record v4-v8 and the transform.
//! - [`underground`]: §3.2 behaviour and the §3.3-§3.5 records.
//! - [`model`]: §10, `cSC4NetworkOccupantModelInstance` (deform, passes, fallback).
//!
//! Not complete: connection creation/model geometry (§3.6), pole (§4), pylon
//! (§6); see the module README's per-section owner audit. Also not here: the
//! type-info tables (§8), and routines other than the
//! ordinary placement-demolition helper in [`placement_demolition`].

pub mod identity;
pub mod model;
pub mod piece;
pub mod power_line;
pub mod power_pole;
pub mod record;
pub mod underground;
pub mod xform;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_model;
#[cfg(test)]
mod tests_record;

pub use identity::{is_of_type, query_interface, NetOccClass, QiAnswer};
pub use model::{NetworkModelInstance, PieceRender};
pub use piece::{world_to_cell, NetworkPiece, PieceHost};
pub use power_line::{PowerLine, PowerLineRecord};
pub use power_pole::PowerLineControl;
pub use record::{read_network_record, write_network_record, LoadEffects, NetworkBaseRecord, Vertex};
pub use underground::{
    read_underground_record, write_underground_record, UndergroundHost, UndergroundModel, UndergroundPiece,
    UndergroundRecord,
};
pub use xform::Xform;

pub mod placement_demolition;

#[cfg(test)]
mod tests_placement_demolition;

#[cfg(test)]
mod tests_power_pole;

pub mod pylon;

#[cfg(test)]
mod tests_pylon;

pub mod connection;

#[cfg(test)]
mod tests_connection;
