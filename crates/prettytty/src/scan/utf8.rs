pub(super) fn scan_utf8(bytes: &[u8]) -> std::result::Result<usize, usize> {
    // See https://github.com/rust-lang/rust/blob/master/library/core/src/str/validations.rs
    let mut index = 0;
    let len = bytes.len();

    macro_rules! next {
        () => {{
            index += 1;
            if index >= len {
                return Err(index);
            }
            bytes[index]
        }};
    }

    if len == 0 {
        return Err(0);
    }
    let first = bytes[0];
    if first < 0x80 {
        return Ok(1);
    }

    let width = UTF8_CHAR_WIDTH[(first - 0x80) as usize];
    // 2-byte encoding is for codepoints  \u{0080} to  \u{07ff}
    //        first  C2 80        last DF BF
    // 3-byte encoding is for codepoints  \u{0800} to  \u{ffff}
    //        first  E0 A0 80     last EF BF BF
    //   excluding surrogates codepoints  \u{d800} to  \u{dfff}
    //               ED A0 80 to       ED BF BF
    // 4-byte encoding is for codepoints \u{10000} to \u{10ffff}
    //        first  F0 90 80 80  last F4 8F BF BF
    //
    // Use the UTF-8 syntax from the RFC
    //
    // https://tools.ietf.org/html/rfc3629
    // UTF8-1      = %x00-7F
    // UTF8-2      = %xC2-DF UTF8-tail
    // UTF8-3      = %xE0 %xA0-BF UTF8-tail / %xE1-EC 2( UTF8-tail ) /
    //               %xED %x80-9F UTF8-tail / %xEE-EF 2( UTF8-tail )
    // UTF8-4      = %xF0 %x90-BF 2( UTF8-tail ) / %xF1-F3 3( UTF8-tail ) /
    //               %xF4 %x80-8F 2( UTF8-tail )
    match width {
        2 => {
            if next!() as i8 >= -64 {
                return Err(1);
            }
        }
        3 => {
            match (first, next!()) {
                (0xE0, 0xA0..=0xBF)
                | (0xE1..=0xEC, 0x80..=0xBF)
                | (0xED, 0x80..=0x9F)
                | (0xEE..=0xEF, 0x80..=0xBF) => {}
                _ => return Err(1),
            }
            if next!() as i8 >= -64 {
                return Err(2);
            }
        }
        4 => {
            match (first, next!()) {
                (0xF0, 0x90..=0xBF) | (0xF1..=0xF3, 0x80..=0xBF) | (0xF4, 0x80..=0x8F) => {}
                _ => return Err(1),
            }
            if next!() as i8 >= -64 {
                return Err(2);
            }
            if next!() as i8 >= -64 {
                return Err(3);
            }
        }
        _ => return Err(1),
    }

    Ok(width as usize)
}

#[rustfmt::skip]
const UTF8_CHAR_WIDTH: &[u8; 128] = &[
    // 1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 8
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 9
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // A
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // B
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // C
    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // D
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // E
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // F
];

#[cfg(any())]
mod algo2 {
    // Code based on https://bjoern.hoehrmann.de/utf-8/decoder/dfa/
    // and https://github.com/BurntSushi/bstr/blob/master/src/utf8.rs

    pub(super) fn scan_utf8(bytes: &[u8]) -> std::result::Result<usize, usize> {
        let (mut index, mut state) = (0, UTF8_ACCEPT);

        while index < bytes.len() {
            state = TRANSITIONS[(state + CLASSES[bytes[index] as usize]) as usize];

            if state == UTF8_ACCEPT {
                return Ok(index + 1);
            } else if state == UTF8_REJECT {
                return Err(1.max(index));
            }

            index += 1;
        }

        Err(index)
    }

    #[rustfmt::skip]
    static CLASSES: [u8; 256] = [
        // The first part of the table maps bytes to character classes that
        // to reduce the size of the transition table and create bitmasks.
        0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
        1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,  9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
        7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,  7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
        8,8,2,2,2,2,2,2,2,2,2,2,2,2,2,2,  2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
        10,3,3,3,3,3,3,3,3,3,3,3,3,4,3,3, 11,6,6,6,5,8,8,8,8,8,8,8,8,8,8,8,
    ];

    const UTF8_ACCEPT: u8 = 12;
    const UTF8_REJECT: u8 = 0;

    #[rustfmt::skip]
    static TRANSITIONS: [u8; 108] = [
        // The second part is a transition table that maps a combination
        // of a state of the automaton and a character class to a state.
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        12, 0,24,36,60,96,84, 0, 0, 0,48,72,
        0,12, 0, 0, 0, 0, 0,12, 0,12, 0, 0,
        0,24, 0, 0, 0, 0, 0,24, 0,24, 0, 0,
        0, 0, 0, 0, 0, 0, 0,24, 0, 0, 0, 0,
        0,24, 0, 0, 0, 0, 0, 0, 0,24, 0, 0,
        0, 0, 0, 0, 0, 0, 0,36, 0,36, 0, 0,
        0,36, 0, 0, 0, 0, 0,36, 0,36, 0, 0,
        0,36, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
}
