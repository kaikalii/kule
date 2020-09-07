mod tables;

use crate::fontdue::unicode::tables::*;

const CONT_MASK: u8 = 0b0011_1111;

#[inline(always)]
fn utf8_acc_cont_byte(ch: u32, byte: u8) -> u32 {
    (ch << 6) | (byte & CONT_MASK) as u32
}

/// Returns (length, character). Cannot be run at the end of the string.
pub fn read_utf8(string: &str, byte_offset: &mut usize) -> char {
    let bytes = string.as_bytes();
    let x = bytes[*byte_offset];
    *byte_offset += 1;
    if x < 128 {
        return unsafe { core::char::from_u32_unchecked(x as u32) };
    }
    let init = (x & (0x7F >> 2)) as u32;
    let y = bytes[*byte_offset];
    *byte_offset += 1;
    let mut ch = utf8_acc_cont_byte(init, y);
    if x >= 0xE0 {
        let z = bytes[*byte_offset];
        *byte_offset += 1;
        let y_z = utf8_acc_cont_byte((y & CONT_MASK) as u32, z);
        ch = init << 12 | y_z;
        if x >= 0xF0 {
            let w = bytes[*byte_offset];
            *byte_offset += 1;
            ch = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
        }
    }
    unsafe { core::char::from_u32_unchecked(ch) }
}

pub const LINEBREAK_NONE: u8 = 0; // 0000
pub const LINEBREAK_SOFT: u8 = 1; // 0001
pub const LINEBREAK_HARD: u8 = 2; // 0010

#[inline(always)]
pub fn wrap_mask(wrap: bool, wrap_soft_breaks: bool, wrap_hard_breaks: bool) -> u8 {
    let mut mask = 0;
    if wrap {
        if wrap_hard_breaks {
            mask |= LINEBREAK_HARD;
        }
        if wrap_soft_breaks {
            mask |= LINEBREAK_SOFT;
        }
    }
    mask
}

// [See license/xi-editor/xi-unicode] Copyright 2016 The xi-editor Authors
pub fn linebreak_property(state: &mut u8, codepoint: char) -> u8 {
    let cp = codepoint as usize;
    let lb = if cp < 0x800 {
        LINEBREAK_1_2[cp]
    } else if cp < 0x10000 {
        let child = LINEBREAK_3_ROOT[cp >> 6];
        LINEBREAK_3_CHILD[(child as usize) * 0x40 + (cp & 0x3f)]
    } else {
        let mid = LINEBREAK_4_ROOT[cp >> 12];
        let leaf = LINEBREAK_4_MID[(mid as usize) * 0x40 + ((cp >> 6) & 0x3f)];
        LINEBREAK_4_LEAVES[(leaf as usize) * 0x40 + (cp & 0x3f)]
    };
    let i = (*state as usize) * N_LINEBREAK_CATEGORIES + (lb as usize);
    let new = LINEBREAK_STATE_MACHINE[i];
    if (new as i8) < 0 {
        *state = new & 0x3f;
        if new >= 0xc0 {
            LINEBREAK_HARD
        } else {
            LINEBREAK_SOFT
        }
    } else {
        *state = new;
        LINEBREAK_NONE
    }
}
