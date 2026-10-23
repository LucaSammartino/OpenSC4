| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-font-renderer4: renderer identity and bounded behavior | Codex, `re-font-renderer4` (detached reserved worktree) | ready: bounded docs contract; broader family partial | Coordinator reviews/lands docs; retain FR1–FR3 and unaudited inherited owner bytes. |

## Report

Authorized ROUND2/3, 2026-10-23, resumed own clean `ec6863c9`. This round publishes a substantive
contract in [font-renderer.md](../specs/font-renderer.md), linked from drawing-fonts sections 1 and 9.
No automatic continuation, additional research round or owner/default promotion is requested.

### Outcome and source proof

- **Identity resolved.** Font director instructions `00895FB2–00895FBE` register class **08CDA30B**
  with `00895F65`, whose 8-byte allocation reaches constructor `00897722` and **00AC55B4**.
  QI `0089620D` accepts **A8BC2A75 and 1**. Font Init independently uses the same constructor
  and stores the same pointer view at font +8C (`0089962A–00899654`).
- **Larger holder distinguished.** Drawing director `00825BAB–00825BB8` registers getter
  `00829910` (**8B3255F9**) with factory `0082A2B0` (450 hex-byte allocation), reaching
  `00829F70` / **00AC14BC**. That holder requests the small class/interface into **+43C**.
  Its QI shares the interface, while its Release uses its own deleting-destructor slot.
  Focused unlock/draw branches load +43C and delegate through +18/+1C; the rest of its
  canvas/cache behavior is excluded. The old drawing-fonts vtable identity is corrected on this proof.
- **Behavior published.** Eight typed slots (two semantic names unknown), QI output/refcount order,
  signed Release/free rule, target lock/unlock forwarding, seven-argument batch dispatch, control
  character gate, exact 24 hex-byte record, signed clipping and source offsets, empty/unsupported
  depth effects, target query order, palette/foreground packing and per-pixel modes.
- **Numeric proof.** Every numeric rule is grounded in instruction bytes: `00897246`,
  `00896E63`, `00896053`, `008960C1`, `0089612A`, `0089618D`. The table constant is the PE
  float **3D000000 = 1/32** at A8ED98. The depth-24 path writes dwords; the 565 helper drops
  green bit 5; greatest 16-bit blend weight is 31/32. The packed dword XOR/mask formula is
  specified literally, without assuming conventional alpha behavior.
- **Concrete target view.** Factory/constructor `008298F0` / `008272E0` and raw vtable
  **00AC1400** establish the target fields and callback meanings independently of folded names.
  This checks pointer interpretation; it does not expand the contract to the buffer owner.
- **Literal traces.** Section 6 covers count-zero and control-character no-ops, simultaneous clip
  and target bounds, edge-touch/reversed/empty rectangles, source dimensions not bounding writes,
  depth/pitch behavior, coverage thresholds, raw palette indices, 16/32-bit packing, lifetime,
  lock failure, caller pen advance, and callback-result order. The worksheet performs integer
  arithmetic only; neither original instructions nor engine behavior were executed.

### Accounting and explicit limits

| Category | Supported / partial / unknown |
|---|---|
| Small-renderer path bodies with bounded behavior rules | **12 / 1,666 export bytes**, enumerated individually in spec section 1 |
| Folded/shared bodies | **2 / 12 bytes**: AddRef 7 and true-return 5; true body used in two slots, counted once |
| Slots | **8**, stack widths/results proved; **2** ignored-argument semantic names/types unresolved (FR1) |
| Foreign identity/caller bodies enumerated | **8**; six used for identity/lifetime distinction, two holder forwarding branches only |
| Partial foreign algorithms | Holder unlock/draw forwarding branches traced; their other paths remain foreign, uncredited |
| Inherited labelled 4,444-byte owner correspondence / remaining bytes | **Unknown**, not reassigned or credited (FR3) |
| Source/palette producer storage guarantees | **Unknown** beyond valid backing-storage preconditions (FR2) |
| Native, runtime, replacement, default, whole-owner/family credit | **None** |

The spec requires valid addressed storage and ordinary positive pitches for its literal traces.
Invalid-pointer faults, overflow geometry, negative/zero pitch, concurrency and foreign target behavior
have no safe-equivalence promise. FreeType, font layout/cache/palette production and span kernels stay
foreign. The previously completed font owners and capped FN gaps were not reopened.

### Questions and ABI proposal

Filed **FR1–FR3** in [questions/font-renderer.md](../specs/questions/font-renderer.md): no-op slot
semantics, producer backing domain, inherited owner accounting. No existing question was answered or
moved. Regenerated the question index: exactly three new open entries, next free FR4.

Eight proposed vtable rows are in ignored **`build-runs/abi-rows-re-font-renderer4.tsv`**, in the
11-column vtables TSV layout. They are a coordinator review proposal only. No tracked ABI table,
version, coverage map or generated coverage was changed; no C ABI export changed.

### Validation, environment and milestones

- **PASS:** `python tools/questions.py index`, then `python tools/questions.py check`.
- **PASS:** local documentation consistency check: 14 unique body addresses, sums 1,666+12,
  eight slot rows, local spec/question file links, eight 11-column ABI proposal rows.
- **PASS:** repeated original export `func` lookup agrees with all 14 inventory sizes.
- **PASS:** `git diff --check`.
- Cargo, clippy, ctest, engine builds/tests, game runs and shadow comparison: **n/a**, not run.
  No engine reads, tools edits, tracked ABI edits, native execution, subagents or public actions.

FIRST1 made two live REA calls for 00897722, both `target_unavailable`, category unavailable,
retryable false: **No app is open**. This round did not retry or claim healthy live service:
**live REA health UNKNOWN**. The reopened task expressly authorizes primary static proof without
live preflight. Export/data/instruction queries supplied the complete bounded rules above.
The q.py Bash wrapper still fails `E_ACCESSDENIED`; bounded PowerShell was used. `pefile` is absent;
a local stdlib PE-data reader supplied bounded original dwords instead. Ignored evidence/helpers are
under this worktree's build-runs. No external files or RE output were committed.

Milestones: **f246dbb3** substantive identity + algorithm contract; **64432c22** literal traces,
concrete target view and questions. Each was followed by guard status within budget. Git printed a
`packed-refs.lock` permission denial during each commit but completed both detached-HEAD commits;
no shared ref/config change or maintenance workaround was attempted.

Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-font-renderer4-r2-manifest.json`.
Supervisor publishes the detached committed tip to the reserved lane branch. Normal docs queue:
ready for the bounded result, no game labels and ABI false. Remaining questions do not imply
completion of the larger holder or authorize another round.
