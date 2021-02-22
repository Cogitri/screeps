use super::{Creep, Job, JobOffer};
use log::*;
use screeps::{constants::StructureType, find, prelude::*, ResourceType, Room};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Couldn't make creep do action: `{0:?}`")]
    Creep(#[from] super::work::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Regulator {
    jobs: Vec<JobOffer>,
    room: Room,
}

impl Regulator {
    pub fn distribute_creeps(&mut self) -> Result<()> {
        // FIXME: We probably don't have to run this every tick!
        self.scan();

        for creep in screeps::game::creeps::values() {
            Creep::from_creep(creep).select_job(&mut self.jobs)?
        }

        Ok(())
    }

    pub fn new(room: Room) -> Self {
        Self {
            jobs: Vec::new(),
            room,
        }
    }

    pub fn scan(&mut self) {
        self.jobs.clear();

        self.scan_build_jobs();
        self.scan_harvest_jobs();
        self.scan_maintain_jobs();
        self.scan_repair_jobs();
        self.scan_upgrade_jobs();
    }

    fn scan_build_jobs(&mut self) {
        self.jobs.append(
            &mut self
                .room
                .find(screeps::constants::find::CONSTRUCTION_SITES)
                .into_iter()
                .map(|c| JobOffer::new(Job::Build(c)))
                .collect(),
        )
    }

    fn scan_harvest_jobs(&mut self) {
        self.jobs.append(
            &mut self
                .room
                .find(screeps::constants::find::SOURCES)
                .into_iter()
                .map(|c| JobOffer::new(Job::Harvest(c)))
                .collect(),
        )
    }

    fn scan_maintain_jobs(&mut self) {
        self.jobs.append(
            &mut self
                .room
                .find(screeps::constants::find::STRUCTURES)
                .into_iter()
                .filter_map(|s| {
                    let typ = s.structure_type();
                    if (typ == StructureType::Extension || typ == StructureType::Spawn)
                        && s.as_has_store()
                            .unwrap()
                            .store_free_capacity(Some(ResourceType::Energy))
                            != 0
                    {
                        Some(JobOffer::new(Job::Maintain(s)))
                    } else {
                        None
                    }
                })
                .collect(),
        )
    }

    fn scan_repair_jobs(&mut self) {
        self.jobs.append(
            &mut self
                .room
                .find(find::STRUCTURES)
                .into_iter()
                .filter_map(|s| {
                    if let Some(a) = s.as_attackable() {
                        let hits = a.hits();
                        if hits != 0
                            && hits < self.room.energy_capacity_available()
                            && hits < a.hits_max()
                        {
                            return Some(JobOffer::new(Job::Repair(s)));
                        }
                    }
                    None
                })
                .collect(),
        )
    }

    fn scan_upgrade_jobs(&mut self) {
        if let Some(c) = self.room.controller() {
            self.jobs.push(JobOffer::new(Job::Upgrade(c)));
        }
    }
}
