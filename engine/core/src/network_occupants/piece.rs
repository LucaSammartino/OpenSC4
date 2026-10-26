//! The network piece (`cSC4NetworkOccupant` and its prebuilt, bridge and
//! tunnel subclasses), network-occupants.md §2: network flags, the usable
//! message, the path-connection table, orientation bytes, the occupied cell
//! and the `Init` lifecycle. The save record (§2.3) is not implemented here.

use super::identity::{is_of_type, NetOccClass};

/// The network flags a fresh piece carries (§2.2, ctor).
pub const FLAGS_CTOR: u32 = 0x0600_0000;
/// `IsImmovable`, set by the prebuilt constructor (§2.8).
pub const FLAG_PREBUILT: u32 = 0x0040_0000;
/// The path table is current (§2.6).
pub const FLAG_PATHS_BUILT: u32 = 0x2000;
/// Translucency request (§2.7: setting it calls slot 58).
pub const FLAG_TRANSLUCENT: u32 = 0x4000;
/// Lighting computed (§2.7, `UpdateLighting`).
pub const FLAG_LIT_COMPUTED: u32 = 0x8000;
/// Usable (network-manager.md §5).
pub const FLAG_USABLE: u32 = 0x1000_0000;
/// Cleared by a re-`Init` along with [`FLAG_USABLE`] (§2.5).
pub const FLAG_REINIT_CLEAR: u32 = 0x0080_0000;
/// The bits the tunnel constructor sets (§2.8).
pub const TUNNEL_CTOR_SET: u32 = 0x10C0_0000;
/// The bit the tunnel constructor clears (§2.8, literal).
pub const TUNNEL_CTOR_CLEAR: u32 = 0x400;
/// `+0x48` after the base `Init` (§2.5).
pub const LIFECYCLE_INIT: u32 = 0x1A;
/// The wildcard of `PathConnection` (§2.6).
pub const PATH_WILDCARD: u8 = 0xFF;
/// `SetTranslucency`'s alpha when flag `0x4000` is set (§2.7).
pub const TRANSLUCENT_ALPHA: u8 = 0x80;

/// The calls a piece makes into the rest of the game. The plugin glue
/// implements it over the game's interfaces; tests record the calls.
pub trait PieceHost {
    /// Posts (queued) `0x89D3E9F3` kSC4MessageNetworkOccupantUsableChange with
    /// the piece and data1 = 1 / 0 (§2.7, network-manager.md §5).
    fn post_usable_change(&mut self, usable: bool);
    /// Primary slot 58 `SetTranslucency(on, alpha)` (§2.7).
    fn set_translucency(&mut self, on: bool, alpha: u8);
    /// `GetPathInfo` (slot 49), called by `PathConnection` when flag
    /// `0x2000` is unset (§2.6).
    fn build_path_info(&mut self);
    /// The network manager's `AddToRetrofitList` (§2.5).
    fn add_to_retrofit_list(&mut self);
    /// `cSC4Occupant::Init` (§2.5).
    fn occupant_init(&mut self);
    /// `cSC4Occupant::Shutdown` (§2.5).
    fn occupant_shutdown(&mut self);
    /// The queued message `SetRotation` / `SetVariation` post (§2.4; its ID
    /// is not given by the spec).
    fn post_piece_changed(&mut self);
    /// The prebuilt `Init`'s model steps (§2.8): `SetModel` from the
    /// exemplar, then `RecomputeShearTransform`.
    fn prebuilt_set_model(&mut self) {}
    /// The prebuilt `Shutdown`'s model release (§2.8).
    fn prebuilt_release_model(&mut self) {}
    fn check_for_retrofit(&mut self) -> bool {
        false
    }
}

/// One network piece.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkPiece {
    /// One of `Network`, `PreBuilt`, `Bridge`, `Tunnel`.
    pub class: NetOccClass,
    /// `+0x13C` (§2.2).
    pub network_flags: u32,
    /// `+0x48` lifecycle flags (§2.2).
    pub lifecycle: u32,
    /// The occupant has run `Init` (or was loaded initialised).
    pub initialised: bool,
    /// `GetFlags() & 1`: the occupant is in the city.
    pub in_city: bool,
    /// `+0x104`: low 7 bits orientation/variation, bit 7 the flip (§2.4).
    pub orientation: u8,
    /// `+0x105` (§2.4).
    pub rotation: u8,
    /// `+0x100` (§2.4).
    pub variation: u32,
    /// §2.6: eight path-type rows, four entry bytes, one bit per exit.
    pub connections: [[u8; 4]; 8],
    /// `+0x164`, the path-id override (slot 16).
    pub path_id_override: u32,
    /// `+0x168`/`+0x16C`, the demolition cost (slot 63, quirk 12).
    pub demolition_cost: i64,
    /// World position x, y, z (slot 75).
    pub position: [f32; 3],
}

