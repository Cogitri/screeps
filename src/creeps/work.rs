use super::{Job, JobOffer};
use crate::core::constants;
use log::*;
use screeps::{prelude::*, LineDrawStyle, MoveToOptions, PolyStyle, ResourceType, ReturnCode};
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
            Job::Attack(_) | Job::Heal(_) => unimplemented!(),
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

    pub fn get_name(&self) -> String {
        self.inner.name()
    }

    pub fn set_creep(&mut self, creep: screeps::Creep) {
        self.inner = creep;
    }

    pub fn select_job(&mut self, jobs: &mut [JobOffer]) -> Result<()> {
        debug!(
            "creep {} has {} jobs to choose from",
            self.inner.name(),
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
                .filter(|a| {
                    if matches!(a.job, Job::Attack(_)) {
                        false
                    } else if matches!(a.job, Job::Heal(_)) {
                        false
                    } else {
                        true
                    }
                })
                .filter(|a| a.available_places != 0)
                .filter(|a| {
                    if let Job::Repair(c) = &a.job {
                        debug!(
                            "Repair: {} hits vs {} capacity",
                            c.as_attackable().unwrap().hits(),
                            self.inner.store_capacity(Some(ResourceType::Energy))
                                * constants::MAX_REPAIR_MULTIPLIER
                        );
                        return c.as_attackable().unwrap().hits()
                            < self.inner.store_capacity(Some(ResourceType::Energy))
                                * constants::MAX_REPAIR_MULTIPLIER;
                    } else if self.inner.store_free_capacity(Some(ResourceType::Energy)) == 0
                        || self.inner.ticks_to_live().unwrap_or(0) < 50
                    {
                        if let Job::Harvest(_) = a.job {
                            debug!("Rejecting job because harvest and no free energy storage");
                            return false;
                        }
                    } else if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
                        if let Job::Harvest(_) = a.job {
                            return true;
                        } else {
                            debug!("Rejecting job because no energy and not harvest job");
                            return false;
                        }
                    }

                    true
                })
                .min_by(|a, b| {
                    (a.job.priority() * a.job.get_range_to(pos))
                        .cmp(&(b.job.priority() * b.job.get_range_to(pos)))
                })
            {
                offer.available_places -= 1;

                match &offer.job {
                    Job::Attack(_) | Job::Heal(_) => unimplemented!(),
                    // FIXME: Check if one creep is enough for building
                    Job::Build(_) => self.inner.say("building", false),
                    Job::Harvest(_) => self.inner.say("harvesting", false),
                    Job::Maintain(target) => {
                        // Don't move multiple creeps to maintainance when one creep can fill the spot
                        if offer.available_places != 0
                            && i64::from(
                                target
                                    .as_has_store()
                                    .unwrap()
                                    .store_free_capacity(Some(ResourceType::Energy)),
                            ) <= i64::from(
                                self.inner.store_used_capacity(Some(ResourceType::Energy)),
                            )
                        {
                            offer.available_places = 0;
                        }
                        self.inner.say("maintaining", false)
                    }
                    // FIXME: Check if one creep is enough for repairing
                    Job::Repair(_) => self.inner.say("repairing", false),
                    Job::Upgrade(_) => self.inner.say("upgrading", false),
                };

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

        if let Some(site) = job.get_construction_site() {
            if site.progress() == site.progress_total() {
                debug!("Building done, abandoning build job!");
                return Ok(false);
            }

            let r = self.inner.build(&site);

            if r == ReturnCode::NotInRange {
                self.move_to(&site)?;
            } else if r != ReturnCode::Ok {
                return Err(Error::Build(r));
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn harvest(&self, job: &Job) -> Result<bool> {
        debug!("Running harvest");

        debug!("{} free capacity", self.inner.store_free_capacity(None));

        if self.inner.store_free_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("Energy storage full, abandoning harvest job!");
            return Ok(false);
        }

        if let Some(source) = job.get_source() {
            let r = self.inner.harvest(&source);
            match r {
                ReturnCode::NotInRange => {
                    debug!("Not in range for harvest, moving");
                    Ok(!self.move_to(&source)?)
                }
                ReturnCode::Ok => {
                    debug!("harvesting...");
                    Ok(true)
                }
                ReturnCode::NotEnough => {
                    debug!("Energy source is empty, abandoning job!");
                    Ok(false)
                }
                _ => Err(Error::Harvest(r)),
            }
        } else {
            Ok(false)
        }
    }

    fn move_to<T: ?Sized + HasPosition>(&self, target: &T) -> Result<bool> {
        let r = self.inner.move_to_with_options(
            target,
            MoveToOptions::new()
                .visualize_path_style(PolyStyle::default().line_style(LineDrawStyle::Dashed)),
        );
        match r {
            ReturnCode::Ok => {
                debug!("Ok, moved");
                Ok(false)
            }
            ReturnCode::Tired => {
                debug!("Didn't move because tired!");
                Ok(false)
            }
            ReturnCode::NoPath => {
                debug!("No path!");
                Ok(true)
            }
            _ => Err(Error::Move(r)),
        }
    }

    fn maintain(&self, job: &Job) -> Result<bool> {
        debug!("Running maintaince");

        if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("No energy left, abandoning maintain job!");
            return Ok(false);
        }

        if let Some(target) = job.get_structure() {
            if target
                .as_has_store()
                .unwrap()
                .store_free_capacity(Some(ResourceType::Energy))
                == 0
            {
                debug!("Store is full, abandoning maintain job");
                return Ok(false);
            }

            if self.inner.pos().is_near_to(&target) {
                debug!("Transferring");
                let r = self
                    .inner
                    .transfer_all(target.as_transferable().unwrap(), ResourceType::Energy);
                match r {
                    ReturnCode::NotInRange => {
                        debug!("Not in range");
                        Ok(!self.move_to(&target)?)
                    }
                    ReturnCode::Ok => {
                        debug!("Transferred");
                        Ok(false)
                    }
                    //ReturnCode::Full => {
                    //    debug!("Full");
                    //    Ok(false)
                    //}
                    _ => Err(Error::Maintain(r)),
                }
            } else {
                self.inner.move_to(&target);
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    fn repair(&self, job: &Job) -> Result<bool> {
        debug!("Running repair");

        if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("No energy left, abandoning repair job!");
            return Ok(false);
        }

        if let Some(target) = job.get_structure() {
            let attackable = target.as_attackable().unwrap();
            if attackable.hits() == attackable.hits_max() {
                return Ok(false);
            }

            let r = self.inner.repair(&target);
            match r {
                // FIXME: Handle not being able to reach it
                ReturnCode::NotInRange => self.move_to(&target),
                ReturnCode::Ok => Ok(false),
                _ => Err(Error::Repair(r)),
            }?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn upgrade(&self, job: &Job) -> Result<bool> {
        debug!("Running upgrade");

        if self.inner.store_used_capacity(Some(ResourceType::Energy)) == 0 {
            debug!("No energy left, abandoning upgrade job!");
            return Ok(false);
        }

        let c = job.get_structure_controller();
        let r = self.inner.upgrade_controller(&c);
        match r {
            ReturnCode::NotInRange => self.move_to(&c),
            ReturnCode::NotEnough => Ok(false),
            ReturnCode::Ok => Ok(false),
            _ => Err(Error::Upgrade(r)),
        }?;

        Ok(true)
    }
}
