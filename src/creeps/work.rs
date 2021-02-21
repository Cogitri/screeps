use log::*;
use screeps::{find, prelude::*, ResourceType, ReturnCode};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Couldn't build: `{0:?}`")]
    Build(ReturnCode),
    #[error("Couldn't harvest: `{0:?}`")]
    Harvest(ReturnCode),
    #[error("Creep has no controller!")]
    NoController(),
    #[error("Couldn't upgrade: `{0:?}`")]
    Upgrade(ReturnCode),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Role {
    Building,
    Harvesting,
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
            Role::Upgrading => self.upgrade(),
        }
    }

    pub fn from_creep(inner: screeps::Creep) -> Self {
        let memory = inner.memory();
        let role = if memory.bool("building") {
            Role::Building
        } else if memory.bool("harvesting") {
            Role::Harvesting
        } else if memory.bool("upgrading") {
            Role::Upgrading
        } else {
            unimplemented!()
        };

        Self { inner, role }
    }

    fn enable_building(&self) {
        self.inner.memory().set("building", true);
        self.inner.memory().set("harvesting", false);
        self.inner.memory().set("upgrading", false);
    }

    fn enable_harvesting(&self) {
        self.inner.memory().set("building", false);
        self.inner.memory().set("harvesting", true);
        self.inner.memory().set("upgrading", false);
    }

    fn enable_upgrading(&self) {
        self.inner.memory().set("building", false);
        self.inner.memory().set("harvesting", false);
        self.inner.memory().set("upgrading", true);
    }

    fn build(&self) -> Result<(), Error> {
        assert_eq!(self.role, Role::Building);

        if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
            self.enable_harvesting();
        }

        if let Some(c) = screeps::game::construction_sites::values().first() {
            let r = self.inner.build(&c);
            if r == ReturnCode::NotInRange {
                self.inner.move_to(c);
            } else if r == ReturnCode::Ok {
                if self.inner.store_used_capacity(None) == 0 {
                    self.enable_harvesting();
                }
            } else {
                return Err(Error::Upgrade(r));
            }
        } else {
            self.enable_harvesting()
        }

        Ok(())
    }

    fn harvest(&self) -> Result<(), Error> {
        assert_eq!(self.role, Role::Harvesting);

        let source = &self
            .inner
            .room()
            .expect("room is not visible to you")
            .find(find::SOURCES)[0];
        if self.inner.pos().is_near_to(source) {
            let r = self.inner.harvest(source);
            if r == ReturnCode::Ok {
                if self.inner.store_free_capacity(Some(ResourceType::Energy)) == 0 {
                    if screeps::game::construction_sites::values().is_empty() {
                        self.enable_upgrading()
                    } else {
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

    fn upgrade(&self) -> Result<(), Error> {
        assert_eq!(self.role, Role::Upgrading);

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
