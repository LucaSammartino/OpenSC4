| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| re-persist-update-policy4: PR4 service Update return | Codex / `re-persist-update-policy4` (RE docs, detached HEAD) | ready; proof `590ac4f3`, spec `1b1745d0`; PR4 answered; six independent vectors; docs checks PASS; cargo/clippy/ctest/game n/a; ABI unchanged | Coordinator may merge docs; no further persistence-family RE round. |

## Result and scope

Answered only PR4 in [persistence-manager.md](../specs/persistence-manager.md) §9.1–9.2, with independent
vectors in §10.1. The shipped framework dispatcher ignores this service's Update return. Native AL is
the uninitialized prior stack byte OR every callback AL, without normalization. The clean-room policy
explicitly chooses canonical false for empty/all-false and true otherwise, preserving every callback.
This is a documented deviation from the undefined native baseline, not an assertion that native starts
with false or true. Static evidence is bounded to the traced shipped paths; no claim covers arbitrary
external plugins or dynamically obtained interfaces outside those paths.

No owner expansion, additional attributed-body credit, implementation, executable tests, tools or ABI
changes. The existing manager audit remains 109 bodies / 8,437 B. This is the final third logical
persistence-family round; no retry, self-restart, subagents, merge or push occurred.

## Primary evidence

- Update `0x0097356D`: 75 B; no direct callers; one exported data reference, service-table word
  `0x00AD89C8`. Raw executable word agrees with the export. Constructor stores this service vtable at
  main+4 (`0x009744DE`), with service ID `0x056B906E` (`0x009744C3`).
- Accumulator OR `0x0097359C`, return AL `0x009735AF`. Lock helper `0x0048C9F0–0x0048CA08` writes only
  two 32-bit fields at frame offsets -0xC/-0x8, so it does not initialize the accumulator at -1.
  Empty/all-zero returns S; any canonical true forces bit 0, leaving higher bits dependent on S.
- Director class registration `0x0097525A–0x00975266` links class `0x656B8F27` to creator `0x009750E4`;
  creator enters manager construction at `0x009750F5`. OnStart `0x0097530F` obtains service IID
  `0x287FB697` and passes it to framework slots 3/11 at `0x0097533F`/`0x0097534E`.
- Framework table `0x00AC3090` slot 11 is `0x00879D21`, which appends this service to framework+0xA8.
  Framework slot 16 is dispatcher `0x0087A812` (raw word `0x00AC30D0`). That body snapshots the same list,
  passes the counter from framework+0xC4 and calls service slot 11 at `0x0087A8C1`. Instructions
  `0x0087A8C4–0x0087A8C9` advance/compare the iterator without reading AL/EAX. Later control flow and
  cleanup likewise do not test, save or forward that return.
- The framework interval `0x008793BD–0x0087B0B8` has three indirect +0x2C calls. The other two are
  `0x0087AF73` on the embedded COM interface and `0x0087AFC3` on the application; neither is a service
  Update invocation. Decompilation omits the real service-dispatch loop as supposedly unreachable;
  instructions were used for the return-consumer verdict.
- All 19 immediate service-ID references were checked: constructor plus 18 framework lookups, each with
  manager IID `0x656B8EFC`. The lookup immediate sites are `0x004496DA`, `0x00457146`, `0x0045761B`,
  `0x0045783B`, `0x00457966`, `0x00457B02`, `0x00457BDE`, `0x00457D81`, `0x00457E5B`, `0x00457EB5`,
  `0x00492CEB`, `0x005FBA45`, `0x005FC534`, `0x006023D6`, `0x0066455C`, `0x0078CFE3`, `0x008093CD`,
  `0x008105C3`. These request the manager interface, not the service interface.

Sources were the main checkout's executable, `re_query.py` export queries and bounded decompilation
reads. Numeric widths, accumulator behavior, callback traversal and return consumption were checked
against instructions/bytes. REA was unnecessary and was not invoked. No implementation or test result
was used as evidence. The local read-only helper is `build-runs/pr4-evidence.py`; no decompilation or
assembly listings were copied into committed documentation.

## Vectors, question and validation

- PMU01–PMU06 independently cover empty, one false, all false, all true, mixed false/true/false, and
  reordered callbacks with an early true. Each separates native byte constraints from the chosen return
  and specifies the full callback trace and unchanged 32-bit argument. They are proposed vectors, not
  executed native or game results. PM30 now points to the resolved contract.
- PR4 moved verbatim into [answered/persistence.md](../specs/questions/answered/persistence.md), with the
  dated `DIFFERS` answer and link to the named spec. PR1–PR3 remain open and unchanged. Regenerated
  `docs/specs/QUESTIONS.md`: persistence 4 open / 0 answered becomes 3 open / 1 answered.
- PASS: whitespace check; six unique ordered PMU IDs; PR4 absent from open and present once in answered;
  answer marker and relative spec link valid; only the five intended docs paths changed; clean tracked
  worktree at the spec milestone. Index regeneration was checked for repeatability before finalization.
- Native cargo/clippy/ctest gate and game/shadow runs: n/a (docs-only). No captures, runtime verification,
  shadow exactness or replacement coverage claimed. No ABI proposals or exports changed.

## Commits and remaining work

- `590ac4f3`: initial committed native dispatch proof, within the requested 15–25-call window.
- `1b1745d0`: PR4 specification, explicit policy, six vectors, question movement and regenerated index.
- Final report commit follows those milestones; detached HEAD is intentional and the supervisor owns
  publication to reserved branch `re-persist-update-policy4`.
- No remaining PR4 investigation is requested. Coordinator may merge the docs and use the specified
  deterministic policy. No further persistence-family RE round is proposed.
- Environment note: q.py's Bash wrapper returned CreateInstance/E_ACCESSDENIED; bounded PowerShell and
  local Python queries worked. Git commits succeeded despite a read-only packed-refs lock warning.
- Guard manifest: `C:/Users/tlori/FreeCity4/build-runs/run4-re-persist-update-policy4-r1-manifest.json`.
