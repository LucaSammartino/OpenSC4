// SPDX-License-Identifier: GPL-3.0-or-later
//! Platform printf boundary. The formatter writes into the supplied native-sized
//! buffer and returns its signed printf status. No second printf dialect lives here.
pub trait Formatter {
    fn format(&mut self, format: &[u8], words: &[u32], buffer: &mut [u8]) -> i32;
    /// Decode platform text for the portable logging router.
    fn decode(&self, bytes: &[u8]) -> String;
}

/// Slot-19 bridge, preserving the first variadic word as the third argument.
pub fn command(
    formatter: &mut impl Formatter,
    format: &[u8],
    words: &[u32],
    mut execute: impl FnMut(&[u8], u32, u32) -> i32,
) -> i32 {
    let mut buffer = [0; 512];
    let count = formatter.format(format, words, &mut buffer[..511]);
    buffer[511] = 0;
    if count <= 0 {
        return 2;
    }
    execute(&buffer, count as u32, words[0])
}
