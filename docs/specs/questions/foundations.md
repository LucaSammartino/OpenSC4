# foundations: open questions
<!-- Questions about [foundations.md](../foundations.md). Append new entries at the END; when one is answered, move it to answered/foundations.md. Rules: README.md. -->

<!-- from: From the return-channel audit (2026-10-16) -->
- **HG2:** cISC4App (`vt:cSC4App::vftable_3`) return channels are in no spec: slot 10 RequestQuit (the autotest falls
  back to AutotestFinish when it reads false), slot 13 LoadCity, slots 15/16 SaveCity, slot 21
  GetDebugFunctionalityEnabled (it gates the cheat-code registration in `cFC4Budget.cpp` and `cFC4Simulator.cpp`).
  Does each set AL/EAX, and with what meaning? [SDK bool trusted; LoadCity and SaveCity only logged; a garbage
  RequestQuit true is covered by the 90 s quit grace]

<!-- from: From the return-channel audit (2026-10-16) -->
- **HG3:** framework slots read as bool with no spec: `vt:cGZCOM` slot 3 GetClassObject (every site also tests the out
  pointer), cIGZPersistResourceManager slots 4 GetPrivateResource and 14, cIGZDate slots 4 SetDayNumber and 6
  SetMonthDayYear (passed through by cFC4Date / cFC4Simulator), cIGZVariant slot 54 CreateValString, cIGZWin slot 36
  GetChildAs. Is each a real bool? [SDK bool trusted]
