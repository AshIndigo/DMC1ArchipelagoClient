use crate::game_manager::ItemData;
use bimap::BiMap;
use randomizer_utilities::dmc::dmc_constants::GameConfig;
use std::collections::HashMap;
use std::sync::LazyLock;

pub type BasicNothingFunc = unsafe extern "system" fn();

pub const MAX_HP: u8 = 30;
pub const INITIAL_HP: u8 = 10; // 15 for easy
pub const MAX_MAGIC: u8 = 10;
pub const INITIAL_MAGIC: u8 = 0; // 3 Normal and up, 6 on easy
pub const NO_MISSION: u32 = 0;

pub(crate) static REMOTE_ID: LazyLock<u32> = LazyLock::new(|| 100);
#[derive(Debug)]
pub struct Item {
    pub id: u8,
    pub name: &'static str,
    pub category: u8,
    pub mission: Option<u32>, // Mission the key item is used in, typically the same that it is acquired in
    pub group: ItemCategory,
}

impl Item {
    pub fn construct_item_data(&self) -> ItemData {
        ItemData {
            category: self.category,
            id: self.id,
            count: 1,
        }
    }
}

#[derive(PartialEq, Debug)]
pub(crate) enum ItemCategory {
    Key,
    Consumable,
    Weapon,
    Misc,
}

