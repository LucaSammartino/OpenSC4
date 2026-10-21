| Lane | Agent/worktree | State at stop | Next steps |
|---|---|---|---|
| glue-formats4 (GZCOM glue: data formats, DBPF/exemplars) | `glue-formats4` / (engine lane) | in progress (round 2: questions EX41-EX44 filed; extending shadow to cohorts + effective list §4.4) | see Handoff |

## Handoff
- goal: run4 round 2 -- file factory contract questions; extend formats_shadow to cohorts 0x05342861 and the effective list (§4.4). Shadow only, never replaced/enable=formats.
- done: round 1 report at a7dee6fc (8957/8957 exemplars exact, gf4-formats-shadow-2). Questions EX41-EX44 in docs/specs/questions/data-formats.md (QUESTIONS.md not regenerated: lanes may not run tools/; coordinator regenerates).
- done: capi fc4_exstore_* 4a41fa8e; plugin store phase + EX45 011ef609. Game gf4r2-store-1 (turbo to 2032-01-19): clean, ref_warnings=0; store 15 files listed/9 packages/6 no magic; 8959 exemplar + 388 cohort loaded, 0 failed; sets exemplar own 8959/8959 (148906 props, 6232147 B), exemplar effective 8959/8959 (169852, 6515808 B), cohort own 388/388 (3419, 57221 B), cohort effective 388/388 (4996, 74363 B); EXACT, 2297 ms.
- next: qbuild all gate, then final report + merge-queue json.
