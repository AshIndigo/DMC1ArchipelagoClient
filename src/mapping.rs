use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use archipelago_rs::protocol::NetworkVersion;
use randomizer_utilities::archipelago_utilities::CONNECTED;
use randomizer_utilities::mapping_utilities::LocationData;
use std::sync::{LazyLock, RwLock};


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
            // TODO Can't remove starting Ebony & Ivory
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

#[derive(Deserialize, Serialize, Debug)]
pub struct Mapping {
    // For mapping JSON
    pub seed: String,
    pub items: HashMap<String, LocationData>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission_order: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_version: Option<NetworkVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_version: Option<NetworkVersion>
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum Goal {
    /// Beat M20 in linear order M1-M20 (Default)
    Standard,
    /// Beat all missions, all are unlocked at start
    All,
    /// Beat all missions in a randomized linear order
    RandomOrder,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum DeathlinkSetting {
    DeathLink, // Normal DeathLink Behavior
    HurtLink,  // Sends out DeathLink messages when you die. But only hurts you if you receive one
    Off,       // Don't send/receive DL related messages
}

pub(crate) fn parse_slot_data() -> Result<(), Box<dyn std::error::Error>> {
    match CONNECTED.read() {
        Ok(conn_opt) => {
            if let Some(connected) = conn_opt.as_ref() {
                MAPPING.write()?.replace(serde_path_to_error::deserialize(
                    connected.slot_data.clone(),
                )?);
                Ok(())
            } else {
                Err("No mapping found, cannot parse".into())
            }
        }
        Err(err) => Err(err.into()),
    }
}