impl NetworkPiece {
    /// A freshly constructed piece of `class` (§2.2, §2.8). Panics on a
    /// class outside the §2 family.
    pub fn new(class: NetOccClass) -> NetworkPiece {
        let mut flags = FLAGS_CTOR;
        match class {
            NetOccClass::Network => {}
            NetOccClass::PreBuilt | NetOccClass::Bridge => flags |= FLAG_PREBUILT,
            NetOccClass::Tunnel => {
                // §2.8: `+0x13C &= ~0x400 | 0x10C00000`, "clearing the
                // prebuilt bit" -- but the prebuilt bit is 0x400000, not
                // 0x400. The literal expression is applied (question NO11),
                // so a tunnel keeps 0x400000.
                flags |= FLAG_PREBUILT;
                flags = (flags & !TUNNEL_CTOR_CLEAR) | TUNNEL_CTOR_SET;
            }
            // §3.1: the underground ctor (also the pipe/subway base ctor)
            // builds on this base; §3 names no extra ctor flags.
            NetOccClass::Underground | NetOccClass::Pipe | NetOccClass::Subway => {}
            other => panic!("{other:?} is not a §2 network piece"),
        }
        NetworkPiece {
            class,
            network_flags: flags,
            lifecycle: 0,
            initialised: false,
            in_city: false,
            orientation: 0,
            rotation: 0,
            variation: 0,
            connections: [[0; 4]; 8],
            path_id_override: 0,
            demolition_cost: 0,
            position: [0.0; 3],
        }
    }

    pub fn has_all_network_flags(&self, bits: u32) -> bool {
        self.network_flags & bits == bits
    }

    pub fn has_any_network_flag(&self, bits: u32) -> bool {
        self.network_flags & bits != 0
    }

    /// `SetNetworkFlag` (§2.7, network-manager.md §5).
    pub fn set_network_flag(&mut self, bits: u32, host: &mut dyn PieceHost) {
        if bits & FLAG_USABLE != 0 && self.network_flags & FLAG_USABLE == 0 && self.in_city {
            host.post_usable_change(true);
        }
        self.network_flags |= bits;
        if bits & FLAG_TRANSLUCENT != 0 {
            host.set_translucency(true, TRANSLUCENT_ALPHA);
        }
    }

    /// `ClearNetworkFlag` (§2.7): the usable message does not check
    /// in-city.
    pub fn clear_network_flag(&mut self, bits: u32, host: &mut dyn PieceHost) {
        if bits & FLAG_USABLE != 0 && self.network_flags & FLAG_USABLE != 0 {
            host.post_usable_change(false);
        }
        self.network_flags &= !bits;
    }

    /// `IsOfType` (slot 22, §1.4).
    pub fn is_of_type(&self, t: u32) -> bool {
        is_of_type(self.network_flags, t)
    }

    /// `IsManagedGlobally` (slot 5, §2.7).
    pub fn is_managed_globally(&self) -> bool {
        self.class.is_managed_globally().unwrap_or(false)
    }

    /// `Init` (slot 3, §2.5) and the subclass `Init`s (§2.8).
    pub fn init(&mut self, host: &mut dyn PieceHost) {
        if !self.initialised {
            self.clear_network_flag(FLAG_PATHS_BUILT, host);
            self.lifecycle = LIFECYCLE_INIT;
            if host.check_for_retrofit() {
                host.add_to_retrofit_list();
            }
            host.occupant_init();
            self.initialised = true;
        } else {
            for flag in [FLAG_USABLE, FLAG_REINIT_CLEAR] {
                if self.network_flags & flag != 0 {
                    self.network_flags &= !flag;
                    self.set_network_flag(flag, host);
                }
            }
        }
        if self.class != NetOccClass::Network {
            // The prebuilt `Init` (also the bridge's): base `Init`, then
            // `+0x48 & ~0x10 | 0x40`, `SetModel`, `RecomputeShearTransform`.
            self.lifecycle = (self.lifecycle & !0x10) | 0x40;
            host.prebuilt_set_model();
            if self.class == NetOccClass::Tunnel {
                self.clear_network_flag(FLAG_TRANSLUCENT, host);
            }
        }
    }

