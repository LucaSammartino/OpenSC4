| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| core-camera-contractfix4 (CA3–CA6 core corrections) | `core-camera-contractfix4` / (engine lane, detached worktree) | ready: code 1360e09b, b59b4e80; cargo 4650/0, clippy 0 diagnostics, ctest 68/68; camera 48/48; fmt 5 files PASS; CA7 filed; ABI unchanged; game n/a; no complete-owner/shadow/replacement credit | 1. Train regenerates question index. 2. Resolve CA3/CA7 and per-class residuals within logical cap 3. 3. Native binding, independent shadow and exercised replacement before promotion. |

## Report

Corrected the landed pure core from public camera §§1–2, 5–7 and the independent
re-camera-contracts4 answers. The parent's 39 tests were reconciled with those
contracts; they were not behavioral authority. Source scope is the five named
camera Rust files and their README. No ABI/plugin/CMake/default/maps changes;
no game request was needed or submitted.

### Changes

- CA3 resolved portions: box −1…+1, remembered up zero, projection state 1,
  unchanged-box invalidation and unchecked degenerate look-at normalization.
  Added the specified reset-subview body. Native perspective entry remains blocked.
- CA4: direction/right zero, up Y, signed viewport, loaded-before-version Read,
  immediate partial/raw/NaN mutations with all primitive reads attempted and no
  camera/city work. Loaded Init copies current pair into new pair and retains
  the saved target; camera-less Init is idempotent. Detach/adapter shutdown retain
  Init. Attach acquires before old-camera release; destruction does not detach
  the native reference ledger. Invalid corner writes y only; bounded malformed
  zoom lookup is explicitly safety policy, not native out-of-array behavior.
- CA5: integer-pair magnification reset precedes unchanged-pair rejection;
  rotation preserves magnification, invalid rotation returns true, float equality
  precedes zoom truncation. Camera-less changed commands store state then return
  false. Tweak replaces the pair, retains nonempty bounds and sends no messages;
  resize only changes camera viewport. Yaw base corrected to −π/8.
- Messages synchronously call an optional Send+Sync server, with mutable control
  access for reentry and live borrows of the pair fields. CameraUpdated refers
  to the control; each post-change triple reuses a message object. No camera lock
  spans callbacks. Copied events are diagnostic snapshots only.
- CA6: fixed directed half-planes, previous→current edges, strict crossing and
  first-edge ties, double length threshold followed by float length normalization,
  float fraction thresholds/low-fraction zeroing, original-direction history stop,
  and original-start exhaustion. Removed winding repair, outside-start projection
  and sliding-edge skipping. Target clamp starts at the rotation corner. Ground
  failure still updates/sends: angle scroll retains its plane point and velocity;
  target clamp retains old target and velocity.

### Verification

- **1360e09b**: correction bodies, tests and native body audit, committed within
  the requested call 20–30 window. **b59b4e80**: test qualification, CA7 and final
  terrain/velocity/reset checks. Final gate tested b59b4e80; subsequent edits
  are this report and README qualification only.
- Stable quick PASS: core **2712/0**, dependents compiled, native build **4/4**,
  **134.9 s**. Earlier quick overlapped edits and was invalidated; the next
  exposed four inherited yaw/truncation expectations, subsequently corrected.
- One stable final `qbuild all` PASS: workspace cargo **4650/0**, clippy **0
  diagnostics**, native build **4/4** in **95.1 s**, ctest **68/68** in
  **137.4 s**, fc4verify PASS in **234.6 s**.
- Camera **48/48** (nine added independent partial-read/boundary/order/reentry
  tests). Absolute-path qbuild fmt **PASS on all five Rust files**; whitespace
  clean. Environment canonicalize-path warning is not a formatting diff.
- Logs: `build-runs/qbuild-test.log`, `qbuild-clippy.log`,
  `fc4verify-quick-test.log`, `fc4verify-quick-deps.log`, `fc4verify-build.log`,
  `fc4verify-ctest.log`. Task-specific single-final-gate instruction avoided a
  redundant full baseline; the parent's baseline/final gate remains recorded.

### Owner qualification and questions

**Neither cS3DCamera nor cSC4CameraControl earns complete-owner credit.**
The README accounts for every native body group in public §7 separately.
S3D retains eager-cache, explicit-transform-mutation and Rust heap/lifetime
policies; remembered-counter synchronization is not modeled. CA3's native
perspective setter/signature/counter is unidentified: `set_frustum` remains an
implementation extension, never an inferred native slot.

Control still lacks native QI/AddRef/Release/factory binding, the generic
update-kind helper, separate rotate-left/right and zoom-in/out bodies, and
optional view-utilities position-update integration. Rust synchronous messages
are core behavior, not native server-slot/object binding. Mixed float/double
arithmetic is portable, not x87 bit-exact proof. No family/shadow/replacement
credit or default promotion is claimed.

Filed **CA7** in `docs/specs/questions/camera.md` for unspecified pre-init
plane equation/target-plane point/conversion-scale storage. Zero plane/point
and unit scale are provisional, marked in code and uncredited. CA1/CA3 remain
open; CA4–CA6 are answered and corrected. Only the per-spec question file was
edited; train must regenerate the question index.

### Next steps and budget

This manifest used its single round: parent logical round 1 plus this correction
is logical **2/3**, first of at most two answer-correction rounds. No restart,
fourth logical round or phase reset is authorized here. Preserve the worktree
and guard accounting (`build-runs/run4-core-camera-contractfix4-r1-manifest.json`
in the main checkout).

Resolve CA3/CA7 and per-class residuals within the remaining cap if assigned.
Native C ABI/plugin binding, independent same-process shadow and exercised
replacement must precede native shadow/replacement/default promotion.
