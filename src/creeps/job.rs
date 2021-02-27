use crate::core::constants;
use log::*;
use screeps::{ConstructionSite, Creep, HasId, Position, Source, Structure, StructureController};

#[derive(Clone)]
pub enum Job {
    Attack(Creep),
    Build(ConstructionSite),
    Harvest(Source),
    Heal(Creep),
    Maintain(Structure),
    Repair(Structure),
    Upgrade(StructureController),
}

impl Job {
    pub fn priority(&self) -> u32 {
        match self {
            Job::Attack(_) => constants::PRIORITY_ATTACK,
            Job::Build(_) => constants::PRIORITY_BUILDING,
            Job::Harvest(_) => constants::PRIORITY_HARVESTING,
            Job::Heal(_) => constants::PRIORITY_HEALING,
            Job::Maintain(_) => constants::PRIORITY_MAINTAINING,
            Job::Repair(_) => constants::PRIORITY_REPAIRING,
            Job::Upgrade(_) => constants::PRIORITY_UPGRADING,
        }
    }

    pub fn get_construction_site(&self) -> Option<ConstructionSite> {
        match self {
            Job::Build(c) => screeps::game::get_object_typed(c.id()).unwrap(),
            _ => {
                error!(
                    "Tried to get construction site when job is a {}",
                    self.get_type()
                );
                unimplemented!()
            }
        }
    }

    pub fn get_creep(&self) -> Option<Creep> {
        match self {
            Job::Attack(c) | Job::Heal(c) => screeps::game::get_object_typed(c.id()).unwrap(),
            _ => {
                error!("Tried to get creep when job is a {}", self.get_type());
                unimplemented!()
            }
        }
    }

    pub fn get_type(&self) -> &'static str {
        match self {
            Job::Attack(_) => "attack",
            Job::Build(_) => "build",
            Job::Harvest(_) => "harvest",
            Job::Heal(_) => "heal",
            Job::Maintain(_) => "maintain",
            Job::Repair(_) => "repair",
            Job::Upgrade(_) => "upgrade",
        }
    }

    pub fn get_range_to(&self, pos: Position) -> u32 {
        match self {
            Job::Attack(c) => pos.get_range_to(c),
            Job::Build(c) => pos.get_range_to(c),
            Job::Harvest(c) => pos.get_range_to(c),
            Job::Heal(c) => pos.get_range_to(c),
            Job::Maintain(c) => pos.get_range_to(c),
            Job::Repair(c) => pos.get_range_to(c),
            Job::Upgrade(c) => pos.get_range_to(c),
        }
    }

    pub fn get_source(&self) -> Option<Source> {
        match self {
            Job::Harvest(c) => screeps::game::get_object_typed(c.id()).unwrap(),
            _ => {
                error!("Tried to get source when job is a {}", self.get_type());
                unimplemented!()
            }
        }
    }

    pub fn get_structure(&self) -> Option<Structure> {
        match self {
            Job::Maintain(c) | Job::Repair(c) => screeps::game::get_object_typed(c.id()).unwrap(),
            _ => {
                error!("Tried to get structure when job is a {}", self.get_type());
                unimplemented!()
            }
        }
    }

    pub fn get_structure_controller(&self) -> StructureController {
        match self {
            Job::Upgrade(c) => screeps::game::get_object_typed(c.id()).unwrap().unwrap(),
            _ => {
                error!(
                    "Tried to get structure controller when job is a {}",
                    self.get_type()
                );
                unimplemented!()
            }
        }
    }
}
