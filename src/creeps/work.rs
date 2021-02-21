use log::*;
use screeps::{find, prelude::*, ResourceType, ReturnCode};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Couldn't harvest: `{0:?}`")]
    Harvest(ReturnCode),
    #[error("Creep has no controller!")]
    NoController(),
    #[error("Couldn't upgrade: `{0:?}`")]
    Upgrade(ReturnCode),
}

pub fn work() -> Result<(), Error> {
    debug!("running creeps");

    for creep in screeps::game::creeps::values() {
        let name = creep.name();
        debug!("running creep {}", name);
        if creep.spawning() {
            continue;
        }

        if creep.memory().bool("harvesting") {
            if creep.store_free_capacity(Some(ResourceType::Energy)) == 0 {
                creep.memory().set("harvesting", false);
            }
        } else {
            if creep.store_used_capacity(None) == 0 {
                creep.memory().set("harvesting", true);
            }
        }

        if creep.memory().bool("harvesting") {
            let source = &creep
                .room()
                .expect("room is not visible to you")
                .find(find::SOURCES)[0];
            if creep.pos().is_near_to(source) {
                let r = creep.harvest(source);
                if r != ReturnCode::Ok {
                    return Err(Error::Harvest(r));
                }
            } else {
                creep.move_to(source);
            }
        } else {
            if let Some(c) = creep
                .room()
                .expect("room is not visible to you")
                .controller()
            {
                let r = creep.upgrade_controller(&c);
                if r == ReturnCode::NotInRange {
                    creep.move_to(&c);
                } else if r != ReturnCode::Ok {
                    return Err(Error::Upgrade(r));
                }
            } else {
                return Err(Error::NoController());
            }
        }
    }

    Ok(())
}
