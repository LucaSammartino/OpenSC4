# prop-lotconfig: open questions
<!-- Questions about [prop-lotconfig.md](../prop-lotconfig.md). Append new entries at the END; when one is answered, move it to answered/prop-lotconfig.md. Rules: README.md. -->

## From the parked glue21 lotcfg replacement (copied verbatim by re-pl19-run3, 2026-10-21)

- **PL19:** (§B5.2, §B7 slot 9; the SimNewDay growth-pass release wave) with the PL18 double-add
  (2 references per matched configuration per slot-9 query) the day-29 deletes are gone and the
  deletes move to day 30 and shrink to the configurations with no game-lot reference - but they do
  not stop: the game's release wave (SimCity 4.exe `0x00709798`) drains a touched configuration to
  ZERO whatever its reference count. Measured with the per-op trace restored
  (glue21fix2-ref2-2: no cushion; -ref2-3: a 40-reference diagnostic cushion per matched config):
  per configuration the load is create +1, core `add_configuration` +1, creation release -1
  (count 1), the game's own bulk wave +1 (exe `0x0069C385`), the game's per-candidate AddRefs
  (exe `0x006BF53D`, 1..66 per configuration over 736 of 1909), the slot-9 query adds (+2 per
  matched configuration: the core's §B5.2 add plus the plugin loop's), then the wave releases
  -1 per configuration per pass until the wrapper is DELETED: in ref2-3 five configurations of
  count 44 were released 43 logged times plus the deleting 44th; two others of the same count 44
  were released exactly once and stopped; in ref2-2 (no cushion) a count-4 configuration was
  released 4 times (deleted) and a count-5 one 4 times (survived at 1). The release count per
  configuration therefore follows the game's own candidate/pass structure, not the references it
  acquired (a configuration the game never AddRef'd was released 44 times), and no reference
  hand-out can balance it. Which container does `0x00709798` iterate (the growth developer's
  per-candidate list? the configuration set's entries per Grow pass), when does it stop, and what
  does the ORIGINAL do so its configurations are never drained to 0 (its catalogue reference is
  evidently never released)? Until this is answered from the exe, no FreeCity4 reading of slot 9 /
  slot 20 can make the lotcfg replacement game-clean; the double-add is kept (it strictly improves
  the ledger) and the fault stands between the run and a clean verdict.
