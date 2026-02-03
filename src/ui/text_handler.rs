use archipelago_rs::LocatedItem;
use std::collections::HashMap;
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
const SLOW_TEXT: u8 = 0xA;
const WHITE: u8 = 0;
const RED: u8 = 1;
const GREEN: u8 = 2;
const BLUE: u8 = 3;

#[repr(C)]
#[derive(Clone)]
struct TextInfo {
    spacer: [u8; 5],
    text_type: u8,
    color: u8,
    text: Vec<u8>,
    end_chars: [u8; 2],
}
impl TextInfo {
    fn new(text: String) -> TextInfo {
        TextInfo {
            spacer: [0x7E, 0x0E, 0xFF, 0xFF, 0x7E],
            text_type: NORMAL_TEXT,
            color: BLUE,
            text: translate_string(text),
            end_chars: [0x7E, 0x0E],
        }
    }

    fn to_bytes(&self) -> Box<[u8]> {
        let mut res = vec![];
        res.extend(self.spacer);
        res.push(self.text_type);
        res.push(self.color);
        res.extend(self.text.clone());
        //res.extend(self.end_chars);
        res.into()
    }
}

pub static REPLACE_TEXT: AtomicBool = AtomicBool::new(false);
// TODO Need to set this to None, once the item get screen is closed
pub static FOUND_ITEM: RwLock<Option<LocatedItem>> = RwLock::new(None);

pub const DRAW_TEXT_ADDR: usize = 0x2669a0;
pub static ORIGINAL_DRAW_TEXT: OnceLock<unsafe extern "C" fn(usize)> = OnceLock::new();
static TEXT: LazyLock<RwLock<TextInfo>> =
    LazyLock::new(|| RwLock::new(TextInfo::new("Test hi!\nHello!".to_owned())));
pub fn draw_text_hook(fuck: usize) {
    if let Some(orig) = ORIGINAL_DRAW_TEXT.get() {
        // let param_1 = read_data_from_address::<usize>(*DMC1_ADDRESS + 0x60aff8);
        // let arr = TEXT.read().unwrap().to_bytes();
        // let len = &arr.len();
        unsafe {
            // if REPLACE_TEXT.load(Ordering::Relaxed) {
            //     copy_nonoverlapping(
            //         arr.as_ptr(),
            //         (read_data_from_address::<usize>(param_1 + 0x7c80) - len - 1) as *mut u8,
            //         *len,
            //     );
            //     REPLACE_TEXT.store(false, Ordering::Relaxed);
            // }
            if !REPLACE_TEXT.load(Ordering::Relaxed) {
                orig(fuck)
            }
        }
    } else {
        panic!("Original draw text not found")
    }
}

/*
pub fn draw_text_hook(param_1: usize) {
    if let Some(orig) = ORIGINAL_DRAW_TEXT.get() {
        let arr = TEXT.read().unwrap().to_bytes();
        let len = &arr.len();
        unsafe {
            if REPLACE_TEXT.load(Ordering::Relaxed) {
                copy_nonoverlapping(
                    arr.as_ptr(),
                    (read_data_from_address::<usize>(param_1 + 0x7c80) - len - 1) as *mut u8,
                    *len,
                );
                REPLACE_TEXT.store(false, Ordering::Relaxed);
            }
            orig(param_1);
        }
    }
}
 */
