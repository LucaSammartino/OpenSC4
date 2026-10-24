| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-effects-shadow7: effects record shadow | Codex / glue-effects-shadow7 | in progress | Inspect public native contracts, implement measurable comparisons, gate and run through 2032. |

## Handoff

- Goal: default-off effects_shadow, exact dated native/core comparisons through 2032.
- Worktree is reserved with detached HEAD; commit without changing refs.
- Initial record: 802997a2f.
- Implementation: a03fb28b4. Focused ABI test 1/1 passed; explicit two-file fmt 0 diff lines.
- qbuild all at a03fb28b4: cargo 6918/0/0, Clippy 0 diagnostics; native qbuild-20261024-044021-build-ctest terminal FAIL, C2027 cIGZCOM incomplete in FC4EffectsShadow.cpp:288, CTest skipped. Operation 57577 complete. Logs preserved in build-runs/gate-r1-failed. Added explicit SDK header as diagnosed fix.
- qbuild all at 73cf23514: cargo 6918/0/0, Clippy 0 diagnostics; native qbuild-20261024-044547-build-ctest build PASS 104/104, CTest 73/74, only ret_channel_check missing review marker on HasVisualEffect bool. Logs in build-runs/gate-r2-failed. Added [abi-ret] comment citing section 4.2 and SDK boolean; no native channel/signature invented. Operation 58740 complete.
- Final qbuild all at cef2f983b PASS: cargo 6918/0/0; Clippy 0 diagnostics; queued native qbuild-20261024-044936-build-ctest build 51/51 (2.7s), CTest 74/74 (74.7s), overall 78.9s. Operation 59870 terminal exit 0. Stable final source, no source edits during gate.
- First game run effects7-names-2032-20261024a: terminal clean, harness effects 73/73, 0 resyncs, ref_warnings=0, end 2032-01-04, 32.140s. Record docs/runs/effects7-names-2032-20261024a-410.md. Every sample compared 1154 raw name entries/3465 queries from 1094484 bytes. Load date was invalid because PublishedJulian was zero; corrected to live simulator interface date. This necessitates a new gate and game run despite exact fields. Earlier full gate remains valid for the earlier source only.
- Implemented record-only packed-resource name parser/native HasVisualEffect comparison; original, upper and lower case plus three boundary/missing probes, load and month lines. Native mode has no tick subscription or host writes.
- EF20 filed: collection getter ABI/container framing and queue/bootstrap capture remain unspecified. Scope is name membership only, no EF18/EF19 owner equivalence claimed.
- New ABI export fc4_effects_verify_names; version unchanged. Added failure/difference/case/priority-zero ABI test.
- Next: qbuild all after load-date fix, then new unique game label effects7-names-2032-20261024b. Do not edit tools profile (clean-room prohibition); coordinator must append after exact run.
- No outstanding game request; operation 41934 timed out but already-completed verdict A was accepted. The local filesystem watcher missed its event; helper now rechecks existence before throwing. Never resend A.
