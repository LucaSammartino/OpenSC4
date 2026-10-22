| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-overlays-final4: final native overlay contracts | Codex / `re-overlays-final4` (RE docs, detached worktree) | ready bounded docs: a300262d, bfcdc36b, a7d5e49a, a8e0b923; OV2/OV3 answered; 93-entry checklist; docs checks pass; ABI unchanged; game n/a | 1. Use proved owner contracts. 2. Keep parked OV1/OV7/OV8 residuals and scaffolding limits; no fourth RE round requested. |

## Report (2026-10-22, sole final round)

This is ready for review/merge as a bounded specification improvement, not as a complete renderer or native
equivalence verdict. This was the authorized final round; the exact residuals below are parked, with no
fourth round or restart requested. Only docs/specs and this lane report changed. No engine files were read;
no engine, tools, maps, ABI tables, exports, game files or external material were edited or committed.

### Published native contracts

- **a300262d:** §3.6 proves the rectangle helper and city add. City factory slot 5 makes a direct record,
  flags `0x13`, city bounds, and the unusual matrix whose texture coefficient is M[1,1]=F(2/nx), independent
  of nz/cell width. §5 proves the entire `00736100` handler: render-settings refresh, three stores and true
  return; no re-placement/index rebuild/redisplay. SetRenderProperties provides independent sender evidence.
- **bfcdc36b:** §3.7 specifies all six model-add arguments, cached-zero early rejection, separately read live
  quality ≥5 branch, lighting calls/return boundaries, local fallback box conversion with binary32 epsilon
  0.07000000029802322 and height factor 1/16, direct flags-9 record, pair-bound scan and all sixteen secondary
  transform entries with native x87/store boundaries. Static insertion/redisplay precedes secondary stores.
  Lighting production remains explicitly outside owner; no guessed projection or TR33–TR37 contract is used.
- **bfcdc36b:** §1.2 distinguishes Shutdown and destruction. Shutdown releases overlay payloads and clears
  overlay/handle live ranges/free queues but retains group/index storage and shadow binding. Destruction
  frees those resources without unsubscribing or invoking Shutdown. Ordinary Remove still retains payloads.
- **a7d5e49a/a8e0b923:** §§4.7/4.8 prove unguarded singular inverse control flow and binary32 reciprocal
  storage; normal/debug shadow stage/binding/composition choices; conditional location-strength midpoint,
  strict ordered-below-0.05 rejection and counter skip; exact water terrain-slot-53 comparison and routing;
  rough-pass differences; static +0x20 index-selection switch. Corrected the previous mask-only count claim
  and removed the inferred direct terrain-height query.

### Measured disposition and independent discriminators

- New companion `docs/specs/overlay-manager-native-inventory.md` contains **93 unique rows**, reconciled
  against **97 exported starts**: merge three fragments, exclude unrelated folded body/shared packed-state
  helper, add missing slot-20 start. Dispositions: **11 N** newly traced bodies, **53 P** retained preceding
  contracts, **4 R** partially specified render bodies, **25 S** storage/scaffolding entries without a full
  standalone exceptional/allocation contract. This is not a claim that 93 bodies are completely proved.
- Paper vectors cover nx-only city matrix/direct ID; model fallback `(1,0,5,4)`; identity lighting projection
  mapping pair extrema into `[1/4,3/4]`; cached versus live quality; unrelated versus settings message;
  retained shutdown resources; local-strength rejection versus submission; clipped midpoint `(32,48)`;
  full-water terrain returns 8 versus 9; debug setting 4 versus 5. Lighting/terrain outputs are stated
  conditional service inputs, not implementation-derived evidence or a replacement host overlay API.
- `questions.py index` and `questions.py check`: PASS. `git diff --check`: PASS. Ignored check-docs.py
  verifies exact checklist set/count and question disposition, and verifies the previous answered archive
  survives byte-for-byte as a prefix, preserving OV4/OV5/OV6/OV9. Cargo/clippy/ctest: n/a, docs-only.
- Game/shadow runs: n/a; no requested labels, no execution or exactness verdict. ABI false; no proposed
  rows, ABI-file edits or ABI version change.

### Questions and exact parked residuals

- **OV2/OV3 answered DIFFERS**, moved to `docs/specs/questions/answered/overlay-manager.md` after writing
  their rules into §§3.5–3.7/5; generated question index updated.
- **OV1 remains open, narrowed to outside-owner production:** lighting `007D8C80` source-geometry and
  descriptor layout, primary projection/pair output; `007D6CC0` high-quality terrain bounds/precision.
  The overlay owner arguments, construction, fallback and postprocessing are now published.
- **OV7 remains open:** inline inverse cofactor/spill equations `00738ADA–00739020` and all sixteen
  accumulation schedules in decal/full/rough shadow products. Composition direction/stages, reciprocal
  store and no singular guard are proved; a generic library inverse is not a bit-exact contract.
- **OV8 remains open:** terrain consumer bodies `007541C0`, `007543C0`, `007545C0`, `00754910`,
  `00754C60`, `00754F10`, `00755560`, `00755700`; terrain slot-53 internals; lighting slot-15
  `007D6BB0` strength interpolation/terrain inputs. Owner routes are specified without blanket terrain or
  renderer claims. No complete geometry/GPU success is inferred.
- Open entries live in `docs/specs/questions/overlay-manager.md`. Inventory S rows explicitly preserve
  additional scaffolding/exceptional gaps; they must not be counted as complete implemented coverage.

### Evidence and operational notes

Evidence is primary native export instructions with bytes, bounded existing decompilation and image constants
from the main checkout, queried with re_query.py first. No REA query was needed or attempted this round.
q.py failed to launch Bash (`E_ACCESSDENIED`), so bounded PowerShell calls were used; qgrep also needed UTF-8
stdout for spec Unicode. Commits succeeded detached despite the known nonfatal packed-refs.lock permission
diagnostic. The supervisor publishes the reserved branch; no refs/config/hooks were changed.

Ignored worktree evidence: `build-runs/native-*.txt`, `cache-native.py`, `inspect-native.py`, `inventory.tsv`,
`check-docs.py`, and UTF-8 commit messages. These are not staged or implementation inputs.
Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-overlays-final4-r1-manifest.json`.
