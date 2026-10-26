// SPDX-License-Identifier: GPL-3.0-or-later
//! Read invocation precision without changing the native thread's control word.
use fc4_core::x87::Precision;

pub(crate) fn precision() -> Precision {
    #[cfg(all(target_arch = "x86", target_os = "windows"))]
    {
        let mut word = 0_u16;
        // SAFETY: fnstcw only writes this local word, without changing x87
        // state or consuming a register. No FFI callback runs in this scope.
        unsafe {
            std::arch::asm!("fnstcw [{word}]", word = in(reg) &mut word, options(nostack, preserves_flags));
        }
        // These numeric paths reproduce PC24/PC53; binary64 also bounds
        // their arithmetic if a caller selects the unmodeled PC64 context.
        if word & 0x0300 != 0 {
            return Precision::Bits53;
        }
    }
    Precision::Bits24
}
