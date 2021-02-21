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

impl Creep {
    pub fn do_action(&self) -> Result<(), Error> {
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
        let role = if memory.bool("building") {
            Role::Building
        } else if memory.bool("harvesting") {
            Role::Harvesting
        } else if memory.bool("maintainer") {
            Role::Maintainer
        } else if memory.bool("upgrading") {
            Role::Upgrading
        } else {
            warn!("Unknown role, falling back to harvesting!");
            memory.set("harvesting", true);
            Role::Harvesting
        };

        Self { inner, role }
    }

    fn enable_building(&self) {
        assert_eq!(self.inner.say("Building!", false), ReturnCode::Ok);

        self.inner.memory().set("building", true);
        self.inner.memory().set("harvesting", false);
        self.inner.memory().set("maintaining", false);
        self.inner.memory().set("upgrading", false);
    }

    fn enable_maintaining(&self) {
        assert_eq!(self.inner.say("Maintaining!", false), ReturnCode::Ok);

        self.inner.memory().set("building", false);
        self.inner.memory().set("harvesting", false);
        self.inner.memory().set("maintaining", true);
        self.inner.memory().set("upgrading", false);
    }

    fn enable_harvesting(&self) {
        assert_eq!(self.inner.say("Harvesting!", false), ReturnCode::Ok);

        self.inner.memory().set("building", false);
        self.inner.memory().set("harvesting", true);
        self.inner.memory().set("maintaining", false);
        self.inner.memory().set("upgrading", false);
    }

    fn enable_upgrading(&self) {
        assert_eq!(self.inner.say("Upgrading!", false), ReturnCode::Ok);

        self.inner.memory().set("building", false);
        self.inner.memory().set("harvesting", false);
        self.inner.memory().set("maintaining", false);
        self.inner.memory().set("upgrading", true);
    }

    fn build(&self) -> Result<(), Error> {
        assert_eq!(self.role, Role::Building);

        debug!("Running build");

        if let Some(c) = screeps::game::construction_sites::values().first() {
            let r = self.inner.build(&c);
            if r == ReturnCode::NotInRange {
                self.inner.move_to(c);
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

    fn harvest(&self) -> Result<(), Error> {
        assert_eq!(self.role, Role::Harvesting);

        debug!("Running harvest");

        let source = &self
            .inner
            .room()
            .expect("room is not visible to you")
            .find(find::SOURCES)[0];
        if self.inner.pos().is_near_to(source) {
            let r = self.inner.harvest(source);
            if r == ReturnCode::Ok {
                if self.inner.store_free_capacity(Some(ResourceType::Energy)) == 0 {
                    debug!("Full, switching to other mode!");

                    if !self.get_maintainable_structures().is_empty() {
                        debug!("Switching to maintaining since there are maintainable structures");
                        self.enable_maintaining()
                    } else if screeps::game::construction_sites::values().is_empty() {
                        debug!("Switching to upgrading since there are no construction sites");
                        self.enable_upgrading()
                    } else {
                        debug!("Switching to building");
                        self.enable_building()
                    }
                }
            } else {
                return Err(Error::Harvest(r));
            }
        } else {
            self.inner.move_to(source);
        }

        Ok(())
    }

    fn maintain(&self) -> Result<(), Error> {
        if let Some(target) = self.get_maintainable_structures().first() {
            let r = self
                .inner
                .transfer_all(target.as_transferable().unwrap(), ResourceType::Energy);
            if r == ReturnCode::NotInRange {
                self.inner.move_to(target);
            } else if r == ReturnCode::Ok {
                if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
                    debug!("No energy left; switching to harvesting");
                    self.enable_harvesting();
                }
            } else {
                return Err(Error::Maintain(r));
            }
        }

        Ok(())
    }

    fn upgrade(&self) -> Result<(), Error> {
        assert_eq!(self.role, Role::Upgrading);

        debug!("Running upgrade");

        if let Some(c) = self
            .inner
            .room()
            .expect("room is not visible to you")
            .controller()
        {
            let r = self.inner.upgrade_controller(&c);
            if r == ReturnCode::NotInRange {
                self.inner.move_to(&c);
            } else if r == ReturnCode::Ok {
                if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
                    debug!("No energy left; switching to harvesting");
                    self.enable_harvesting();
                }
            } else {
                return Err(Error::Upgrade(r));
            }
        } else {
            return Err(Error::NoController());
        }

        Ok(())
    }
}

pub fn work() -> Result<(), Error> {
    debug!("running creeps");

    for s_creep in screeps::game::creeps::values() {
        Creep::from_creep(s_creep).do_action()?
    }

    Ok(())
}
