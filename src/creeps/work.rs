use crate::core::constants;
use log::*;
use screeps::{
    constants::StructureType, find, prelude::*, ConstructionSite, ResourceType, ReturnCode,
    Structure,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Couldn't build: `{0:?}`")]
    Build(ReturnCode),
    #[error("Couldn't harvest: `{0:?}`")]
    Harvest(ReturnCode),
    #[error("Couldn't maintain: `{0:?}`")]
    Maintain(ReturnCode),
    #[error("Couldn't move: `{0:?}`")]
    Move(ReturnCode),
    #[error("Creep has no controller!")]
    NoController(),
    #[error("Couldn't repair: `{0:?}`")]
    Repair(ReturnCode),
    #[error("Couldn't upgrade: `{0:?}`")]
    Upgrade(ReturnCode),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Role {
    Building,
    Harvesting,
    Maintainer,
    Repairing,
    Upgrading,
}

struct Creep {
    inner: screeps::Creep,
    role: Role,
}

type Result<T> = std::result::Result<T, crate::creeps::work::Error>;

impl Creep {
    pub fn do_action(&self) -> Result<()> {
        debug!("running creep {}", self.inner.name());

        if self.inner.spawning() {
            return Ok(());
        }

        match self.role {
            Role::Building => self.build(),
            Role::Harvesting => self.harvest(),
            Role::Maintainer => self.maintain(),
            Role::Repairing => self.repair(),
            Role::Upgrading => self.upgrade(),
        }
    }

    pub fn from_creep(inner: screeps::Creep) -> Self {
        let memory = inner.memory();
        let role = if memory.bool(constants::ROLE_BUILDING) {
            Role::Building
        } else if memory.bool(constants::ROLE_HARVESTING) {
            Role::Harvesting
        } else if memory.bool(constants::ROLE_MAINTAINING) {
            Role::Maintainer
        } else if memory.bool(constants::ROLE_UPGRADING) {
            Role::Upgrading
        } else if memory.bool(constants::ROLE_REPAIRING) {
            Role::Repairing
        } else {
            error!("Unknown role, falling back to harvesting!");
            unimplemented!()
        };

        Self { inner, role }
    }

    fn enable_building(&self) {
        assert_eq!(self.inner.say("Building!", false), ReturnCode::Ok);

        self.inner.memory().set(constants::ROLE_BUILDING, true);
        self.inner.memory().set(constants::ROLE_HARVESTING, false);
        self.inner.memory().set(constants::ROLE_MAINTAINING, false);
        self.inner.memory().set(constants::ROLE_REPAIRING, false);
        self.inner.memory().set(constants::ROLE_UPGRADING, false);
    }

    fn enable_maintaining(&self) {
        assert_eq!(self.inner.say("Maintaining!", false), ReturnCode::Ok);

        self.inner.memory().set(constants::ROLE_BUILDING, false);
        self.inner.memory().set(constants::ROLE_HARVESTING, false);
        self.inner.memory().set(constants::ROLE_MAINTAINING, true);
        self.inner.memory().set(constants::ROLE_REPAIRING, false);
        self.inner.memory().set(constants::ROLE_UPGRADING, false);
    }

    fn enable_harvesting(&self) {
        assert_eq!(self.inner.say("Harvesting!", false), ReturnCode::Ok);

        self.inner.memory().set(constants::ROLE_BUILDING, false);
        self.inner.memory().set(constants::ROLE_HARVESTING, true);
        self.inner.memory().set(constants::ROLE_MAINTAINING, false);
        self.inner.memory().set(constants::ROLE_REPAIRING, false);
        self.inner.memory().set(constants::ROLE_UPGRADING, false);
    }

    fn enable_repair(&self) {
        assert_eq!(self.inner.say("Reparing!", false), ReturnCode::Ok);

        self.inner.memory().set(constants::ROLE_BUILDING, false);
        self.inner.memory().set(constants::ROLE_HARVESTING, false);
        self.inner.memory().set(constants::ROLE_MAINTAINING, false);
        self.inner.memory().set(constants::ROLE_REPAIRING, true);
        self.inner.memory().set(constants::ROLE_UPGRADING, false);
    }

    fn enable_upgrading(&self) {
        assert_eq!(self.inner.say("Upgrading!", false), ReturnCode::Ok);

        self.inner.memory().set(constants::ROLE_BUILDING, false);
        self.inner.memory().set(constants::ROLE_HARVESTING, false);
        self.inner.memory().set(constants::ROLE_MAINTAINING, false);
        self.inner.memory().set(constants::ROLE_REPAIRING, false);
        self.inner.memory().set(constants::ROLE_UPGRADING, true);
    }

    fn build(&self) -> Result<()> {
        assert_eq!(self.role, Role::Building);

        debug!("Running build");

        if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("No energy left; switching to harvesting");
            self.enable_harvesting();
        } else if let Some(c) = self.get_buildable_structures().first() {
            let r = self.inner.build(&c);
            if r == ReturnCode::NotInRange {
                // FIXME: Handle moving to other construction site
                self.move_to(c)?;
            } else if r != ReturnCode::Ok {
                return Err(Error::Build(r));
            }
        } else {
            debug!("No construction sites, switching to upgrading");
            self.enable_upgrading()
        }

        Ok(())
    }

    fn get_buildable_structures(&self) -> Vec<ConstructionSite> {
        let mut v = self
            .inner
            .room()
            .expect("room is not visible to you")
            .find(screeps::constants::find::CONSTRUCTION_SITES);

        v.sort_by(|a, b| {
            self.inner
                .pos()
                .get_range_to(a)
                .cmp(&self.inner.pos().get_range_to(b))
        });

        v
    }

    fn get_maintainable_structures(&self) -> Vec<Structure> {
        let mut v: Vec<Structure> = self
            .inner
            .room()
            .expect("room is not visible to you")
            .find(screeps::constants::find::STRUCTURES)
            .into_iter()
            .filter(|s| {
                let typ = s.structure_type();
                (typ == StructureType::Extension || typ == StructureType::Spawn)
                    && s.as_has_store()
                        .unwrap()
                        .store_free_capacity(Some(ResourceType::Energy))
                        != 0
            })
            .collect();

        v.sort_by(|a, b| {
            self.inner
                .pos()
                .get_range_to(a)
                .cmp(&self.inner.pos().get_range_to(b))
        });

        v
    }

    fn get_repairable_structures(&self) -> Vec<Structure> {
        let mut v = self
            .inner
            .room()
            .expect("room is not visible to you")
            .find(find::STRUCTURES)
            .into_iter()
            .filter(|s| {
                if let Some(a) = s.as_attackable() {
                    let hits = a.hits();
                    if hits != 0
                        && hits
                            < self.inner.store_capacity(Some(ResourceType::Energy))
                                * constants::MAX_REPAIR_MULTIPLIER
                        && hits < a.hits_max()
                    {
                        return true;
                    }
                }
                false
            })
            .collect::<Vec<_>>();

        v.sort_by(|a, b| {
            a.as_attackable()
                .unwrap()
                .hits()
                .cmp(&b.as_attackable().unwrap().hits())
        });

        v
    }

    fn reassing_harvest_role(&self) {
        if !self.get_repairable_structures().is_empty() {
            debug!("Switching to reparing since there are repairable structures");
            self.enable_repair()
        } else if !self.get_maintainable_structures().is_empty() {
            debug!("Switching to maintaining since there are maintainable structures");
            self.enable_maintaining()
        } else if !screeps::game::construction_sites::values().is_empty() {
            debug!("Switching to building");
            self.enable_building()
        } else {
            debug!("Switching to upgrading since there are no construction sites");
            self.enable_upgrading()
        }
    }

    fn harvest(&self) -> Result<()> {
        assert_eq!(self.role, Role::Harvesting);

        debug!("Running harvest");

        if let Ok(ttl) = self.inner.ticks_to_live() {
            if ttl < 50 {
                debug!("About to die, switching to other mode!");

                self.reassing_harvest_role();
            }
        }

        for source in &self
            .inner
            .room()
            .expect("room is not visible to you")
            .find(find::SOURCES)
        {
            let r = self.inner.harvest(source);
            let con = match r {
                ReturnCode::NotInRange => {
                    debug!("Not in range for harvest, moving");
                    self.move_to(source)
                }
                ReturnCode::Ok => {
                    debug!("harvesting...");
                    if self.inner.store_free_capacity(None) == 0 {
                        debug!("Full, switching to other mode!");

                        self.reassing_harvest_role();
                    }
                    Ok(false)
                }
                _ => Err(Error::Harvest(r)),
            }?;

            // Only try the next source if there's no path to this source
            if !con {
                break;
            }
        }

        Ok(())
    }

    fn move_to<T: ?Sized + HasPosition>(&self, target: &T) -> Result<bool> {
        let r = self.inner.move_to(target);
        match r {
            ReturnCode::Ok => Ok(false),
            ReturnCode::Tired => {
                debug!("Didn't move because tired!");
                Ok(false)
            }
            ReturnCode::NoPath => Ok(true),
            _ => Err(Error::Move(r)),
        }
    }

    fn maintain(&self) -> Result<()> {
        if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("No energy left; switching to harvesting");
            self.enable_harvesting();
        } else if let Some(target) = self.get_maintainable_structures().first() {
            let r = self
                .inner
                .transfer_all(target.as_transferable().unwrap(), ResourceType::Energy);
            match r {
                // FIXME: If we can't find a path to the first structure we should try the next one
                ReturnCode::NotInRange => self.move_to(target),
                ReturnCode::Ok => Ok(false),
                _ => Err(Error::Maintain(r)),
            }?;
        } else {
            debug!("No target left; switching to harvesting");
            self.enable_harvesting();
        }

        Ok(())
    }

    fn repair(&self) -> Result<()> {
        assert_eq!(self.role, Role::Repairing);

        debug!("Running repair");

        if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("No energy left; switching to harvesting");
            self.enable_harvesting();
        } else if let Some(c) = self.get_repairable_structures().first() {
            let r = self.inner.repair(c);
            match r {
                // FIXME: Handle not being able to reach it
                ReturnCode::NotInRange => self.move_to(c),
                ReturnCode::Ok => Ok(false),
                _ => Err(Error::Repair(r)),
            }?;
        } else {
            debug!("No target left; switching to harvesting");
            self.enable_harvesting();
        }

        Ok(())
    }

    fn upgrade(&self) -> Result<()> {
        assert_eq!(self.role, Role::Upgrading);

        debug!("Running upgrade");

        if let Some(c) = self
            .inner
            .room()
            .expect("room is not visible to you")
            .controller()
        {
            let r = self.inner.upgrade_controller(&c);
            match r {
                ReturnCode::NotInRange => self.move_to(&c),
                ReturnCode::NotEnough => {
                    debug!("No energy left; switching to harvesting");
                    self.enable_harvesting();
                    Ok(false)
                }
                ReturnCode::Ok => Ok(false),
                _ => Err(Error::Upgrade(r)),
            }?;
        } else {
            return Err(Error::NoController());
        }

        Ok(())
    }
}

pub fn work() -> Result<()> {
    debug!("running creeps");

    for s_creep in screeps::game::creeps::values() {
        Creep::from_creep(s_creep).do_action()?
    }

    Ok(())
}
