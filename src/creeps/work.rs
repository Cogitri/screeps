use super::{Job, JobOffer};
use log::*;
use screeps::{prelude::*, ResourceType, ReturnCode};
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
    #[error("Couldn't repair: `{0:?}`")]
    Repair(ReturnCode),
    #[error("Couldn't upgrade: `{0:?}`")]
    Upgrade(ReturnCode),
}

pub struct Creep {
    current_job: Option<Job>,
    inner: screeps::Creep,
}

type Result<T> = std::result::Result<T, crate::creeps::work::Error>;

impl Creep {
    pub fn execute_job(&self, job: &Job) -> Result<bool> {
        Ok(match job {
            Job::Build(_) => self.build(&job)?,
            Job::Harvest(_) => self.harvest(&job)?,
            Job::Maintain(_) => self.maintain(&job)?,
            Job::Repair(_) => self.repair(&job)?,
            Job::Upgrade(_) => self.upgrade(&job)?,
        })
    }

    pub fn from_creep(inner: screeps::Creep) -> Self {
        Self {
            current_job: None,
            inner,
        }
    }

    pub fn select_job(&mut self, jobs: &mut [JobOffer]) -> Result<()> {
        if let Some(job) = &self.current_job {
            if !self.execute_job(job)? {
                self.current_job = None;
            }
        } else {
            let pos = self.inner.pos();

            if let Some(offer) = jobs
                .iter_mut()
                .filter(|a| !a.taken)
                .filter(|a| {
                    if self.inner.store_free_capacity(Some(ResourceType::Energy)) == 0
                        || self.inner.ticks_to_live().unwrap_or(0) < 50
                    {
                        if let Job::Harvest(_) = a.job {
                            false
                        } else {
                            true
                        }
                    } else if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
                        if let Job::Harvest(_) = a.job {
                            true
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                })
                .min_by(|a, b| {
                    (a.job.priority() * a.job.get_range_to(pos))
                        .cmp(&(b.job.priority() * b.job.get_range_to(pos)))
                })
            {
                offer.taken = true;

                if self.execute_job(&offer.job)? {
                    self.current_job = Some(offer.job.clone());
                }
            } else {
                warn!("No job available for creep {}", self.inner.name());
            }
        }

        Ok(())
    }

    fn build(&self, job: &Job) -> Result<bool> {
        debug!("Running build");

        if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("No energy left, abandoning build job!");
            return Ok(false);
        }

        let site = job.get_construction_site();

        let r = self.inner.build(site);

        if r == ReturnCode::NotInRange {
            self.move_to(site)?;
        } else if r != ReturnCode::Ok {
            return Err(Error::Build(r));
        }

        Ok(true)
    }

    fn harvest(&self, job: &Job) -> Result<bool> {
        debug!("Running harvest");

        if self.inner.store_free_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("Energy storage full, abandoning harvest job!");
            return Ok(false);
        }

        let source = job.get_source();
        let r = self.inner.harvest(source);
        match r {
            ReturnCode::NotInRange => {
                debug!("Not in range for harvest, moving");
                self.move_to(source)
            }
            ReturnCode::Ok => {
                debug!("harvesting...");
                Ok(false)
            }
            _ => Err(Error::Harvest(r)),
        }?;

        Ok(true)
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

    fn maintain(&self, job: &Job) -> Result<bool> {
        debug!("Running maintaince");

        if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("No energy left, abandoning maintain job!");
            return Ok(false);
        }

        let target = job.get_structure();
        let r = self
            .inner
            .transfer_all(target.as_transferable().unwrap(), ResourceType::Energy);
        match r {
            // FIXME: If we can't find a path to the first structure we should try the next one
            ReturnCode::NotInRange => self.move_to(target),
            ReturnCode::Ok => Ok(false),
            _ => Err(Error::Maintain(r)),
        }?;

        Ok(true)
    }

    fn repair(&self, job: &Job) -> Result<bool> {
        debug!("Running repair");

        if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("No energy left, abandoning repair job!");
            return Ok(false);
        }

        let target = job.get_structure();
        let r = self.inner.repair(target);
        match r {
            // FIXME: Handle not being able to reach it
            ReturnCode::NotInRange => self.move_to(target),
            ReturnCode::Ok => Ok(false),
            _ => Err(Error::Repair(r)),
        }?;

        Ok(true)
    }

    fn upgrade(&self, job: &Job) -> Result<bool> {
        debug!("Running upgrade");

        if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("No energy left, abandoning upgrade job!");
            return Ok(false);
        }

        let c = job.get_structure_controller();
        let r = self.inner.upgrade_controller(c);
        match r {
            ReturnCode::NotInRange => self.move_to(c),
            ReturnCode::NotEnough => Ok(false),
            ReturnCode::Ok => Ok(false),
            _ => Err(Error::Upgrade(r)),
        }?;

        Ok(true)
    }
}
