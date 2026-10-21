//! Identity of the network-occupant family (network-occupants.md §1):
//! class IDs, occupant types, `QueryInterface` answers, `GetGZCLSID` and
//! `IsOfType`.

/// `cSC4Occupant`'s bare occupant type (§1.1), what the base constructor
/// writes before a subclass overwrites it.
pub const OCCUPANT_TYPE_BARE: u32 = 0x7475_8925;
/// The network-piece occupant type shared by nine classes (§1.1, quirk 1).
pub const OCCUPANT_TYPE_NETWORK_PIECE: u32 = 0xC772_BF98;
pub const OCCUPANT_TYPE_PIPE_CONNECTION: u32 = 0x2A18_3167;
pub const OCCUPANT_TYPE_SUBWAY_CONNECTION: u32 = 0x8A13_918A;
pub const OCCUPANT_TYPE_POWER_POLE: u32 = 0x2890_D4DE;
pub const OCCUPANT_TYPE_POWER_LINE: u32 = 0x0990_B2EA;
/// The pylon's own type (§6.1, quirk 8).
pub const OCCUPANT_TYPE_PYLON: u32 = 0xA9B6_D7DE;

/// `cIGZUnknown`.
pub const IID_UNKNOWN: u32 = 0x0000_0001;
/// `cISC4Occupant`.
pub const IID_OCCUPANT: u32 = 0x26BB_767E;
/// `cIGZSerializable`.
pub const IID_SERIALIZABLE: u32 = 0xE4FD_A3D4;
/// `cISC4NetworkOccupant`.
pub const IID_NETWORK_OCCUPANT: u32 = 0xA821_EF94;
/// The family's shared "class ID as IID" (§1.2).
pub const IID_FAMILY_CLASS: u32 = 0xA976_4C8C;
/// `cISC4PowerPoleOccupant`.
pub const IID_POWER_POLE: u32 = 0xA8C8_6142;
/// `cISC4PowerLineOccupant`.
pub const IID_POWER_LINE: u32 = 0xC98F_3B24;
/// `cISC4PylonOccupant`.
pub const IID_PYLON: u32 = 0xC9B6_D7D6;
/// `cIGZMessageTarget2`.
pub const IID_MESSAGE_TARGET2: u32 = 0x4522_94AA;
/// The power pole's delegate interface (identity unknown, NO2).
pub const IID_POLE_DELEGATE: u32 = 0x26A6_8448;

/// The thirteen classes of §1.1 (the `cSC4Occupant` root is not a member).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NetOccClass {
    Network,
    PreBuilt,
    Bridge,
    Tunnel,
    Underground,
    Pipe,
    Subway,
    UndergroundConnection,
    PipeConnection,
    SubwayConnection,
    PowerPole,
    PowerLine,
    Pylon,
}

impl NetOccClass {
    pub const ALL: [NetOccClass; 13] = [
        NetOccClass::Network,
        NetOccClass::PreBuilt,
        NetOccClass::Bridge,
        NetOccClass::Tunnel,
        NetOccClass::Underground,
        NetOccClass::Pipe,
        NetOccClass::Subway,
        NetOccClass::UndergroundConnection,
        NetOccClass::PipeConnection,
        NetOccClass::SubwayConnection,
        NetOccClass::PowerPole,
        NetOccClass::PowerLine,
        NetOccClass::Pylon,
    ];

    /// The class ID (§1.1).
    pub fn class_id(self) -> u32 {
        match self {
            NetOccClass::Network => 0xC9C0_5C6E,
            NetOccClass::PreBuilt => 0x49C1_A034,
            NetOccClass::Bridge => 0x49CC_1BCD,
            NetOccClass::Tunnel => 0x8A4B_D52B,
            NetOccClass::Underground => 0x4A10_AFC2,
            NetOccClass::Pipe => 0x49C0_5B9F,
            NetOccClass::Subway => 0xCA16_374F,
            NetOccClass::UndergroundConnection => 0x8A13_9184,
            NetOccClass::PipeConnection => 0xEA0A_5ACF,
            NetOccClass::SubwayConnection => 0xCA15_ED6B,
            NetOccClass::PowerPole => 0x09C0_5C6A,
            NetOccClass::PowerLine => 0xC9C0_5C5D,
            NetOccClass::Pylon => 0x49B6_D69B,
        }
    }

    /// The class whose class ID is `id`.
    pub fn from_class_id(id: u32) -> Option<NetOccClass> {
        NetOccClass::ALL.into_iter().find(|c| c.class_id() == id)
    }

