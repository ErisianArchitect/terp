
use ::core::{
    ops::{
        Range,
    },
};

use byteset::{
    ByteSet,
};

#[derive(Debug, thiserror::Error)]
pub enum EscapeError<'a> {
    #[error("{0:?} is not an escape sequence.")]
    NotAnEscapeSequence(&'a str),
    #[error("Value of \"\\x{0:02X}\" is out of the ascii range.")]
    HexEscapeOutOfRange(u8),
    #[error("{0:?} is not a valid hex escape sequence.")]
    InvalidHexSequence(&'a str),
    #[error("The sequence was empty")]
    EmptySequence,
}

#[must_use]
#[inline]
pub const fn is_hex_digit_u8(chr: u8) -> bool {
    ByteSet::HEX_DIGITS.has(chr)
}

#[must_use]
#[inline]
pub const fn is_hex_digit_char(chr: char) -> bool {
    ByteSet::HEX_DIGITS.has_char(chr)
}

#[repr(transparent)]
#[derive(Clone, Copy)]
struct HexDigitFlags(u64);

impl HexDigitFlags {
    #[inline(always)]
    const fn set_bit(&mut self, bit: u32) {
        self.0 |= (1 << bit);
    }

    const fn set_bit_range(&mut self, range: Range<u32>) {
        let mut i = range.start;
        let end = if range.end > 64 {
            64
        } else {
            range.end
        };
        while i < end {
            self.set_bit(i);
            i += 1;
        }
    }

    const fn with_hex_range(mut self, range: Range<u8>) -> Self {
        self.set_bit_range((range.start - 48) as u32..(range.end - 48) as u32);
        self
    }

    #[must_use]
    #[inline(always)]
    const fn get_bit(self, index: u8) -> bool {
        let bit = 1 << index;
        self.0 & bit != 0
    }

    #[must_use]
    #[inline(always)]
    const fn get_hex_bit(self, index: u8) -> bool {
        index >= 48 && self.0 & (1 << (index - 48)) != 0
    }

    const fn count_bits_until(self, index: u8) -> u32 {
        let mask = (1u64 << index) - 1;
        let masked = self.0 & mask;
        masked.count_ones()
    }
}

const HEX_FLAGS: HexDigitFlags = HexDigitFlags(0)
    .with_hex_range(b'0'..b'9'+1)
    .with_hex_range(b'a'..b'g')
    .with_hex_range(b'A'..b'G');

const HEX_VALUES_TABLE: [u8; 22] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    10, 11, 12, 13, 14, 15,
    10, 11, 12, 13, 14, 15,
];

pub const fn unchecked_decode_hex_digit(hex_digit: u8) -> u8 {
    let bit = hex_digit - 48;
    let index = HEX_FLAGS.count_bits_until(bit);
    HEX_VALUES_TABLE[index as usize]
}

pub const fn decode_hex_digit(hex_digit: u8) -> Option<u8> {
    if ByteSet::HEX_DIGITS.has(hex_digit) {
        Some(unchecked_decode_hex_digit(hex_digit))
    } else {
        None
    }
}

#[inline]
pub const unsafe fn subslice_unchecked<'a, T>(
    range: Range<usize>,
    slice: &'a [T],
) -> &'a [T] {
    unsafe {
        let ptr = slice.as_ptr().offset(range.start as isize);
        ::core::slice::from_raw_parts(ptr, range.end - range.start)
    }
}

pub const fn subslice<'a, T>(range: Range<usize>, slice: &'a [T]) -> &'a [T] {
    if range.end > slice.len() {
        panic!("Range out of bounds.");
    }
    if range.start > range.end {
        panic!("You put your Range on backwards.");
    }
    unsafe {
        subslice_unchecked(range, slice)
    }
}

#[inline]
pub const unsafe fn subslice_mut_unchecked<'a, T>(
    range: Range<usize>,
    slice: &'a mut [T],
) -> &'a mut [T] {
    unsafe {
        let ptr = slice.as_mut_ptr().offset(range.start as isize);
        ::core::slice::from_raw_parts_mut(ptr, range.end - range.start)
    }
}

pub const fn subslice_mut<'a, T>(
    range: Range<usize>,
    slice: &'a mut [T],
) -> &'a mut [T] {
    if range.end > slice.len() {
        panic!("Range out of bounds.");
    }
    if range.start > range.end {
        panic!("You put your Range on backwards.");
    }
    unsafe {
        subslice_mut_unchecked(range, slice)
    }
}

pub const fn escape_sequence(seq: &str) -> Result<char, EscapeError<'_>> {
    match seq.len() {
        0 => {
            return Err(EscapeError::EmptySequence);
        }
        1 => {
            Ok(match seq.as_bytes()[0] {
                b'n' => '\n',
                b't' => '\t',
                b'r' => '\r',
                b'0' => '\0',
                b'{' => '{',
                b'}' => '}',
                b'[' => '[',
                b']' => ']',
                b'(' => '(',
                b')' => ')',
                b'<' => '<',
                b'>' => '>',
                b'@' => '@',
                b'#' => '#',
                b'$' => '$',
                b'%' => '%',
                _ => {
                    return Err(EscapeError::NotAnEscapeSequence(seq));
                }
            })
        }
        2 => {
            return Err(EscapeError::NotAnEscapeSequence(seq));
        }
        3 => {
            match seq.as_bytes()[0] {
                b'x' | b'X' => {
                    if seq.len() == 3 {
                        if ByteSet::HEX_DIGITS.has_all(
                            unsafe { subslice_unchecked(1..3, seq.as_bytes()) }
                        ) {
                            let upper = unchecked_decode_hex_digit(seq.as_bytes()[1]);
                            let lower = unchecked_decode_hex_digit(seq.as_bytes()[2]);
                            let full = lower | (upper << 4);
                            if upper > 0x7 {
                                return Err(EscapeError::HexEscapeOutOfRange(full));
                            }
                            Ok(unsafe { char::from_u32_unchecked(full as u32) })
                        } else {
                            return Err(EscapeError::InvalidHexSequence(seq));
                        }
                    } else {
                        return Err(EscapeError::InvalidHexSequence(seq));
                    }
                }
                _ => return Err(EscapeError::NotAnEscapeSequence(seq)),
            }
        }
        _ => {
            return Err(EscapeError::NotAnEscapeSequence(seq));
        }
    }
}
