use crate::game_manager;
use crate::game_manager::ArchipelagoData;
use std::collections::HashMap;
use std::ops::BitOrAssign;

use std::sync::LazyLock;

struct SkillData {
    id: usize,
    index: usize,
    flag: u8,
}

pub static ID_SKILL_MAP: LazyLock<HashMap<usize, &'static str>> = LazyLock::new(|| {
    let map: HashMap<usize, &'static str> = SKILLS_MAP
        .iter()
        .map(|(name, data)| (data.id, *name))
        .collect();
    map
});

static SKILLS_MAP: LazyLock<HashMap<&str, SkillData>> = LazyLock::new(|| {
    HashMap::from([
        (
            "Alastor - Stinger Level 1",
            SkillData {
                id: 100,
                index: 3,
                flag: 16,
            },
        ),
        (
            "Alastor - Stinger Level 2",
            SkillData {
                id: 101,
                index: 3,
                flag: 8,
            },
        ),
        (
            "Alastor - Round Trip",
            SkillData {
                id: 102,
                index: 3,
                flag: 32,
            },
        ),
        (
            "Alastor - Air Hike",
            SkillData {
                id: 103,
                index: 3,
                flag: 64,
            },
        ),
        (
            "Alastor - Air Raid",
            SkillData {
                id: 104,
                index: 3,
                flag: 1,
            },
        ),
        (
            "Alastor - Vortex Level 1",
            SkillData {
                id: 105,
                index: 3,
                flag: 4,
            },
        ),
        (
            "Alastor - Vortex Level 2",
            SkillData {
                id: 106,
                index: 3,
                flag: 2,
            },
        ),
        (
            "Ifrit - Rolling Blaze",
            SkillData {
                id: 107,
                index: 2,
                flag: 128,
            },
        ),
        (
            "Ifrit - Magma Drive",
            SkillData {
                id: 108,
                index: 2,
                flag: 64,
            },
        ),
        (
            "Ifrit - Kick 13 Level 1",
            SkillData {
                id: 109,
                index: 2,
                flag: 32,
            },
        ),
        (
            "Ifrit - Kick 13 Level 2",
            SkillData {
                id: 110,
                index: 2,
                flag: 16,
            },
        ),
        (
            "Ifrit - Meteor Level 1",
            SkillData {
                id: 111,
                index: 2,
                flag: 8,
            },
        ),
        (
            "Ifrit - Meteor Level 2",
            SkillData {
                id: 112,
                index: 2,
                flag: 4,
            },
        ),
        (
            "Ifrit - Inferno",
            SkillData {
                id: 113,
                index: 2,
                flag: 2,
            },
        ),
    ])
});
static DEFAULT_SKILLS: [u8; 4] = [0x0, 0x0, 0x0, 0x0]; // I should see what else this lets me control...

pub(crate) fn reset_expertise() {
    match game_manager::with_session(|s| {
        s.expertise = DEFAULT_SKILLS;
    }) {
        Ok(_) => {}
        Err(err) => {
            log::error!("Failed to reset expertise: {:?}", err);
        }
    }
}

fn give_skill(skill_name: &&'static str) {
    // This works, might not update files? need to double-check
    let data = SKILLS_MAP.get(skill_name).unwrap();
    game_manager::with_session(|s| {
        s.expertise[data.index].bitor_assign(data.flag);
    })
    .expect("Unable to give skill");
}

pub(crate) fn set_skills(data: &ArchipelagoData) {
    // I kinda don't like this tbh, but oh well, shouldn't really be an issue.
    reset_expertise();
    for skill in data.skills.iter() {
        give_skill(skill);
    }
}

// Certain skills have two levels they can gain
pub(crate) fn add_skill(id: usize, data: &mut ArchipelagoData) {
    match id {
        100 => {
            data.add_stinger_level();
        }
        105 => {
            data.add_vortex_level();
        }
        109 => {
            data.add_kick_13_level();
        }
        111 => {
            data.add_meteor_level();
        }
        _ => {}
    }

    let skill_name = match id {
        100 => match data.stinger_level {
            1 => 100,
            2 => 101,
            _ => unreachable!(),
        },
        105 => match data.vortex_level {
            1 => 105,
            2 => 106,
            _ => unreachable!(),
        },
        109 => match data.kick_13_level {
            1 => 109,
            2 => 110,
            _ => unreachable!(),
        },
        111 => match data.meteor_level {
            1 => 111,
            2 => 112,
            _ => unreachable!(),
        },
        _ => id,
    };
    data.add_skill(ID_SKILL_MAP.get(&skill_name).unwrap());
}
