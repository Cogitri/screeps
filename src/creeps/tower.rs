use super::{Job, JobOffer};
use crate::constants;
use log::*;
use screeps::{prelude::*, Attackable, ResourceType, ReturnCode, StructureTower};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Couldn't attack: `{0:?}`")]
    Attack(ReturnCode),
    #[error("Couldn't repair: `{0:?}`")]
    Repair(ReturnCode),
}

type Result<T> = std::result::Result<T, crate::creeps::tower::Error>;

pub struct Tower {
    current_job: Option<Job>,
    inner: StructureTower,
}

impl Tower {
    pub fn execute_job(&self, job: &Job) -> Result<bool> {
        // FIXME: Allow Health
        Ok(match job {
            Job::Attack(_) => self.attack(&job)?,
            Job::Repair(_) => self.repair(&job)?,
            _ => unimplemented!(),
        })
    }

    pub fn get_id(&self) -> String {
        self.inner.id().to_string()
    }

    pub fn from_tower(inner: StructureTower) -> Self {
        Self {
            current_job: None,
            inner,
        }
    }

    pub fn set_tower(&mut self, tower: StructureTower) {
        self.inner = tower;
    }

    pub fn select_job(&mut self, jobs: &mut [JobOffer]) -> Result<()> {
        debug!(
            "tower {} has {} jobs to choose from",
            self.inner.id(),
            jobs.len()
        );

        if let Some(job) = &self.current_job {
            debug!("Keeping job");
            if !self.execute_job(job)? {
                self.current_job = None;
            }
        } else {
            debug!("Changing job");
            let pos = self.inner.pos();

            if let Some(offer) = jobs
                .iter_mut()
                .filter(|a| a.available_places != 0)
                .filter(|a| match &a.job {
                    Job::Attack(_) => true,
                    Job::Repair(c) => {
                        debug!(
                            "Repair: {} hits vs {} capacity",
                            c.as_attackable().unwrap().hits(),
                            self.inner.store_capacity(Some(ResourceType::Energy))
                                * constants::MAX_REPAIR_MULTIPLIER
                        );
                        return c.as_attackable().unwrap().hits()
                            < self.inner.store_capacity(Some(ResourceType::Energy))
                                * constants::MAX_REPAIR_MULTIPLIER;
                    }
                    _ => false,
                })
                .min_by(|a, b| {
                    (a.job.priority() * a.job.get_range_to(pos))
                        .cmp(&(b.job.priority() * b.job.get_range_to(pos)))
                })
            {
                offer.available_places -= 1;

                if self.execute_job(&offer.job)? {
                    self.current_job = Some(offer.job.clone());
                }
            } else {
                debug!("No job available for tower {}", self.inner.id());
            }
        }

        Ok(())
    }

    fn attack(&self, job: &Job) -> Result<bool> {
        debug!("Running attack");

        if let Some(creep) = job.get_creep() {
            let r = self.inner.attack(&creep);
            match r {
                ReturnCode::Ok => {
                    if job.get_creep().map(|c| c.hits()).unwrap_or(0) == 0 {
                        info!("Killed enemy, abandoning job!");
                        Ok(false)
                    } else {
                        Ok(true)
                    }
                }
                _ => Err(Error::Attack(r)),
            }
        } else {
            Ok(false)
        }
    }

    fn repair(&self, job: &Job) -> Result<bool> {
        debug!("Running repair");

        if let Some(target) = job.get_structure() {
            let attackable = target.as_attackable().unwrap();
            if attackable.hits() == attackable.hits_max()
                || attackable.hits()
                    > self.inner.store_capacity(Some(ResourceType::Energy))
                        * constants::MAX_REPAIR_MULTIPLIER
            {
                return Ok(false);
            }

            let r = self.inner.repair(&target);
            match r {
                ReturnCode::Ok => Ok(true),
                ReturnCode::NotEnough => Ok(true),
                _ => Err(Error::Repair(r)),
            }
        } else {
            Ok(false)
        }
    }
}
