| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-lua-scriptserver4: native ScriptServer boundary | Codex / `glue-lua-scriptserver4` (engine, detached) | blocked: LH1/LH2 committed 25e25c43; native insertion and generator ABI unpublished; no ABI or plugin changes; baseline cargo 4641 PASS, clippy 0 diagnostics, build 208/208, ctest 68/68 PASS; game n/a | 1. Publish LH1/LH2 contracts. 2. Reconcile core ID constants. 3. Implement and exercise only this owner after insertion is established. |

## Report

Native ScriptServer replacement is blocked at the required acquisition audit. This
round commits the exact public-contract gaps in `docs/specs/questions/lua-host.md`
(LH1 and LH2), rather than introducing an unverified registration or adapter.
The completed portable owner remains available and passes its independent vectors.

### Acquisition, views and lifetime findings

- `lua-host.md` section 1.2 publishes the VM, thread and generator class registrations.
  It does not publish a ScriptServer class-registration entry. The factory address
  in section 6.3.1 alone does not establish whether the application reaches it through
  GZCOM or direct construction.
- Section 1.4 identifies `cSC4App::AddSystemServices` as installing this owner, and
  section 12 names `cRZSysServPtr<cIGZLuaScriptServer>` as a console caller. Neither
  publishes the caller's service-ID/IID pair, factory request, registration sequence
  or point at which references are cached. Existing director class dispatch is an
  available plugin mechanism; its applicability to this specific owner is unproven.
- Section 6.3.2 supplies the values `0x4BD10133` and `0xFFF0BDC1` as constructor
  identifiers. The SDK declares `cRZBaseSystemService(uint32_t serviceID,
  int32_t servicePriority)` at `include/cRZBaseSystemService.h:55`. The portable core
  instead names these values `CLASS_ID` and `SERVICE_ID`. LH1 asks for explicit
  reconciliation before either is used for native acquisition. No correction is
  guessed from the SDK constructor signature alone.
- The primary interface and service view at +4, the further destruction view at +8,
  QI/AddRef, zero/one Release dispatch, Shutdown and deletion ordering are specified
  in section 6.3. These are usable owner contracts. They do not establish a public
  method to replace a service that is already live or to repair cached references.
- The SDK has the real 25-slot `cIGZLuaScriptServer` declaration, `cIGZLua5` and
  `cIGZLua5Thread`, and framework `AddSystemService`/`GetSystemService` declarations
  (`include/cIGZFrameWork.h:81` and `:96`). A generic service API is not evidence of
  this owner's acquisition IDs or insertion timing.

### Foreign boundary finding

Section 6.3.2 requires a command-system accessor, generator initialization at +0x10,
generator target retrieval at +0x0C, and target cleanup at +0x10. The spec defines
the owner operation order and generator creation pair `(0x2BD13FB4,0x2BD144DF)`.
It does not publish a public acquisition route for the command-system temporary or
complete typed ABI/ownership of these foreign slots. No command-generator or
command-system declaration was found by targeted searches of the published SDK
include tree. LH2 requests those exact endpoints and reference obligations. The
existing abstract Rust `Foreign` operations cannot safely become typed native calls
merely by copying their abstract argument list.

### Validation and limits

- Baseline `qbuild.py --wt <WT> all` was started before implementation decisions.
  Cargo workspace: 4641 passed, zero failed; clippy zero diagnostics; native build
  PASS, 208/208 edges; full ctest 68/68 PASS. qbuild PASS. The baseline noted tree
  changes (only this lane's documentation); a second all gate on committed state
  is the next step, with no edits while it runs.
- Existing independent ScriptServer conformance: 31/31 PASS; integration edge cases:
  27/27 PASS, in that same workspace gate. These include Init failure latch/retry,
  current/last IDs, signed pause/resume, breakpoint operations, ordered/reentrant
  ticks and notification ownership. They validate portable behavior, not native
  integration.
- Logs are local under `build-runs/`: `qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-build.log`, `fc4verify-ctest.log`.
- No native same-process comparison or exercised game request was issued. There is
  no established replacement insertion to exercise. No load/tick/save/reload/shutdown
  callback trace, 2032/ref0 result, shadow exactness, union-plus-candidate result or
  owner replacement credit is claimed. An idle game run would not resolve LH1/LH2.
- No engine/CAPI/plugin changes, new exports, opt-in key, ABI-table edits, ABI-version
  bump, defaults, maps or tools changes. In particular other Lua host/VM/tool owners
  remain outside this lane. The queue is docs-only with state `wip`; native work is
  blocked.
- The questions index requires regeneration by the coordinator. This lane did not
  run the prohibited `tools/` question-index scripts or hand-edit the generated index.

### Commits and next steps

Initial handoff: `6fec4c86`. Exact boundary questions: `25e25c43` (substantive audit
finding committed within the initial audit).

After LH1/LH2 are answered: reconcile the core constants against the published
roles, bind the public foreign VM/generator services, add the CAPI and concrete
multi-view wrapper with correct shared lifetime, and introduce only `luascripts`.
Then compare independently against the original in one process and record genuine
game scripting execution across load, tick, save, reload and shutdown before the
required exercised 2032/ref0 and current literal 39-owner union plus candidate runs.
