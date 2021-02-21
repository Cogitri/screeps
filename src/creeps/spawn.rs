use crate::core::constants;
use log::*;
use screeps::{prelude::*, Part, ReturnCode};

pub fn replenish_creeps() -> Result<(), ReturnCode> {
    debug!("running spawns");

    if screeps::game::creeps::keys().len() <= constants::MAX_CREEPS {
        debug!("Enough creeps spawned, not spawning more");
        return Ok(());
    }

    for spawn in screeps::game::spawns::values() {
        debug!("running spawn {}", spawn.name());
        let body = [Part::Move, Part::Move, Part::Carry, Part::Work];

        if spawn.energy() >= body.iter().map(|p| p.cost()).sum() {
            // create a unique name, spawn.
            let name_base = screeps::game::time();
            let mut additional = 0;
            let (name, res) = loop {
                let name = format!("{}-{}", name_base, additional);
                let res = spawn.spawn_creep(&body, &name);

                if res == ReturnCode::NameExists {
                    additional += 1;
                } else {
                    break (name, res);
                }
            };

            if res == ReturnCode::Ok {
                screeps::game::creeps::get(&name)
                    .unwrap()
                    .memory()
                    .set("harvesting", true);
            } else {
                return Err(res);
            }
        }
    }

    Ok(())
}
