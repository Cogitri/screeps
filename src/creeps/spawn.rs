use log::*;
use screeps::{prelude::*, Part, ReturnCode};

pub fn replenish_creeps() -> Result<(), ReturnCode> {
    debug!("running spawns");

    for spawn in screeps::game::spawns::values() {
        debug!("running spawn {}", spawn.name());
        let body = [Part::Move, Part::Move, Part::Carry, Part::Work];

        if spawn.energy() >= body.iter().map(|p| p.cost()).sum() {
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

            if res != ReturnCode::Ok {
                return Err(res);
            }
        }
    }

    Ok(())
}
