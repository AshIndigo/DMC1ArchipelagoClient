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

fn default_goal() -> Goal {
    Goal::Standard
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

fn parse_hint<'de, D>(deserializer: D) -> Result<AutoHint, D::Error>
where
    D: Deserializer<'de>,
{
    let val = Value::deserialize(deserializer)?;
    match val {
        Value::Number(n) => match AutoHint::from_repr(n.as_i64().unwrap_or_default() as usize) {
            None => Err(serde::de::Error::custom(format!(
                "Invalid autohint option: {}",
                n
            ))),
            Some(n) => Ok(n),
        },
        other => Err(serde::de::Error::custom(format!(
            "Unexpected type: {:?}",
            other
        ))),
    }
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize,
    PartialEq,
    PartialOrd,
    strum_macros::Display,
    strum_macros::FromRepr,
)]
pub enum AutoHint {
    All,
    Current,
    /// Only Relevant for weapons/guns
    Obtained,
    #[default]
    None,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Mapping {
    pub randomize_skills: bool,
    pub purple_orb_mode: bool,
    pub devil_trigger_mode: bool,
    #[serde(deserialize_with = "parse_death_link")]
    pub death_link: DeathlinkSetting,
    #[serde(default = "default_goal")]
    #[serde(deserialize_with = "parse_goal")]
    pub goal: Goal,
    pub shop_orb_checks: bool,
    #[serde(deserialize_with = "parse_hint")]
    pub auto_orb_hints: AutoHint,
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
