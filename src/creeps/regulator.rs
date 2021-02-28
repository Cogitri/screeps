use super::{Creep, Job, JobOffer, Tower};
use crate::core::{constants, NumHelper};
use log::*;
use screeps::{
    constants::StructureType, find, prelude::*, Attackable, LookResult, Position, ResourceType,
    Room, Structure, StructureTower, Terrain,
};
use std::{collections::HashMap, convert::TryInto};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Couldn't make creep do action: `{0:?}`")]
    Creep(#[from] super::work::Error),
    #[error("Couldn't make tower do action `{0:?}`")]
    Tower(#[from] super::tower::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Regulator {
    creeps: HashMap<String, Creep>,
    towers: HashMap<String, Tower>,
    jobs: Vec<JobOffer>,
    room: Room,
}

impl Regulator {
    pub fn distribute_jobs(&mut self, respawned: bool) -> Result<()> {
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

        let towers = self
            .room
            .find(screeps::constants::find::MY_STRUCTURES)
            .into_iter()
            .filter_map(|s| match s.as_structure() {
                Structure::Tower(t) => Some(t),
                _ => None,
            })
            .collect::<Vec<StructureTower>>();

        // Filter out removed towers
        self.towers = self
            .towers
            .drain()
            .filter(|(id, _)| towers.iter().any(|t| &t.id().to_string() == id))
            .collect();

        for s_tower in towers {
            if let Some(tower) = self.towers.get_mut(&s_tower.id().to_string()) {
                tower.set_tower(s_tower);
                tower.select_job(&mut self.jobs)?;
            } else {
                let mut tower = Tower::from_tower(s_tower);
                tower.select_job(&mut self.jobs)?;
                self.towers.insert(tower.get_id(), tower);
            }
        }

        Ok(())
    }
    pub fn new(room: Room) -> Self {
        Self {
            creeps: screeps::game::creeps::values()
                .into_iter()
                .map(|c| (c.name(), Creep::from_creep(c)))
                .collect(),
            towers: room
                .find(screeps::constants::find::MY_STRUCTURES)
                .into_iter()
                .filter_map(|s| match s.as_structure() {
                    Structure::Tower(t) => Some((t.id().to_string(), Tower::from_tower(t))),
                    _ => None,
                })
                .collect(),
            jobs: Vec::new(),
            room,
        }
    }

    pub fn scan(&mut self) {
        self.jobs.clear();

        self.scan_attack_jobs();
        self.scan_build_jobs();
        self.scan_harvest_jobs();
        self.scan_heal_jobs();
        self.scan_maintain_jobs();
        self.scan_repair_jobs();
        self.scan_upgrade_jobs();
    }

    pub fn set_room(&mut self, room: Room) {
        self.room = room;
    }

    fn get_free_spots(&self, pos: Position, range: u32) -> u32 {
        let c = self
            .room
            .look_at_area(
                (pos.y() - range).limit_min(0),
                (pos.x() - range).limit_min(0),
                (pos.y() + range).limit_max(constants::ROOM_Y - 1),
                (pos.x() + range).limit_max(constants::ROOM_X - 1),
            )
            .into_iter()
            .filter(|res| match res.look_result {
                LookResult::Terrain(t) => t != Terrain::Wall,
                _ => false,
            })
            .count();

        debug!("{} free spots for job", c);

        c.try_into().unwrap()
    }

    fn scan_attack_jobs(&mut self) {
        self.jobs.append(
            &mut self
                .room
                .find(screeps::constants::find::HOSTILE_CREEPS)
                .into_iter()
                .map(|c| JobOffer::new(Job::Attack(c), 5))
                .collect(),
        )
    }

    fn scan_build_jobs(&mut self) {
        self.jobs.append(
            &mut self
                .room
                .find(screeps::constants::find::CONSTRUCTION_SITES)
                .into_iter()
                .map(|c| {
                    let spots = self.get_free_spots(c.pos(), constants::RANGE_BUILD);
                    JobOffer::new(Job::Build(c), spots)
                })
                .collect(),
        )
    }

    fn scan_harvest_jobs(&mut self) {
        self.jobs.append(
            &mut self
                .room
                .find(screeps::constants::find::SOURCES)
                .into_iter()
                .filter_map(|c| {
                    if c.energy() != 0 {
                        let spots = self.get_free_spots(c.pos(), constants::RANGE_HARVEST);
                        Some(JobOffer::new(Job::Harvest(c), spots))
                    } else {
                        None
                    }
                })
                .collect(),
        )
    }

    fn scan_heal_jobs(&mut self) {
        self.jobs.append(
            &mut self
                .room
                .find(screeps::constants::find::MY_CREEPS)
                .into_iter()
                .filter_map(|c| {
                    if c.hits() == c.hits_max() {
                        None
                    } else {
                        Some(JobOffer::new(Job::Heal(c), 1))
                    }
                })
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
                    if (typ == StructureType::Extension
                        || typ == StructureType::Spawn
                        || typ == StructureType::Tower)
                        && s.as_has_store()
                            .unwrap()
                            .store_free_capacity(Some(ResourceType::Energy))
                            != 0
                    {
                        let spots = self.get_free_spots(s.pos(), constants::RANGE_TRANSFER);
                        Some(JobOffer::new(Job::Maintain(s), spots))
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
                            return Some(JobOffer::new(Job::Repair(s), 1));
                        }
                    }
                    None
                })
                .collect(),
        )
    }

    fn scan_upgrade_jobs(&mut self) {
        if let Some(c) = self.room.controller() {
            let spots = self.get_free_spots(c.pos(), constants::RANGE_UPGRADE_CONTROLLER);
            self.jobs.push(JobOffer::new(Job::Upgrade(c), spots));
        }
    }
}
