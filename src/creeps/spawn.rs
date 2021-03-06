use crate::core::constants;
use log::*;
use screeps::{prelude::*, Part, ReturnCode};

struct BodyParts {
    mode: u8,
}

impl BodyParts {
    pub fn new() -> Self {
        Self { mode: 0 }
    }
}

impl Iterator for BodyParts {
    type Item = Part;

    fn next(&mut self) -> Option<Self::Item> {
        match self.mode {
            3 => {
                self.mode += 1;
                Some(Part::Carry)
            }
            4 => {
                self.mode = 0;
                Some(Part::Move)
            }
            _ => {
                self.mode += 1;
                Some(Part::Work)
            }
        }
    }
}

pub fn replenish_creeps() -> Result<bool, ReturnCode> {
    debug!("running spawns");

    if screeps::game::creeps::keys().len() >= constants::MAX_CREEPS {
        debug!("Enough creeps spawned, not spawning more");
        return Ok(false);
    }

    for spawn in screeps::game::spawns::values() {
        debug!("running spawn {}", spawn.name());

        let room = spawn.room().expect("room isn't visible");

        debug!(
            "room available energy: {}, capacity: {}",
            room.energy_available(),
            room.energy_capacity_available()
        );

        if room.energy_available() < room.energy_capacity_available() {
            debug!("Waiting for spawn to be full to spawn big mob");
            return Ok(false);
        }

        let mut body = vec![Part::Move, Part::Move, Part::Carry, Part::Work];
        let energy = room.energy_available();
        let mut sum = body.iter().map(|p| p.cost()).sum();
        let mut iter = BodyParts::new();
        let mut next = iter.next().unwrap();
        let mut num_bodyparts = 0;

        while energy >= (sum + next.cost()) && num_bodyparts <= constants::MAX_CREEPS_BODY_PARTS {
            body.push(next);
            sum = body.iter().map(|p| p.cost()).sum();
            next = iter.next().unwrap();
            num_bodyparts += 1;
        }

        if energy >= sum {
            // create a unique name, spawn.
            let name_base = screeps::game::time();
            let mut additional = 0;
            let res = loop {
                let name = format!("{}-{}", name_base, additional);
                let res = spawn.spawn_creep(&body, &name);

                if res == ReturnCode::NameExists {
                    additional += 1;
                } else {
                    break res;
                }
            };

            if res == ReturnCode::Ok {
                return Ok(true);
            } else {
                return Err(res);
            }
        } else {
            warn!("Not enough energy!");
        }
    }

    Ok(false)
}
