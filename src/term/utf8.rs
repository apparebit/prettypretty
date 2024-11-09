// Code based on https://bjoern.hoehrmann.de/utf-8/decoder/dfa/
// and https://github.com/BurntSushi/bstr/blob/master/src/utf8.rs

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

pub(crate) const UTF8_ACCEPT: u8 = 12;
pub(crate) const UTF8_REJECT: u8 = 0;

#[rustfmt::skip]
static TRANSITIONS: [u8; 108] = [
    // The second part is a transition table that maps a combination
    // of a state of the automaton and a character class to a state.
    12, 0,24,36,60,96,84, 0, 0, 0,48,72,
     0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
     0,12, 0, 0, 0, 0, 0,12, 0,12, 0, 0,
     0,24, 0, 0, 0, 0, 0,24, 0,24, 0, 0,
     0, 0, 0, 0, 0, 0, 0,24, 0, 0, 0, 0,
     0,24, 0, 0, 0, 0, 0, 0, 0,24, 0, 0,
     0, 0, 0, 0, 0, 0, 0,36, 0,36, 0, 0,
     0,36, 0, 0, 0, 0, 0,36, 0,36, 0, 0,
     0,36, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub(crate) fn decode(input: u8, state: &mut u8, codepoint: &mut u32) {
    let class = CLASSES[input as usize];
    if *state == UTF8_ACCEPT {
        *codepoint = (0xff >> class) & (input as u32);
    } else {
        *codepoint = (input as u32 & 0b111111) | (*codepoint << 6);
    }
    *state = TRANSITIONS[(*state + class) as usize]
}
