# text-decal-occupant: open questions

- **TDO1:** The native UpdateDecal store points and x87 operation order are specified in
  text-decal-occupant.md §5. What precision-control and rounding mode are guaranteed across
  all admissible city-detail TextDecal entry paths? This static round did not establish the
  global FPU environment. Arbitrary non-dyadic geometry vectors must remain conditional on it;
  do not substitute unconditional host-f64 expected bits. No live measurement was authorized.
