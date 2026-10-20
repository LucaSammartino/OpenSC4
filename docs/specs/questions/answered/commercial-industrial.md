# commercial-industrial: answered questions (archive)
<!-- Archive for [commercial-industrial.md](../../commercial-industrial.md). Lanes do not read this file. Open questions are in ../commercial-industrial.md. -->

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CM1:** AO1, the commercial part: which AddAgent name does the commercial simulator use, and does it count at the
  month level? [`cSC4CommercialSimulator`. It registers a one-shot, an idle, a day and a month agent; the month agent
  ignores its message but still takes a place in the month order (commercial-industrial.md §3). AO1 and
  foundations §13.4 can take this name. The two port developers are still open]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES (commercial-industrial.md §3, foundations §13.4). The name is `cSC4CommercialSimulator`, and it is in the §13.4 tail table with levels 0, 10, 4, 6. The port developers' name is settled too (AO1, AO3: both use `cSC4PortDeveloper`).

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CM2:** If FreeCity4 ever corrects RG23 (the proximity seed covers only the z = 0 edge), the CO$$ and CO$$$ maps
  stop being constant and offices start to affect growth. Should the correction be offered? [no: compatibility
  keeps the defect. A correction would be a separate *[choice]* behind an option, off by default. This is a
  coordinator decision, not an RE question]~~ **ANSWERED 2026-10-15 (re-answers11):** Not an RE question: the original's behaviour is the defect, and RE has nothing more to add. The bracket stands as the coordinator's policy (compatibility keeps it; any correction is an off-by-default *[choice]*).

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**CM3:** The demand nodes `0x3000` and `0x3800` are held but never read, yet `Init` fails without them. Can the
  shipped game reach `Init` without them? [no case found: the demand simulator creates them first, and FreeCity4
  keeps the failure path. A load test with a city whose demand record is missing would settle it]~~ **ANSWERED 2026-10-16 (re-answers22):** MATCHES (commercial-industrial.md §2.1). The shipped game cannot reach the failure: `GetDemand` (`0x006A2960`) runs the demand simulator's `Init` first when it has not run, and `Init` builds the nodes from the demand exemplars (demand.md §4.1), not from the save. `SimCity_1.dat` holds `0x6534284A / 0xC7BB4816 / 0x3000` ("Commercial") and `/ 0x3800` ("Commercial Connections"), so both nodes exist in every city, whatever its demand record holds. Only a plugin that removes or retypes those exemplars makes `Init` fail; keeping the failure path is right.

<!-- from: From the commercial, industrial, civic, park, history and clock specs (2026-10-14) -->
- ~~**IN1:** Does anything call the industrial simulator beyond the city's lifecycle calls? [no other reference to its
  interface or class ID was found, so the implementation is a stub with a version record. A runtime call trace on
  its vtable would settle it]~~ **ANSWERED 2026-10-15 (re-answers11):** MATCHES as far as static analysis reaches. The interface IID `0x477A9E7D` and class ID `0xA990C022` occur only in `cSC4City::CreateCityComponents` (`0x0045AE5E`), `cSC4City::Read` (`0x0045CF15`), the class's own `QueryInterface`/`GetGZCLSID` (`0x006B8975`, `0x006B8961`) and the class table (`0x00B08E78`); the vtable `0x00AB1530` is referenced only by its constructor and destructor (`0x006B8A4A`, `0x006B8AF0`). A caller through the city getter (slot 60) cannot be ruled out statically; a runtime trace would.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CM4:** commercial `Init` after a failed `Init` (§4.1 sets `initialized` in step 1). [returns true at once and
  creates no grids]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. `Init` sets `initialized` before the demand test (`0x0069EF9C`), so a second call returns true at once (`0x0069F196`) and creates no grids. commercial-industrial §4.1.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CM5:** commercial rebuild with no building map: RG24 says the shared seed helper `0x005E4FA0` writes nothing; do the
  passes of helper `0x005E5130` still run? [yes: nothing is seeded, then the passes run, so a fresh grid stays 0 and a
  loaded grid is only lowered]~~ **ANSWERED 2026-10-15 (re-answers10):** **DIFFERS**. Helper `0x005E5130` tests both pointers first and returns at once for a null building map (`0x005E5136`) or a null grid (`0x005E513F`): no seed and **no passes**, so the grid is left exactly as it was (a loaded grid is not lowered either). The seed helper `0x005E4FA0` is only reached with both present. `DoIdle` still advances the state. commercial-industrial §4.4.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CM6:** a commercial grid absent at a rebuild (failed `Init`, or an unresolved save reference). [the rebuild does
  nothing; the state machine still advances]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. The helper returns at once for a null grid (`0x005E513F`) and `DoIdle` advances the state anyway. commercial-industrial §4.4.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CM7:** a commercial state outside 0..3. [nothing happens]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. The dispatch is an unsigned bounds test (`ja` at `0x0069ED50`), so any value outside 0..3, negative ones included, does nothing. commercial-industrial §4.3.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CM8:** commercial `Read`. [atomic; the old grids are released before being replaced (§8.1 quirk)]~~ **ANSWERED 2026-10-15 (re-answers10):** the original is **not atomic**: `Read` (`0x0069ECB0`) sets `dirty` from the flag before the references (`0x0069ECF6`), each reference read (`0x005D9730`) first stores null without releasing the old grid, and the result is the stream's error state after both reads (`0x0069ED16`). A wrong version changes nothing. The bracket (read into temporaries, release the old grids on success) is a *[choice]* that agrees with the original for every valid save read on a fresh object. commercial-industrial §8.1.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**CM9:** the commercial message server. [unsubscribe and release only when one is held]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. `Shutdown` (`0x0069EBE0`) unsubscribes and releases the message server only when one is held. Note: `RemoveAgent` goes through the global city `0x00B43CEC`, not the held city. commercial-industrial §4.2.

<!-- from: From the commercial, industrial, civic and park cores (2026-10-14) -->
- ~~**IN2:** industrial `Shutdown`: the spec gives only "AL (1)". [clears `initialized`, returns true]~~ **ANSWERED 2026-10-15 (re-answers10):** MATCHES. Industrial `Shutdown` (`0x006B8A10`) clears `initialized` and returns true, nothing else. commercial-industrial §2.2.

<!-- from: From the commercial, industrial, civic and park glue (2026-10-15) -->
- ~~**CM10:** the save sweep for the commercial, industrial, civic and park records runs in `fc4-capi`
  (`capi/src/save_sweep_tests.rs`, over the C ABI); histclock's record mode is in `fc4-formats`
  (`formats/tests/save_records.rs`). Should the four records move there, over core-level hosts (about 100 host methods
  and object-ID-keeping streams)? [they stay in `fc4-capi`; both read `FC4_SAVES_DIR` and both READMEs point at each
  other]~~ **ANSWERED 2026-10-15 (re-answers12):** Not an RE question: the original has no counterpart. Keeping the four records' sweep in `fc4-capi` is the coordinator's choice; the bracket stands.
