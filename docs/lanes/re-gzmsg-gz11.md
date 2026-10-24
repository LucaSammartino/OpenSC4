| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-gzmsg-gz11 (GZ11/GZ12 observation and acquisition) | Codex / `re-gzmsg-gz11` | ready: GZ11/GZ12 answered; doccheck and questions check PASS | Land spec answers; retain default-off/unavailable native diagnostic; full-state shadow requires a separate capture facility. |

## Report

Extended [framework-gz-services](../specs/framework-gz-services.md) §§3.1/3.2 and §5.3,
with eight independent instruction-derived vectors GZ-V15–V22. Archived GZ11/GZ12 in
[answered questions](../specs/questions/answered/framework-gz-services.md) and regenerated
the question index. This spec now has three open questions (GZ8–GZ10) and nine answered.

GZ11: the recovered public interfaces provide no complete read-only ID/content/registration
enumerator, snapshot, or complete mutation-observer hook. Added the previously omitted manager
slot-9 unsigned ID-entry count (`0x009DB663`); it is unlocked and supplies no keys. Counted
lookups preserve returned queue lifetime, not ID bindings or an atomic snapshot. The vectors
cover alias cardinality, hidden IDs, removal with a held queue, equal bounded samples despite
mutations, ordinary-post count isolation, and indistinguishable queue endpoints.

GZ12: `0x64FA8434` / `0x652294BC` is the correct existing-service lookup. The director registers
class factory `0x452294DF`, but normal application service setup does not install the manager.
Missing lookup returns false without touching output; success returns one owning reference.
Factory construction makes a new empty manager and is not an acquisition fallback. There is no
guaranteed city/month availability phase. The spec distinguishes this resolved acquisition
contract from unsupported native full-state observation: neither answer supplies shadow credit.

Evidence: main-checkout `tools/re_query.py` export queries and `decomp/` reads; numeric constants,
argument adjustments, return behaviour and count updates verified against instruction bytes.
Constructor-installed vtables were checked directly against the working executable. Local
read-only evidence helper: `build-runs/inspect-gz11.py`. REA was not called: the task explicitly
states that its session has no binary open, and the export supplied the needed instructions.
No decompiled output or assembly listings were copied into specs or committed.

Proposed ABI row: `build-runs/abi-rows-re-gzmsg-gz11.tsv`, in `docs/abi/interfaces.tsv` format,
for the manager slot-9 count getter. No `docs/abi/` edits or C ABI export changes.

Validation: `python tools/doccheck.py check` PASS; `python tools/questions.py check` PASS;
`git diff --check` PASS after removing a trailing blank line. Cargo, clippy and ctest are n/a
for this documentation-only lane. No new game run. Prior `gzmsg-shadow8-r1-2032` remains
0/73 manager acquisitions (load plus 72 monthly samples), clean shutdown with zero foreign
exceptions/reference warnings; that is inherited evidence, not a new measurement or exact shadow.

No new questions filed. GZ8–GZ10 remain untouched. Next: land these answers; engine/coordinator
may use controlled fixtures and the specified negative acquisition result, while retaining
default-off, unavailable=mismatch and no manager verification credit in normal city play.
Any complete live-state shadow requires a separately established capture/replay facility.

## Handoff
- Goal: answer GZ11/GZ12 in framework-gz-services; spec text only.
- Done: answers/vectors/archive/index `511bacde6`; checks/whitespace milestone `e1038bf4a`.
- Next: coordinator lands final report tip from merge-queue; no further RE needed for GZ11/GZ12.
- Native acquisition and complete-state shadow remain unavailable as described in Report; no new game requests.
- ABI proposal: build-runs/abi-rows-re-gzmsg-gz11.tsv; no ABI export changes.
- Worktree HEAD is intentionally detached; commit normally without changing refs.
- Manifest: C:/Users/tlori/FreeCity4/build-runs/run4-re-gzmsg-gz11-r1-manifest.json.
