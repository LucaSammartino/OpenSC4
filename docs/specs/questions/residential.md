# Residential simulator questions

<!-- from: growth-exact34 r4 (2026-10-28) -->
- **G34-RS1 - EH14 validation timing versus unread references.** May the loaded R$ population grid's own tract-size check run at Init, after the store has read the grid? Residential sections 5.5 and 9.2 put this check in Read, but foundations section 2.7 permits references to objects whose Read has not run. In g34r4-residential-replace-1 on Konradshohe, the host refuses that early geometry access, the empty geometry fails EH14, residential Read returns 0, and Init resets population to 0 (native February DIGEST: 15025). The core now defers this referenced-grid check to Init, still rejects invalid P/E directly in Read, and refuses invalid loaded grid sizes at Init. The existing damaged-grid regression covers the deferred refusal. This reconciles valid loading with the safety choice; please clarify the specification's timing. No exactness claim: the new reference pin is still recording.

<!-- from: growth-exact34 r5 (2026-10-28) -->
- **G34-RS2 - HQ advice EA713A20: which residential query determines activation?**
  Please specify the residential query/observable condition used by this advice, or point to its
  clean-room contract. Advisors section 7.5 enumerates HQ variables and residential accessors, but
  does not map this advice GUID to them. After the referenced-grid load fix, residential alone on
  `gen28-konradshohe-s2-grow` has identical DIGESTs, sampled RNG, object IDs and all residential
  records; only advisor HQ `6A5F8755` and advice `8A09F5F4` differ (qcheck-24). Advice entry 88,
  object 19208, GUID `EA713A20`, has payload flags at +10 equal to reference `0x02`, replacement
  `0x10`. HQ's corresponding entry changes expiry/lastMonth/nextDay/active at payload +1032..1044.
  This is a remaining query/activation finding, not evidence to alter advisor flags or save bytes.
  Replacement run: `compare-20261028-071150-4c5a55ce/result`, label
  `golden-211912c9c5ec4bd59d598195b67c503f`; clean, ref_warnings=0, final 2028-01-28.
  Its key is `4c0732940223c1368bb71b73f2cba7217b95290bb7879ba86cf29684ddecef20`.
  Matching `--replace none` is exact on this scenario (qcheck-25). The other residential cover,
  `ui29-bevopolis`, differs only in traffic network `6A0F82B2` in BOTH replacement and none;
  that separate harness/reference finding is not being chased as residential behavior.