    /// The direct parent within the family (§1.2); `None` for the classes
    /// that derive straight from `cSC4Occupant`.
    pub fn parent(self) -> Option<NetOccClass> {
        match self {
            NetOccClass::PreBuilt | NetOccClass::Tunnel | NetOccClass::Underground => Some(NetOccClass::Network),
            // The bridge is built through the prebuilt constructor and its
            // QI falls back to the prebuilt QI (§1.1, §1.2).
            NetOccClass::Bridge => Some(NetOccClass::PreBuilt),
            NetOccClass::Pipe | NetOccClass::Subway => Some(NetOccClass::Underground),
            NetOccClass::PipeConnection | NetOccClass::SubwayConnection => Some(NetOccClass::UndergroundConnection),
            _ => None,
        }
    }

    /// True for the pieces that carry the `cISC4NetworkOccupant` interface
    /// (rooted in `cSC4NetworkOccupant`).
    pub fn is_network_piece(self) -> bool {
        matches!(
            self,
            NetOccClass::Network
                | NetOccClass::PreBuilt
                | NetOccClass::Bridge
                | NetOccClass::Tunnel
                | NetOccClass::Underground
                | NetOccClass::Pipe
                | NetOccClass::Subway
        )
    }

    /// `cSC4UndergroundOccupant` is abstract (§1.3): never instantiated.
    pub fn is_abstract(self) -> bool {
        self == NetOccClass::Underground
    }

    /// `GetGZCLSID` (§1.3); `None` for the abstract underground base
    /// (`_purecall`).
    pub fn gzclsid(self) -> Option<u32> {
        if self.is_abstract() {
            None
        } else {
            Some(self.class_id())
        }
    }

    /// The occupant type `GetType` returns (§1.1). `initialised` matters only
    /// for the pipe and subway connections, whose `Init` writes their type;
    /// before that (and always for the connection base, whose constructor
    /// sets none) the bare type of the `cSC4Occupant` constructor remains.
    pub fn occupant_type(self, initialised: bool) -> u32 {
        match self {
            // §1.1's table lists `0xC772BF98` for the underground family, but
            // §3.2 states its own constructor leaves the bare type (NO3 open).
            // The explicit §3.2 statement of the stored value is used.
            NetOccClass::Underground | NetOccClass::Pipe | NetOccClass::Subway => OCCUPANT_TYPE_BARE,
            c if c.is_network_piece() => OCCUPANT_TYPE_NETWORK_PIECE,
            NetOccClass::PipeConnection if initialised => OCCUPANT_TYPE_PIPE_CONNECTION,
            NetOccClass::SubwayConnection if initialised => OCCUPANT_TYPE_SUBWAY_CONNECTION,
            NetOccClass::PowerPole => OCCUPANT_TYPE_POWER_POLE,
            NetOccClass::PowerLine => OCCUPANT_TYPE_POWER_LINE,
            NetOccClass::Pylon => OCCUPANT_TYPE_PYLON,
            _ => OCCUPANT_TYPE_BARE,
        }
    }

    /// `IsManagedGlobally` (§2.7): true only for the prebuilt, bridge and
    /// tunnel primaries; `None` for the classes without the slot.
    ///
    /// The underground family's answer is not stated by §2.7; the literal
    /// reading (the slot is inherited from the plain class, `0x00648EF0`,
    /// false) is used (question NO9).
    pub fn is_managed_globally(self) -> Option<bool> {
        match self {
            NetOccClass::PreBuilt | NetOccClass::Bridge | NetOccClass::Tunnel => Some(true),
            c if c.is_network_piece() => Some(false),
            _ => None,
        }
    }

    /// The sub-object offsets from the object base (§1.2, §2.1, §4.1, §5.1,
    /// §6.1): `(occupant view, serializable, property holder, model instance)`.
    pub fn sub_objects(self) -> SubObjects {
        match self {
            c if c.is_network_piece() => SubObjects {
                occupant: 4,
                serializable: 8,
                holder: 0xC,
                model: Some(0x68),
            },
            NetOccClass::UndergroundConnection | NetOccClass::PipeConnection | NetOccClass::SubwayConnection => {
                SubObjects {
                    occupant: 0x1C,
                    serializable: 4,
                    holder: 8,
                    model: Some(0x38),
                }
            }
            NetOccClass::PowerPole | NetOccClass::Pylon => SubObjects {
                occupant: 8,
                serializable: 0xC,
                holder: 0x10,
                model: Some(0x3C),
            },
            _ => SubObjects {
                occupant: 4,
                serializable: 8,
                holder: 0xC,
                model: None,
            },
        }
    }
}

/// Where a class keeps its secondary views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubObjects {
    pub occupant: u32,
    pub serializable: u32,
    pub holder: u32,
    pub model: Option<u32>,
}

