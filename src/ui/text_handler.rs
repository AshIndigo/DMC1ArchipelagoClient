use crate::utilities::DMC1_ADDRESS;
use archipelago_rs::LocatedItem;
use randomizer_utilities::archipelago_utilities::get_description;
use randomizer_utilities::{modify_protected_memory, read_data_from_address};
use std::collections::HashMap;
use std::ptr::{copy_nonoverlapping, write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, OnceLock, RwLock};

static LOOKUP_TABLE: LazyLock<HashMap<char, u8>> = LazyLock::new(|| {
    HashMap::from([
        ('0', 0x0),
        ('1', 0x1),
        ('2', 0x2),
        ('3', 0x3),
        ('4', 0x4),
        ('5', 0x5),
        ('6', 0x6),
        ('7', 0x7),
        ('8', 0x8),
        ('9', 0x9),
        ('/', 0xA),
        ('"', 0xB),
        (']', 0xC),
        ('\'', 0xD),
        ('A', 0xE),
        ('B', 0xF),
        ('C', 0x10),
        ('D', 0x11),
        ('E', 0x12),
        ('F', 0x13),
        ('G', 0x14),
        ('H', 0x15),
        ('I', 0x16),
        ('J', 0x17),
        ('K', 0x18),
        ('L', 0x19),
        ('M', 0x1A),
        ('N', 0x1B),
        ('O', 0x1C),
        ('P', 0x1D),
        ('Q', 0x1E),
        ('R', 0x1F),
        ('S', 0x20),
        ('T', 0x21),
        ('U', 0x22),
        ('V', 0x23),
        ('W', 0x24),
        ('X', 0x25),
        ('Y', 0x26),
        ('Z', 0x27),
        ('a', 0x28),
        ('b', 0x29),
        ('c', 0x2A),
        ('d', 0x2B),
        ('e', 0x2C),
        ('f', 0x2D),
        ('g', 0x2E),
        ('h', 0x2F),
        ('i', 0x30),
        ('j', 0x31),
        ('k', 0x32),
        ('l', 0x33),
        ('m', 0x34),
        ('n', 0x35),
        ('o', 0x36),
        ('p', 0x37),
        ('q', 0x38),
        ('r', 0x39),
        ('s', 0x3A),
        ('t', 0x3B),
        ('u', 0x3C),
        ('v', 0x3D),
        ('w', 0x3E),
        ('x', 0x3F),
        ('y', 0x40),
        ('z', 0x41),
        ('ẞ', 0x42),
        ('.', 0x43),
        (',', 0x44),
        (':', 0x45),
        (';', 0x46),
        (' ', 0x47),
        ('`', 0x48),
        ('&', 0x49),
        ('!', 0x4A),
        ('?', 0x4B),
        ('(', 0x4C),
        (')', 0x4D),
        ('+', 0x4E),
        ('-', 0x4F),
        ('*', 0x50),
        ('À', 0x51),
        ('Â', 0x52),
        ('Ç', 0x53),
        ('É', 0x54),
        ('È', 0x55),
        ('Ë', 0x56),
        ('Ê', 0x57),
        ('Î', 0x58),
        ('Ï', 0x59),
        ('Ô', 0x5A),
        ('Œ', 0x5B),
        ('Ù', 0x5C),
        ('Û', 0x5D),
        ('Ü', 0x5E),
        ('à', 0x5F),
        ('â', 0x60),
        ('ç', 0x61),
        ('é', 0x62),
        ('è', 0x63),
        ('ë', 0x64),
        ('ê', 0x65),
        ('î', 0x66),
        ('ï', 0x67),
        ('ô', 0x68),
        ('œ', 0x69),
        ('ù', 0x6A),
        ('û', 0x6B),
        ('ü', 0x6C),
        ('Ñ', 0x6D),
        ('Á', 0x6E),
        ('Í', 0x6F),
        ('Ú', 0x70),
        ('Ó', 0x71),
        ('á', 0x72),
        //('', 0x73),
        ('█', 0x74), // Blackbox...
    ])
});

