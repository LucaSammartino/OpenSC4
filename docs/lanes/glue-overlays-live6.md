| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-overlays-live6: live terrain overlays | Codex; detached HEAD in glue-overlays-live6 | ready: full gate and both 2032 runs clean; fixture flags differ under OV15 | land lane; resolve OV15 before claiming complete replay |

## Report

Default `overlays_shadow` remains read-only. The implementation observes the four native managers, logs dated sentinel and record-invariant checks, handles the OV14 inactive-vector contract, and offers an explicit `overlays_fixture` opt-in that creates/removes temporary native overlays and compares an add/move/scalar suffix across 35 fields per manager/sample. Sentinel, record-invariant and controlled-replay results are reported separately. Full flags remain compared; the bit-7 difference is neither masked nor guessed away. `overlays_fixture` is registered as a configuration key. The lane adds the private C ABI export `fc4_overlays_replay_decal`; ABI version was not bumped.

Round 2 changed documentation only. It reused clean detached HEAD `618ca267a`, ran one final-source full gate, and tested that DLL in both requested modes. Engine inputs remain identical to that gate. No branch switch, ref update, main merge, or game launch outside the queue was performed this round.

Final gate: **PASS**. Workspace Rust tests **6902 passed, 0 failed, 0 ignored**; clippy **0 diagnostics**; native build **216/216** edges (44.1 s); full CTest **74/74** (97.9 s); native total143.8 s. Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`, `fc4verify-build.log`, `fc4verify-ctest.log`. Earlier failed ABI-equality CTest evidence remains in `build-runs/fc4verify-ctest-failed-a18d780d3.log`; diagnostic source761836e97 forced the stale smoke-test input to recompile without an ABI bump and passed its gate. The final gate above covers the later merged inputs too.

Both round-2 runs used DLL SHA256 `de013955b8774f76364b5c68277cbf4d9ee0ba09fe7c3834587ce6d0c71f1c7d`, Konradshohe, turbo, target2032-01-01, `overlays_shadow=1`, observe/digest off. Both verdicts were **clean**, `ref_warnings=0`, all harness problem counts0, no incomplete captures; only the two expected x87 startup warnings appeared.

| Run record | Mode / final date | Dated overlays shadow | Record observations | Controlled replay |
|---|---|---|---|---|
| [403: ovlive6-r2-ro-2032-20261024d](../runs/ovlive6-r2-ro-2032-20261024d.md) | default read-only /2032-01-17 |73/73 exact;7300/7300 checks |292/292 zero-mismatch lines; empty arenas,0/0 live checks |0 lines; fixture omitted |
| [404: ovlive6-r2-fixture-2032-20261024e](../runs/ovlive6-r2-fixture-2032-20261024e.md) | fixture /2032-01-12 |73/73 exact;7300/7300 checks |292/292 exact;12264/12264 checks;292 live overlays/handles/bindings,584 entries |0/292 fully exact;9928/10220 fields exact;292 differences, all OV15 |

Every fixture sample matched34/35 fields. The only difference is the complete flags word's bit7 (native0xD1/core0x51), already present in all four managers at load before turbo. `question=OV15` is emitted only when the comparison differs solely by that bit. This reproduces the existing original-behaviour question [OV15](../specs/questions/overlay-manager.md); no new question or capture/harness fix was needed. Since it is present before turbo, no turbo-only normal-speed rerun is required. OV13 still limits original mutation history/Init reconstruction and image bytes. These checks do not establish complete rendering or original-history replay.

Earlier committed fixture evidence is preserved: [a/run400](../runs/ovlive6-2032-20261024a.md), [b/run401](../runs/ovlive6-2032-20261024b.md), [c/run402](../runs/ovlive6-2032-20261024c.md). They describe their own tested sources; the round-2 runs verify the final merged source. The initial focused test's fixture-flags expectation was corrected from65 to81; production arithmetic was unchanged. Prior milestones include implementation3e887dd25, corrected source09fe48059, ABI diagnostic761836e97, and round-2 gate recorde071118bd/read-only recordb078b820b.

## Handoff

- Ready to land with the two round-2 game labels; every difference has existing question OV15.
- No game request, wait, test operation, or uncommitted engine change outstanding. Waiters52418 and23966 both collected to terminal; completed unique verdicts retained.
- No new questions; coordinator should ensure OV15 is included when regenerating the question index.
- Next engineering work depends on OV15's specified flags behaviour; preserve full-flags comparison and the read-only default.
- Supervisor publishes detached commits to the reserved lane branch. Queue uses actual full HEAD; manifest is `C:/Users/tlori/FreeCity4/build-runs/run4-glue-overlays-live6-r2-manifest.json`.
