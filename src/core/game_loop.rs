use super::cleanup_memory;
use crate::creeps::{replenish_creeps, Regulator};
use log::*;
use screeps::RoomName;
use std::collections::HashMap;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref REGULATORS: Mutex<HashMap<RoomName, Regulator>> = {
        let mut m = HashMap::new();

        for room in screeps::game::rooms::values() {
            let name = room.name();
            let mut regulator = Regulator::new(room);
            regulator.scan();
            m.insert(name, regulator);
        }

        Mutex::new(m)
    };
}

pub fn game_loop() {
    trace!("loop starting! CPU: {}", screeps::game::cpu::get_used());

    let spawned = replenish_creeps();
    if let Err(e) = spawned {
        warn!("couldn't spawn: {:?}", e);
    }

    for regulator in REGULATORS.lock().unwrap().values_mut() {
        if let Err(e) = regulator.distribute_creeps(spawned.unwrap_or(true)) {
            warn!("{}", e);
        }
    }

    let time = screeps::game::time();

    if time % 32 == 3 {
        debug!("running scan for tasks...");
        for r in REGULATORS.lock().unwrap().values_mut() {
            r.scan();
        }

        trace!("running memory cleanup");
        cleanup_memory().expect("expected Memory.creeps format to be a regular memory object");
    }

    trace!("done! cpu: {}", screeps::game::cpu::get_used())
}
