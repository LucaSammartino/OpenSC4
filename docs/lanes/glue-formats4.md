| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-formats4 (GZCOM glue: data formats, DBPF/exemplars) | `glue-formats4` / (engine lane) | **ready** (data-formats shadow `formats_shadow = 1`; additive C ABI fc4_dbpf_*, no version bump; gate cargo 3591/0, clippy 0, ctest 62/62; in-game gf4-formats-shadow-2 clean, ref_warnings=0, 8957/8957 exemplars exact, 148756 properties) | coordinator: union regress; replacement of the exemplar factory needs the runtime contract questions below |

## Report

What changed
- `engine/capi/src/dbpf.rs` (+ `pub mod dbpf;` at the end of capi/src/lib.rs, a block at the end of fc4.h): `fc4_dbpf_open/close/count_type/key_of_type/read_exemplar`. They use the existing core readers only (`fc4_formats::dbpf::Package`, `Exemplar::parse_record`); read_exemplar returns an fc4_props of the exemplar's OWN properties in the game's variant types (via fc4_props_set_typed), Empty and kind-1 resource properties left out. Unit test `dbpf::tests::opens_a_package_and_reads_own_properties`.
- `engine/plugin/FC4FormatsShadow.{h,cpp}`: at the first city load, every `*.dat` of the install folder (§3.1 step 1; install = parent of the exe folder, [choice]) is opened by the core, each exemplar (0x6534284A) parsed, and compared with the game's loaded exemplar of the same key (EnumProperties own list + CopyToCore, bytes compared via fc4_props_write_variant). Nothing replaced, nothing written to the game. Keys: `formats_shadow` (actual key; there is no "formats" subsystem, so `enable = formats` does not exist), `formats_shadow_max`, `formats_shadow_lines`. Shared edits additive (CMakeLists, FC4Config.h/.cpp key list, FC4Subsystems.cpp startup, FC4Observer.cpp city load).

Measured
- Gate at f2e035e4: cargo test --workspace 3591/0, clippy 0, ctest 62/62.
- gf4-formats-shadow-1 (60 s, Konradshohe, normal speed): clean, ref_warnings=0; SimCity_1.dat 8957 exemplar entries, 8957 exact, 148756 properties; other root .dat files hold no exemplars; unins000.dat (GOG uninstaller) is no DBPF and was tagged a difference, fixed in f2e035e4.
- gf4-formats-shadow-2 (same, f2e035e4): clean, ref_warnings=0, total EXACT, 1203 ms one-shot at load. Not turbo dependent (one-shot at load).

Coverage proposal (coordinator): owner cSCResExemplar (exemplar factory 0x6534284A, own-property read path) shadow=yes verified=yes for the shipped root packages; replaced=no. Not claimed: cohorts (0x05342861), parent/effective list (§4.4), text exemplars outside SimCity_1.dat, DBPF segment / QFS classes (§8: plug-in mode keeps the originals), Plugins-folder load order.

Why no replacement: spec §8 names the factory slots by address only. A replacement needs (no questions filed this round; candidates for docs/specs/questions/data-formats.md): the meaning of RegisterObjectFactory's two u32 arguments and which factory wins for a type already registered; CreateInstance(record, riid, ppv, u1, u2) argument meanings and reference ownership of the result; whether the cIGZPersistDBRecord a factory Read receives holds decompressed bytes at position 0 and who closes it; cISCResExemplar slots 3-5 and 8-15 and cISCPropertyHolder's 25-slot contract for a non-game implementation.

Next
- Extend the shadow to cohorts (IID 0x452160FB) and the effective list (§4.4), and to Plugins packages in §3.1 order.
- File the contract questions above before any factory replacement.
