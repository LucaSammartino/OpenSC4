// SPDX-License-Identifier: GPL-3.0-or-later
// Generated value-only compare cases. Image pointer words are RVAs, never VAs.
#[derive(Debug)]
pub struct Window {
    pub base: u32,
    pub offset: u32,
    pub bytes: &'static [u8],
    pub image_offsets: &'static [u32],
}
#[derive(Debug)]
pub struct Case {
    pub name: &'static str,
    pub source: &'static str,
    pub mutation: bool,
    pub rva: u32,
    pub contract: u32,
    pub args: &'static [u32],
    pub input: &'static [Window],
    pub native: &'static [Window],
    pub ours: &'static [Window],
    pub native_return: u32,
    pub ours_return: u32,
}
pub const CASES: &[Case] = &[
    Case {
        name: "cross",
        source: "natural",
        mutation: true,
        rva: 1928976,
        contract: 1202543251,
        args: &[0, 0, 0],
        input: &[
            Window {
                base: 1,
                offset: 0,
                bytes: &[235, 127, 160, 190, 232, 213, 18, 191, 128, 189, 65, 191],
                image_offsets: &[],
            },
            Window {
                base: 2,
                offset: 0,
                bytes: &[0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 0],
                image_offsets: &[],
            },
            Window {
                base: 3,
                offset: 0,
                bytes: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                image_offsets: &[],
            },
        ],
        native: &[
            Window {
                base: 1,
                offset: 0,
                bytes: &[235, 127, 160, 190, 232, 213, 18, 191, 128, 189, 65, 191],
                image_offsets: &[],
            },
            Window {
                base: 2,
                offset: 0,
                bytes: &[0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 0],
                image_offsets: &[],
            },
            Window {
                base: 3,
                offset: 0,
                bytes: &[128, 189, 65, 63, 0, 0, 0, 0, 235, 127, 160, 190],
                image_offsets: &[],
            },
        ],
        ours: &[
            Window {
                base: 1,
                offset: 0,
                bytes: &[235, 127, 160, 190, 232, 213, 18, 191, 128, 189, 65, 191],
                image_offsets: &[],
            },
            Window {
                base: 2,
                offset: 0,
                bytes: &[0, 0, 0, 0, 0, 0, 128, 63, 0, 0, 0, 0],
                image_offsets: &[],
            },
            Window {
                base: 3,
                offset: 0,
                bytes: &[129, 189, 65, 63, 0, 0, 0, 0, 235, 127, 160, 190],
                image_offsets: &[],
            },
        ],
        native_return: 0,
        ours_return: 0,
    },
    Case {
        name: "cross",
        source: "fixture",
        mutation: true,
        rva: 1928976,
        contract: 1202543251,
        args: &[0, 0, 0],
        input: &[
            Window {
                base: 1,
                offset: 0,
                bytes: &[0, 0, 0, 193, 0, 0, 224, 192, 0, 0, 192, 192],
                image_offsets: &[],
            },
            Window {
                base: 2,
                offset: 0,
                bytes: &[0, 0, 128, 63, 0, 0, 0, 64, 0, 0, 64, 64],
                image_offsets: &[],
            },
            Window {
                base: 3,
                offset: 0,
                bytes: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                image_offsets: &[],
            },
        ],
        native: &[
            Window {
                base: 1,
                offset: 0,
                bytes: &[0, 0, 0, 193, 0, 0, 224, 192, 0, 0, 192, 192],
                image_offsets: &[],
            },
            Window {
                base: 2,
                offset: 0,
                bytes: &[0, 0, 128, 63, 0, 0, 0, 64, 0, 0, 64, 64],
                image_offsets: &[],
            },
            Window {
                base: 3,
                offset: 0,
                bytes: &[0, 0, 16, 193, 0, 0, 144, 65, 0, 0, 16, 193],
                image_offsets: &[],
            },
        ],
        ours: &[
            Window {
                base: 1,
                offset: 0,
                bytes: &[0, 0, 0, 193, 0, 0, 224, 192, 0, 0, 192, 192],
                image_offsets: &[],
            },
            Window {
                base: 2,
                offset: 0,
                bytes: &[0, 0, 128, 63, 0, 0, 0, 64, 0, 0, 64, 64],
                image_offsets: &[],
            },
            Window {
                base: 3,
                offset: 0,
                bytes: &[1, 0, 16, 193, 0, 0, 144, 65, 0, 0, 16, 193],
                image_offsets: &[],
            },
        ],
        native_return: 0,
        ours_return: 0,
    },
];
