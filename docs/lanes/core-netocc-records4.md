| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-netocc-records4 (network occupants: NO15-NO17 records, transform, underground record) | `core-netocc-records4` / (engine lane) | in progress | see Handoff |

## Handoff
- Goal: implement answered NO15-NO17 in engine/core/src/network_occupants (record offsets object-8, transform mask/reset/56-byte legacy, one quad of 4 vertices, 76-byte triangles, interleaved bbox, V5 flag map, read tails, underground v3+v8+tail, heights +0x24C..0x258, GetPosition average, missing neighbour false). Pure Rust, no glue/ABI.
- Done: (none yet)
- Next: read spec §2.2/2.3/3.1-3.5, edit xform.rs, record.rs, underground.rs.
- Facts: base main a01b2687; final logical round 3/3.
