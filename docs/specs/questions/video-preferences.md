# video-preferences: open questions
<!-- Questions about [video-preferences.md](../video-preferences.md). Append new entries at the END; when answered, move to answered/video-preferences.md. Rules: README.md. -->

<!-- from: re-video-preferences4 (2026-10-23) -->
- **VP1 - §6.1, remaining redraw CFG:** qualify every intervening branch of
  `SetButtonsBasedOnPreferences 0079020d–00791201`: raw fields, enum and
  checkbox inversions, first-redraw `W+1e` baseline captures, resolution
  selection thresholds, unsupported-control requests and exact reference order.
  Does the owner consult any mode/driver identity or merely fixed control IDs?
  Establish empty/absent/duplicate/equal/current-mode outcomes only if an owned
  branch actually admits those inputs. [only the first enum group, null-P path,
  final depth group and return cleanup are qualified; no native-mode fallback]

<!-- from: re-video-preferences4 (2026-10-23) -->
- **VP2 - §6.3–6.4, remaining events and resource semantics:** qualify all other
  IDs admitted by §5.1, including selection dependencies and the complete
  jump-table-driven radio groups. Establish literal resource labels/roles for
  `6a57da48`, `ea57da59`, `ea57da6f`, `ea5e99d9`, `2a57cb83/84` and actual
  `8810 +18/+1c` / widget `+110` primitive semantics without importing a
  foreign widget algorithm. Can a disabled native control generate the admitted
  event, and what owner branch then runs? [four action branches and the
  `ab81ded5–d7` group qualified; no UX-derived labels or disabled-event reachability]

<!-- from: re-video-preferences4 (2026-10-23) -->
- **VP3 - §2, §7.1, construction and initialization:** qualify the owner
  allocation/constructor caller, initial count and `W+10/12/14/16/18/1a/1c/1e`
  values, and middle `ShowDialog 00791677–00791d1d` binding sequence. Prove the
  complete raw-control list and null preconditions. Which owned instruction
  first sets `W+1e`, and which words remain original snapshots after an Auto
  request/redraw? [no candidate preference-object copy in the audited prefix;
  existing P is retained or borrowed from `00b43cd8`; full owner lifetime unknown]

<!-- from: re-video-preferences4 (2026-10-23) -->
- **VP4 - §6.2–6.3, callsite-only App boundary:** establish declared calling
  convention/types and any caller-visible guarantees for A `+4c(P+f0c)` and
  `+50(P+f0c)` from existing authorized boundary evidence, plus independently
  supplied literal Auto output fixtures. The selected owner ignores return
  channels and calls SavePreferences before ApplyVideoPreferences. Does any
  other owned branch constrain the pointer's valid domain or notification
  sequencing? [do not reopen capped App/AP3–9, persistence, renderer or mode
  enumeration bodies; no literal defaults or default-then-close golden yet]

<!-- from: re-video-preferences4 (2026-10-23) -->
- **VP5 - §7.2, modal primitive qualification:** provide the interface identity
  and declared types for service `(a417445e,5a4)`, its raw `+90/+94/+60` calls,
  and service `00b43ce0 +28/+18`, using interface/callsite evidence only. The
  owner retains R across the modal call, conditionally restores F on nonzero
  `+60(F)` AL and always returns true AL; does any supported foreign-failure
  domain exist before its unchecked service/manager dereferences? [normal
  valid-service sequence qualified; no graceful missing-service policy, no
  modal-loop implementation or global startup/catalogue timing closure]
