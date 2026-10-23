| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-property-setter-helpers4 (original setter helper contracts, round 1 of 3) | Codex / `re-property-setter-helpers4` (RE spec, detached delivery) | ready: eleven helper-owned normal contracts; original 11/11 bodies, 1,585 bytes matched; proposed traces only; ABI unchanged; gates/game n/a; questions none | - |

## Report

Completed 2026-10-23. [Property setter helpers](../specs/property-setter-helpers.md) is the
first dedicated original function-family study, round 1 of at most 3. It specifies eleven
helper-owned normal returned-call contracts and explicitly leaves wider entry behavior
Partial. No all-method, whole-class, native/default-profile or blanket byte credit follows.

### Prior-family audit and scope

Before substantive research, searched public specs/lane reports by family and addresses.
RG16/residential-education-health already covers bool/u32/f32 scalar lookup/create/install;
PR9/protection covers scalar versus copied u32 array and inherited-property mutation.
Plumbing names typed setters; occupant-manager, lot-records and lot-developer publish
consumers. These remain valid partial inputs, not a claim that no setter was ever studied.
No prior dedicated complete same-function-family contract was found. EX52f's different
holder-slot-14/16 wrappers do not establish these helpers' return channels.

The copied `build-runs/evidence-run4/coordinator-property-setter-helpers-first-host-proof.json`
records the coordinator's FULL formats RE GO (exit 0). No isolated global preflight was
rerun. Old converter/Variant/native research profiles, caps and helper data are unchanged.

### Findings and deliverables

- New spec: exact entry/argument/stack widths, acquisition/store/call order, raw return widths,
  per-function Supported/Partial/Foreign/Unknown inventory, and proposed trace ledger.
- Only other public edit: narrow data-formats section 7.5 link. No ABI rows/exports changed.
- Lookup tests a full pointer, not AL or a status/output pair. A returned inherited property
  is updated in place. Missing properties are taken from the pool, constructed and installed
  before the value call. Install return bits are ignored; no relookup follows.
- No helper AddRef/Release, rollback or failure cleanup. Final typed-call EAX survives the
  epilogue; this is void-like operationally, not a Boolean success API or Release result.
- Original scalar primitive residues distinguish AL bool/u8, EAX s32/u32, high-word EAX
  s64, and f32 adapter residue. `005FE580` is an unchanged string-interface pointer input.
- Vector inputs are copied u8/u32/f32/s64 despite Ref* names. Helpers read begin then end
  after GetVariant; 32-bit byte differences use shifts 0/2/2/3. Begin/count pass unchanged,
  including empty inputs. Actual copy/storage/allocation algorithms remain foreign.
- Ledger is **PROPOSED/UNEXECUTED**: 12 scalar/pointer/branch rows; 12 literal array
  type/count cases expanded across existing/new paths (24 traces); empty begin-pointer
  distinguishers; conditional refcount fixtures; failure prefixes. No primitive supplies
  a precomputed helper outcome. No native comparison or portable implementation oracle.

### Evidence and checks

Primary `re_query.py func/callers/asm --bytes` queries preceded decomp navigation. Main PE
and export identity match SHA-256
`3bc5c7fe807fe5aa24a3abb06e40f13486670bc1150612c09baf4ecedafe1a4b`.
Read-only PE mapping verifies x86 `0x014C`, base `0x00400000`, eleven distinct contiguous
non-thunk bodies and all 1,585 exported instruction bytes. Their normalized common prefix
is 115 bytes each: 1,265 repeated inline bytes plus 320 suffix bytes. This is shared
logic attribution, not eleven independent acquisition algorithms or whole-body qualification.
PE vtable words verify final dispatch targets. Ignored analytic evidence:
`build-runs/audit-setter-evidence.py`, `build-runs/setter-native-byte-audit.json`.

Independent literal arithmetic checked the signed s32 example and prefix/suffix totals.
`git diff --check` passed. Gates/game/shadow n/a: no tests, builds, cargo, clippy, ctest,
game/native execution or process/RPC actions. No engine/Rust/C API/SDK implementation,
foreign-worktree or portable-test oracle was read. REA was not needed or queried; no
NoApp/provider error is claimed.

Actual errors: q.py's Bash wrapper failed once with
`Bash/Service/CreateInstance/E_ACCESSDENIED`; direct PowerShell was used thereafter.
An initial analytic prefix assertion compared absolute jump targets; relative normalization
corrected it and passed 11/11. This was not a failed native comparison. Commits succeeded
at detached HEAD despite `packed-refs.lock: Permission denied`; no shared ref/config or
Git maintenance action was attempted. A report patch context mismatch was corrected before commit.

### Milestones, guard and delivery

- `9986e1eb`: overlap audit and common acquisition/dispatch contract.
- `3fe3a1ad`: primitive boundaries, proposed ledger, qualification and narrow link.
- Final report committed separately; own queue records actual delivery HEAD.
- Manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-property-setter-helpers4-r1-manifest.json`;
  own path/RE role/round 1, 150000 context and 85-call cap. Post-second-milestone guard:
  85,172 / 150,000, within budget. No STOP/OVER returned.
- No questions filed/moved/closed; no generated question index, coverage, ABI, SDK, tools,
  names/maps or coordinator state edits. Queue kind docs, abi false, game_runs empty, ready.
- No further work required for this bounded publication. Null/allocation/alias/reentrant/
  nonreturning domains remain unsupported, with no automatic next research phase.
