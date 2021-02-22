use crate::core::constants;
use screeps::{ConstructionSite, Position, Source, Structure, StructureController};

#[derive(Clone)]
pub enum Job {
    Build(ConstructionSite),
    Harvest(Source),
    Maintain(Structure),
    Repair(Structure),
    Upgrade(StructureController),
}

impl Job {
    pub fn priority(&self) -> u32 {
        match self {
            Job::Build(_) => constants::PRIORITY_BUILDING,
            Job::Harvest(_) => constants::PRIORITY_HARVESTING,
            Job::Maintain(_) => constants::PRIORITY_MAINTAINING,
            Job::Repair(_) => constants::PRIORITY_REPAIRING,
            Job::Upgrade(_) => constants::PRIORITY_UPGRADING,
        }
    }

    pub fn get_construction_site(&self) -> &ConstructionSite {
        match self {
            Job::Build(c) => c,
            _ => unimplemented!(),
        }
    }

    pub fn get_range_to(&self, pos: Position) -> u32 {
        match self {
            Job::Build(c) => pos.get_range_to(c),
            Job::Harvest(c) => pos.get_range_to(c),
            Job::Maintain(c) => pos.get_range_to(c),
            Job::Repair(c) => pos.get_range_to(c),
            Job::Upgrade(c) => pos.get_range_to(c),
        }
    }

    pub fn get_source(&self) -> &Source {
        match self {
            Job::Harvest(c) => c,
            _ => unimplemented!(),
        }
    }

    pub fn get_structure(&self) -> &Structure {
        match self {
            Job::Maintain(c) | Job::Repair(c) => c,
            _ => unimplemented!(),
        }
    }

    pub fn get_structure_controller(&self) -> &StructureController {
        match self {
            Job::Upgrade(c) => c,
            _ => unimplemented!(),
        }
    }
}
