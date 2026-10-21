| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-formats4 (GZCOM glue: data formats, DBPF/exemplars) | `glue-formats4` / (engine lane) | **ready** (round 2: questions EX41-EX45 filed; formats shadow extended to cohorts 0x05342861 and §4.4 effective lists over §3.1 steps 1/3/4; additive C ABI fc4_exstore_*, no version bump; gate cargo 3592/0, clippy 0, ctest 62/62; in-game gf4r2-store-1 to 2032-01-19 clean, ref_warnings=0, all four sets exact) | coordinator: regenerate QUESTIONS.md (EX41-EX45; lanes may not run tools/), regress shadow override; no replacement (§8, EX41-EX44) |

## Report (round 2)

What changed
- `docs/specs/questions/data-formats.md` (new): EX41 RegisterObjectFactory arguments/precedence/unregister; EX42 CreateInstance arguments, unknown IID, reference ownership; EX43 SerialRead record position and decompression; EX44 record close and object identity/slot contracts; EX45 §3.1 step 3 Sku_Data file set and non-DBPF files in a multi segment. QUESTIONS.md not regenerated (tools/questions.py is off limits to engine lanes).
- `engine/capi/src/dbpf.rs` (+ fc4.h block after fc4_dbpf_*): `fc4_exstore_create/destroy/add_package/resolve/key/props/untyped_ids/failure`, a thin wrapper over the existing `ExemplarStore::load_package` / `load_all` / `LoadedExemplar::{own_properties, properties}` (no new parser logic). Unit test `store_combines_packages_and_builds_effective_lists` (front-registration override, parent, effective vs own, untyped IDs, parent failure).
- `engine/plugin/FC4FormatsShadow.cpp`: round-1 per-file own-exemplar compare kept; a root `.dat` the core rejects is skipped only when its first four bytes are not `DBPF` (proven magic), otherwise counted a difference. New store phase: packages in §3.1 order (step 1 root `*.dat`, step 3 Sku_Data `*.dat` recursive files-first, step 4 install Plugins non-.dat then .dat); step 2 (language folder, §12 Q1 unresolved) and step 5 (user Documents Plugins) not read [choice, commented]. Every loaded record is fetched from the game (GetResource with IID cISCResExemplar or cohort 0x452160FB, FC4_ACQUIRED/FC4_RELEASE) and compared in four sets: own list (EnumProperties) and effective list (exemplars: props::EffectiveProperties; cohorts: GetParent walk), values via the holder's effective GetProperty, variant bytes compared; Empty/kind-1 IDs must be untyped on both sides. Core load failures must also fail in the game. cIGZUnknownList (GetPropertyList) not used: its slots are not in docs/abi.

Measured
- Gate at 60a2b64c: cargo test --workspace 3592/0, clippy 0, ctest 62/62.
- gf4r2-store-1 (turbo, end_date 2032-01-01, reached 2032-01-19, Konradshohe): clean, ref_warnings=0. Per-file: SimCity_1.dat 8957/8957 exact; unins000.dat no DBPF magic, skipped. Store: 15 files listed, 9 packages, 6 without DBPF magic skipped, 0 rejected; 8959 exemplar + 388 cohort records, 0 unreadable; 9347 loaded, 0 failed. Sets: exemplar own 8959/8959 clean (148906 properties, 6232147 value bytes); exemplar effective 8959/8959 (169852, 6515808 B); cohort own 388/388 (3419, 57221 B); cohort effective 388/388 (4996, 74363 B); 0 not loaded by the game; EXACT, 2297 ms one-shot at the first city load (not turbo dependent).

Coverage proposal (coordinator, after regress): cSCResExemplar and cSCResExemplarCohort read paths shadow=yes verified=yes for the install's root, Sku_Data and install-Plugins packages; replaced=no (spec §8: plug-in mode keeps the originals; factory replacement needs EX41-EX44). Not claimed: language folder (step 2), user Plugins (step 5), DBPF segment / QFS classes.

Next
- Answers to EX41-EX44 before any factory replacement; EX45 and §12 Q1 to add steps 2/3 exactly.
