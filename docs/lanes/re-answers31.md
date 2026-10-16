# Lane: re-answers31 (RE answers: A4/A1 automata, glue22 q9 effects, GLUE21-1/-2, DQ2/DQ4, NR8, DS38)

| Lane | Worktree | State at stop | Next steps |
|---|---|---|---|
| re-answers31 (RE answers: A4/A1 automata, glue22 q9 effects, GLUE21-1/-2, DQ2/DQ4 demolition, NR8 network-rules, DS38 disasters) | `C:\Users\tlori\FreeCity4\.claude\worktrees\re-answers31` / branch `re-answers31` | **ready, not merged** (docs only: `automata.md`, `effects.md`, `lot-configuration.md`, `prop-developer.md`, `demolition.md`, `network-rules.md`, `disasters.md`, `QUESTIONS.md`; **no `engine/` touched**, no game runs). **ABI rows NOT applied** — 4 rows wait in `build-runs/pending-abi/re-answers31-rows.tsv` (12-column `interfaces.tsv` layout): 2 `cISC4City` renames (slots 93/94 are `CellCountX`/`CellCountZ`, not `GetLightingManager`/`GetDrawContext`) and 2 `cISC4LotManager` names (slot 5 `GetLot(cS3DVector3 const&)`, slot 6 `GetLot(int32,int32,bool)`), plus a `game_class` misattribution on the lot-manager rows. | Merge into main with `git merge --no-ff` (docs only; nothing to gate or regress), then apply `build-runs/pending-abi/re-answers31-rows.tsv` to `docs/abi/interfaces.tsv`. The four `enable =` blockers are now unblocked on paper and each lane has a concrete change (below). NR9-NR13, TR24-TR27 and PO2-PO6/PO1b are **not** answered here and need their own lanes; `prop-occupant.md` is still only on branch `re-answers27`. |

## The nine questions, with the change each implementer must make

### A. Blocking `enable =` replacements

**1. GLUE21-1 — `lot-configuration.md` §1.1 — PARTLY.** `cSC4LotConfiguration` really is not GZCOM-creatable
(CLID `0xA99062B4` exists only for serialization refs), **but there is a named route to the
allocate-and-construct pair**: `FUN_00489910` (`0x00489910`) = `FUN_005E55E0(0xC0)` allocate → `FUN_004797A0`
construct → AddRef → `Load(key, detail)` at slot 31 (`+0x7C`) → `FUN_00648F00` insert, releasing on failure. The
public routes are manager slots 5 (`0x00489A60`), 6 (`0x0048A690`) and 22 (`0x0048BCF0`), and the **manager is
GZCOM-creatable** (CLSID `0x299062C0`). **Change:** `enable = lotcfg` must replace or drive the *manager*, not
probe GZCOM for the configuration class. Probing GZCOM is the wrong lookup; that is what blocks the feature.

**2. A4 (automata) — DIFFERS — top priority, found.** Fault at `SimCity 4.exe+0x0045BF1B` is
`MOV EDX, dword ptr [ECX]` (`8b 11`) with `ECX = 0`, a null vtable fetch, followed by `CALL [EDX+0x0C]`. The
pointer is loaded at `0x45BF15` from **`cSC4City + 0x1E4`**, which is the automata **script system** (CLSID
`0x4A1B664E`, IID `0x8A1B6647`; `decomp/cSC4City.c:2144`). The city creates it with
`CreateObject(0x4A1B664E, 0x8A1B6647, &field)`. **The third cause is that `QueryInterface` answers the CLSID but
not the IID**, so the out parameter is left null (`*ppvObj = nullptr` runs first) and `FUN_0045bd20` calls
`Init` with no null check. All six classes have the same gap. **Change:** accept the **interface IID** and
return the interface sub-object — `0x8A1B6647` for the script system, `0xCA14C9FC` for the controller manager,
`0x098F9BF4` for all four managers. Keep the CLSID branch if you like, but it is not a substitute.

**3. glue22 question 9 (effects) — DIFFERS.** The original `cSC4EffectsManager` is **six** sub-objects, not one.
Vptrs at `+0x00` (`0x00A9F264`), `+0x04` (`0x00A9F250`, draw pass + `Pick`), **`+0x08` (`0x00A9F240`,
`cIGZMessageTarget2` / `DoMessage` `0x00594120`)**, `+0x0C`, `+0x10`, `+0x94` (`0x00A9F198`, collection).
Destructor `0x0058FA60` stores all six; the Mac `__ZThnN_` adjustments (`+4, +8, +12, +16, +148`) agree.
`QueryInterface` (`0x0058D9E0`) returns `this + 8` for IID `0x452294AA` and `this + 0x94` for `0xC97CF5CE`.
**Change:** size the object for all six and return the offset sub-objects. The `+4` draw-pass experiment had the
right offset for the draw pass but the dispatch that faults is on **`+0x08`**.

