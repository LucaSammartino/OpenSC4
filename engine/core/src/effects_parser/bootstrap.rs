// SPDX-License-Identifier: GPL-3.0-or-later
//! Separate EffectsParser queue wrapper and EffectsBootstrap boundary.
//! effects §6.1 names ParseQueuedFiles but supplies no queue/file algorithm;
//! effects-parser §1 expressly excludes it. Do not silently report success.

use super::script::{Error, Result};

/// Not the native cSC4EffectsParser command owner implemented by `Parser`.
pub struct EffectsParser;

impl EffectsParser {
    pub fn parse_queued_files(&mut self) -> Result<()> {
        Err(Error::Unsupported("EF18: EffectsParser queue/file contract"))
    }
}

pub struct EffectsBootstrap;

impl EffectsBootstrap {
    pub fn initialize(&mut self) -> Result<()> {
        Err(Error::Unsupported("EF18: EffectsBootstrap lifecycle contract"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unspecified_queue_and_bootstrap_do_not_claim_success() {
        assert_eq!(
            EffectsParser.parse_queued_files(),
            Err(Error::Unsupported("EF18: EffectsParser queue/file contract"))
        );
        assert_eq!(
            EffectsBootstrap.initialize(),
            Err(Error::Unsupported("EF18: EffectsBootstrap lifecycle contract"))
        );
    }
}
