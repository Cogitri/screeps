use crate::core::constants;
use log::*;
use screeps::{constants::StructureType, find, prelude::*, ResourceType, ReturnCode, Structure};
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
    #[error("Couldn't upgrade: `{0:?}`")]
    Upgrade(ReturnCode),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Role {
    Building,
    Harvesting,
    Maintainer,
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
        self.inner.memory().set(constants::ROLE_UPGRADING, false);
    }

    fn enable_maintaining(&self) {
        assert_eq!(self.inner.say("Maintaining!", false), ReturnCode::Ok);

        self.inner.memory().set(constants::ROLE_BUILDING, false);
        self.inner.memory().set(constants::ROLE_HARVESTING, false);
        self.inner.memory().set(constants::ROLE_MAINTAINING, true);
        self.inner.memory().set(constants::ROLE_UPGRADING, false);
    }

    fn enable_harvesting(&self) {
        assert_eq!(self.inner.say("Harvesting!", false), ReturnCode::Ok);

        self.inner.memory().set(constants::ROLE_BUILDING, false);
        self.inner.memory().set(constants::ROLE_HARVESTING, true);
        self.inner.memory().set(constants::ROLE_MAINTAINING, false);
        self.inner.memory().set(constants::ROLE_UPGRADING, false);
    }

    fn enable_upgrading(&self) {
        assert_eq!(self.inner.say("Upgrading!", false), ReturnCode::Ok);

        self.inner.memory().set(constants::ROLE_BUILDING, false);
        self.inner.memory().set(constants::ROLE_HARVESTING, false);
        self.inner.memory().set(constants::ROLE_MAINTAINING, false);
        self.inner.memory().set(constants::ROLE_UPGRADING, true);
    }

    fn build(&self) -> Result<()> {
        assert_eq!(self.role, Role::Building);

        debug!("Running build");

        if let Some(c) = screeps::game::construction_sites::values().first() {
            let r = self.inner.build(&c);
            if r == ReturnCode::NotInRange {
                let r = self.inner.move_to(c);
                if r != ReturnCode::Ok {
                    return Err(Error::Move(r));
                }
            } else if r == ReturnCode::Ok {
                if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
                    debug!("No energy left; switching to harvesting");
                    self.enable_harvesting();
                }
            } else {
                return Err(Error::Build(r));
            }
        } else {
            debug!("No construction sites, switching to upgrading");
            self.enable_upgrading()
        }

        Ok(())
    }

    fn get_maintainable_structures(&self) -> Vec<Structure> {
        self.inner
            .room()
            .unwrap()
            .find(screeps::constants::find::STRUCTURES)
            .into_iter()
            .filter(|s| {
                let typ = s.structure_type();
                (typ == StructureType::Extension || typ == StructureType::Spawn)
                    && s.as_has_store()
                        .unwrap()
                        .store_free_capacity(Some(ResourceType::Energy))
                        > 50
            })
            .collect::<Vec<_>>()
    }

    fn reassing_harvest_role(&self) {
        if !self.get_maintainable_structures().is_empty() {
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
            match r {
                ReturnCode::NotInRange => {
                    debug!("Not in range for harvest, moving");
                    let r = self.inner.move_to(source);
                    match r {
                        ReturnCode::Ok | ReturnCode::NoPath => Ok(()),
                        _ => Err(Error::Move(r)),
                    }
                }
                ReturnCode::Ok => {
                    debug!("harvesting...");
                    if self.inner.store_free_capacity(None) == 0 {
                        debug!("Full, switching to other mode!");

                        self.reassing_harvest_role();
                    }
                    Ok(())
                }
                _ => Err(Error::Harvest(r)),
            }?
        }

        Ok(())
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
                ReturnCode::NotInRange => {
                    let r = self.inner.move_to(target);
                    if r == ReturnCode::Ok {
                        Ok(())
                    } else {
                        Err(Error::Move(r))
                    }
                }
                ReturnCode::Ok => Ok(()),
                _ => Err(Error::Maintain(r)),
            }?
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
                ReturnCode::NotInRange => {
                    let r = self.inner.move_to(&c);
                    if r == ReturnCode::Ok {
                        Ok(())
                    } else {
                        Err(Error::Move(r))
                    }
                }
                ReturnCode::NotEnough => {
                    debug!("No energy left; switching to harvesting");
                    self.enable_harvesting();
                    Ok(())
                }
                ReturnCode::Ok => Ok(()),
                _ => Err(Error::Upgrade(r)),
            }?
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
