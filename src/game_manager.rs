use crate::constants::Difficulty;
use crate::mapping::MAPPING;
use crate::utilities::DMC1_ADDRESS;
use randomizer_utilities::read_data_from_address;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::sync::{LazyLock, RwLock};

#[derive(Debug, Default)]
pub(crate) struct ArchipelagoData {
    pub(crate) blue_orbs: i32,
    pub(crate) purple_orbs: i32,
    pub(crate) dt_unlocked: bool,
    pub(crate) stinger_level: u8,
    pub(crate) vortex_level: u8,
    pub(crate) kick_13_level: u8,
    pub(crate) meteor_level: u8,
    pub(crate) items: HashSet<&'static str>,
    pub(crate) skills: HashSet<&'static str>,
}

pub static ARCHIPELAGO_DATA: LazyLock<RwLock<ArchipelagoData>> =
    LazyLock::new(|| RwLock::new(ArchipelagoData::default()));

impl ArchipelagoData {
    pub fn add_item(&mut self, item: &'static str) {
        self.items.insert(item);
    }

    pub fn add_skill(&mut self, item: &'static str) {
        self.skills.insert(item);
    }

    pub(crate) fn add_blue_orb(&mut self) {
        self.blue_orbs = (self.blue_orbs + 1).min(14);
    }

    pub(crate) fn add_purple_orb(&mut self) {
        self.purple_orbs = (self.purple_orbs + 1).min(10);
        if let Some(mappings) = MAPPING.read().unwrap().as_ref() {
            if !mappings.devil_trigger_mode {
                self.dt_unlocked = true;
            }
        }
    }

    pub(crate) fn add_dt(&mut self) {
        if let Some(mappings) = MAPPING.read().unwrap().as_ref() {
            if mappings.devil_trigger_mode {
                self.dt_unlocked = true;
            }
            if !mappings.purple_orb_mode {
                self.purple_orbs = (self.purple_orbs + 3).min(10);
            }
        }
    }

    pub(crate) fn add_stinger_level(&mut self) {
        self.stinger_level = (self.stinger_level + 1).min(2);
    }

    pub(crate) fn add_vortex_level(&mut self) {
        self.vortex_level = (self.vortex_level + 1).min(2);
    }
    pub(crate) fn add_kick_13_level(&mut self) {
        self.kick_13_level = (self.kick_13_level + 1).min(2);
    }

    pub(crate) fn add_meteor_level(&mut self) {
        self.meteor_level = (self.meteor_level + 1).min(2);
    }
}

pub(crate) const GAME_SESSION_DATA: usize = 0x5EAB88;

#[repr(C)]
#[derive(Debug)]
pub struct ItemData {
    // Size 4
    pub category: u8,
    pub id: u8,
    pub count: u16,
}

impl PartialEq<&ItemData> for ItemData {
    fn eq(&self, other: &&ItemData) -> bool {
        self.category == other.category && self.id == other.id
    }
}

impl PartialEq for ItemData {
    fn eq(&self, other: &Self) -> bool {
        self.category == other.category && self.id == other.id
    }
}

impl Display for ItemData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Category: {}, ID: {}, Count: {}",
            self.category, self.id, self.count
        )
    }
}

#[repr(C)]
pub struct SessionData {
    // Most of these don't matter, but I need to match with the struct in game
    buttons: [u16; 4],
    unknown: [u8; 46],
    right_stick_x: u8,
    right_stick_y: u8,
    left_stick_x: u8,
    left_stick_y: u8,
    unknown2: [u8; 86],
    var_90: [u32; 5],
    unknown3: [u8; 7168],
    event: u32,
    unknown4: [u8; 444],
    mission: u8,
    unknown5: u8,
    difficulty: u8,
    character: u8,
    unknown6: [u8; 288],
    var_1f88: u32,
    unknown7: [u8; 8],
    var_1f94: u32,
    var_1f98: bool,
    unknown8: [u8; 3],
    var_1f9c: u32,
    unknown9: [u8; 52],
    var_1fd4: u32,
    unknown10: [u8; 104],
    state: u32,
    unknown11: [u8; 55],
    pub(crate) item_count: u8,
    unknown12: [u8; 20],
    pub(crate) item_data: [ItemData; size_of::<ItemData>() + (976 / size_of::<ItemData>())], // This... does not seem like a good idea (245?
    //unknown13: [u8; 976],
    pub(crate) hp: u8,
    magic: u8,
    unknown14: [u8; 2],
    pub(crate) expertise: [u8; 4], // ?? ?? Ifrit Alastor
    unknown15: [u8; 8],
    red_orbs: u32,
    unknown16: [u8; 4],
    orb_flags: u32,
}

/// Error type for session access
#[derive(Debug)]
pub enum SessionError {
    NotUsable, // If a save slot has not been loaded for whatever reason
}

static SESSION_PTR: LazyLock<usize> = LazyLock::new(|| *DMC1_ADDRESS + GAME_SESSION_DATA);