// Skipping over items I don't find useful
pub(crate) const ALL_ITEMS: [Item; 43] = [
    Item {
        id: 0,
        name: "Handgun",
        category: 0,
        mission: None,
        group: ItemCategory::Weapon,
    },
    Item {
        id: 1,
        name: "Shotgun",
        category: 0,
        mission: None,
        group: ItemCategory::Weapon,
    },
    Item {
        id: 2,
        name: "Needlegun",
        category: 0,
        mission: None,
        group: ItemCategory::Weapon,
    },
    Item {
        id: 3,
        name: "Grenade Launcher",
        category: 0,
        mission: None,
        group: ItemCategory::Weapon,
    },
    Item {
        id: 4,
        name: "Nightmare Beta",
        category: 0,
        mission: None,
        group: ItemCategory::Weapon,
    },
    Item {
        id: 5,
        name: "Force Edge",
        category: 0,
        mission: None,
        group: ItemCategory::Weapon,
    },
    Item {
        id: 6,
        name: "Alastor",
        category: 0,
        mission: None,
        group: ItemCategory::Weapon,
    },
    Item {
        id: 7,
        name: "Ifrit",
        category: 0,
        mission: None,
        group: ItemCategory::Weapon,
    },
    Item {
        id: 8,
        name: "Sparda", // I'd want to somehow add an option to enable Sparda's DT for use everywhere
        category: 0,
        mission: None,
        group: ItemCategory::Weapon,
    },
    Item {
        id: 9,
        name: "Yamato", // Crashes the game when I'm Dante - Needs DDMK Fix
        category: 0,
        mission: None,
        group: ItemCategory::Weapon,
    },
    Item {
        id: 0,
        name: "Bangle of Time",
        category: 1,
        mission: None,
        group: ItemCategory::Misc,
    },
    Item {
        id: 1,
        name: "Luminite",
        category: 1,
        mission: None,
        group: ItemCategory::Key,
    },
    Item {
        id: 2,
        name: "Yellow Orb",
        category: 2,
        mission: None,
        group: ItemCategory::Consumable,
    },
    Item {
        id: 4,
        name: "Blue Orb Fragment",
        category: 2,
        mission: None,
        group: ItemCategory::Misc,
    },
    // TODO These are in both Category 1 and 2. I'll only keep one instance though. I think category 2 is the one I should care about
    Item {
        id: 10, // cat 1 id 12
        name: "Vital Star",
        category: 2,
        mission: None,
        group: ItemCategory::Consumable,
    },
    Item {
        id: 13,
        name: "Devil Star",
        category: 1,
        mission: None,
        group: ItemCategory::Consumable,
    },
    Item {
        id: 14,
        name: "Untouchable",
        category: 1,
        mission: None,
        group: ItemCategory::Consumable,
    },
    Item {
        id: 15,
        name: "Holy Water",
        category: 1,
        mission: None,
        group: ItemCategory::Consumable,
    },
    Item {
        id: 14, // or maybe cat 1 id 16
        name: "Rusty Key (Mission #1)",
        category: 2,
        mission: Some(1),
        group: ItemCategory::Key,
    },
    Item {
        id: 14, // 14  cat 2
        name: "Rusty Key (Mission #2)",
        category: 2,
        mission: Some(2),
        group: ItemCategory::Key,
    },
    Item {
        id: 14, // 14  cat 2
        name: "Rusty Key (Mission #6)",
        category: 2,
        mission: Some(3),
        group: ItemCategory::Key,
    },
    Item {
        id: 17,
        name: "Staff of Hermes",
        category: 1,
        mission: Some(16),
        group: ItemCategory::Key,
    },
    Item {
        id: 19,
        name: "Emblem Shield",
        category: 1,
        mission: Some(15),
        group: ItemCategory::Key,
    },
    Item {
        id: 0,
        name: "Staff of Judgement",
        category: 4,
        mission: Some(2),
        group: ItemCategory::Key,
    },
    Item {
        id: 1,
        name: "Death Sentence",
        category: 4,
        mission: Some(4),
        group: ItemCategory::Key,
    },
    Item {
        id: 2,
        name: "Death Sentence (2)", // Unused item/texture
        category: 4,
        mission: Some(4),
        group: ItemCategory::Key,
    },
    Item {
        id: 3,
        name: "Melancholy Soul",
        category: 4,
        mission: Some(5),
        group: ItemCategory::Key,
    },
    Item {
        id: 4,
        name: "Trident",
        category: 4,
        mission: Some(8),
        group: ItemCategory::Key,
    },
    Item {
        id: 5,
        name: "Guiding Light",
        category: 4,
        mission: Some(7),
        group: ItemCategory::Key,
    },
    Item {
        id: 6,
        name: "Pride of Lion",
        category: 4,
        mission: Some(4),
        group: ItemCategory::Key,
    },
    Item {
        id: 0,
        name: "Emblem Shield",
        category: 5,
        mission: Some(15),
        group: ItemCategory::Key,
    },
    Item {
        id: 1,
        name: "Knight Portrait", // Unused
        category: 5,
        mission: None,
        group: ItemCategory::Key,
    },
    Item {
        id: 2,
        name: "Sign of Chastity",
        category: 5,
        mission: Some(11),
        group: ItemCategory::Key,
    },
    Item {
        id: 3,
        name: "Sign of Humbleness", // Unused
        category: 5,
        mission: None,
        group: ItemCategory::Key,
    },
    Item {
        id: 4,
        name: "Remote", // Unused, Sign of Perseverance
        category: 5,
        mission: None,
        group: ItemCategory::Key,
    },
    Item {
        id: 5,
        name: "Chalice",
        category: 5,
        mission: Some(11),
        group: ItemCategory::Key,
    },
    Item {
        id: 6,
        name: "Pair of Lances",
        category: 5,
        mission: Some(15),
        group: ItemCategory::Key,
    },
    Item {
        id: 7,
        name: "Wheel of Destiny",
        category: 5,
        mission: Some(16),
        group: ItemCategory::Key,
    },
    Item {
        id: 0,
        name: "Token of Philosophy", // Unused
        category: 6,
        mission: None,
        group: ItemCategory::Key,
    },
    Item {
        id: 1,
        name: "Philosopher's Egg",
        category: 6,
        mission: Some(18),
        group: ItemCategory::Key,
    },
    Item {
        id: 2,
        name: "Elixir", // Tracks will mess up how this
        category: 6,
        mission: Some(19),
        group: ItemCategory::Key,
    },
    Item {
        id: 3,
        name: "Quicksilver",
        category: 6,
        mission: Some(17),
        group: ItemCategory::Key,
    },
    Item {
        id: 4,
        name: "Philosopher's Stone",
        category: 6,
        mission: Some(19),
        group: ItemCategory::Key,
    },
];

