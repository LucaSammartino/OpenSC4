| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-contract4: native overlay owner contracts | Codex / `re-overlays-contract4` (RE docs) | ready: bounded native contracts; OV4/OV5/OV6/OV9 answered; docs checks pass; ABI unchanged; game n/a | 1. Implement proved owner subsets. 2. Retain OV1/OV2/OV3/OV7/OV8 gaps listed below. |

## Report (2026-10-22, round 2)

This queue is ready for the bounded docs change. It does not claim a completed overlay renderer or native/game
equivalence for unproved owner bodies. No engine files were read or edited; no tools, ABI tables or exports changed.

### Committed native contracts

- Preserved round-1 proofs: 4912da47 placement/alpha; 4d11b61a Init/getters/FIFO/slot21; 1ebda27a rotated
  rounding/defaults/clipping/index rules. OV4/OV5/OV6 remain answered with their existing evidence.
- 96b4dc27: §3.5 slots 9/10 accept a resource key/constructed three-word key, not a binding; fifth argument
  tests only its byte, optionally flips primary V, then derives a separately stored secondary matrix with
  exact native store boundaries. §4.4 proves independent last-nonempty-pass counters, all reset/increment
  sites, masks, empty/disabled/snapshot preservation, and eligible submissions counting even without geometry.
- d558bfa2: OV9 answered in §3.1 and moved to answered/overlay-manager.md. Eight rectangles outline a ring
  with an omitted centre; outer/90% inner chord samples, four split rows, part order, clipping, shared matrix,
  static redisplay order and allocated empty parts are specified. No guessed filled-disc partition survives.
- ad51cfe5: §§4.5–4.7 prove visible vertex endpoints, native encounter order, 16-bit stamp wrap without reset,
  DrawRect/DrawShadowRect dispatch and +1 vertex maxima, exact shadow alpha minimum/reference, bounded exit
  state actions, post-reset binary32 context input and row-vector C×M composition. Full inverse precision is
  explicitly deferred rather than equated with a generic library inverse.

### Discriminators and validation

- Published paper conformance vectors: width-256 outline omits cell (8,8); cell-space height 20 splits at 3/5
  because shipped binary32 coefficients exceed exact .2/.4; shadow edge V distinguishes both matrices/flip;
  skipped draws preserve counters; visibility lists yield [9,2,7], not sorted IDs; stamp zero is not special;
  camera translation yields U=1.5 instead of 16.5 under reversed composition.
- Evidence comes from paginated export instructions with bytes and PE constants, plus tightly bounded existing
  same-owner decompilation. No decompiled code/assembly/game binary is committed. Previous REA requests failed
  target_unavailable; this round used export/raw-byte fallback throughout, without adding tools or sessions.
- questions.py index/check and final git diff --check passed before publication. The intermediate
  OV9 milestone had one blank EOF line; the following milestone removed it and passed diff --check.
- cargo/clippy/ctest and game/shadow runs: n/a, docs-only; no requested run labels and no exactness verdict.
- ABI false; no proposed rows or abi-rows file required. Git commits succeeded detached, with the known
  nonfatal packed-refs.lock permission diagnostic; supervisor publishes the reserved lane branch.

### Exact remaining contracts/questions

1. OV1/OV2: AddModelShadow slot 11 (`0x0073C0D0–0x0073C838`) arguments, projected footprint, native
   placement/bounds and matrices. ComputeCellRect (`0x007354E0`) remains a distinct helper, not ordinary
   placement's bounds. Slot 9/10 proofs must not stand in for these bodies.
2. OV2: city add slot 8 (`0x0073C9A0`, helper `0x0073B8A0`) and differences from ordinary/shadow adds.
   Slot 20 and slots 9/10 portions are already specified; the open entry is narrowed accordingly.
3. OV7: inline Draw inverse (`0x00738ADA–0x00739148`) complete cofactor/spill/singular behavior; all sixteen
   matrix-product accumulation orders, beyond the proven examples, and shadow camera composition branches.
4. OV8: full shadow second-stage/debug branch and terrain-height coordinate/arithmetic in `0x00736BF0`,
   water adjustment/short-rectangle route near `0x007377BE–0x00737828`; exact terrain vertex/index consumers
   `0x007541C0`, `0x007543C0`, `0x007545C0`, `0x00754910`, `0x00754C60`, `0x00754F10`,
   `0x00755560`, `0x00755700`. These bodies remain necessary before a complete successful renderer contract.
5. OV3: message `0xC9F775BB` meaning/dispatch/re-placement remains unproved.

Open entries are in docs/specs/questions/overlay-manager.md; OV9 is in its answered counterpart. Index regenerated.
The next implementation work can use proved subsets without accepting host-computed placement/visibility/results.
Any further RE round is a coordinator decision; no restart or additional round is requested by this queue.

Scratch evidence (ignored, never staged): build-runs/asm-*.txt (paginated query caches), cache-overlay-asm.py,
native-data.py and commit messages. The first place-native.txt is only a first-page extract, not a whole body.
Active manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-overlays-contract4-r2-manifest.json.