pub fn with_session_read<F, R>(f: F) -> Result<R, SessionError>
where
    F: FnOnce(&SessionData) -> R,
{
    let addr = *SESSION_PTR;
    unsafe {
        let ptr_to_data = read_data_from_address::<*const SessionData>(addr);
        if ptr_to_data.is_null() {
            return Err(SessionError::NotUsable);
        }
        let s =  &*(ptr_to_data);
        if !session_is_valid(s) {
            return Err(SessionError::NotUsable);
        }
        Ok(f(s))
    }
}

pub fn with_session<F, R>(f: F) -> Result<R, SessionError>
where
    F: FnOnce(&mut SessionData) -> R,
{
    let addr = *SESSION_PTR;
    unsafe {
        let ptr_to_data = read_data_from_address::<*mut SessionData>(addr);
        if ptr_to_data.is_null() {
            return Err(SessionError::NotUsable);
        }
        let s = &mut *(ptr_to_data);
        if !session_is_valid(s) {
            return Err(SessionError::NotUsable);
        }
        Ok(f(s))
    }
}

fn session_is_valid(_s: &SessionData) -> bool {
    true
}

/// Get current mission
pub fn get_mission() -> u8 {
    with_session_read(|s| s.mission).unwrap()
}

/// Get current difficulty
pub fn get_difficulty() -> Difficulty {
    Difficulty::from_repr(with_session_read(|s| s.difficulty).unwrap() as usize).unwrap()
}

// Player data

#[repr(C)]
struct Vec4 {
    x: f32,
    y: f32,
    z: f32,
    a: f32,
}

#[repr(C)]
pub struct PlayerData {
    state: [u8; 8],
    unknown: [u8; 284],
    rotation: f32,
    unknown2: [u8; 72],
    position: Vec4,
    unknown3: [u8; 5666],
    pub(crate) hp: u16,
    unknown4: [u8; 620],
    idle_timer: u16,
    unknown5: [u8; 438],
    pub(crate) max_hp: u16,
    unknown6: [u8; 2],
    pub(crate) melee: u8,
    unknown7: u8,
    magic_human: u16,
    max_magic_human: u16,
    magic_demon: u16,
    max_magic_demon: u16,
    unknown8: [u8; 114],
    pub(crate) gun: u8,
    unknown9: [u8; 3],
    melee_form: u8,
    unknown10: [u8; 25],
    charge_timer: [i16; 2],
}

/// Error type for player data access
#[derive(Debug)]
pub enum PlayerDataError {
    NotUsable, // Player data is unavailable, chances are this is because we're on the main menu
}

pub(crate) const PLAYER_DATA: usize = 0x60ACD0;
static PLAYER_PTR: LazyLock<usize> = LazyLock::new(|| *DMC1_ADDRESS + PLAYER_DATA);

pub fn with_active_player_data_read<F, R>(f: F) -> Result<R, PlayerDataError>
where
    F: FnOnce(&PlayerData) -> R,
{
    let addr = *PLAYER_PTR;
    unsafe {
        let ptr_to_data = read_data_from_address::<*const PlayerData>(addr);
        if ptr_to_data.is_null() {
            return Err(PlayerDataError::NotUsable);
        }
        let s =  &*(ptr_to_data);
        if !player_data_valid(s) {
            return Err(PlayerDataError::NotUsable);
        }
        Ok(f(s))
    }
}

pub fn with_active_player_data<F, R>(f: F) -> Result<R, PlayerDataError>
where
    F: FnOnce(&mut PlayerData) -> R,
{
    let addr = *PLAYER_PTR;
    unsafe {
        let ptr_to_data = read_data_from_address::<*mut PlayerData>(addr);
        if ptr_to_data.is_null() {
            return Err(PlayerDataError::NotUsable);
        }
        let s = &mut *(ptr_to_data);
        if !player_data_valid(s) {
            return Err(PlayerDataError::NotUsable);
        }
        Ok(f(s))
    }
}

fn player_data_valid(_s: &PlayerData) -> bool {
    if *PLAYER_PTR != 0 {
        return true;
    }
    false
}

#[repr(C)]
pub struct EventData {
    unknown: [u8; 624],
    track: u32,
    room: u32,
    unknown1: [u8; 16],
    next_track: u32,
    next_room: u32,
}

const EVENT_DATA: usize = 0x60B148;

static EVENT_DATA_PTR: LazyLock<usize> = LazyLock::new(|| *DMC1_ADDRESS + EVENT_DATA);

pub fn with_event_data_read<F, R>(f: F) -> Result<R, PlayerDataError>
where
    F: FnOnce(&EventData) -> R,
{
    let addr = *EVENT_DATA_PTR;
    unsafe {
        let s = &*(read_data_from_address::<*const EventData>(addr));
        if !event_data_valid(s) {
            return Err(PlayerDataError::NotUsable);
        }
        Ok(f(s))
    }
}

fn event_data_valid(_s: &EventData) -> bool {
    if *EVENT_DATA_PTR != 0 {
        return true;
    }
    false
}

/// Get current room
pub fn get_room() -> i32 {
    with_event_data_read(|s| s.room).unwrap() as i32
}

/// Get the current track
/// Tracks:
/// - Track 1: Castle area
/// - Track 2: Outside castle
/// - Track 3: Evil Castle IIRC?
/// - Track 4: Hell
/// - Track 5: Boat
pub fn get_track() -> i32 {
    with_event_data_read(|s| s.track).unwrap() as i32
}