pub static ITEM_DATA_MAP: LazyLock<HashMap<&'static str, ItemData>> = LazyLock::new(|| {
    ALL_ITEMS
        .iter()
        .map(|item| (item.name, item.construct_item_data()))
        .collect()
});

pub fn get_items_by_category(category: ItemCategory) -> Vec<&'static str> {
    ALL_ITEMS
        .iter()
        .filter(|item| item.group == category)
        .map(|item| item.name)
        .collect()
}

pub fn find_item_by_data(data: &ItemData) -> Option<&'static str> {
    find_item_by_vals(data.id, data.category)
}

pub fn find_item_by_vals(id: u8, category: u8) -> Option<&'static str> {
    let results: Vec<_> = ALL_ITEMS
        .iter()
        .filter(|i| i.id == id && i.category == category)
        .collect();
    if results.is_empty() {
        None
    } else {
        Some(results[0].name)
    }
}

#[derive(Copy, Clone, strum_macros::Display, strum_macros::FromRepr)]
#[allow(dead_code)]
pub(crate) enum Difficulty {
    Easy = 2,
    Normal = 3,
    Hard = 5,
    #[strum(to_string = "Dante Must Die")]
    DanteMustDie = 6,
}

#[derive(Debug)]
pub struct ItemEntry {
    // Represents an item on the ground
    pub offset: usize,    // Offset for the item table
    pub room_number: i32, // Room number
    pub track_number: i32,
    pub item_id: u32, // Default Item ID
    pub mission: u32, // Mission Number
    pub coordinates: Coordinates,
}

pub const EMPTY_COORDINATES: Coordinates = Coordinates { x: 0, y: 0, z: 0 };

#[derive(Clone, Copy, Debug)]
pub struct Coordinates {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) z: u32,
}

impl Coordinates {
    pub fn has_coords(&self) -> bool {
        self.x > 0
    }
}

impl PartialEq for Coordinates {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z
    }
}

pub static MISSION_ITEM_MAP: LazyLock<HashMap<u32, Vec<&'static str>>> = LazyLock::new(|| {
    let mut map: HashMap<u32, Vec<&'static str>> = HashMap::new();
    for item in ALL_ITEMS.iter() {
        if let Some(mission) = item.mission {
            map.entry(mission).or_default().push(item.name);
        }
    }
    map
});

pub const GAME_NAME: &str = "Devil May Cry 1";
pub struct DMC1Config;
impl GameConfig for DMC1Config {
    const REMOTE_ID: u32 = 0x35;
    const GAME_NAME: &'static str = GAME_NAME;
}

pub static MELEE_MAP: LazyLock<BiMap<&str, u8>> = LazyLock::new(|| {
    let mut map = BiMap::new();
    map.insert("Alastor", 0);
    map.insert("Ifrit", 1);
    map.insert("Sparda Air", 2);
    map.insert("Sparda", 3);
    map.insert("Force Edge", 4);

    map
});
pub static GUN_MAP: LazyLock<BiMap<&str, u8>> = LazyLock::new(|| {
    let mut map = BiMap::new();
    map.insert("Handgun", 1);
    map.insert("Shotgun", 2);
    map.insert("Grenade Launcher", 3);
    map.insert("Nightmare Beta", 4);
    map.insert("Needlegun", 5);

    map
});

#[derive(Copy, Clone, strum_macros::Display, strum_macros::FromRepr)]
pub(crate) enum Rank {
    S = 0,
    A = 1,
    B = 2,
    C = 3,
    D = 4,
}