**4. GLUE21-2 (propmgr) — DIFFERS.** `cSC4PropManager::CreateProp` (`0x004A3040`) never returns a null. It
validates the type (slot 8), tries `CreateObject(cls, 0xE9793A65, &out)`, and on failure **allocates and
constructs the occupant itself** (`0x94` / `0xA0` / `0x68` by class family), returning the **`+4`** sub-object.
**Change:** implement that fallback. Failing the call is not the original's behaviour and does not stop the
`+0x7C` fault by itself, because the caller dereferences the out parameter without a null check.

### B. Spec contradictions

**5. DQ2 (demolition §2) — MATCHES the spec; `interfaces.tsv` is wrong.** Slots 93/94 are `CellCountX`/`CellCountZ`
(`0x007C1910` `mov eax,[ecx+0x118]`, `0x004E82A0` `mov eax,[ecx+0x11c]`). `cISC4City.h:218-219` declares them
there; `GetCitySizeType` (slot 87) discriminates `+0x118` against `0x40`/`0x80`. The in-game 128x128 is right.
**Change:** none for demolition; the two `interfaces.tsv` rows are renamed in the held ABI rows.

**6. DQ4 (demolition §4.4 step 4) — DIFFERS.** The cell overload is **slot 6** (`0x0072B410`, `ret 0x0C`,
`GetLot(int32, int32, bool)`); slot 5 (`0x0072A1C0`, pop 4) is `GetLot(cS3DVector3 const&)`. The SDK
`cISC4LotManager.h:42-43` has them the other way round; `crime.md` already records this. **Change:** call slot 6
with the third `bool`; use `false` for a cell walk (the flag is a containment re-check against the lot's bounds).

**7. A1 (automata §14 test 29) — DIFFERS from the test as written.** The stream value is authoritative:
`tAutomataState::Read` (`0x00505E50`) reads the `u32` straight into `state+0x0C` (`0x505E99`-`0x505E9F`) and
never derives it, and `Write` emits it. **Change:** keep the corpus behaviour (store as read); recompute `+0x0C`
only after a stack *mutation*. Test 29 and quirk 10 are corrected in place.

**8. NR8 (network-rules §6.5 step 2.4) — DIFFERS.** A skipped side **keeps** its flow code, on all three skip
reasons. The propagation only reads the code field and writes two places; the sole zeroing is the conflict sweep
(`decomp/cSC4NetworkTool.c:11343`). Test 26's X east drop is that sweep: edge word `0x00000202` maps sides 0 and
2 both to partner side 2 with contradictory flows. **Change:** drop the `[choice]` in `solve.rs`
(`resolve_one_way_conflicts`) that clears a skipped side's code; let step 3's sweep do the clearing.

**9. DS38 leftover (disasters §5.4) — exit 4.** The log's own values rule out exits 1, 2, 3 and 5
(`paused=1 emergency=1` defeats exit 1, `type=278128A0` clears exit 2, `flammability=40` clears exit 3,
`burning=0` clears exit 5). `coverage=1` is exactly exit 4's input. **Change:** reproduce the average-coverage
test (occupant slot 11 rect, intersect with the city, `GetAverageValueInCellRect` `0x0045DE50`, the D9 curve at
instance `+0xA4`, the 12582912 rounding at `0x00A9E434`) and refuse when it is `> 0` and the call is not
forced. That also removes the one-step lot-condition lag the same run logged.

### C. Not settled

**NR9-NR13** (cores22), **TR24-TR27** (glue18) and **PO2/PO3/PO5/PO6/PO1b** (cores21) are **not** answered —
each needs its own pass over `network-rules.md` §10, `network-bridges.md` §7-§8, `terrain.md` §15.1/§10.2 and
`prop-occupant.md` §10. Probes named in `QUESTIONS.md` under "Not settled by this lane".

Two further questions could not be answered at all because their text was never committed: **glue19 A2/A3** and
**cores21 "Questions 1-12"**. Those lanes' `docs/lanes/*.md` were trimmed to their merge-train rows and the
numbered lists were not written to any tracked file. They have to be re-filed.

## Notes

- Every claim above carries an exe address. Claims marked *[inferred]* in the specs (effects §18.3's exact
  dispatch site, the `+0x0C`/`+0x10` interface identities) are flagged as such and each names the runtime probe
  that would settle it.
- Where two reports disagreed (DQ4's slot 5 vs 6, DQ2's slot names), both readings are written with their
  addresses and the conflict is called out rather than silently resolved.
- Disk hygiene performed during this lane: the 1.7 TB `C:` volume hit 100% full and writes were failing.
  Reclaimed `engine/target` and `engine/build-x86` from four **merged, clean, 0-commits-ahead** worktrees
  (`cores12b`, `cores12c`, `cores12a`, `devperf`) — about 55 GB of regenerable build output, per the documented
  merge-step policy. No source, branch or commit was touched. Other lanes' build caches remain; they are the
  bulk of the 650 GB still under `.claude/worktrees`.
