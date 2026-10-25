| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-effects-replace16 (factory and native-name shadow) | `Codex` / `glue-effects-replace16` (engine lane) | Ready measured partial: cargo 7501/0; clippy clean; CTest 85/85; native names 73/73 to 2032; replacement remains unsafe | Complete section 18 native adapter before adding effects to the house replace list |

Code carried forward: standard factory registration, constructor ownership, +8 message-target identity, guarded replacement, and native-shadow selection (cb162c09c through a706ff437).
This round fixes section 18.2 QueryInterface rejection: unsupported IIDs leave the output untouched.
The factory CTest now checks failed-query ownership, null output, identity round-trip from +8, and shared reference counts. Corrected the adapter's layout comment.
Final stable-source gate: qbuild all PASS; Rust 7501 passed / 0 failed; clippy 0 diagnostics; native build 49/49; CTest 85/85.
Native queue request: qbuild-20261025-015012-build-ctest; logs under build-runs/qbuild-*.log and build-runs/fc4verify-*.log.
Exact result: glue-effects16-guarded-names-20261025-r3 73/73.
House-profile turbo run reached 2032-01-01 clean in 107.344 real seconds; ref_warnings=0, foreign_exceptions=0, panics=0, other warnings=0; DLL SHA256 prefix 8a0a342b.
Evidence: [verdict](../../build-runs/game-results/glue-effects16-guarded-names-20261025-r3.verdict), [log](../../build-runs/game-results/glue-effects16-guarded-names-20261025-r3.log).
Every load/month sample: 1154 entries, 3465 queries, refusal=0, mismatches=0. Payload/history unavailable; this measures native name membership only.
The AddCls skip is now deliberate: effects_replace=1 is required for unsafe diagnostic replacement; guarded runs retain the native manager. Shadows remain default-off.
Prior genuinely registered isolated trials glue-effects16-iso-20261025-r1/r2 faulted after Init (write 0x4D); house trial glue-effects16-house-20261025-r1 faulted at 0x18 before a date.
Those failures remain valid: this round does not change native layout. Full replacement, rendering, collection publication, and parser execution are not verified.
Section 18.5 still leaves internal-base contracts unresolved; no signatures guessed. Remaining collection/direct-field adapter work prevents replacement credit.
Questions filed: none under revised lane rules. C ABI exports/version, tools, coverage maps and house profiles unchanged. Ready queue entry covers measured partial work only.
