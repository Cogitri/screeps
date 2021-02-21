use super::cleanup_memory;
use crate::creeps::{replenish_creeps, work};
use log::*;

pub fn game_loop() {
    trace!("loop starting! CPU: {}", screeps::game::cpu::get_used());

    if let Err(e) = replenish_creeps() {
        warn!("couldn't spawn: {:?}", e);
    }

    if let Err(e) = work() {
        warn!("{}", e);
    }

    let time = screeps::game::time();

    if time % 32 == 3 {
        trace!("running memory cleanup");
        cleanup_memory().expect("expected Memory.creeps format to be a regular memory object");
    }

    trace!("done! cpu: {}", screeps::game::cpu::get_used())
}
