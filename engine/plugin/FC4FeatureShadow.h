// SPDX-License-Identifier: GPL-3.0-or-later
#pragma once
// Opt-in feature_shadow=1: app-owned original versus one independent Rust scan.
// Complete-byte input captures only; no streaming failure or FM1/FM2 reachability
// credit. Reports query exact/diff/refusal/resync and balanced lifetime counters.
namespace fc4 {
void FeatureShadowLoaded();
void FeatureShadowMonth();
void FeatureShadowShutdown();
}
