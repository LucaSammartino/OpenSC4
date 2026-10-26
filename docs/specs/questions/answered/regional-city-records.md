# regional-city-records: answered questions

<!-- from: re-regional-city-records (2026-10-21) -->
- ~~**RR6 - §5.4:** slot 13 of `cISC4RegionalCity` (`GetWorkforcePercentage`, `0x00800640`) and the residential
  simulator's counterpart return the address of the float in the integer result register. Does the SDK header declare
  the method as returning `float` (a plug-in caller would then read the x87 stack and get garbage), and do all
  callers in the executable dereference the address? [as read: the interface returns an address]~~

  **ANSWERED 2026-10-26 (re-rr29):** DIFFERS — [regional-city-records.md](../../regional-city-records.md)
  §5.4 confirms both SDK float declarations and the regional slot's address return, but the residential
  method writes through a caller-provided destination and returns that pointer. The cache caller passes
  storage and dereferences EAX. A vtable-slot trace with return-consumer capture is specified to settle the
  remaining exhaustive claim about unresolved virtual callers; the native ABI itself is settled.