    /// `Shutdown` (slot 4, §2.5, §2.8). The bridge's structure list and the
    /// tunnel's paired portal are not modelled here.
    pub fn shutdown(&mut self, host: &mut dyn PieceHost) {
        if self.class != NetOccClass::Network {
            host.prebuilt_release_model();
        }
        host.occupant_shutdown();
    }

    /// `AddPathConnection(pathType, entry, exit)` (slot 54 in the compiled
    /// order, §2.6): the exit bit in the path-type and type-zero entry bytes,
    /// then flag `0x2000`. Out-of-range entries are ignored.
    pub fn add_path_connection(&mut self, path_type: u8, entry: u8, exit: u8, host: &mut dyn PieceHost) {
        if path_type < 8 && entry < 4 && exit < 4 {
            self.connections[path_type as usize][entry as usize] |= 1 << exit;
            self.connections[0][entry as usize] |= 1 << exit;
        }
        self.set_network_flag(FLAG_PATHS_BUILT, host);
    }

    /// `PathConnection(pathType, entry, exit)` (slot 53, §2.6). Builds the
    /// path info first when flag `0x2000` is unset. Without wildcards it
    /// returns 0/1; entry/exit wildcards count matching low-four exit bits.
    pub fn path_connection(&mut self, path_type: u8, entry: u8, exit: u8, host: &mut dyn PieceHost) -> u8 {
        if self.network_flags & FLAG_PATHS_BUILT == 0 {
            host.build_path_info();
        }
        // SPEC-GAP: §2.6 defines no path-type wildcard; invalid types return zero.
        let Some(row) = self.connections.get(path_type as usize) else {
            return 0;
        };
        let range = |v: u8, n: u8| -> std::ops::Range<u8> {
            if v == PATH_WILDCARD {
                0..n
            } else {
                v..v.saturating_add(1)
            }
        };
        let mut count: u32 = 0;
        for e in range(entry, 4) {
            for x in range(exit, 4) {
                if e < 4 && x < 4 && row[e as usize] & (1 << x) != 0 {
                    count += 1;
                }
            }
        }
        let wild = entry == PATH_WILDCARD || exit == PATH_WILDCARD;
        if wild {
            count.min(u8::MAX as u32) as u8
        } else {
            (count != 0) as u8
        }
    }

    /// `GetRotationAndFlip` (slot 26): the complete byte `+0x104` (NO14).
    pub fn get_orientation(&self) -> u8 {
        self.orientation
    }

    /// `GetFlip` (slot 25): bit 7 of `+0x104`.
    pub fn get_flip(&self) -> u8 {
        self.orientation >> 7
    }

    /// `GetRotation` (slot 24): low seven bits of `+0x104`.
    pub fn get_rotation(&self) -> u8 {
        self.orientation & 0x7F
    }

    /// Convenience rotation edit; the interface sets this byte via SetQuadDraw.
    pub fn set_rotation(&mut self, rotation: u8, host: &mut dyn PieceHost) {
        self.orientation = (self.orientation & 0x80) | (rotation & 0x7F);
        host.post_piece_changed();
    }

    /// `SetVariation` (slot 27): writes the byte `+0x105` on a change.
    pub fn set_variation(&mut self, variation: u32, host: &mut dyn PieceHost) {
        let value = variation as u8;
        if self.rotation != value {
            self.rotation = value;
            host.post_piece_changed();
        }
    }

    /// `SetUnderTexture` (slot 29): writes `+0x100` on a change.
    pub fn set_under_texture(&mut self, texture: u32, host: &mut dyn PieceHost) {
        if self.variation != texture {
            self.variation = texture;
            host.post_piece_changed();
        }
    }

    /// `GetOccupiedCell` (slot 59, §2.4).
    pub fn get_occupied_cell(&self) -> (i32, i32) {
        (world_to_cell(self.position[0]), world_to_cell(self.position[2]))
    }

    /// The demolition cost (slot 63, quirk 12).
    pub fn get_demolition_cost(&self) -> i64 {
        self.demolition_cost
    }
}

/// The 16-metre cell quantiser of §2.4 (quirk 9): `r24(v * 0.0625 +
/// 12582912.0) − 12582912.0`. The float32 add rounds to the nearest integer
/// (ties to even), so this is round-to-nearest of `v / 16`, not floor.
pub fn world_to_cell(v: f32) -> i32 {
    const MAGIC: f32 = 12_582_912.0;
    let s: f32 = v * 0.0625 + MAGIC;
    (s.trunc() - MAGIC) as i32
}
