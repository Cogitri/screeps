use super::{Creep, Job, JobOffer};
use log::*;
use screeps::{constants::StructureType, find, prelude::*, ResourceType, Room};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Couldn't make creep do action: `{0:?}`")]
    Creep(#[from] super::work::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Regulator {
    creeps: HashMap<String, Creep>,
    jobs: Vec<JobOffer>,
    room: Room,
}

impl Regulator {
    pub fn distribute_creeps(&mut self, respawned: bool) -> Result<()> {
        let creeps = screeps::game::creeps::values();

        if respawned {
            // Remove dead creeps
            self.creeps = self
                .creeps
                .drain()
                .filter(|(name, _)| creeps.iter().any(|c| &c.name() == name))
                .collect();
        }

        for s_creep in creeps {
            if let Some(creep) = self.creeps.get_mut(&s_creep.name()) {
                creep.set_creep(s_creep);
                creep.select_job(&mut self.jobs)?
            } else {
                let mut creep = Creep::from_creep(s_creep);
                creep.select_job(&mut self.jobs)?;
                self.creeps.insert(creep.get_name(), creep);
            }
        }

        Ok(())
    }

    pub fn new(room: Room) -> Self {
        let mut m = HashMap::new();

        for creep in screeps::game::creeps::values() {
            m.insert(creep.name(), Creep::from_creep(creep));
        }

        Self {
            creeps: m,
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
