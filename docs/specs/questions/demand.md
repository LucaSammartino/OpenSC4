# Demand: open questions

- **DM-GC29-1 — empty tax-history window (§8.1).** The spec's current choice uses
  `maxRate = neutral * 10` when the history host reports no values. In census
  `sh28-census3/empty-small.log:3433`, the first SimIdle (2000-01-01) reports
  original tax modifiers `0.300000012` for all twelve types, versus core `0`,
  with no history in the window. The shipped curve gives 0.3 at its low end
  and 0 at ratio 1. What value does the original initialize the history maximum
  to, and does a failed/empty history query overwrite it? Please specify both
  the no-series and empty-date-window cases. Blocks the first-day demand
  comparison in `empty-small`; no tolerance or comparison was changed.