/// What `QueryInterface` resolves an IID to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QiAnswer {
    /// The view at this offset from the object base (`AddRef`ed).
    View(u32),
    /// The pole's delegate at `+0xBC` answers (NO2).
    PoleDelegate,
    /// Not answered by the class itself: the property holder is asked
    /// (when `holder` is true), then the model instance (when `model` is
    /// true). The core cannot answer for those host objects.
    Fallthrough { holder: bool, model: bool },
}

/// `QueryInterface` for `class` (§1.2).
pub fn query_interface(class: NetOccClass, iid: u32) -> QiAnswer {
    use NetOccClass::*;
    let own = |ids: &[u32]| ids.contains(&iid);
    match class {
        Network => {
            if own(&[IID_NETWORK_OCCUPANT, IID_FAMILY_CLASS]) {
                QiAnswer::View(0)
            } else {
                occupant_qi(class, iid, true)
            }
        }
        PreBuilt | Bridge | Tunnel | Pipe | Subway | PipeConnection | SubwayConnection => {
            let extra: &[u32] = if class == Pipe { &[IID_FAMILY_CLASS] } else { &[] };
            if iid == class.class_id() || own(extra) {
                QiAnswer::View(0)
            } else {
                // `parent()` is always Some for these classes.
                query_interface(class.parent().unwrap_or(Network), iid)
            }
        }
        Underground => {
            if own(&[class.class_id(), IID_NETWORK_OCCUPANT, IID_FAMILY_CLASS]) {
                QiAnswer::View(0)
            } else {
                query_interface(Network, iid)
            }
        }
        UndergroundConnection => {
            if own(&[class.class_id(), IID_FAMILY_CLASS]) {
                QiAnswer::View(0)
            } else {
                occupant_qi(class, iid, true)
            }
        }
        PowerPole => {
            if own(&[IID_POWER_POLE, class.class_id(), IID_FAMILY_CLASS]) {
                QiAnswer::View(0)
            } else if iid == IID_MESSAGE_TARGET2 {
                QiAnswer::View(4)
            } else if iid == IID_POLE_DELEGATE {
                QiAnswer::PoleDelegate
            } else {
                occupant_qi(class, iid, true)
            }
        }
        PowerLine => {
            if own(&[IID_POWER_LINE, class.class_id(), IID_FAMILY_CLASS]) {
                QiAnswer::View(0)
            } else {
                // Tail jump into the occupant QI: no model fallback.
                occupant_qi(class, iid, false)
            }
        }
        Pylon => {
            // The §1.2 row lists only `0xC9B6D7D6` (not the family IID
            // `0xA9764C8C` the prose says every class accepts, nor the class
            // ID): literal row kept (question NO10).
            if iid == IID_PYLON {
                QiAnswer::View(0)
            } else if iid == IID_MESSAGE_TARGET2 {
                QiAnswer::View(4)
            } else {
                occupant_qi(class, iid, true)
            }
        }
    }
}

/// The `cSC4Occupant` QI as reached from `class`: IID 1 or `cISC4Occupant`
/// → the occupant view, `cIGZSerializable` → the serializable view, else the
/// property holder (then the model instance when the class has that
/// fallback).
fn occupant_qi(class: NetOccClass, iid: u32, model_fallback: bool) -> QiAnswer {
    let s = class.sub_objects();
    if iid == IID_UNKNOWN || iid == IID_OCCUPANT {
        QiAnswer::View(s.occupant)
    } else if iid == IID_SERIALIZABLE {
        QiAnswer::View(s.serializable)
    } else {
        QiAnswer::Fallthrough {
            holder: true,
            model: model_fallback && s.model.is_some(),
        }
    }
}

/// The null-`this` quirk (§1.2, quirk 2): the IIDs for which `QueryInterface`
/// on a null object writes 0 to the out-pointer and still returns 1. Per
/// §1.2 the pole has it on its own IIDs and `0x452294AA` (quirk 2), the
/// pylon on `0x452294AA`.
pub fn null_this_returns_true(class: NetOccClass, iid: u32) -> bool {
    match class {
        NetOccClass::PowerPole => {
            [IID_POWER_POLE, class.class_id(), IID_FAMILY_CLASS, IID_MESSAGE_TARGET2].contains(&iid)
        }
        NetOccClass::Pylon => iid == IID_MESSAGE_TARGET2,
        _ => false,
    }
}

/// `IsOfType(t)` (slot 22, §1.4, quirk 3): the piece carries network flag
/// `1 << t`. Indices outside the 13-entry table read past it in the game;
/// the core answers false for them *[choice]*.
pub fn is_of_type(network_flags: u32, t: u32) -> bool {
    t < 13 && network_flags & (1 << t) != 0
}