pub fn translate_string(input: String) -> Vec<u8> {
    let mut res = vec![];
    for c in input.chars() {
        match c {
            ' ' => {
                res.push(0x7E);
                res.push(0x05);
            }
            '\n' => {
                res.push(0x7E);
                res.push(0x09);
                res.push(0x00);
                res.push(0x7E);
                res.push(0x0C);
            }
            _ => {
                res.push(LOOKUP_TABLE.get(&c).unwrap_or(&0x4Bu8).to_owned());
            }
        }
    }
    res
}
const NORMAL_TEXT: u8 = 0x9;
// Only useful when actively printing text
const _SLOW_TEXT: u8 = 0xA;
const WHITE: u8 = 0;
const RED: u8 = 1;
const GREEN: u8 = 2;
const BLUE: u8 = 3;

// 0x7e 0x06 is a closer space?

#[repr(C)]
#[derive(Clone)]
pub struct TextInfo {
    spacer: u8,
    text_type: u8,
    color: u8,
    text: Vec<u8>,
    end_chars: [u8; 2],
}
impl TextInfo {
    pub(crate) fn new(text: String, color: u8) -> TextInfo {
        TextInfo {
            spacer: 0x7E,
            text_type: NORMAL_TEXT,
            color,
            text: translate_string(text),
            end_chars: [0x7E, 0x0E],
        }
    }

    pub(crate) fn to_bytes(&self) -> [u8; 256] {
        let mut res = [0u8; 256];
        let mut i = 0;

        res[i] = self.spacer;
        i += 1;

        res[i] = self.text_type;
        i += 1;

        res[i] = self.color;
        i += 1;

        let text_len = self.text.len().min(256 - i);
        res[i..i + text_len].copy_from_slice(&self.text[..text_len]);

        res
    }

    pub fn get_length(&self) -> usize {
        5 + self.text.len() - 2
    }
}

pub static REPLACE_TEXT: AtomicBool = AtomicBool::new(false);
pub static FOUND_ITEM: RwLock<Option<LocatedItem>> = RwLock::new(None);

pub const DRAW_TEXT_ADDR: usize = 0x2661f0;
pub static ORIGINAL_DRAW_TEXT: OnceLock<unsafe extern "C" fn(usize)> = OnceLock::new();
pub fn draw_text_hook(param_1: usize) {
    if let Some(orig) = ORIGINAL_DRAW_TEXT.get() {
        unsafe {
            orig(param_1);
            if REPLACE_TEXT.load(Ordering::Relaxed) {
                REPLACE_TEXT.store(false, Ordering::Relaxed);
                const MAX_LENGTH: usize = 256;

                let text = if let Some(item) = FOUND_ITEM.read().unwrap().as_ref() {
                    TextInfo::new(
                        format!("AP Item\n{}", get_description(item)),
                        match (item.is_trap(), item.is_useful(), item.is_progression()) {
                            (true, _, _) => RED,
                            (false, _, true) => BLUE,
                            (false, true, false) => GREEN,
                            (false, false, false) => WHITE,
                        },
                    )
                } else {
                    TextInfo::new("Error\nFound Item was\nnot properly\nset.".to_string(), RED)
                };
                let arr: [u8; MAX_LENGTH] = text.to_bytes();

                let dst_addr = *DMC1_ADDRESS + 0x4cb3a88 + 0x18;
                let dst = dst_addr as *mut u8;
                modify_protected_memory(
                    || {
                        let text_range = read_data_from_address::<usize>(*DMC1_ADDRESS + 0x60AFF8);
                        write(
                            // Edit the range of text to be drawn
                            (text_range + 0x7C98) as *mut usize,
                            // Get the start of the range, then add on our text length
                            read_data_from_address::<usize>(text_range + 0x7C90)
                                + text.get_length(),
                        );
                        // Clear old bytes, probably not needed, oh well
                        std::ptr::write_bytes(dst, 0, MAX_LENGTH);
                        // Actually put in our new text
                        copy_nonoverlapping(arr.as_ptr(), dst, MAX_LENGTH);
                    },
                    dst,
                )
                .unwrap();
            }
        }
    }
}
