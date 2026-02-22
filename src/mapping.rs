use crate::data::generated_locations;
use archipelago_rs::{Client, CreateAsHint, Location};
use randomizer_utilities::{APVersion, archipelago_utilities};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::sync::{LazyLock, RwLock};
pub static OVERLAY_INFO: LazyLock<RwLock<OverlayInfo>> =
    LazyLock::new(|| RwLock::new(OverlayInfo::default()));

#[derive(Debug, Default)]
pub struct OverlayInfo {
    pub client_version: Option<APVersion>,
    pub generated_version: Option<APVersion>,
}

pub static MAPPING: LazyLock<RwLock<Option<Mapping>>> = LazyLock::new(|| RwLock::new(None));

fn default_gun() -> String {
    "Handgun".to_string()
}

fn default_melee() -> String {
    "Force Edge".to_string()
}

fn default_goal() -> Goal {
    Goal::Standard
}

/// Converts the option number from the slot data into a more usable gun name
fn parse_gun_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let val = Value::deserialize(deserializer)?;
    match val {
        Value::Number(n) => match n.as_i64().unwrap_or_default() {
            0 => Ok("Handgun".to_string()),
            1 => Ok("Shotgun".to_string()),
            // Needlegun wouldn't be usable
            //2 => Ok("Needlegun".to_string()),
            3 => Ok("Grenade Launcher".to_string()),
            4 => Ok("Nightmare Beta".to_string()),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid gun number: {}",
                n
            ))),
        },
        Value::String(s) => Ok(s),
        other => Err(serde::de::Error::custom(format!(
            "Unexpected type: {:?}",
            other
        ))),
    }
}

/// Converts the option number from the slot data into a more usable melee name
fn parse_melee_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let val = Value::deserialize(deserializer)?;
    match val {
        Value::Number(n) => match n.as_i64().unwrap_or_default() {
            0 => Ok("Force Edge".to_string()),
            1 => Ok("Alastor".to_string()),
            2 => Ok("Ifrit".to_string()),
            3 => Ok("Sparda".to_string()),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid melee number: {}",
                n
            ))),
        },
        Value::String(s) => Ok(s),
        other => Err(serde::de::Error::custom(format!(
            "Unexpected type: {:?}",
            other
        ))),
    }
}

/// Figure out which DL setting were on
fn parse_death_link<'de, D>(deserializer: D) -> Result<DeathlinkSetting, D::Error>
where
    D: Deserializer<'de>,
{
    let val = Value::deserialize(deserializer)?;
    match val {
        Value::Number(n) => match n.as_i64().unwrap_or_default() {
            0 => Ok(DeathlinkSetting::Off),
            1 => Ok(DeathlinkSetting::DeathLink),
            2 => Ok(DeathlinkSetting::HurtLink),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid DL option: {}",
                n
            ))),
        },
        other => Err(serde::de::Error::custom(format!(
            "Unexpected type: {:?}",
            other
        ))),
    }
}

/// Parse which goal we are on
fn parse_goal<'de, D>(deserializer: D) -> Result<Goal, D::Error>
where
    D: Deserializer<'de>,
{
    let val = Value::deserialize(deserializer)?;
    match val {
        Value::Number(n) => match n.as_i64().unwrap_or_default() {
            0 => Ok(Goal::Standard),
            1 => Ok(Goal::All),
            2 => Ok(Goal::RandomOrder),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid goal option: {}",
                n
            ))),
        },
        other => Err(serde::de::Error::custom(format!(
            "Unexpected type: {:?}",
            other
        ))),
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Mapping {
    // For mapping JSON
    pub starter_items: Vec<String>,
    #[serde(default = "default_melee")]
    #[serde(deserialize_with = "parse_melee_number")]
    pub start_melee: String,
    #[serde(default = "default_gun")]
    #[serde(deserialize_with = "parse_gun_number")]
    pub start_gun: String,
    pub randomize_skills: bool,
    pub purple_orb_mode: bool,
    pub devil_trigger_mode: bool,
    #[serde(deserialize_with = "parse_death_link")]
    pub death_link: DeathlinkSetting,
    #[serde(default = "default_goal")]
    #[serde(deserialize_with = "parse_goal")]
    pub goal: Goal,
    pub mission_order: Option<Vec<u8>>,
    pub generated_version: Option<APVersion>,
    pub client_version: Option<APVersion>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum Goal {
    /// Beat M20 in linear order M1-M20 (Default)
    Standard,
    /// Beat all missions, all are unlocked at start
    All,
    /// Beat all missions in a randomized linear order
    RandomOrder,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum DeathlinkSetting {
    DeathLink, // Normal DeathLink Behavior
    HurtLink,  // Sends out DeathLink messages when you die. But only hurts you if you receive one
    Off,       // Don't send/receive DL related messages
}

pub fn run_scouts_for_mission(client: &mut Client<Mapping>, mission: u32, hint: CreateAsHint) {
    archipelago_utilities::run_scouts(
        client.scout_locations(get_locations_by_mission(client, mission), hint),
    );
}
pub fn run_scouts_for_secret_mission(client: &mut Client<Mapping>) {
    archipelago_utilities::run_scouts(
        client.scout_locations(get_secret_missions(client), CreateAsHint::No),
    );
}

pub fn get_locations_by_mission(client: &Client<Mapping>, mission: u32) -> Vec<Location> {
    let current_game = client.this_game();
    generated_locations::ITEM_MISSION_MAP
        .iter()
        .filter(|(_k, v)| v.mission == mission)
        .filter_map(|(k, _v)| current_game.location_by_name(*k))
        .collect()
}

pub fn get_secret_missions(client: &Client<Mapping>) -> Vec<Location> {
    let current_game = client.this_game();
    generated_locations::ITEM_MISSION_MAP
        .iter()
        .filter(|(k, _v)| k.contains("Secret Mission"))
        .filter_map(|(k, _v)| current_game.location_by_name(*k))
        .collect()
}
